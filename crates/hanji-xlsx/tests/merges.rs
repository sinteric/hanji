//! Merged cells as `<merge range/>` lines: parse/print, add/remove/change,
//! invalid input, GetPut of an unedited file, and the many-merges keep.

use std::path::PathBuf;

use hanji_core::{Engine, ImportOptions};
use hanji_format::sheet::{parse_spreadsheet, serialize_spreadsheet, SheetItem};
use hanji_xlsx::{package, xml, XlsxEngine};

mod common;
use common::getput;

fn corpus_file(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("corpus")
            .join(name),
    )
    .unwrap()
}

fn merge_lines(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| l.starts_with("<merge range="))
        .map(str::to_string)
        .collect()
}

fn sheet_merges(bytes: &[u8], part: &str) -> Vec<String> {
    let parts = package::read(bytes).unwrap();
    let data = parts.iter().find(|p| p.name == part).unwrap();
    let doc = xml::parse(&data.data).unwrap();
    let list = doc
        .root
        .elements()
        .find(|e| e.local() == "mergeCells")
        .expect("the worksheet has <mergeCells>");
    list.elements()
        .filter(|e| e.local() == "mergeCell")
        .map(|e| e.get("ref").unwrap())
        .collect()
}

#[test]
fn unedited_file_keeps_its_merge_xml() {
    let bytes = corpus_file("rx_merge_range01.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    assert_eq!(merge_lines(&imp.text), ["<merge range=\"B2:D2\"/>"]);
    let out = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
    getput(&bytes, &out).unwrap();
}

#[test]
fn adding_a_merge_writes_it_and_reads_back() {
    let bytes = corpus_file("rx_merge_range01.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let edited = imp.text.replace(
        "<merge range=\"B2:D2\"/>",
        "<merge range=\"B2:D2\"/>\n<merge range=\"A4:B5\"/>",
    );
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    assert_eq!(
        merge_lines(&a.text),
        ["<merge range=\"B2:D2\"/>", "<merge range=\"A4:B5\"/>"]
    );
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    assert_eq!(
        sheet_merges(&out, "xl/worksheets/sheet1.xml"),
        ["B2:D2", "A4:B5"]
    );
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(again.text, a.text);
}

#[test]
fn removing_a_merge_drops_it() {
    let bytes = corpus_file("simple-monthly-budget.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let before = merge_lines(&imp.text);
    assert!(before.iter().any(|l| l.contains("B8:C8")), "{before:?}");
    let edited = imp.text.replace("<merge range=\"B8:C8\"/>\n", "");
    assert_ne!(edited, imp.text);
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    let after = merge_lines(&a.text);
    assert!(!after.iter().any(|l| l.contains("B8:C8")), "{after:?}");
    assert_eq!(after.len(), before.len() - 1);
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    let refs = sheet_merges(&out, "xl/worksheets/sheet1.xml");
    assert!(!refs.iter().any(|r| r == "B8:C8"), "{refs:?}");
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(again.text, a.text);
}

#[test]
fn changing_a_merge_updates_its_ref() {
    let bytes = corpus_file("korean-sales.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    assert!(
        imp.text.contains("<merge range=\"A1:F1\"/>"),
        "{}",
        imp.text
    );
    let edited = imp
        .text
        .replace("<merge range=\"A1:F1\"/>", "<merge range=\"A1:G1\"/>");
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    assert!(a.text.contains("<merge range=\"A1:G1\"/>"), "{}", a.text);
    assert!(!a.text.contains("<merge range=\"A1:F1\"/>"), "{}", a.text);
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    assert_eq!(sheet_merges(&out, "xl/worksheets/sheet1.xml"), ["A1:G1"]);
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(again.text, a.text);
}

#[test]
fn overlapping_and_out_of_range_merges_are_refused() {
    let bytes = corpus_file("rx_merge_range01.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let overlap = imp.text.replace(
        "<merge range=\"B2:D2\"/>",
        "<merge range=\"B2:D2\"/>\n<merge range=\"D2:E3\"/>",
    );
    let e = XlsxEngine::apply(&overlap, &imp.remainder, "[]").unwrap_err();
    let msg = e.to_string();
    assert!(
        msg.contains("overlaps B2:D2") && msg.contains("do not overlap"),
        "{msg}"
    );

    let oob = imp.text.replace(
        "<merge range=\"B2:D2\"/>",
        "<merge range=\"B2:D2\"/>\n<merge range=\"A1048577:B1048577\"/>",
    );
    let e = XlsxEngine::apply(&oob, &imp.remainder, "[]").unwrap_err();
    let msg = e.to_string();
    assert!(
        msg.contains("A1048577:B1048577") && msg.contains("XFD1048576"),
        "{msg}"
    );

    let one = imp.text.replace("B2:D2", "B2");
    let e = XlsxEngine::apply(&one, &imp.remainder, "[]").unwrap_err();
    assert!(e.to_string().contains("one cell"), "{e}");
}

#[test]
fn a_sheet_with_tens_of_thousands_of_merges_stays_a_placeholder() {
    let bytes = corpus_file("57893-many-merges.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    assert!(
        merge_lines(&imp.text).is_empty(),
        "50,000 merges must not be listed as text"
    );
    assert!(imp.text.contains("kind=\"merged-cells\""), "{}", imp.text);
    let out = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
    getput(&bytes, &out).unwrap();
}

#[test]
fn corpus_merge_files_round_trip_the_text() {
    for name in [
        "korean-sales.xlsx",
        "rx_merge_range01.xlsx",
        "simple-monthly-budget.xlsx",
        "DataValidations-49244.xlsx",
        "Tables.xlsx",
    ] {
        let bytes = corpus_file(name);
        let imp = XlsxEngine
            .import(&bytes, &ImportOptions::default())
            .unwrap();
        assert!(
            !merge_lines(&imp.text).is_empty(),
            "{name} should show <merge range/> lines"
        );
        let names = XlsxEngine::names(&imp.remainder);
        let parsed = parse_spreadsheet(&imp.text, &names).unwrap();
        assert_eq!(serialize_spreadsheet(&parsed), imp.text, "{name}");
        assert!(
            parsed
                .sheets
                .iter()
                .any(|s| s.items.iter().any(|i| matches!(i, SheetItem::Merge(_)))),
            "{name}"
        );
        let out = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
        getput(&bytes, &out).unwrap();
    }
}
