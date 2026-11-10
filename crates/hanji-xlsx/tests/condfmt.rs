//! Conditional formats as `<conditional-format/>` lines: parse/print, edit,
//! add/remove, GetPut of an unedited file. `dataBar` is a line with
//! `min`/`max`/`fill` (round 7 A); x14 extensions stay when unchanged.

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

fn sheet_xml(bytes: &[u8], part: &str) -> String {
    let parts = package::read(bytes).unwrap();
    let data = parts.iter().find(|p| p.name == part).unwrap();
    String::from_utf8(data.data.clone()).unwrap()
}

fn with_cf_and_ext(bytes: &[u8], cf: &str, ext: &str) -> Vec<u8> {
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
    let mut next = format!("{}{}{}", &s[..start], cf, &s[end..]);
    if let Some(i) = next.rfind("</worksheet>") {
        next.insert_str(i, ext);
    }
    sheet.data = next.into_bytes();
    package::write(&parts).unwrap()
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
fn a_data_bar_is_a_line_and_round_trips() {
    let bytes = corpus_file("simple-monthly-budget.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let lines = cf_lines(&imp.text);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(cf_attr(&lines[0], "range"), "E4");
    assert_eq!(cf_attr(&lines[0], "type"), "dataBar");
    assert_eq!(cf_attr(&lines[0], "min"), "num:0");
    assert_eq!(cf_attr(&lines[0], "max"), "num:TotalMonthlyIncome");
    assert_eq!(cf_attr(&lines[0], "fill"), "accent1");
    assert!(
        !imp.text.contains("kind=\"conditional-format\""),
        "{}",
        imp.text
    );
    let names = XlsxEngine::names(&imp.remainder);
    let parsed = parse_spreadsheet(&imp.text, &names).unwrap();
    assert_eq!(serialize_spreadsheet(&parsed), imp.text);
    let out = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
    getput(&bytes, &out).unwrap();
    let sheet = sheet_xml(&out, "xl/worksheets/sheet1.xml");
    assert!(
        sheet.contains("gradient=\"0\"") && sheet.contains("negativeFillColor"),
        "unedited x14 dataBar extensions stay: {sheet}"
    );
}

#[test]
fn editing_a_data_bar_range_min_and_fill_keeps_x14_extensions() {
    let bytes = corpus_file("simple-monthly-budget.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let first = cf_lines(&imp.text)[0].clone();
    let edited = imp.text.replace(
        &first,
        r##"<conditional-format range="E4:E10" type="dataBar" min="num:0" max="max" fill="#FFC7CE"/>"##,
    );
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    let lines = cf_lines(&a.text);
    assert_eq!(cf_attr(&lines[0], "range"), "E4:E10");
    assert_eq!(cf_attr(&lines[0], "max"), "max");
    assert_eq!(cf_attr(&lines[0], "fill"), "#FFC7CE");
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    let sheet = sheet_xml(&out, "xl/worksheets/sheet1.xml");
    assert!(sheet.contains("sqref=\"E4:E10\""), "{sheet}");
    assert!(
        sheet.contains("<xm:sqref>E4:E10</xm:sqref>") || sheet.contains(">E4:E10</"),
        "x14 sqref follows the range: {sheet}"
    );
    assert!(
        sheet.contains("type=\"max\"") || sheet.contains("type=\"autoMax\""),
        "max cfvo: {sheet}"
    );
    assert!(
        sheet.contains("rgb=\"FFFFC7CE\"") || sheet.contains("rgb=\"FFC7CE\""),
        "fill colour: {sheet}"
    );
    assert!(
        sheet.contains("gradient=\"0\"")
            && sheet.contains("negativeFillColor")
            && sheet.contains("axisColor"),
        "x14 extras stay on edit: {sheet}"
    );
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(again.text, a.text);
}

#[test]
fn removing_the_corpus_data_bar_drops_its_x14() {
    let bytes = corpus_file("simple-monthly-budget.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let first = cf_lines(&imp.text)[0].clone();
    let edited = imp.text.replace(&format!("{first}\n"), "");
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    assert!(cf_lines(&a.text).is_empty(), "{:?}", a.text);
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    let sheet = sheet_xml(&out, "xl/worksheets/sheet1.xml");
    assert!(!sheet.contains("conditionalFormatting"), "{sheet}");
    assert!(
        !sheet.contains("conditionalFormattings"),
        "x14 dataBar goes with the rule: {sheet}"
    );
}

#[test]
fn adding_and_removing_a_data_bar() {
    let bytes = corpus_file("rx_cond_format01.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let added = imp.text.replace(
        "</sheet>",
        "<conditional-format range=\"A2:A4\" type=\"dataBar\" min=\"min\" max=\"max\" fill=\"#638EC6\"/>\n</sheet>",
    );
    let a = XlsxEngine::apply(&added, &imp.remainder, "[]").unwrap();
    assert_eq!(cf_lines(&a.text).len(), 2, "{:?}", cf_lines(&a.text));
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    let rules = sheet_cf_rules(&out, "xl/worksheets/sheet1.xml");
    assert!(
        rules
            .iter()
            .any(|(r, ty, _)| r == "A2:A4" && ty == "dataBar"),
        "{rules:?}"
    );
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    let bar = cf_lines(&again.text)
        .into_iter()
        .find(|l| l.contains("type=\"dataBar\""))
        .unwrap();
    let gone = again.text.replace(&format!("{bar}\n"), "");
    let b = XlsxEngine::apply(&gone, &again.remainder, "[]").unwrap();
    assert!(
        cf_lines(&b.text)
            .iter()
            .all(|l| !l.contains("type=\"dataBar\"")),
        "{:?}",
        cf_lines(&b.text)
    );
    let out = XlsxEngine.export(&b.text, &b.remainder).unwrap();
    let sheet = sheet_xml(&out, "xl/worksheets/sheet1.xml");
    assert!(!sheet.contains("type=\"dataBar\""), "{sheet}");
    assert!(
        !sheet.contains("conditionalFormattings"),
        "x14 dataBar goes with the rule: {sheet}"
    );
}

#[test]
fn a_color_scale_stays_a_placeholder() {
    let bytes = with_cf_xml(
        &corpus_file("rx_cond_format01.xlsx"),
        concat!(
            r#"<conditionalFormatting sqref="A1">"#,
            r#"<cfRule type="colorScale" priority="1">"#,
            r#"<colorScale>"#,
            r#"<cfvo type="min"/><cfvo type="max"/>"#,
            r#"<color rgb="FFFF0000"/><color rgb="FF00FF00"/>"#,
            r#"</colorScale></cfRule></conditionalFormatting>"#,
        ),
    );
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    assert!(
        cf_lines(&imp.text).is_empty(),
        "color scales must not be listed as text:\n{}",
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
fn a_data_bar_whose_x14_cannot_be_patched_stays_a_placeholder() {
    let bytes = with_cf_and_ext(
        &corpus_file("rx_cond_format01.xlsx"),
        concat!(
            r#"<conditionalFormatting sqref="A1">"#,
            r#"<cfRule type="dataBar" priority="1">"#,
            r#"<dataBar><cfvo type="min"/><cfvo type="max"/><color rgb="FF638EC6"/></dataBar>"#,
            r#"<extLst><ext uri="{B025F937-C7B1-47D3-B67F-A62EFF666E3E}" xmlns:x14="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main">"#,
            r#"<x14:id>{11111111-1111-1111-1111-111111111111}</x14:id></ext></extLst>"#,
            r#"</cfRule></conditionalFormatting>"#,
        ),
        concat!(
            r#"<extLst><ext uri="{78C0D931-6437-407d-A8EE-F0AAD7539E65}" xmlns:x14="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main">"#,
            r#"<x14:conditionalFormattings><x14:conditionalFormatting xmlns:xm="http://schemas.microsoft.com/office/excel/2006/main">"#,
            r#"<x14:cfRule type="dataBar" id="{11111111-1111-1111-1111-111111111111}">"#,
            r#"<x14:dataBar minLength="0" maxLength="100">"#,
            r#"<x14:cfvo type="min"/><x14:cfvo type="max"/><x14:cfvo type="percent"><xm:f>50</xm:f></x14:cfvo>"#,
            r#"<x14:negativeFillColor rgb="FFFF0000"/>"#,
            r#"</x14:dataBar></x14:cfRule><xm:sqref>A1</xm:sqref>"#,
            r#"</x14:conditionalFormatting></x14:conditionalFormattings></ext></extLst>"#,
        ),
    );
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    assert!(
        cf_lines(&imp.text).is_empty(),
        "an x14 dataBar with three cfvo cannot be a line:\n{}",
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
fn data_bar_and_cellis_share_priority_order() {
    let bytes = with_cf_xml(
        &corpus_file("rx_cond_format01.xlsx"),
        concat!(
            r#"<conditionalFormatting sqref="A1">"#,
            r#"<cfRule type="cellIs" dxfId="0" priority="2" operator="greaterThan">"#,
            r#"<formula>5</formula></cfRule>"#,
            r#"</conditionalFormatting>"#,
            r#"<conditionalFormatting sqref="B1">"#,
            r#"<cfRule type="dataBar" priority="1">"#,
            r#"<dataBar><cfvo type="num" val="0"/><cfvo type="max"/><color theme="4"/></dataBar>"#,
            r#"</cfRule></conditionalFormatting>"#,
        ),
    );
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let lines = cf_lines(&imp.text);
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert_eq!(cf_attr(&lines[0], "type"), "dataBar");
    assert_eq!(cf_attr(&lines[0], "range"), "B1");
    assert_eq!(cf_attr(&lines[1], "type"), "cellIs");
    assert_eq!(cf_attr(&lines[1], "range"), "A1");
    let edited = imp.text.replace(
        &lines[1],
        &lines[1].replace("formula=\"5\"", "formula=\"9\""),
    );
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    let got: Vec<(String, u32, String)> = sheet_cf_priorities(&out, "xl/worksheets/sheet1.xml")
        .into_iter()
        .map(|(sqref, prio, ty, _)| (sqref, prio, ty))
        .collect();
    assert_eq!(
        got,
        [
            ("B1".into(), 1, "dataBar".into()),
            ("A1".into(), 2, "cellIs".into()),
        ]
    );
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
