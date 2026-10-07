//! The docx engine: splits `word/document.xml` into model text and remainder
//! entries at the XML level, and puts them back. Every other part is copied
//! through. No I/O: package bytes in, package bytes out.
//!
//! rdocx is not used on the import/export path: its model is typed
//! (`CT_P`, `CT_RPr`, hyperlinks as run spans), so the original `pPr`/`rPr`
//! fragments, bookmarks and wrappers are not available as the XML the
//! remainder must store.

pub mod export;
pub mod format;
pub mod import;
pub mod numbering;
pub mod ooxml;
pub mod safety;
pub mod styles;
pub mod track;

pub use hanji_package::{package, xml};
pub use track::{ExportOptions, History, Reviewer};

use std::collections::HashMap;

use hanji_core::{Capabilities, Engine, EngineError, ImportOptions, ImportReport, Imported, Remainder};
use hanji_format::{self as fmt, FrontMatter};
use serde::{Deserialize, Serialize};

use crate::ooxml::{DOC_PART, W_NS, W_NS_STRICT};

pub struct DocxEngine;

/// The parts of `document.xml` around the body, stored once.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct DocShell {
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

impl DocShell {
    pub(crate) fn of(rem: &Remainder) -> Result<DocShell, EngineError> {
        let s = rem.shell.first().ok_or_else(|| EngineError::Package("remainder has no document shell".into()))?;
        serde_json::from_str(s).map_err(|e| EngineError::Package(e.to_string()))
    }

    /// `document.xml` around the body's children.
    pub(crate) fn document(&self, body: &[xml::Node]) -> String {
        let mut s = self.prolog.clone();
        s.push_str(&self.root_open);
        s.push_str(&self.before_body);
        s.push_str(&self.body_open);
        s.push_str(&xml::write_nodes(body));
        s.push_str("</w:body>");
        s.push_str(&self.after_body);
        s.push_str(&format!("</{}>", self.root_name));
        s.push_str(&self.epilog);
        s
    }
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

    /// A new revision's blocks with what its text could not show (§5.2): a
    /// paragraph that had no text, and has now without writing its own
    /// `{…}`, keeps the formatting it had, and the text shows it from now
    /// on. `rem`'s entries stop marking a paragraph whose text shows it.
    pub fn complete(blocks: &mut [hanji_core::Block], rem: &mut Remainder) {
        use hanji_core::{Block, Kind};
        use hanji_format::{styled, Cell, Inline, Props};
        if !rem.styles.formatting {
            return;
        }
        let f = export::Fmt::new(&rem.styles, format::Theme::read(&rem.parts));
        let default_id = rem.styles.default_paragraph_id().to_string();
        let mut fill = |path: Vec<usize>, style: &str, content: &Inline, props: &mut Props| {
            if !styled::shows(content) {
                return;
            }
            let Some(e) = rem.entries.iter_mut().find(|e| e.kind == Kind::Ppr && e.path == path && e.meta.unshown)
            else {
                return;
            };
            e.meta.unshown = false;
            if props.is_empty() {
                let style = if style.is_empty() {
                    let id = e.meta.style.clone().unwrap_or_else(|| default_id.clone());
                    rem.styles.paragraph_name(&id).unwrap_or(&rem.styles.default_paragraph).to_string()
                } else {
                    style.to_string()
                };
                let base = f.values.get(&style).cloned().unwrap_or_default();
                let size = base.get(hanji_format::Key::Size).and_then(|v| v.length()).unwrap_or(1000);
                if let Some(x) = e.xml.get(1).map(|x| xml::fragment(x)) {
                    *props = format::ppr_props(&x, size).only(&styled::PARA_OWN).diff(&base);
                }
            }
        };
        for (bi, b) in blocks.iter_mut().enumerate() {
            match b {
                Block::Para(p) => fill(vec![bi], &p.style, &p.content, &mut p.props),
                Block::Table(t) => {
                    for (ri, row) in t.rows.iter_mut().enumerate() {
                        for (ci, c) in row.iter_mut().enumerate() {
                            if let Cell::Text(ps) = c {
                                for (k, p) in ps.iter_mut().enumerate() {
                                    let style = p.style.clone().unwrap_or_default();
                                    fill(vec![bi, ri, ci, k], &style, &p.content, &mut p.props);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
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
        let (mut styles, default_table) = styles::read(package::get(&parts, "word/styles.xml"));
        let fmt = format::Styles::read(&parts);
        fmt.lines_into(&mut styles);
        let numbering = numbering::Numbering::read(&parts);
        // A list item that names no style is in List Paragraph, as Word
        // gives a new item.
        styles.default_item =
            numbering.list_paragraph.as_deref().and_then(|id| styles.paragraph_name(id)).map(str::to_string);
        let notes = notes(&parts);
        let body_at = root.children.iter().position(|n| matches!(n, xml::Node::El(e) if e.is("w:body")));
        let Some(body_at) = body_at else { return Err(EngineError::Package("document.xml has no w:body".into())) };
        let xml::Node::El(body) = &root.children[body_at] else { unreachable!() };
        let mut split = import::Importer::new(&scope, styles, default_table, &notes, &numbering, &fmt)
            .body(body)
            .map_err(|e| EngineError::Package(format!("{DOC_PART}: {e}")))?;
        hanji_core::model::mark_shown(&mut split.styles, &split.blocks);
        let shell = DocShell {
            prolog: doc.prolog.clone(),
            epilog: doc.epilog.clone(),
            root_open: root.open_tag(),
            root_name: root.name.clone(),
            before_body: xml::write_nodes(&root.children[..body_at]),
            body_open: body.open_tag(),
            after_body: xml::write_nodes(&root.children[body_at + 1..]),
        };
        let namespaces =
            root.attrs.iter().filter_map(|(k, v)| xml::ns_prefix(k).map(|p| (p.to_string(), v.clone()))).collect();
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
                out.insert((kind.to_string(), id), hanji_package::clip(&n.text_of(&["w:t"]), 40));
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
        Capabilities { formatting: true, table_place: true, ..Default::default() }
    }

    fn import(&self, package: &[u8], opts: &ImportOptions) -> Result<Imported, EngineError> {
        let (blocks, remainder, report, _) = Self::split(package, opts)?;
        let text = Self::text_of(&blocks, &remainder, opts.template.as_deref());
        Ok(Imported { text, remainder, report })
    }

    fn export(&self, text: &str, rem: &Remainder) -> Result<Vec<u8>, EngineError> {
        let (_, blocks) = hanji_core::model_of(text, rem, self.capabilities()).map_err(EngineError::Invalid)?;
        // The style section as the text has it.
        let mut rem = rem.clone();
        hanji_core::TextModel::restyle(&hanji_core::DocumentModel, text, &mut rem, self.capabilities());
        write_package(&rem, export_document(&blocks, &rem)?)
    }
}

/// What export writes: `word/document.xml`, `word/numbering.xml` when a
/// new list needed its own numbering, and `word/styles.xml` when the style
/// section changed a style or made one.
pub struct Exported {
    pub document: Vec<u8>,
    pub numbering: Option<Vec<u8>>,
    pub styles: Option<Vec<u8>>,
}

/// The package: `rem`'s parts, with the exported ones in place.
pub fn write_package(rem: &Remainder, out: Exported) -> Result<Vec<u8>, EngineError> {
    let mut parts = rem.parts.clone();
    for p in &mut parts {
        if p.name == DOC_PART {
            p.data = out.document.clone();
        } else if p.name == numbering::NUMBERING_PART {
            if let Some(n) = &out.numbering {
                p.data = n.clone();
            }
        } else if p.name == "word/styles.xml" {
            if let Some(n) = &out.styles {
                p.data = n.clone();
            }
        }
    }
    if let Some(data) = out.styles {
        if package::get(&parts, "word/styles.xml").is_none() {
            add_styles_part(&mut parts, data)?;
        }
    }
    package::write(&parts).map_err(EngineError::Package)
}

/// Register a newly created styles part without replacing unrelated metadata.
fn add_styles_part(parts: &mut Vec<hanji_core::Part>, data: Vec<u8>) -> Result<(), EngineError> {
    use hanji_package::opc;
    use xml::{Element, Node};

    const NAME: &str = "word/styles.xml";
    const TYPE: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml";
    const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles";
    const STRICT_REL: &str = "http://purl.oclc.org/ooxml/officeDocument/relationships/styles";
    let strict = xml::parse(&data).is_ok_and(|d| d.root.get("xmlns:w").as_deref() == Some(W_NS_STRICT));
    let relation = if strict { STRICT_REL } else { REL };
    let parse = |name: &str, root: &str, ns: &str| -> Result<xml::Doc, EngineError> {
        let d = package::get(parts, name).and_then(|bytes| xml::parse(bytes).ok());
        d.filter(|d| {
            let xmlns = d.root.name.split_once(':').map_or("xmlns".to_string(), |(p, _)| format!("xmlns:{p}"));
            d.root.local() == root && d.root.get(&xmlns).as_deref() == Some(ns)
        })
        .ok_or_else(|| {
            EngineError::Refused(format!(
                "{name} is missing or cannot be parsed, so the styles part cannot be registered"
            ))
        })
    };
    let qualified = |root: &Element, name: &str| {
        root.name.split_once(':').map_or_else(|| name.to_string(), |(prefix, _)| format!("{prefix}:{name}"))
    };
    let mut types = parse(opc::CT_PART, "Types", "http://schemas.openxmlformats.org/package/2006/content-types")?;
    let rels_name = opc::rels_part(DOC_PART);
    const REL_TYPE: &str = "application/vnd.openxmlformats-package.relationships+xml";
    let mut added_types = vec![(NAME, TYPE)];
    if package::get(parts, &rels_name).is_none()
        && !types.root.elements().any(|e| {
            e.local() == "Default"
                && e.get("Extension").as_deref() == Some("rels")
                && e.get("ContentType").as_deref() == Some(REL_TYPE)
        })
    {
        added_types.push((rels_name.as_str(), REL_TYPE));
    }
    let mut changed_types = false;
    for (part, expected) in added_types {
        let path = format!("/{part}");
        let content_type = types
            .root
            .elements()
            .find(|e| e.local() == "Override" && e.get("PartName").as_deref() == Some(path.as_str()))
            .map(|e| e.get("ContentType"));
        if let Some(content_type) = content_type {
            if content_type.as_deref() != Some(expected) {
                return Err(EngineError::Refused(format!("[Content_Types].xml names another content type for {part}")));
            }
        } else {
            let name = qualified(&types.root, "Override");
            types
                .root
                .children
                .push(Node::El(Element::new(&name).with_attr("PartName", &path).with_attr("ContentType", expected)));
            changed_types = true;
        }
    }
    let existing = opc::rels_of(parts, DOC_PART);
    let styles_rels: Vec<_> = existing.iter().filter(|r| r.ty == REL || r.ty == STRICT_REL).collect();
    if styles_rels.len() > 1
        || styles_rels.iter().any(|r| r.external || opc::resolve_target(DOC_PART, &r.target) != NAME)
    {
        return Err(EngineError::Refused(format!("{rels_name} already names another styles target")));
    }
    let rels_data = if package::get(parts, &rels_name).is_some() {
        let mut d = parse(&rels_name, "Relationships", opc::RELS_NS)?;
        if styles_rels.is_empty() {
            let name = qualified(&d.root, "Relationship");
            d.root.children.push(Node::El(
                Element::new(&name)
                    .with_attr("Id", &opc::free_rel_id(&existing))
                    .with_attr("Type", relation)
                    .with_attr("Target", "styles.xml"),
            ));
            Some(xml::write_doc(&d))
        } else {
            None
        }
    } else {
        Some(opc::write_rels(&[opc::Rel {
            id: "rId1".into(),
            ty: relation.into(),
            target: "styles.xml".into(),
            external: false,
        }]))
    };
    let template = parts
        .iter()
        .find(|p| p.name == DOC_PART)
        .ok_or_else(|| EngineError::Package("package has no word/document.xml".into()))?;
    let (dos_time, external_attr) = (template.dos_time, template.external_attr);
    let mut put = |name: &str, data: Vec<u8>| {
        if let Some(p) = parts.iter_mut().find(|p| p.name == name) {
            p.data = data;
        } else {
            parts.push(hanji_core::Part { name: name.into(), data, dos_time, external_attr, deflate: true });
        }
    };
    if changed_types {
        put(opc::CT_PART, xml::write_doc(&types));
    }
    if let Some(data) = rels_data {
        put(&rels_name, data);
    }
    put(NAME, data);
    Ok(())
}

/// `document.xml` (and numbering) for resolved blocks placed against `rem`.
/// The style section is `rem`'s (the text's, once re-anchored).
pub fn export_document(blocks: &[hanji_core::Block], rem: &Remainder) -> Result<Exported, EngineError> {
    let shell = DocShell::of(rem)?;
    let mut numbering = numbering::Numbering::read(&rem.parts);
    let lists = hanji_core::plan_lists(blocks, &rem.entries, &mut numbering).map_err(EngineError::Refused)?;
    let (styles, styles_part) = written_styles(rem)?;
    let mut ex = export::Exporter::new(&styles, &rem.entries, &numbering, lists);
    if styles.formatting {
        ex = ex.formatted(export::Fmt::new(&styles, format::Theme::read(&rem.parts)));
    }
    let body = ex.body(blocks).map_err(EngineError::Refused)?;
    Ok(Exported { document: shell.document(&body).into_bytes(), numbering: numbering.part(), styles: styles_part })
}

/// The style set with new styles' ids, and `word/styles.xml` when the style
/// section changes it.
pub(crate) fn written_styles(rem: &Remainder) -> Result<(hanji_core::StyleSet, Option<Vec<u8>>), EngineError> {
    let mut styles = rem.styles.clone();
    if !styles.formatting {
        return Ok((styles, None));
    }
    let namespace = rem.namespaces.iter().find(|(prefix, _)| prefix == "w").map_or(W_NS, |(_, uri)| uri.as_str());
    let part =
        format::write_styles_in_namespace(&rem.parts, &mut styles, &[], namespace).map_err(EngineError::Refused)?;
    Ok((styles, part))
}
