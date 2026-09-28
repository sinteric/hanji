//! The docx engine: splits `word/document.xml` into model text and remainder
//! entries at the XML level, and puts them back. Every other part is copied
//! through. No I/O: package bytes in, package bytes out.
//!
//! rdocx is not used on the import/export path: its model is typed
//! (`CT_P`, `CT_RPr`, hyperlinks as run spans), so the original `pPr`/`rPr`
//! fragments, bookmarks and wrappers are not available as the XML the
//! remainder must store.

pub mod export;
pub mod import;
pub mod ooxml;
pub mod package;
pub mod safety;
pub mod styles;
pub mod xml;

use std::collections::HashMap;

use hanji_core::{Capabilities, Engine, EngineError, ImportOptions, ImportReport, Imported, Remainder};
use hanji_format::{self as fmt, FrontMatter};
use serde::{Deserialize, Serialize};

use crate::ooxml::{DOC_PART, W_NS, W_NS_STRICT};

pub struct DocxEngine;

/// The parts of `document.xml` around the body, stored once.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct DocShell {
    prolog: String,
    epilog: String,
    /// `<w:document …>` start tag.
    root_open: String,
    root_name: String,
    before_body: String,
    /// `<w:body …>` start tag.
    body_open: String,
    after_body: String,
}

fn open_tag(e: &xml::Element) -> String {
    let s = e.shell().to_xml();
    format!("{}>", &s[..s.len() - 2])
}

impl DocxEngine {
    /// Model text for resolved blocks and a remainder.
    pub fn text_of(blocks: &[hanji_core::Block], rem: &Remainder, template: Option<&str>) -> String {
        let front = FrontMatter::document("docx", template);
        let d = hanji_core::model::unresolve(blocks, &rem.styles, front, &|id| {
            rem.keep(id).cloned().expect("keep in remainder")
        });
        fmt::serialize(&d)
    }

    /// Model blocks of an import (for tests and tools).
    pub fn split(
        package: &[u8],
        opts: &ImportOptions,
    ) -> Result<(Vec<hanji_core::Block>, Remainder, ImportReport, import::Stats), EngineError> {
        let mut parts = package::read(package).map_err(EngineError::Package)?;
        let doc_data = package::get(&parts, DOC_PART).ok_or_else(|| EngineError::Package(format!("no {DOC_PART}")))?;
        let mut doc = xml::parse(doc_data).map_err(|e| EngineError::Package(format!("{DOC_PART}: {e}")))?;
        let mut report = ImportReport::default();
        if opts.neutralise {
            safety::neutralise(&mut parts, &mut doc.root, &mut report).map_err(EngineError::Package)?;
        }
        safety::surface(&parts, &doc.root, &mut report);
        let root = &doc.root;
        let w_prefix = root.attrs.iter().find(|a| a.1 == W_NS || a.1 == W_NS_STRICT).map(|a| a.0.as_str());
        if w_prefix != Some("xmlns:w") || root.name != "w:document" {
            return Err(EngineError::Package(
                "the WordprocessingML namespace must use the prefix w: (other prefixes are not supported yet)".into(),
            ));
        }
        let scope = xml::scope_of(root);
        let styles = styles::read(package::get(&parts, "word/styles.xml"));
        let notes = notes(&parts);
        let body_at = root.children.iter().position(|n| matches!(n, xml::Node::El(e) if e.is("w:body")));
        let Some(body_at) = body_at else { return Err(EngineError::Package("document.xml has no w:body".into())) };
        let xml::Node::El(body) = &root.children[body_at] else { unreachable!() };
        let split = import::Importer::new(&scope, styles, &notes)
            .body(body)
            .map_err(|e| EngineError::Package(format!("{DOC_PART}: {e}")))?;
        let shell = DocShell {
            prolog: doc.prolog.clone(),
            epilog: doc.epilog.clone(),
            root_open: open_tag(root),
            root_name: root.name.clone(),
            before_body: xml::write_nodes(&root.children[..body_at]),
            body_open: open_tag(body),
            after_body: xml::write_nodes(&root.children[body_at + 1..]),
        };
        let namespaces = root
            .attrs
            .iter()
            .filter_map(|(k, v)| {
                if k == "xmlns" {
                    Some((String::new(), v.clone()))
                } else {
                    k.strip_prefix("xmlns:").map(|p| (p.to_string(), v.clone()))
                }
            })
            .collect();
        for p in &mut parts {
            if p.name == DOC_PART {
                p.data.clear();
            }
        }
        let rem = Remainder {
            format: "docx".into(),
            namespaces,
            shell: vec![serde_json::to_string(&shell).unwrap()],
            styles: split.styles,
            entries: split.entries,
            parts,
            next_id: split.next_id,
        };
        Ok((split.blocks, rem, report, split.stats))
    }
}

/// Comment, footnote and endnote text by id, for placeholder summaries.
fn notes(parts: &[hanji_core::Part]) -> HashMap<(String, String), String> {
    let mut out = HashMap::new();
    for (part, tag, kind) in [
        ("word/comments.xml", "w:comment", "comment"),
        ("word/footnotes.xml", "w:footnote", "footnote"),
        ("word/endnotes.xml", "w:endnote", "endnote"),
    ] {
        let Some(d) = package::get(parts, part).and_then(|d| xml::parse(d).ok()) else { continue };
        for n in d.root.elements().filter(|e| e.is(tag)) {
            if let Some(id) = n.get("w:id") {
                out.insert((kind.to_string(), id), import::clip(&n.text_of(&["w:t"]), 40));
            }
        }
    }
    out
}

impl Engine for DocxEngine {
    fn format(&self) -> &'static str {
        "docx"
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::default()
    }

    fn import(&self, package: &[u8], opts: &ImportOptions) -> Result<Imported, EngineError> {
        let (blocks, remainder, report, _) = Self::split(package, opts)?;
        let text = Self::text_of(&blocks, &remainder, opts.template.as_deref());
        Ok(Imported { text, remainder, report })
    }

    fn export(&self, text: &str, rem: &Remainder) -> Result<Vec<u8>, EngineError> {
        let (_, blocks) = hanji_core::model_of(text, rem, self.capabilities()).map_err(EngineError::Invalid)?;
        let doc = export_document(&blocks, rem)?;
        let mut parts = rem.parts.clone();
        for p in &mut parts {
            if p.name == DOC_PART {
                p.data = doc.clone();
            }
        }
        package::write(&parts).map_err(EngineError::Package)
    }
}

/// `document.xml` for resolved blocks placed against `rem`.
pub fn export_document(blocks: &[hanji_core::Block], rem: &Remainder) -> Result<Vec<u8>, EngineError> {
    let shell: DocShell = serde_json::from_str(
        rem.shell.first().ok_or_else(|| EngineError::Package("remainder has no document shell".into()))?,
    )
    .map_err(|e| EngineError::Package(e.to_string()))?;
    let ex = export::Exporter::new(&rem.styles, &rem.entries);
    let body = ex.body(blocks).map_err(EngineError::Refused)?;
    let mut s = shell.prolog.clone();
    s.push_str(&shell.root_open);
    s.push_str(&shell.before_body);
    s.push_str(&shell.body_open);
    s.push_str(&xml::write_nodes(&body));
    s.push_str("</w:body>");
    s.push_str(&shell.after_body);
    s.push_str(&format!("</{}>", shell.root_name));
    s.push_str(&shell.epilog);
    Ok(s.into_bytes())
}
