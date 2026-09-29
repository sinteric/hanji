//! The engines as the shared harness (hanji-testkit) sees them: the same
//! `Format` implementations as each engine's corpus test, which this tool
//! cannot import (they live in test targets).

use hanji_core::{Block, Engine, EngineError, Entry, ImportOptions, ImportReport, Kind, Remainder, TextModel};
use hanji_docx::{export_document, DocxEngine};
use hanji_hwpx::owpml::section_number;
use hanji_hwpx::{export_sections, HwpxEngine};
use hanji_package::xml;
use hanji_pptx::{export_parts, PptxEngine, PptxModel};
use hanji_testkit::Format;

// ---------------------------------------------------------------- docx (crates/hanji-docx/tests/corpus.rs)

pub struct Docx;

const VISIBLE_RPR: &[&str] = &["color", "sz", "highlight", "u", "shd", "strike", "caps", "vertAlign"];

impl Format for Docx {
    fn engine(&self) -> &dyn Engine {
        &DocxEngine
    }
    fn ext(&self) -> &'static str {
        "docx"
    }
    fn split(
        &self,
        pkg: &[u8],
        opts: &ImportOptions,
    ) -> Result<(Vec<Block>, Remainder, ImportReport, String), EngineError> {
        let (b, r, rep, s) = DocxEngine::split(pkg, opts)?;
        Ok((b, r, rep, format!("tables {}/{} kept {:?}", s.tables_modelled, s.tables_kept, s.kept_reasons)))
    }
    fn text_of(&self, blocks: &[Block], rem: &Remainder) -> String {
        DocxEngine::text_of(blocks, rem, None)
    }
    fn export_blocks(&self, blocks: &[Block], rem: &Remainder) -> Result<(Vec<u8>, bool), EngineError> {
        let doc = export_document(blocks, rem)?;
        let well_formed = xml::parse(&doc.document).is_ok();
        Ok((hanji_docx::write_package(rem, doc)?, well_formed))
    }
    fn is_split_part(&self, name: &str) -> bool {
        name == "word/document.xml"
    }
    fn visible_run(&self, e: &Entry) -> bool {
        e.kind == Kind::Run
            && e.xml.len() >= 2
            && xml::fragment(&e.xml[1]).elements().any(|c| VISIBLE_RPR.contains(&c.local()))
    }
    fn holds_section(&self, entries: &[&Entry], _: usize) -> bool {
        entries.iter().any(|e| e.kind == Kind::Ppr && e.xml.len() > 1 && e.xml[1].contains("sectPr"))
    }
    fn preferred_styles(&self) -> &'static [&'static str] {
        &["Quote", "Intense Quote", "List Paragraph", "Body Text", "Subtitle", "Title", "Note", "Quotations"]
    }
    fn is_drawing(&self, xml: &str) -> bool {
        xml.contains("drawing") || xml.contains("pict")
    }
}

// ---------------------------------------------------------------- pptx (crates/hanji-pptx/tests/corpus.rs)

pub struct Pptx;

/// `a:rPr` attributes that change nothing a person sees.
const QUIET_RPR: &[&str] =
    &["lang", "altLang", "dirty", "err", "noProof", "smtClean", "smtId", "b", "i", "u", "strike", "bmk"];

impl Format for Pptx {
    fn engine(&self) -> &dyn Engine {
        &PptxEngine
    }
    fn ext(&self) -> &'static str {
        "pptx"
    }
    fn split(
        &self,
        pkg: &[u8],
        opts: &ImportOptions,
    ) -> Result<(Vec<Block>, Remainder, ImportReport, String), EngineError> {
        let (b, r, rep, s) = PptxEngine::split(pkg, opts)?;
        let line = format!(
            "slides {} slots {} (objects {}) shapes {} latent {} notes {} kept {} list items {}",
            s.slides, s.slots, s.keep_slots, s.shapes, s.latent, s.notes, s.kept, s.list_items
        );
        Ok((b, r, rep, line))
    }
    fn text_of(&self, blocks: &[Block], rem: &Remainder) -> String {
        PptxEngine::text_of(blocks, rem, None)
    }
    fn export_blocks(&self, blocks: &[Block], rem: &Remainder) -> Result<(Vec<u8>, bool), EngineError> {
        let parts = export_parts(blocks, rem)?;
        let well_formed = parts
            .iter()
            .filter(|p| p.name.ends_with(".xml") || p.name.ends_with(".rels"))
            .all(|p| xml::parse(&p.data).is_ok());
        Ok((hanji_pptx::write_package(&parts)?, well_formed))
    }
    fn is_split_part(&self, name: &str) -> bool {
        name == "ppt/presentation.xml"
            || numbered(name, "ppt/slides/slide")
            || numbered(name, "ppt/notesSlides/notesSlide")
    }
    fn visible_run(&self, e: &Entry) -> bool {
        e.kind == Kind::Run
            && e.xml.get(1).is_some_and(|x| {
                let r = xml::fragment(x);
                r.has_elements() || r.attrs.iter().any(|a| !QUIET_RPR.contains(&a.0.as_str()))
            })
    }
    fn holds_section(&self, _: &[&Entry], _: usize) -> bool {
        false
    }
    fn preferred_styles(&self) -> &'static [&'static str] {
        &[]
    }
    fn is_drawing(&self, xml: &str) -> bool {
        xml.contains("<p:pic") || xml.contains("<p:graphicFrame")
    }
    fn model(&self) -> &dyn TextModel {
        &PptxModel
    }
}

fn numbered(name: &str, base: &str) -> bool {
    name.strip_prefix(base).and_then(|r| r.strip_suffix(".xml")).is_some_and(|n| n.parse::<u32>().is_ok())
}

// ---------------------------------------------------------------- hwpx (crates/hanji-hwpx/tests/corpus.rs)

pub struct Hwpx;

impl Format for Hwpx {
    fn engine(&self) -> &dyn Engine {
        &HwpxEngine
    }
    fn ext(&self) -> &'static str {
        "hwpx"
    }
    fn split(
        &self,
        pkg: &[u8],
        opts: &ImportOptions,
    ) -> Result<(Vec<Block>, Remainder, ImportReport, String), EngineError> {
        let (b, r, rep, s) = HwpxEngine::split(pkg, opts)?;
        let line = format!(
            "tables {}/{} (side by side: {}) kept {:?}, list items {}, tracked paragraphs {}",
            s.tables_modelled, s.tables_kept, s.side_by_side, s.kept_reasons, s.list_items, s.tracked_paragraphs
        );
        Ok((b, r, rep, line))
    }
    fn text_of(&self, blocks: &[Block], rem: &Remainder) -> String {
        HwpxEngine::text_of(blocks, rem, None)
    }
    fn export_blocks(&self, blocks: &[Block], rem: &Remainder) -> Result<(Vec<u8>, bool), EngineError> {
        let out = export_sections(blocks, rem)?;
        let well_formed = out.sections.iter().all(|(_, d)| xml::parse(d).is_ok())
            && out.header.as_deref().is_none_or(|h| xml::parse(h).is_ok());
        Ok((hanji_hwpx::write_package(rem, out)?, well_formed))
    }
    fn is_split_part(&self, name: &str) -> bool {
        section_number(name).is_some()
    }
    fn visible_run(&self, e: &Entry) -> bool {
        // A run whose character shape differs from its style's beyond the four marks.
        e.kind == Kind::Run && !e.fp.is_empty()
    }
    fn holds_section(&self, _: &[&Entry], _: usize) -> bool {
        // A section's settings go with the section, not with a paragraph.
        false
    }
    fn preferred_styles(&self) -> &'static [&'static str] {
        &["본문", "Body Text", "Quote"]
    }
    fn is_drawing(&self, xml: &str) -> bool {
        ["<hp:pic", "<hp:rect", "<hp:container", "<hp:ellipse", "<hp:polygon", "<hp:line", "<hp:curve", "<hp:arc"]
            .iter()
            .any(|t| xml.contains(t))
    }
}

