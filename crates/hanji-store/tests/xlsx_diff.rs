//! Regression coverage for #69: cells are not part of a spreadsheet's
//! structure text, but their changes must be visible in revision reports.

use hanji_core::{Engine, ImportOptions, Remainder};
use hanji_package::package;
use hanji_store::{blank, Format, MemStorage, Window, Workspace};
use hanji_xlsx::XlsxEngine;

fn replace(bytes: &[u8], part: &str, old: &str, new: &str) -> Vec<u8> {
    let mut parts = package::read(bytes).unwrap();
    let p = parts.iter_mut().find(|p| p.name == part).unwrap();
    let text = std::str::from_utf8(&p.data).unwrap();
    assert!(text.contains(old), "{part} has no {old:?}");
    p.data = text.replacen(old, new, 1).into_bytes();
    package::write(&parts).unwrap()
}

fn workbook(rows: &str) -> Vec<u8> {
    replace(
        &blank::package(Format::Xlsx),
        "xl/worksheets/sheet1.xml",
        "<sheetData/>",
        &format!("<sheetData>{rows}</sheetData>"),
    )
}

fn rem(bytes: &[u8]) -> Remainder {
    XlsxEngine.import(bytes, &ImportOptions::default()).unwrap().remainder
}

fn sample() -> Vec<u8> {
    workbook(r#"<row r="1"><c r="A1"><v>10</v></c><c r="B1"><f>A1*2</f><v>20</v></c></row>"#)
}

#[test]
fn operations_report_the_input_and_recalculated_cache_without_structure_changes() {
    let mut ws = Workspace::new(MemStorage::new());
    let o = ws.open_bytes("cells.xlsx", &sample(), None).unwrap();
    let before = ws.read(&o.doc_id, None, &Window::default()).unwrap().text;
    let changed = ws.ops(&o.doc_id, 1, r#"[{"op":"set","range":"Sheet1!A1","values":[[15]]}]"#).unwrap();
    assert!(changed.recalc.unwrap().written >= 1);
    assert_eq!(before, ws.read(&o.doc_id, None, &Window::default()).unwrap().text);

    let diff = ws.diff(&o.doc_id, 1, 2).unwrap().diff;
    assert!(diff.starts_with("Cell changes: 2 "), "{diff}");
    assert!(
        diff.contains("@@ \"Sheet1\"!A1 @@\n- {\"value\":{\"number\":\"10\"}}\n+ {\"value\":{\"number\":\"15\"}}"),
        "{diff}"
    );
    assert!(diff.contains("@@ \"Sheet1\"!B1 @@"), "{diff}");
    assert!(diff.contains(r#"{"value":{"number":"20"},"formula":{"text":"=A1*2"}}"#), "{diff}");
    assert!(diff.contains(r#"{"value":{"number":"30"},"formula":{"text":"=A1*2"}}"#), "{diff}");
    assert!(ws.diff(&o.doc_id, 2, 2).unwrap().diff.is_empty());
    assert_eq!(diff, ws.diff(&o.doc_id, 1, 2).unwrap().diff, "deterministic output");
    let backwards = ws.diff(&o.doc_id, 2, 1).unwrap().diff;
    assert!(backwards.contains("- {\"value\":{\"number\":\"15\"}}\n+ {\"value\":{\"number\":\"10\"}}"));
}

#[test]
fn reimport_reports_value_only_changes_and_identical_reimports_stay_empty() {
    let mut ws = Workspace::new(MemStorage::new());
    let bytes = sample();
    let o = ws.open_bytes("cells.xlsx", &bytes, None).unwrap();
    let polished = replace(&bytes, "xl/worksheets/sheet1.xml", "<v>10</v>", "<v>12</v>");
    let r = ws.reimport_bytes(&o.doc_id, "cells.xlsx", &polished).unwrap();
    assert!(!r.unchanged);
    assert_eq!((r.parent, r.revision), (1, 2));
    assert!(r.diff.starts_with("Cell changes: 1 "), "{}", r.diff);
    assert!(r.diff.contains("@@ \"Sheet1\"!A1 @@"), "{}", r.diff);
    assert_eq!(r.diff, ws.diff(&o.doc_id, 1, 2).unwrap().diff);

    let (_, exported) = ws.export_bytes(&o.doc_id, None, &Default::default()).unwrap();
    let r = ws.reimport_bytes(&o.doc_id, "cells.xlsx", &exported).unwrap();
    assert!(r.unchanged && r.diff.is_empty(), "{}", r.diff);
    assert_eq!(r.revision, 2);
}

#[test]
fn a_formula_change_and_a_missing_or_changed_cache_are_distinct_changes() {
    let mut ws = Workspace::new(MemStorage::new());
    let original = sample();
    let o = ws.open_bytes("cells.xlsx", &original, None).unwrap();
    let formula = replace(&original, "xl/worksheets/sheet1.xml", "<f>A1*2</f>", "<f>A1+10</f>");
    let r = ws.reimport_bytes(&o.doc_id, "cells.xlsx", &formula).unwrap();
    assert!(r.diff.starts_with("Cell changes: 1 "), "{}", r.diff);
    assert!(r.diff.contains(r#""formula":{"text":"=A1*2"}"#), "{}", r.diff);
    assert!(r.diff.contains(r#""formula":{"text":"=A1+10"}"#), "{}", r.diff);

    let cached = replace(&formula, "xl/worksheets/sheet1.xml", "<v>20</v>", "<v>21</v>");
    let r = ws.reimport_bytes(&o.doc_id, "cells.xlsx", &cached).unwrap();
    assert!(r.diff.contains(r#"{"value":{"number":"21"},"formula":{"text":"=A1+10"}}"#), "{}", r.diff);
    let uncached = replace(&cached, "xl/worksheets/sheet1.xml", "<v>21</v>", "<v/>");
    let r = ws.reimport_bytes(&o.doc_id, "cells.xlsx", &uncached).unwrap();
    assert!(r.diff.contains(r#"+ {"value":"empty","formula":{"text":"=A1+10"}}"#), "{}", r.diff);
    assert_eq!(r.diff, ws.diff(&o.doc_id, 3, 4).unwrap().diff);
}

#[test]
fn clearing_cells_and_structure_changes_are_both_reported() {
    let mut ws = Workspace::new(MemStorage::new());
    let o = ws.open_bytes("cells.xlsx", &sample(), None).unwrap();
    ws.ops(&o.doc_id, 1, r#"[{"op":"set","range":"Sheet1!C2","values":[[99]]}]"#).unwrap();
    let added = ws.diff(&o.doc_id, 1, 2).unwrap().diff;
    assert!(added.starts_with("--- revision 1\n+++ revision 2\n"), "{added}");
    assert!(added.contains("+<sheet name=\"Sheet1\" range=\"A1:C2\">"), "{added}");
    assert!(
        added.contains("@@ \"Sheet1\"!C2 @@\n- {\"value\":\"empty\"}\n+ {\"value\":{\"number\":\"99\"}}"),
        "{added}"
    );
    ws.ops(&o.doc_id, 2, r#"[{"op":"set","range":"Sheet1!C2","values":[[null]]}]"#).unwrap();
    let cleared = ws.diff(&o.doc_id, 2, 3).unwrap().diff;
    assert!(
        cleared.contains("@@ \"Sheet1\"!C2 @@\n- {\"value\":{\"number\":\"99\"}}\n+ {\"value\":\"empty\"}"),
        "{cleared}"
    );
}

#[test]
fn values_keep_source_precision_and_type_while_text_encodings_are_equivalent() {
    let old = workbook(
        r#"<row r="1"><c r="A1"><v>9007199254740992</v></c><c r="B1" t="b"><v>1</v></c><c r="C1" t="inlineStr"><is><t>한글 &amp; text</t></is></c></row>"#,
    );
    let new = replace(&old, "xl/worksheets/sheet1.xml", "9007199254740992", "9007199254740993");
    let new = replace(&new, "xl/worksheets/sheet1.xml", r#"r="B1" t="b""#, r#"r="B1""#);
    let new = replace(
        &new,
        "xl/worksheets/sheet1.xml",
        r#"t="inlineStr"><is><t>한글 &amp; text</t></is>"#,
        r#"t="str"><v>한글 &amp; text</v>"#,
    );
    let diff = XlsxEngine::cell_diff(&rem(&old), &rem(&new)).unwrap();
    assert!(diff.starts_with("Cell changes: 2 "), "{diff}");
    assert!(diff.contains("9007199254740992") && diff.contains("9007199254740993"), "{diff}");
    assert!(diff.contains(r#"{"bool":true}"#) && diff.contains(r#"{"number":"1"}"#), "{diff}");
    assert!(!diff.contains("!C1"), "equal text must not depend on its encoding: {diff}");
}

#[test]
fn shared_strings_are_compared_by_text_not_index() {
    let mut old = rem(&workbook(r#"<row r="1"><c r="A1" t="s"><v>0</v></c></row>"#));
    let sst = old.parts.iter_mut().find(|p| p.name == "xl/sharedStrings.xml").unwrap();
    sst.data = br#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><si><t>old</t></si><si><t>new</t></si></sst>"#.to_vec();
    let mut new = old.clone();
    new.parts.iter_mut().find(|p| p.name == "xl/sharedStrings.xml").unwrap().data = br#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><si><t>new</t></si><si><t>old</t></si></sst>"#.to_vec();
    let diff = XlsxEngine::cell_diff(&old, &new).unwrap();
    assert!(diff.contains(r#"- {"value":{"text":"old"}}"#), "{diff}");
    assert!(diff.contains(r#"+ {"value":{"text":"new"}}"#), "{diff}");
    let part = new.parts.iter_mut().find(|p| p.name == "xl/worksheets/sheet1.xml").unwrap();
    part.data = std::str::from_utf8(&part.data).unwrap().replace("<v>0</v>", "<v>1</v>").into_bytes();
    assert!(XlsxEngine::cell_diff(&old, &new).unwrap().is_empty(), "same text at a new SST index");
}

#[test]
fn shared_formulas_are_expanded_for_followers_and_equal_to_plain_formulas() {
    let shared = workbook(
        r#"<row r="1"><c r="A1"><v>1</v></c><c r="B1"><f t="shared" si="0" ref="B1:B2">A1*2</f><v>2</v></c></row><row r="2"><c r="A2"><v>2</v></c><c r="B2"><f t="shared" si="0"/><v>4</v></c></row>"#,
    );
    let changed = replace(&shared, "xl/worksheets/sheet1.xml", "A1*2", "A1*3");
    let diff = XlsxEngine::cell_diff(&rem(&shared), &rem(&changed)).unwrap();
    assert!(diff.starts_with("Cell changes: 2 "), "{diff}");
    assert!(diff.contains("@@ \"Sheet1\"!B2 @@"), "{diff}");
    assert!(diff.contains(r#""text":"=A2*2""#) && diff.contains(r#""text":"=A2*3""#), "{diff}");
    let plain = replace(&shared, "xl/worksheets/sheet1.xml", r#"<f t="shared" si="0" ref="B1:B2">"#, "<f>");
    let plain = replace(&plain, "xl/worksheets/sheet1.xml", r#"<f t="shared" si="0"/>"#, "<f>A2*2</f>");
    assert!(XlsxEngine::cell_diff(&rem(&shared), &rem(&plain)).unwrap().is_empty());
}

#[test]
fn shared_external_references_adjust_relatively_without_evaluating_them() {
    let shared = workbook(
        r#"<row r="1"><c r="B1"><f t="shared" si="0" ref="B1:B2">[1]Sheet1!A1+[1]Sheet1!$A$1</f><v>2</v></c></row><row r="2"><c r="B2"><f t="shared" si="0"/><v>4</v></c></row>"#,
    );
    // Exercise the low-level diff on stored source formulas. Import normally
    // neutralises external links; this read-only comparison does no fetching.
    let imported = |bytes: &[u8]| {
        XlsxEngine.import(bytes, &ImportOptions { neutralise: false, ..Default::default() }).unwrap().remainder
    };
    let plain = replace(&shared, "xl/worksheets/sheet1.xml", r#"<f t="shared" si="0" ref="B1:B2">"#, "<f>");
    let plain =
        replace(&plain, "xl/worksheets/sheet1.xml", r#"<f t="shared" si="0"/>"#, "<f>[1]Sheet1!A2+[1]Sheet1!$A$1</f>");
    assert!(XlsxEngine::cell_diff(&imported(&shared), &imported(&plain)).unwrap().is_empty());
    let changed = replace(&plain, "xl/worksheets/sheet1.xml", "[1]Sheet1!A2", "[1]Sheet1!A1");
    let diff = XlsxEngine::cell_diff(&imported(&shared), &imported(&changed)).unwrap();
    assert!(diff.starts_with("Cell changes: 1 "), "{diff}");
    assert!(diff.contains("@@ \"Sheet1\"!B2 @@"), "{diff}");
    assert!(diff.contains("[1]Sheet1!A2") && diff.contains("[1]Sheet1!A1"), "{diff}");
}

#[test]
fn sparse_large_diffs_are_counted_and_long_values_have_distinct_truncation_fingerprints() {
    let prefix = "한글".repeat(2000);
    let rows = |end: &str| {
        (1..=103)
            .map(|i| {
                let row = i * 1000;
                format!(r#"<row r="{row}"><c r="XFD{row}" t="inlineStr"><is><t>{prefix}{end}</t></is></c></row>"#)
            })
            .collect::<String>()
    };
    let diff = XlsxEngine::cell_diff(&rem(&workbook(&rows("old"))), &rem(&workbook(&rows("new")))).unwrap();
    assert!(diff.starts_with("Cell changes: 103 "), "{}", &diff[..100]);
    assert_eq!(diff.matches("@@ ").count(), 100);
    assert!(diff.contains("3 more changed cells omitted"));
    assert!(diff.contains("@@ \"Sheet1\"!XFD1000 @@") && diff.contains("@@ \"Sheet1\"!XFD100000 @@"));
    assert!(diff.len() < 100_000, "{} bytes", diff.len());
    let old = diff.lines().find_map(|l| l.strip_prefix("- ")).unwrap();
    let new = diff.lines().find_map(|l| l.strip_prefix("+ ")).unwrap();
    assert!(old.contains("truncated;") && new.contains("fnv1a64:"));
    assert_ne!(old, new, "identical prefixes must not disguise different tails");
}

#[test]
fn a_change_outside_structure_and_cells_is_not_reported_as_an_empty_diff() {
    let mut ws = Workspace::new(MemStorage::new());
    let bytes = sample();
    let o = ws.open_bytes("cells.xlsx", &bytes, None).unwrap();
    let polished =
        replace(&bytes, "xl/worksheets/sheet1.xml", r#"workbookViewId="0""#, r#"workbookViewId="0" showGridLines="0""#);
    let r = ws.reimport_bytes(&o.doc_id, "cells.xlsx", &polished).unwrap();
    assert!(!r.unchanged);
    assert!(r.diff.contains("No structural text or cell value/formula/cache changes detected"), "{}", r.diff);
    assert!(r.diff.contains("other preserved workbook data or representation differs"), "{}", r.diff);
    assert_eq!(r.diff, ws.diff(&o.doc_id, 1, 2).unwrap().diff);
}

#[test]
fn malformed_cell_rows_are_errors_instead_of_silently_empty_diffs() {
    let old = rem(&sample());
    let mut new = old.clone();
    let part = new.parts.iter_mut().find(|p| p.name == "xl/worksheets/sheet1.xml").unwrap();
    part.data = std::str::from_utf8(&part.data).unwrap().replace("</v>", "</wrong>").into_bytes();
    assert!(XlsxEngine::cell_diff(&old, &new).is_err());
}
