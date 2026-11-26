//! Data validations as `<validation/>` lines: parse/print, edit, add/remove,
//! GetPut of an unedited file.

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

fn validation_lines(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| l.starts_with("<validation "))
        .map(str::to_string)
        .collect()
}

fn validation_attr(bytes: &[u8], part: &str, sqref: &str, attr: &str) -> Option<String> {
    let parts = package::read(bytes).unwrap();
    let data = parts.iter().find(|p| p.name == part).unwrap();
    let doc = xml::parse(&data.data).unwrap();
    let list = doc
        .root
        .elements()
        .find(|e| e.local() == "dataValidations")
        .expect("the worksheet has <dataValidations>");
    let v = list
        .elements()
        .find(|e| e.local() == "dataValidation" && e.get("sqref").as_deref() == Some(sqref))
        .and_then(|e| e.get(attr));
    v
}

fn sheet_validations(bytes: &[u8], part: &str) -> Vec<(String, String, Option<String>)> {
    let parts = package::read(bytes).unwrap();
    let data = parts.iter().find(|p| p.name == part).unwrap();
    let doc = xml::parse(&data.data).unwrap();
    let list = doc
        .root
        .elements()
        .find(|e| e.local() == "dataValidations")
        .expect("the worksheet has <dataValidations>");
    list.elements()
        .filter(|e| e.local() == "dataValidation")
        .map(|e| {
            let sqref = e.get("sqref").unwrap();
            let ty = e.get("type").unwrap();
            let f1 = e
                .elements()
                .find(|c| c.local() == "formula1")
                .map(|c| c.text_of(&[c.name.as_str()]));
            (sqref, ty, f1)
        })
        .collect()
}

#[test]
fn unedited_file_keeps_its_validation_xml() {
    let bytes = corpus_file("rx_data_validation01.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    assert_eq!(
        validation_lines(&imp.text),
        ["<validation range=\"C2\" type=\"list\" formula1=\"&quot;Foo,Bar,Baz&quot;\" allowBlank=\"true\"/>"]
    );
    let out = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
    getput(&bytes, &out).unwrap();
}

#[test]
fn editing_a_validation_writes_it_and_reads_back() {
    let bytes = corpus_file("rx_data_validation01.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let edited = imp.text.replace("Foo,Bar,Baz", "A,B,C");
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    assert_eq!(
        validation_lines(&a.text),
        ["<validation range=\"C2\" type=\"list\" formula1=\"&quot;A,B,C&quot;\" allowBlank=\"true\"/>"]
    );
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    let got = sheet_validations(&out, "xl/worksheets/sheet1.xml");
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0].0, "C2");
    assert_eq!(got[0].1, "list");
    assert_eq!(got[0].2.as_deref(), Some("\"A,B,C\""));
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(again.text, a.text);
}

#[test]
fn adding_a_validation_writes_it_and_reads_back() {
    let bytes = corpus_file("rx_data_validation01.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let edited = imp.text.replace(
        "<validation range=\"C2\" type=\"list\" formula1=\"&quot;Foo,Bar,Baz&quot;\" allowBlank=\"true\"/>",
        "<validation range=\"C2\" type=\"list\" formula1=\"&quot;Foo,Bar,Baz&quot;\" allowBlank=\"true\"/>\n<validation range=\"D2\" type=\"whole\" operator=\"greaterThan\" formula1=\"0\" allowBlank=\"true\"/>",
    );
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    assert_eq!(
        validation_lines(&a.text),
        [
            "<validation range=\"C2\" type=\"list\" formula1=\"&quot;Foo,Bar,Baz&quot;\" allowBlank=\"true\"/>",
            "<validation range=\"D2\" type=\"whole\" operator=\"greaterThan\" formula1=\"0\" allowBlank=\"true\"/>"
        ]
    );
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    let got = sheet_validations(&out, "xl/worksheets/sheet1.xml");
    assert_eq!(got.len(), 2, "{got:?}");
    assert_eq!(got[1].0, "D2");
    assert_eq!(got[1].1, "whole");
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(again.text, a.text);
}

#[test]
fn removing_a_validation_drops_it() {
    let bytes = corpus_file("rx_data_validation01.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let edited = imp.text.replace(
        "<validation range=\"C2\" type=\"list\" formula1=\"&quot;Foo,Bar,Baz&quot;\" allowBlank=\"true\"/>\n",
        "",
    );
    assert_ne!(edited, imp.text);
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    assert!(validation_lines(&a.text).is_empty(), "{:?}", a.text);
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    let parts = package::read(&out).unwrap();
    let sheet = parts
        .iter()
        .find(|p| p.name == "xl/worksheets/sheet1.xml")
        .unwrap();
    let doc = xml::parse(&sheet.data).unwrap();
    assert!(
        doc.root.elements().all(|e| e.local() != "dataValidations"),
        "dataValidations should go with the last rule"
    );
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(again.text, a.text);
}

#[test]
fn corpus_validation_files_round_trip_the_text() {
    for name in [
        "rx_data_validation01.xlsx",
        "DataValidations-49244.xlsx",
        "korean-sales.xlsx",
        "korean-sales-lo.xlsx",
    ] {
        let bytes = corpus_file(name);
        let imp = XlsxEngine
            .import(&bytes, &ImportOptions::default())
            .unwrap();
        assert!(
            !validation_lines(&imp.text).is_empty(),
            "{name} should show <validation/> lines:\n{}",
            imp.text
        );
        assert!(
            !imp.text.contains("kind=\"data-validation\""),
            "{name} still has a data-validation placeholder:\n{}",
            imp.text
        );
        let names = XlsxEngine::names(&imp.remainder);
        let parsed = parse_spreadsheet(&imp.text, &names).unwrap();
        assert_eq!(serialize_spreadsheet(&parsed), imp.text, "{name}");
        assert!(
            parsed.sheets.iter().any(|s| s
                .items
                .iter()
                .any(|i| matches!(i, SheetItem::Validation(_)))),
            "{name}"
        );
        let out = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
        getput(&bytes, &out).unwrap();
    }
}

fn with_list_formula1(bytes: &[u8], inner: &str) -> Vec<u8> {
    let mut parts = package::read(bytes).unwrap();
    let sheet = parts
        .iter_mut()
        .find(|p| p.name == "xl/worksheets/sheet1.xml")
        .unwrap();
    let s = String::from_utf8(sheet.data.clone()).unwrap();
    let next = s.replace(
        r#"<formula1>"Foo,Bar,Baz"</formula1>"#,
        &format!("<formula1>{inner}</formula1>"),
    );
    assert_ne!(next, s, "formula1 replacement missed");
    sheet.data = next.into_bytes();
    package::write(&parts).unwrap()
}

#[test]
fn a_one_item_list_literal_keeps_its_quotes() {
    let bytes = with_list_formula1(&corpus_file("rx_data_validation01.xlsx"), r#""Yes""#);
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    assert_eq!(
        validation_lines(&imp.text),
        ["<validation range=\"C2\" type=\"list\" formula1=\"&quot;Yes&quot;\" allowBlank=\"true\"/>"]
    );
    let out = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
    getput(&bytes, &out).unwrap();
    let got = sheet_validations(&out, "xl/worksheets/sheet1.xml");
    assert_eq!(got[0].2.as_deref(), Some("\"Yes\""));
}

#[test]
fn an_edited_or_new_list_literal_stays_quoted() {
    let bytes = with_list_formula1(&corpus_file("rx_data_validation01.xlsx"), r#""Yes""#);
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let edited = imp.text.replace(
        "<validation range=\"C2\" type=\"list\" formula1=\"&quot;Yes&quot;\" allowBlank=\"true\"/>",
        "<validation range=\"C2\" type=\"list\" formula1=\"&quot;Open&quot;\" allowBlank=\"true\"/>\n<validation range=\"D2\" type=\"list\" formula1=\"&quot;No&quot;\"/>",
    );
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    assert_eq!(
        validation_lines(&a.text),
        [
            "<validation range=\"C2\" type=\"list\" formula1=\"&quot;Open&quot;\" allowBlank=\"true\"/>",
            "<validation range=\"D2\" type=\"list\" formula1=\"&quot;No&quot;\"/>"
        ]
    );
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    let got = sheet_validations(&out, "xl/worksheets/sheet1.xml");
    assert_eq!(
        got.iter().map(|v| v.2.as_deref()).collect::<Vec<_>>(),
        [Some("\"Open\""), Some("\"No\"")]
    );
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(again.text, a.text);
}

#[test]
fn a_concatenated_list_formula_round_trips_unchanged() {
    let bytes = with_list_formula1(&corpus_file("rx_data_validation01.xlsx"), r#""A"&amp;"B""#);
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    assert_eq!(
        validation_lines(&imp.text),
        ["<validation range=\"C2\" type=\"list\" formula1=\"&quot;A&quot;&amp;&quot;B&quot;\" allowBlank=\"true\"/>"]
    );
    let out = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
    getput(&bytes, &out).unwrap();
    let got = sheet_validations(&out, "xl/worksheets/sheet1.xml");
    assert_eq!(got[0].2.as_deref(), Some("\"A\"&\"B\""));
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(again.text, imp.text);
}

#[test]
fn a_list_range_reference_stays_unquoted() {
    let bytes = with_list_formula1(&corpus_file("rx_data_validation01.xlsx"), "$A$1:$A$5");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    assert_eq!(
        validation_lines(&imp.text),
        ["<validation range=\"C2\" type=\"list\" formula1=\"$A$1:$A$5\" allowBlank=\"true\"/>"]
    );
    let out = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
    getput(&bytes, &out).unwrap();
    let got = sheet_validations(&out, "xl/worksheets/sheet1.xml");
    assert_eq!(got[0].2.as_deref(), Some("$A$1:$A$5"));

    let corpus = corpus_file("DataValidations-49244.xlsx");
    let imp = XlsxEngine
        .import(&corpus, &ImportOptions::default())
        .unwrap();
    let lines = validation_lines(&imp.text);
    assert!(
        lines.iter().any(|l| l.contains("formula1=\"$D$1:$F$1\"")),
        "{lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("formula1=\"&quot;IN,US,UK&quot;\"")),
        "{lines:?}"
    );
    let out = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
    getput(&corpus, &out).unwrap();
    let got = sheet_validations(&out, "xl/worksheets/sheet1.xml");
    assert!(
        got.iter()
            .any(|v| v.1 == "list" && v.2.as_deref() == Some("$D$1:$F$1")),
        "{got:?}"
    );
    assert!(
        got.iter()
            .any(|v| v.1 == "list" && v.2.as_deref() == Some("\"IN,US,UK\"")),
        "{got:?}"
    );
}

#[test]
fn a_new_validation_writes_show_error_message_on() {
    let bytes = corpus_file("rx_data_validation01.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let edited = imp.text.replace(
        "<validation range=\"C2\" type=\"list\" formula1=\"&quot;Foo,Bar,Baz&quot;\" allowBlank=\"true\"/>",
        "<validation range=\"C2\" type=\"list\" formula1=\"&quot;Foo,Bar,Baz&quot;\" allowBlank=\"true\"/>\n<validation range=\"D2\" type=\"whole\" operator=\"greaterThan\" formula1=\"0\"/>",
    );
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    assert!(
        a.text.contains(
            "<validation range=\"D2\" type=\"whole\" operator=\"greaterThan\" formula1=\"0\"/>"
        ),
        "{}",
        a.text
    );
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    assert_eq!(
        validation_attr(&out, "xl/worksheets/sheet1.xml", "D2", "showErrorMessage").as_deref(),
        Some("1")
    );
    assert_eq!(
        validation_attr(&out, "xl/worksheets/sheet1.xml", "D2", "showInputMessage").as_deref(),
        Some("1")
    );
}

#[test]
fn an_existing_show_error_message_on_survives_an_edit() {
    let bytes = corpus_file("rx_data_validation01.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    assert_eq!(
        validation_attr(&bytes, "xl/worksheets/sheet1.xml", "C2", "showErrorMessage").as_deref(),
        Some("1")
    );
    let edited = imp.text.replace("Foo,Bar,Baz", "A,B,C");
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    assert_eq!(
        validation_attr(&out, "xl/worksheets/sheet1.xml", "C2", "showErrorMessage").as_deref(),
        Some("1")
    );
    assert_eq!(
        validation_attr(&out, "xl/worksheets/sheet1.xml", "C2", "showInputMessage").as_deref(),
        Some("1")
    );
}

#[test]
fn an_absent_show_error_message_reads_as_false_and_round_trips() {
    let orig = corpus_file("rx_data_validation01.xlsx");
    let mut parts = package::read(&orig).unwrap();
    let sheet = parts
        .iter_mut()
        .find(|p| p.name == "xl/worksheets/sheet1.xml")
        .unwrap();
    let s = String::from_utf8(sheet.data.clone()).unwrap();
    let next = s
        .replace(r#" showInputMessage="1""#, "")
        .replace(r#" showErrorMessage="1""#, "");
    assert_ne!(next, s);
    sheet.data = next.into_bytes();
    let bytes = package::write(&parts).unwrap();
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    assert_eq!(
        validation_lines(&imp.text),
        ["<validation range=\"C2\" type=\"list\" formula1=\"&quot;Foo,Bar,Baz&quot;\" allowBlank=\"true\" showInputMessage=\"false\" showErrorMessage=\"false\"/>"]
    );
    let out = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
    getput(&bytes, &out).unwrap();
    assert!(validation_attr(&out, "xl/worksheets/sheet1.xml", "C2", "showErrorMessage").is_none());
    assert!(validation_attr(&out, "xl/worksheets/sheet1.xml", "C2", "showInputMessage").is_none());
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(again.text, imp.text);
}
