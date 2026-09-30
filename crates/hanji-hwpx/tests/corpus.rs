//! §9 on the hwpx corpus (`corpus/`, see its SOURCES.md) through the shared
//! harness (hanji-testkit): GetPut, the scripted edits E1–E9 and the
//! formatting edits F1–F4, then all in one (E10), with PutGet and
//! remainder outcomes against the oracle, and validity. Set HANJI_REPORT=1
//! to print per-file numbers.

use std::path::PathBuf;

use hanji_core::{Block, Engine, EngineError, Entry, ImportOptions, ImportReport, Kind, Remainder};
use hanji_hwpx::owpml::{layout_problems, section_number};
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
        // Well-formed, and no layout cache past its paragraph's end (Hancom's repair prompt).
        let valid = |(name, d): &(String, Vec<u8>)| {
            xml::parse(d).is_ok_and(|x| {
                let problems = layout_problems(&x.root);
                for p in &problems {
                    eprintln!("{name}: {p}");
                }
                problems.is_empty()
            })
        };
        let well_formed = out.sections.iter().all(valid) && out.header.as_deref().is_none_or(|h| xml::parse(h).is_ok());
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
    fn complete(&self, blocks: &mut [Block], rem: &mut Remainder) {
        HwpxEngine::complete(blocks, rem);
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
    let sum = hanji_testkit::run_corpus_with(&Hwpx, &files, &out, &hanji_testkit::FORMATTED_EDITS, "E10");
    assert!(sum.failures.is_empty(), "{}", sum.failures.join("\n"));
    assert_eq!(sum.getput_ok, files.len());
    // An exact-span edit loses no entry of the blocks it touches.
    assert_eq!(sum.touched["exact"][2], 0, "{:?}", sum.touched);
}

#[test]
fn neutralised_import_keeps_getput_for_the_rest() {
    hanji_testkit::neutralised_import_keeps_getput_for_the_rest(&Hwpx, &corpus());
}

#[test]
fn a_serialized_remainder_exports_the_same_package() {
    hanji_testkit::a_serialized_remainder_exports_the_same_package(&Hwpx, &corpus());
}

/// A new table at the end of every corpus file exports, reads back as
/// written, and goes next to the corpus exports for the rhwp check: in a
/// file with a table it takes that table's layout, in one without, the
/// default look.
#[test]
fn a_new_table_can_be_added_to_every_file() {
    let out = Out { dir: PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("hwpx-corpus-out"), ext: "hwpx" };
    let caps = HwpxEngine.capabilities();
    for (name, pkg) in corpus() {
        let imp = HwpxEngine.import(&pkg, &ImportOptions::default()).unwrap();
        let new = format!("{}\n| 새 표 | 값 |\n|---|---|\n| 가 | 1 |\n", imp.text);
        let r = hanji_core::rewrite(&imp.remainder, &imp.text, &new, caps).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        // As the store returns it: the new table shows the look it takes.
        let (_, mut blocks) = hanji_core::model_of(&r.text, &r.remainder, caps).unwrap();
        let mut rem = r.remainder.clone();
        HwpxEngine::complete(&mut blocks, &mut rem);
        let text = HwpxEngine::text_of(&blocks, &rem, None);
        let bytes = HwpxEngine.export(&text, &rem).unwrap_or_else(|e| panic!("{name}: {e}"));
        let back = HwpxEngine.import(&bytes, &ImportOptions::default()).unwrap();
        assert_eq!(back.text, text, "{name}: PutGet");
        out.save(&name, "ORIGINAL", &pkg);
        out.save(&name, "new-table", &bytes);
    }
}

/// The layout check (`layout_problems`, which the corpus run applies to
/// every export) flags no cache Hancom wrote: every corpus file passes it.
#[test]
fn the_layout_check_passes_every_corpus_file() {
    for (name, pkg) in corpus() {
        let problems = hanji_hwpx::layout_problems(&pkg).unwrap();
        assert!(problems.is_empty(), "{name}: {problems:?}");
    }
}
