//! Cell notes as `<note/>` lines: parse/print, edit text, add/remove, GetPut
//! of an unedited file.

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

fn note_lines(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| l.starts_with("<note "))
        .map(str::to_string)
        .collect()
}

fn comments_of(bytes: &[u8]) -> Vec<(String, String, String)> {
    let parts = package::read(bytes).unwrap();
    let data = parts
        .iter()
        .find(|p| xml::parse(&p.data).is_ok_and(|d| d.root.local() == "comments"))
        .unwrap_or_else(|| {
            panic!(
                "the package has a comments part: {:?}",
                parts.iter().map(|p| &p.name).collect::<Vec<_>>()
            )
        });
    let doc = xml::parse(&data.data).unwrap();
    let authors: Vec<String> = doc
        .root
        .elements()
        .find(|e| e.local() == "authors")
        .map(|a| {
            a.elements()
                .filter(|e| e.local() == "author")
                .map(|e| e.text_of(&[e.name.as_str()]))
                .collect()
        })
        .unwrap_or_default();
    let list = doc
        .root
        .elements()
        .find(|e| e.local() == "commentList")
        .expect("commentList");
    list.elements()
        .filter(|e| e.local() == "comment")
        .map(|c| {
            let r = c.get("ref").unwrap_or_default();
            let author = c
                .get("authorId")
                .and_then(|id| id.parse::<usize>().ok())
                .and_then(|k| authors.get(k).cloned())
                .unwrap_or_default();
            let mut text = String::new();
            c.walk(&mut |e| {
                if e.local() == "t" {
                    for n in &e.children {
                        if let xml::Node::Text(t) = n {
                            text.push_str(&xml::unescape(t));
                        }
                    }
                }
            });
            (r, author, text)
        })
        .collect()
}

#[test]
fn unedited_file_keeps_its_note_parts() {
    let bytes = corpus_file("rx_comment01.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    assert_eq!(
        note_lines(&imp.text),
        ["<note range=\"B2\" author=\"John\" text=\"Some text\"/>"]
    );
    let out = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
    getput(&bytes, &out).unwrap();
}

#[test]
fn editing_note_text_writes_it_and_reads_back() {
    let bytes = corpus_file("rx_comment01.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let edited = imp.text.replace("Some text", "Edited note");
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    assert_eq!(
        note_lines(&a.text),
        ["<note range=\"B2\" author=\"John\" text=\"Edited note\"/>"]
    );
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    let comments = comments_of(&out);
    assert_eq!(
        comments,
        [("B2".into(), "John".into(), "Edited note".into())]
    );
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(again.text, a.text);
}

#[test]
fn adding_a_note_writes_it_and_reads_back() {
    let bytes = corpus_file("rx_comment01.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let edited = imp.text.replace(
        "<note range=\"B2\" author=\"John\" text=\"Some text\"/>",
        "<note range=\"B2\" author=\"John\" text=\"Some text\"/>\n<note range=\"C3\" author=\"Ada\" text=\"A new note\"/>",
    );
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    assert_eq!(
        note_lines(&a.text),
        [
            "<note range=\"B2\" author=\"John\" text=\"Some text\"/>",
            "<note range=\"C3\" author=\"Ada\" text=\"A new note\"/>"
        ]
    );
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    let comments = comments_of(&out);
    assert_eq!(comments.len(), 2, "{comments:?}");
    assert!(
        comments
            .iter()
            .any(|(r, a, t)| r == "C3" && a == "Ada" && t == "A new note"),
        "{comments:?}"
    );
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(again.text, a.text);
}

#[test]
fn removing_a_note_drops_it() {
    let bytes = corpus_file("SimpleWithComments.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let before = note_lines(&imp.text);
    assert!(before.len() >= 2, "{before:?}");
    let first = before[0].clone();
    let edited = imp.text.replacen(&format!("{first}\n"), "", 1);
    assert_ne!(edited, imp.text);
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    let after = note_lines(&a.text);
    assert_eq!(after.len(), before.len() - 1, "{after:?}");
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    let comments = comments_of(&out);
    assert_eq!(comments.len(), after.len(), "{comments:?}");
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(again.text, a.text);
}

#[test]
fn adding_the_first_note_to_a_sheet_without_any() {
    let bytes = corpus_file("rx_hyperlink01.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    assert!(note_lines(&imp.text).is_empty(), "{}", imp.text);
    let edited = imp.text.replace(
        "</sheet>",
        "<note range=\"A1\" author=\"Ada\" text=\"hello\"/>\n</sheet>",
    );
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    assert_eq!(
        note_lines(&a.text),
        ["<note range=\"A1\" author=\"Ada\" text=\"hello\"/>"]
    );
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    assert_eq!(
        comments_of(&out),
        [("A1".into(), "Ada".into(), "hello".into())]
    );
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(again.text, a.text);
}

#[test]
fn overlapping_notes_are_refused() {
    let bytes = corpus_file("rx_comment01.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let overlap = imp.text.replace(
        "<note range=\"B2\" author=\"John\" text=\"Some text\"/>",
        "<note range=\"B2\" author=\"John\" text=\"Some text\"/>\n<note range=\"B2\" author=\"Ada\" text=\"other\"/>",
    );
    let e = XlsxEngine::apply(&overlap, &imp.remainder, "[]").unwrap_err();
    let msg = e.to_string();
    assert!(
        msg.contains("overlaps B2") && msg.contains("do not overlap"),
        "{msg}"
    );
}

#[test]
fn corpus_note_files_round_trip_the_text() {
    for name in [
        "rx_comment01.xlsx",
        "SimpleWithComments.xlsx",
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
            !note_lines(&imp.text).is_empty(),
            "{name} should show <note/> lines:\n{}",
            imp.text
        );
        assert!(
            !imp.text.contains("kind=\"notes\""),
            "{name} still has a notes placeholder:\n{}",
            imp.text
        );
        let names = XlsxEngine::names(&imp.remainder);
        let parsed = parse_spreadsheet(&imp.text, &names).unwrap();
        assert_eq!(serialize_spreadsheet(&parsed), imp.text, "{name}");
        assert!(
            parsed
                .sheets
                .iter()
                .any(|s| s.items.iter().any(|i| matches!(i, SheetItem::Note(_)))),
            "{name}"
        );
        let out = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
        if name.ends_with(".xlsm") {
            let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
            assert_eq!(again.text, imp.text, "{name}");
        } else {
            getput(&bytes, &out).unwrap();
        }
    }
}
