//! Conditional formats as `<conditional-format/>` lines: parse/print, edit,
//! add/remove, GetPut of an unedited file. Data bars stay a placeholder.

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

fn cf_lines(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| l.starts_with("<conditional-format "))
        .map(str::to_string)
        .collect()
}

fn sheet_cf_rules(bytes: &[u8], part: &str) -> Vec<(String, String, Option<String>)> {
    let parts = package::read(bytes).unwrap();
    let data = parts.iter().find(|p| p.name == part).unwrap();
    let doc = xml::parse(&data.data).unwrap();
    let mut out = vec![];
    for cf in doc
        .root
        .elements()
        .filter(|e| e.local() == "conditionalFormatting")
    {
        let sqref = cf.get("sqref").unwrap_or_default();
        for rule in cf.elements().filter(|e| e.local() == "cfRule") {
            let ty = rule.get("type").unwrap_or_default();
            let f = rule
                .elements()
                .find(|c| c.local() == "formula")
                .map(|c| c.text_of(&[c.name.as_str()]));
            out.push((sqref.clone(), ty, f));
        }
    }
    out
}

#[test]
fn unedited_file_keeps_its_conditional_format_xml() {
    let bytes = corpus_file("rx_cond_format01.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let lines = cf_lines(&imp.text);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("range=\"A1\""), "{lines:?}");
    assert!(lines[0].contains("type=\"cellIs\""), "{lines:?}");
    assert!(lines[0].contains("operator=\"greaterThan\""), "{lines:?}");
    assert!(lines[0].contains("formula=\"5\""), "{lines:?}");
    let out = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
    getput(&bytes, &out).unwrap();
}

#[test]
fn editing_a_conditional_format_writes_it_and_reads_back() {
    let bytes = corpus_file("rx_cond_format01.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let edited = imp.text.replace("formula=\"5\"", "formula=\"10\"");
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    assert!(
        cf_lines(&a.text)
            .iter()
            .any(|l| l.contains("formula=\"10\"")),
        "{:?}",
        cf_lines(&a.text)
    );
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    let got = sheet_cf_rules(&out, "xl/worksheets/sheet1.xml");
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0].2.as_deref(), Some("10"));
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(again.text, a.text);
}

#[test]
fn removing_a_conditional_format_drops_it() {
    let bytes = corpus_file("rx_cond_format01.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let first = cf_lines(&imp.text)[0].clone();
    let edited = imp.text.replace(&format!("{first}\n"), "");
    assert_ne!(edited, imp.text);
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    assert!(cf_lines(&a.text).is_empty(), "{:?}", a.text);
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    let parts = package::read(&out).unwrap();
    let sheet = parts
        .iter()
        .find(|p| p.name == "xl/worksheets/sheet1.xml")
        .unwrap();
    let doc = xml::parse(&sheet.data).unwrap();
    assert!(
        doc.root
            .elements()
            .all(|e| e.local() != "conditionalFormatting"),
        "conditionalFormatting should go with the last rule"
    );
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(again.text, a.text);
}

#[test]
fn a_data_bar_stays_a_placeholder() {
    let bytes = corpus_file("simple-monthly-budget.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    assert!(
        cf_lines(&imp.text).is_empty(),
        "data bars must not be listed as text:\n{}",
        imp.text
    );
    assert!(
        imp.text.contains("kind=\"conditional-format\""),
        "{}",
        imp.text
    );
    let out = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
    getput(&bytes, &out).unwrap();
}

#[test]
fn corpus_cellis_files_round_trip_the_text() {
    for name in [
        "rx_cond_format01.xlsx",
        "WithConditionalFormatting.xlsx",
        "korean-sales.xlsx",
        "korean-sales-lo.xlsx",
    ] {
        let bytes = corpus_file(name);
        let imp = XlsxEngine
            .import(&bytes, &ImportOptions::default())
            .unwrap();
        assert!(
            !cf_lines(&imp.text).is_empty(),
            "{name} should show <conditional-format/> lines:\n{}",
            imp.text
        );
        assert!(
            !imp.text.contains("kind=\"conditional-format\""),
            "{name} still has a conditional-format placeholder:\n{}",
            imp.text
        );
        let names = XlsxEngine::names(&imp.remainder);
        let parsed = parse_spreadsheet(&imp.text, &names).unwrap();
        assert_eq!(serialize_spreadsheet(&parsed), imp.text, "{name}");
        assert!(
            parsed.sheets.iter().any(|s| s
                .items
                .iter()
                .any(|i| matches!(i, SheetItem::ConditionalFormat(_)))),
            "{name}"
        );
        let out = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
        getput(&bytes, &out).unwrap();
    }
}
