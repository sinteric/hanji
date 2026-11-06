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
    sheet_cf_priorities(bytes, part)
        .into_iter()
        .map(|(sqref, _, ty, f)| (sqref, ty, f))
        .collect()
}

fn sheet_cf_priorities(bytes: &[u8], part: &str) -> Vec<(String, u32, String, Option<String>)> {
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
            let prio = rule
                .get("priority")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let f = rule
                .elements()
                .find(|c| c.local() == "formula")
                .map(|c| c.text_of(&[c.name.as_str()]));
            out.push((sqref.clone(), prio, ty, f));
        }
    }
    out
}

fn with_cf_xml(bytes: &[u8], xml: &str) -> Vec<u8> {
    let mut parts = package::read(bytes).unwrap();
    let sheet = parts
        .iter_mut()
        .find(|p| p.name == "xl/worksheets/sheet1.xml")
        .unwrap();
    let s = String::from_utf8(sheet.data.clone()).unwrap();
    let start = s
        .find("<conditionalFormatting")
        .expect("the worksheet has conditionalFormatting");
    let end = s
        .rfind("</conditionalFormatting>")
        .expect("the worksheet has a closing conditionalFormatting")
        + "</conditionalFormatting>".len();
    let next = format!("{}{}{}", &s[..start], xml, &s[end..]);
    assert_ne!(next, s);
    sheet.data = next.into_bytes();
    package::write(&parts).unwrap()
}

fn cf_attr(line: &str, name: &str) -> String {
    let p = format!("{name}=\"");
    let i = line.find(&p).unwrap_or_else(|| panic!("{name} in {line}"));
    let rest = &line[i + p.len()..];
    rest[..rest.find('"').expect("quoted attr")].to_string()
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

#[test]
fn interleaved_same_range_rules_keep_relative_priority_after_an_unrelated_edit() {
    let bytes = with_cf_xml(
        &corpus_file("rx_cond_format01.xlsx"),
        concat!(
            r#"<conditionalFormatting sqref="A1">"#,
            r#"<cfRule type="cellIs" dxfId="0" priority="1" operator="greaterThan">"#,
            r#"<formula>1</formula></cfRule>"#,
            r#"<cfRule type="cellIs" dxfId="0" priority="3" operator="equal">"#,
            r#"<formula>3</formula></cfRule>"#,
            r#"</conditionalFormatting>"#,
            r#"<conditionalFormatting sqref="B1">"#,
            r#"<cfRule type="cellIs" dxfId="0" priority="2" operator="lessThan">"#,
            r#"<formula>2</formula></cfRule>"#,
            r#"</conditionalFormatting>"#,
        ),
    );
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let lines = cf_lines(&imp.text);
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert_eq!(cf_attr(&lines[0], "range"), "A1");
    assert_eq!(cf_attr(&lines[0], "formula"), "1");
    assert_eq!(cf_attr(&lines[1], "range"), "B1");
    assert_eq!(cf_attr(&lines[1], "formula"), "2");
    assert_eq!(cf_attr(&lines[2], "range"), "A1");
    assert_eq!(cf_attr(&lines[2], "formula"), "3");
    let edited = imp.text.replace(
        &lines[1],
        &lines[1].replace("formula=\"2\"", "formula=\"20\""),
    );
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    let got: Vec<(String, u32, Option<String>)> =
        sheet_cf_priorities(&out, "xl/worksheets/sheet1.xml")
            .into_iter()
            .map(|(sqref, prio, _, f)| (sqref, prio, f))
            .collect();
    assert_eq!(
        got,
        [
            ("A1".into(), 1, Some("1".into())),
            ("B1".into(), 2, Some("20".into())),
            ("A1".into(), 3, Some("3".into())),
        ]
    );
}

#[test]
fn priorities_not_in_document_order_import_in_priority_order_and_round_trip() {
    let bytes = corpus_file("WithConditionalFormatting.xlsx");
    let orig = sheet_cf_priorities(&bytes, "xl/worksheets/sheet1.xml");
    assert_eq!(
        orig.iter()
            .map(|(r, p, _, _)| (r.as_str(), *p))
            .collect::<Vec<_>>(),
        [
            ("A1:A8", 5),
            ("A1:A8", 6),
            ("B9", 3),
            ("B1:B7", 1),
            ("B1:B7", 2)
        ]
    );
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let lines = cf_lines(&imp.text);
    assert_eq!(
        lines
            .iter()
            .map(|l| cf_attr(l, "range"))
            .collect::<Vec<_>>(),
        ["B1:B7", "B1:B7", "B9", "A1:A8", "A1:A8"]
    );
    assert_eq!(cf_attr(&lines[0], "operator"), "lessThanOrEqual");
    assert!(
        lines[1].contains("formula2="),
        "between omits operator: {}",
        lines[1]
    );
    assert_eq!(cf_attr(&lines[2], "type"), "expression");
    assert_eq!(cf_attr(&lines[3], "operator"), "greaterThan");
    assert_eq!(cf_attr(&lines[4], "operator"), "lessThan");
    let out = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
    getput(&bytes, &out).unwrap();
    assert_eq!(sheet_cf_priorities(&out, "xl/worksheets/sheet1.xml"), orig);
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(again.text, imp.text);
}
