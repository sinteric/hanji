//! §9 on the hwpx corpus (`corpus/`, see its SOURCES.md) through the shared
//! harness (hanji-testkit): GetPut, the scripted edits E1–E10 with PutGet
//! and remainder outcomes against the oracle, and validity. Set
//! HANJI_REPORT=1 to print per-file numbers.

use std::path::PathBuf;

use hanji_core::{Block, Engine, EngineError, Entry, ImportOptions, ImportReport, Kind, Remainder};
use hanji_hwpx::owpml::section_number;
use hanji_hwpx::{export_sections, write_package, xml, HwpxEngine};
use hanji_testkit::{Format, Out};

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
        Ok((write_package(rem, out)?, well_formed))
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

pub fn corpus() -> Vec<(String, Vec<u8>)> {
    hanji_testkit::corpus(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus"), "hwpx")
}

#[test]
fn corpus_getput_putget_remainder() {
    let files = corpus();
    assert!(files.len() >= 10, "the corpus has {} files", files.len());
    let out = Out { dir: PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("hwpx-corpus-out"), ext: "hwpx" };
    let sum = hanji_testkit::run_corpus(&Hwpx, &files, &out);
    assert!(sum.failures.is_empty(), "{}", sum.failures.join("\n"));
    assert_eq!(sum.getput_ok, files.len());
}

#[test]
fn neutralised_import_keeps_getput_for_the_rest() {
    hanji_testkit::neutralised_import_keeps_getput_for_the_rest(&Hwpx, &corpus());
}

#[test]
fn a_serialized_remainder_exports_the_same_package() {
    hanji_testkit::a_serialized_remainder_exports_the_same_package(&Hwpx, &corpus());
}
