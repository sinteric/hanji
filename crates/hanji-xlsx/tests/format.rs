//! Cell formatting (DESIGN.md §5.4, round 6 part D): the `<format …/>` lines
//! a workbook's text shows, and the `format` range operation that writes
//! them. A value left as the file has it keeps its XML; a format operation
//! changes only the cells of its range and only the keys it sets, reusing
//! the file's fonts, fills, borders and cell styles where one fits (PutGet
//! on the whole corpus); what a workbook cannot hold is refused.

use hanji_core::{Engine, EngineError, ImportOptions};
use hanji_xlsx::{package, xml, XlsxEngine};

mod common;
use common::*;

fn open(name: &str) -> hanji_core::Imported {
    let bytes = corpus().into_iter().find(|(n, _)| n == name).unwrap_or_else(|| panic!("{name}")).1;
    XlsxEngine.import(&bytes, &ImportOptions::default()).unwrap()
}

fn part(pkg: &[u8], name: &str) -> String {
    String::from_utf8(package::get(&package::read(pkg).unwrap(), name).unwrap().to_vec()).unwrap()
}

/// The elements of list `list` (`fonts`, `cellXfs`) in styles.xml.
fn items(styles: &str, list: &str) -> Vec<String> {
    let doc = xml::parse(styles.as_bytes()).unwrap();
    let l = doc.root.elements().find(|e| e.local() == list).unwrap();
    l.elements().map(|e| e.to_xml()).collect()
}

fn refused(imp: &hanji_core::Imported, ops: &str) -> String {
    match XlsxEngine::apply(&imp.text, &imp.remainder, ops) {
        Err(EngineError::Refused(m)) => m,
        other => panic!("{ops}: {:?}", other.map(|a| a.text)),
    }
}

#[test]
fn a_workbook_shows_its_formatting_as_range_lines() {
    let imp = open("simple-monthly-budget.xlsx");
    let t = &imp.text;
    assert!(
        t.contains("---\n\n<format default valign=middle font=\"Century Gothic\" size=9pt color=tx2/>\n\n<sheet "),
        "{t}"
    );
    // Theme colours by name, Excel's widths, one line per rectangle.
    assert!(t.contains("<format range=\"E4\" fill=bg2-10% border-top=\"2.25pt solid bg1\" border-left=\"2.25pt solid bg1\"/>\n<format range=\"F4:G4\" fill=bg2-10% border-top=\"2.25pt solid bg1\"/>"), "{t}");
    // A named cell style is shown with the cells in it.
    assert!(t.contains("<format range=\"B4\" style=\"Heading 2\" "), "{t}");
    // The lines are read-only: an edit of one is refused with the operation to use.
    let edited =
        t.replace("fill=bg2-10% border-top=\"2.25pt solid bg1\"/>", "fill=#FF0000 border-top=\"2.25pt solid bg1\"/>");
    let m = match XlsxEngine::apply(&edited, &imp.remainder, "[]") {
        Err(EngineError::Refused(m)) => m,
        other => panic!("{:?}", other.map(|a| a.text)),
    };
    assert!(m.contains("read-only") && m.contains("\"op\": \"format\""), "{m}");
}

fn corpus_bytes(name: &str) -> Vec<u8> {
    corpus().into_iter().find(|(n, _)| n == name).unwrap().1
}

#[test]
fn a_format_operation_changes_only_its_range_and_its_keys() {
    let bytes = corpus_bytes("simple-monthly-budget.xlsx");
    let imp = XlsxEngine.import(&bytes, &ImportOptions::default()).unwrap();
    let ops = r##"[{"op": "format", "range": "Simple Monthly Budget!B5:C7", "set": {"fill": "accent2+80%", "bold": true}},
                  {"op": "format", "range": "Simple Monthly Budget!E12:F14", "set": {"outline": "2pt solid #1F3864"}}]"##;
    let a = XlsxEngine::apply(&imp.text, &imp.remainder, ops).unwrap_or_else(|e| panic!("{e}"));
    assert!(a.text.contains("<format range=\"B5:C7\" fill=accent2+80% bold/>"), "{}", a.text);
    // The outline snaps to Excel's thick border and goes round the range's edges.
    assert!(
        a.text.contains(
            "<format range=\"E12\" border-top=\"2.25pt solid #1F3864\" border-left=\"2.25pt solid #1F3864\"/>"
        ),
        "{}",
        a.text
    );
    assert!(a.text.contains("<format range=\"E13\" border-left=\"2.25pt solid #1F3864\"/>"), "{}", a.text);
    // The other lines are as they were.
    let lines = |t: &str| t.lines().filter(|l| l.starts_with("<format")).map(String::from).collect::<Vec<_>>();
    let (old, new) = (lines(&imp.text), lines(&a.text));
    let gone: Vec<&String> = old.iter().filter(|l| !new.contains(l)).collect();
    assert!(gone.is_empty(), "{gone:?}");
    // PutGet.
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    let back = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(back.text, a.text);
    // styles.xml: what it had is kept as it was, the new entries follow.
    let (s0, s1) = (part(&bytes, "xl/styles.xml"), part(&out, "xl/styles.xml"));
    for list in ["fonts", "fills", "borders", "cellXfs"] {
        let (a, b) = (items(&s0, list), items(&s1, list));
        assert_eq!(&b[..a.len()], &a[..], "{list}");
    }
    assert!(s1.contains("<fgColor theme=\"5\" tint=\"0.8\"/>"), "accent2 is theme 5: {s1}");
    // The same operation again adds nothing.
    let again = XlsxEngine::apply(&a.text, &a.remainder, ops).unwrap();
    let out2 = XlsxEngine.export(&again.text, &again.remainder).unwrap();
    assert_eq!(part(&out2, "xl/styles.xml"), s1);
    assert_eq!(again.text, a.text);
}

#[test]
fn what_a_workbook_cannot_hold_is_refused() {
    let imp = open("korean-sales.xlsx");
    let op = |set: &str| format!(r##"[{{"op": "format", "range": "매출!A1:B2", "set": {set}}}]"##);
    for (set, why) in [
        (r##"{"fill": "gradient"}"##, "gradients and patterns"),
        (r##"{"fill": "#FF0000/50%"}"##, "transparent"),
        (r##"{"border": "1pt triple #000000"}"##, "no triple border"),
        (r##"{"indent": "10pt"}"##, "indent levels"),
        (r##"{"style": "Heading 1"}"##, "not written by an operation"),
        (r##"{"first-line": "10pt"}"##, "indent=N"),
        (r##"{}"##, "at least one key"),
    ] {
        let m = refused(&imp, &op(set));
        assert!(m.contains(why), "{set}: {m}");
    }
    let m = refused(&imp, r##"[{"op": "format", "range": "없음!A1", "set": {"bold": true}}]"##);
    assert!(m.contains("there is no sheet"), "{m}");
    let m = refused(&imp, r##"[{"op": "format", "range": "매출!A1:Z10000", "set": {"bold": true}}]"##);
    assert!(m.contains("at most 100000"), "{m}");
}

/// Every corpus workbook: a fill, a border and bold on a range of its first
/// worksheet export well-formed and read back as the text returned.
#[test]
fn format_operations_put_get_on_the_corpus() {
    let mut failures = vec![];
    for (name, bytes) in corpus() {
        let imp = XlsxEngine.import(&bytes, &ImportOptions::default()).unwrap();
        let Some(sheet) = hanji_format::parse_spreadsheet(&imp.text, &XlsxEngine::names(&imp.remainder))
            .unwrap()
            .sheets
            .into_iter()
            .find(|s| s.range.is_some())
        else {
            continue;
        };
        let quoted = format!("'{}'", sheet.name.replace('\'', "''"));
        let ops = format!(
            r##"[{{"op": "format", "range": "{quoted}!B2:D4", "set": {{"fill": "#FFF2CC", "bold": true, "border-bottom": "0.75pt solid accent1"}}}}]"##
        );
        let a = match XlsxEngine::apply(&imp.text, &imp.remainder, &ops) {
            Ok(a) => a,
            Err(e) => {
                // A range across an array formula or the like is refused, with the reason.
                if !e.to_string().contains("array formula") {
                    failures.push(format!("{name}: {e}"));
                }
                continue;
            }
        };
        let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
        for p in package::read(&out).unwrap() {
            if p.name.ends_with(".xml") && xml::parse(&p.data).is_err() {
                failures.push(format!("{name}: {} is not well-formed", p.name));
            }
        }
        let back = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
        if back.text != a.text {
            failures.push(format!("{name}: PutGet"));
        }
        if !a.text.contains("fill=#FFF2CC") {
            failures.push(format!("{name}: no line shows the fill"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
