//! Cell hyperlinks as `<hyperlink/>` lines: parse/print, edit a target,
//! add/remove, GetPut of an unedited file.

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

fn hyperlink_lines(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| l.starts_with("<hyperlink "))
        .map(str::to_string)
        .collect()
}

fn sheet_hyperlinks(bytes: &[u8], part: &str) -> Vec<(String, Option<String>, Option<String>)> {
    let parts = package::read(bytes).unwrap();
    let data = parts.iter().find(|p| p.name == part).unwrap();
    let doc = xml::parse(&data.data).unwrap();
    let list = doc
        .root
        .elements()
        .find(|e| e.local() == "hyperlinks")
        .expect("the worksheet has <hyperlinks>");
    let rels = sheet_rels(&parts, part);
    list.elements()
        .filter(|e| e.local() == "hyperlink")
        .map(|e| {
            let r = e.get("ref").unwrap();
            let rid = e
                .attrs
                .iter()
                .find(|a| a.0.ends_with(":id"))
                .map(|a| xml::unescape(&a.1));
            let target = rid.and_then(|id| rels.iter().find(|x| x.0 == id).map(|x| x.1.clone()));
            (r, target, e.get("display"))
        })
        .collect()
}

fn sheet_rels(parts: &[hanji_core::Part], part: &str) -> Vec<(String, String)> {
    let rels_name = match part.rsplit_once('/') {
        Some((d, f)) => format!("{d}/_rels/{f}.rels"),
        None => format!("_rels/{part}.rels"),
    };
    let Some(data) = parts.iter().find(|p| p.name == rels_name) else {
        return vec![];
    };
    let doc = xml::parse(&data.data).unwrap();
    doc.root
        .elements()
        .filter(|e| e.local() == "Relationship")
        .map(|e| {
            (
                e.get("Id").unwrap_or_default(),
                e.get("Target").unwrap_or_default(),
            )
        })
        .collect()
}

#[test]
fn unedited_file_keeps_its_hyperlink_xml() {
    let bytes = corpus_file("rx_hyperlink01.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    assert_eq!(
        hyperlink_lines(&imp.text),
        ["<hyperlink range=\"A1\" target=\"http://www.perl.org/\"/>"]
    );
    let out = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
    getput(&bytes, &out).unwrap();
}

#[test]
fn editing_a_hyperlink_target_writes_it_and_reads_back() {
    let bytes = corpus_file("rx_hyperlink01.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let edited = imp
        .text
        .replace("http://www.perl.org/", "https://example.com/docs");
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    assert_eq!(
        hyperlink_lines(&a.text),
        ["<hyperlink range=\"A1\" target=\"https://example.com/docs\"/>"]
    );
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    assert_eq!(
        sheet_hyperlinks(&out, "xl/worksheets/sheet1.xml"),
        [("A1".into(), Some("https://example.com/docs".into()), None)]
    );
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(again.text, a.text);
}

#[test]
fn adding_a_hyperlink_writes_it_and_reads_back() {
    let bytes = corpus_file("rx_hyperlink01.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let edited = imp.text.replace(
        "<hyperlink range=\"A1\" target=\"http://www.perl.org/\"/>",
        "<hyperlink range=\"A1\" target=\"http://www.perl.org/\"/>\n<hyperlink range=\"B2\" target=\"https://example.com/\" tooltip=\"go\"/>",
    );
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    assert_eq!(
        hyperlink_lines(&a.text),
        [
            "<hyperlink range=\"A1\" target=\"http://www.perl.org/\"/>",
            "<hyperlink range=\"B2\" target=\"https://example.com/\" tooltip=\"go\"/>"
        ]
    );
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    let got = sheet_hyperlinks(&out, "xl/worksheets/sheet1.xml");
    assert_eq!(got.len(), 2, "{got:?}");
    assert_eq!(got[0].0, "A1");
    assert_eq!(got[0].1.as_deref(), Some("http://www.perl.org/"));
    assert_eq!(got[1].0, "B2");
    assert_eq!(got[1].1.as_deref(), Some("https://example.com/"));
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(again.text, a.text);
}

#[test]
fn removing_a_hyperlink_drops_it() {
    let bytes = corpus_file("sharedhyperlink.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let before = hyperlink_lines(&imp.text);
    assert!(before.len() >= 2, "{before:?}");
    let first = before[0].clone();
    let edited = imp.text.replacen(&format!("{first}\n"), "", 1);
    assert_ne!(edited, imp.text);
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    let after = hyperlink_lines(&a.text);
    assert_eq!(after.len(), before.len() - 1, "{after:?}");
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    let refs: Vec<_> = sheet_hyperlinks(&out, "xl/worksheets/sheet1.xml")
        .into_iter()
        .map(|(r, _, _)| r)
        .collect();
    assert_eq!(refs.len(), after.len(), "{refs:?}");
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(again.text, a.text);
}

#[test]
fn overlapping_hyperlinks_are_refused() {
    let bytes = corpus_file("rx_hyperlink01.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let overlap = imp.text.replace(
        "<hyperlink range=\"A1\" target=\"http://www.perl.org/\"/>",
        "<hyperlink range=\"A1\" target=\"http://www.perl.org/\"/>\n<hyperlink range=\"A1:B2\" target=\"https://example.com/\"/>",
    );
    let e = XlsxEngine::apply(&overlap, &imp.remainder, "[]").unwrap_err();
    let msg = e.to_string();
    assert!(
        msg.contains("overlaps A1") && msg.contains("do not overlap"),
        "{msg}"
    );
}

#[test]
fn corpus_hyperlink_files_round_trip_the_text() {
    for name in [
        "rx_hyperlink01.xlsx",
        "sharedhyperlink.xlsx",
        "WithVariousData.xlsx",
        "korean-sales.xlsx",
        "korean-sales-lo.xlsx",
        "rx_table05.xlsx",
        "ExcelWithAttachments.xlsm",
    ] {
        let bytes = corpus_file(name);
        let imp = XlsxEngine
            .import(&bytes, &ImportOptions::default())
            .unwrap();
        assert!(
            !hyperlink_lines(&imp.text).is_empty(),
            "{name} should show <hyperlink/> lines"
        );
        assert!(
            !imp.text.contains("kind=\"hyperlinks\""),
            "{name} still has a hyperlinks placeholder:\n{}",
            imp.text
        );
        let names = XlsxEngine::names(&imp.remainder);
        let parsed = parse_spreadsheet(&imp.text, &names).unwrap();
        assert_eq!(serialize_spreadsheet(&parsed), imp.text, "{name}");
        assert!(
            parsed
                .sheets
                .iter()
                .any(|s| s.items.iter().any(|i| matches!(i, SheetItem::Hyperlink(_)))),
            "{name}"
        );
        let out = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
        if name.ends_with(".xlsm") {
            // Macros and OLE are stripped on import; GetPut vs the original
            // package is the corpus test.
            let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
            assert_eq!(again.text, imp.text, "{name}");
        } else {
            getput(&bytes, &out).unwrap();
        }
    }
}
