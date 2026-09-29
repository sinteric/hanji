//! The pptx engine (PresentationML): splits the slides, their notes pages
//! and the slide list in `presentation.xml` into model text (§5.3) and
//! remainder entries at the XML level, and puts them back. Every other part
//! is copied through byte for byte, except where new, deleted or moved slides
//! change relationships and content types. No I/O: package bytes in, package
//! bytes out.
//!
//! rpptx is not on the import/export path: its model is typed and it writes
//! parts from it (text bodies as `CT_TextBody`, shapes re-serialized in
//! schema order), so the original `a:rPr`/`a:pPr` fragments and the slide
//! XML around the text, which the remainder stores, are not available as
//! written. Its `add_slide` makes a slide from the layout's placeholder types
//! and indices, which is what export does here too.

pub mod deck;
pub mod export;
pub mod import;
pub mod pml;
pub mod safety;

pub use hanji_package::{package, xml};

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use hanji_core::{
    Block, BlockSrc, Capabilities, Engine, EngineError, ImportOptions, ImportReport, Imported, Part, Remainder,
    StyleSet, TextModel,
};
use hanji_format::{self as fmt, Diagnostic, FrontMatter, Layout, Names};
use hanji_package::opc;
use hanji_package::xml::{Element, Node};

use crate::deck::Deck;
use crate::import::{Importer, NotesInfo, SlideInfo, Stats};
use crate::pml::*;

pub struct PptxEngine;

/// What export needs besides the entries, stored once.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DeckShell {
    pub pres_part: String,
    pub prolog: String,
    pub epilog: String,
    /// `presentation.xml` with an empty slide list.
    pub pres: String,
    pub deck: Deck,
    /// The slide parts at import, in slide order.
    pub slides: Vec<String>,
    /// Their `p:sldId` ids, in slide order.
    pub order: Vec<u32>,
    /// Every slide id the file had (new slides take higher ones).
    pub slide_ids: Vec<u32>,
    /// Shapes shown as `<shape>`: `(id, name)`.
    pub shapes: Vec<(String, String)>,
}

impl DeckShell {
    pub fn of(rem: &Remainder) -> Result<DeckShell, String> {
        serde_json::from_str(rem.shell.first().ok_or("the remainder has no deck shell")?).map_err(|e| e.to_string())
    }

    /// The names the text may use: layouts and their slots, shapes,
    /// placeholders and which of them are slide objects.
    pub fn names(&self, rem: &Remainder) -> Names {
        Names {
            layouts: Some(
                self.deck
                    .layouts
                    .iter()
                    .map(|l| Layout { name: l.name.clone(), slots: l.slots.iter().map(|s| s.name.clone()).collect() })
                    .collect(),
            ),
            shapes: Some(self.shapes.clone()),
            keeps: Some(rem.keep_list()),
            objects: Some(
                rem.entries
                    .iter()
                    .filter(|e| e.kind == hanji_core::Kind::Bkeep && e.meta.tag == import::OBJECT_TAG)
                    .filter_map(|e| e.meta.keep.as_ref().map(|k| k.id.clone()))
                    .collect(),
            ),
            formats: Some(vec!["pptx".into()]),
            ..Default::default()
        }
    }
}

/// The Presentation grammar against a pptx remainder.
pub struct PptxModel;

impl TextModel for PptxModel {
    fn resolve(
        &self,
        text: &str,
        rem: &Remainder,
        caps: Capabilities,
    ) -> Result<(Vec<Block>, Vec<BlockSrc>), Vec<Diagnostic>> {
        let shell = DeckShell::of(rem).map_err(|m| vec![Diagnostic { line: 1, col: 1, message: m }])?;
        let (mut blocks, maps) =
            hanji_core::presentation::model_of(text, &shell.names(rem), caps, &|id| rem.is_block_keep(id))?;
        // A slide paragraph has no list of its own: items in a row are one list.
        for k in 1..blocks.len() {
            let prev_item = matches!(&blocks[k - 1], Block::Para(p) if p.item.is_some());
            if let (true, Block::Para(p)) = (prev_item, &mut blocks[k]) {
                if let Some(it) = &mut p.item {
                    it.first = false;
                }
            }
        }
        Ok((blocks, maps))
    }
}

impl PptxEngine {
    /// Model text for resolved blocks and a remainder.
    pub fn text_of(blocks: &[Block], rem: &Remainder, template: Option<&str>) -> String {
        let front = FrontMatter::presentation("pptx", template);
        let p =
            hanji_core::presentation::unresolve(blocks, front, &|id| rem.keep(id).cloned().expect("keep in remainder"));
        fmt::serialize_presentation(&p)
    }

    /// Model blocks of an import (for tests and tools).
    pub fn split(
        package: &[u8],
        opts: &ImportOptions,
    ) -> Result<(Vec<Block>, Remainder, ImportReport, Stats), EngineError> {
        let pkg = EngineError::Package;
        let mut parts = package::read(package).map_err(pkg)?;
        let root_rels =
            opc::parse_rels(package::get(&parts, "_rels/.rels").ok_or_else(|| pkg("no _rels/.rels".into()))?);
        let pres_part = root_rels
            .iter()
            .find(|r| r.short_type() == "officeDocument" && !r.external)
            .map(|r| opc::resolve_target("", &r.target))
            .ok_or_else(|| pkg("the package has no main part".into()))?;
        let parse = |parts: &[Part], name: &str| -> Result<xml::Doc, EngineError> {
            let d = package::get(parts, name).ok_or_else(|| pkg(format!("no {name}")))?;
            xml::parse(d).map_err(|e| pkg(format!("{name}: {e}")))
        };
        let pres_doc = parse(&parts, &pres_part)?;
        check_prefixes(&pres_doc.root, &pres_part, "p:presentation")?;
        // The slides, in slide-list order, and their notes pages.
        let mut slides: Vec<(Element, String)> = vec![];
        for s in
            pres_doc.root.child("p:sldIdLst").map(|l| l.elements().cloned().collect::<Vec<_>>()).unwrap_or_default()
        {
            let rid = s.get("r:id").unwrap_or_default();
            let part =
                opc::target_of(&parts, &pres_part, &rid).ok_or_else(|| pkg(format!("slide {rid} has no part")))?;
            slides.push((s, part));
        }
        let mut docs: BTreeMap<String, xml::Doc> = BTreeMap::new();
        let mut notes_of: BTreeMap<String, String> = BTreeMap::new();
        for (_, part) in &slides {
            let d = parse(&parts, part)?;
            check_prefixes(&d.root, part, "p:sld")?;
            docs.insert(part.clone(), d);
            if let Some(r) = opc::rels_of(&parts, part).into_iter().find(|r| r.ty == REL_NOTES && !r.external) {
                let n = opc::resolve_target(part, &r.target);
                let d = parse(&parts, &n)?;
                check_prefixes(&d.root, &n, "p:notes")?;
                docs.insert(n.clone(), d);
                notes_of.insert(part.clone(), n);
            }
        }
        let mut report = ImportReport::default();
        let mut roots: BTreeMap<String, Element> = docs.iter().map(|(k, d)| (k.clone(), d.root.clone())).collect();
        roots.insert(pres_part.clone(), pres_doc.root.clone());
        if opts.neutralise {
            safety::neutralise(&mut parts, &mut roots, &mut report).map_err(pkg)?;
        }
        {
            let view: Vec<(String, &Element)> = slides.iter().map(|(_, p)| (p.clone(), &roots[p])).collect();
            safety::surface(&parts, &view, &mut report);
        }
        let pres_root = roots.remove(&pres_part).unwrap();
        let deck = Deck::read(&parts, &pres_part, &pres_root).map_err(pkg)?;
        let scope = xml::scope_of(&pres_root);
        let mut imp = Importer::new(scope);
        for (sld, part) in &slides {
            let d = &docs[part];
            let lpart = opc::rels_of(&parts, part)
                .into_iter()
                .find(|r| r.ty == REL_LAYOUT)
                .map(|r| opc::resolve_target(part, &r.target))
                .ok_or_else(|| pkg(format!("{part} has no layout")))?;
            let layout = deck
                .layout_of_part(&lpart)
                .ok_or_else(|| pkg(format!("{part}: its layout {lpart} is not one of a master's layouts")))?;
            let info = SlideInfo {
                part: part.clone(),
                prolog: d.prolog.clone(),
                epilog: d.epilog.clone(),
                sld_id: sld.to_xml(),
                layout: lpart.clone(),
                ids: vec![],
                named: vec![],
            };
            let notes = notes_of.get(part).map(|n| {
                let nd = &docs[n];
                let ni = NotesInfo { part: n.clone(), prolog: nd.prolog.clone(), epilog: nd.epilog.clone() };
                (ni, &roots[n], deck.notes.as_ref().map_or([Bu::Unset; 9], |m| m.bullets))
            });
            imp.slide(info, &roots[part], layout, notes).map_err(|e| pkg(format!("{part}: {e}")))?;
        }
        let mut skel = pres_root.clone();
        let ids: Vec<u32> = slides.iter().filter_map(|(s, _)| s.get("id").and_then(|v| v.parse().ok())).collect();
        if let Some(l) = skel.child_mut("p:sldIdLst") {
            l.children.retain(|n| !matches!(n, Node::El(_)));
        }
        let shell = DeckShell {
            pres_part: pres_part.clone(),
            prolog: pres_doc.prolog.clone(),
            epilog: pres_doc.epilog.clone(),
            pres: skel.to_xml(),
            deck,
            slides: slides.iter().map(|s| s.1.clone()).collect(),
            order: ids.clone(),
            slide_ids: ids,
            shapes: imp.shapes.clone(),
        };
        let namespaces =
            pres_root.attrs.iter().filter_map(|(k, v)| xml::ns_prefix(k).map(|p| (p.to_string(), v.clone()))).collect();
        let split: std::collections::BTreeSet<&String> = docs.keys().chain([&pres_part]).collect();
        for p in &mut parts {
            if split.contains(&p.name) {
                p.data.clear();
            }
        }
        let stats = imp.stats.clone();
        let rem = Remainder {
            format: "pptx".into(),
            namespaces,
            shell: vec![serde_json::to_string(&shell).unwrap()],
            styles: StyleSet::default(),
            entries: imp.entries,
            parts,
            next_id: imp.next_id,
        };
        Ok((imp.blocks, rem, report, stats))
    }
}

/// PresentationML must use the prefixes `p:` and `a:` (other prefixes are not supported yet).
fn check_prefixes(root: &Element, part: &str, name: &str) -> Result<(), EngineError> {
    let bound = |prefix: &str, ns: &str| root.attr(&format!("xmlns:{prefix}")).is_some_and(|v| v == ns);
    if root.name != name || !bound("p", P_NS) || (root.name != "p:presentation" && !bound("a", A_NS)) {
        return Err(EngineError::Package(format!(
            "{part}: PresentationML must use the prefixes p: and a: (other prefixes are not supported yet)"
        )));
    }
    Ok(())
}

impl Engine for PptxEngine {
    fn format(&self) -> &'static str {
        "pptx"
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
        let (blocks, _) = PptxModel.resolve(text, rem, self.capabilities()).map_err(EngineError::Invalid)?;
        write_package(&export_parts(&blocks, rem)?)
    }
}

/// The package's parts for resolved blocks placed against `rem`.
pub fn export_parts(blocks: &[Block], rem: &Remainder) -> Result<Vec<Part>, EngineError> {
    let shell = DeckShell::of(rem).map_err(EngineError::Package)?;
    export::Exporter::new(blocks, rem, &shell).package().map_err(EngineError::Refused)
}

pub fn write_package(parts: &[Part]) -> Result<Vec<u8>, EngineError> {
    package::write(parts).map_err(EngineError::Package)
}
