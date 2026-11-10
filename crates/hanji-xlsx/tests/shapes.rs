//! Drawing shapes as `<shape/>` / `<group>` lines: parse/print, retext, move,
//! add/remove, GetPut of an unedited file. Geometry the syntax does not name
//! stays as the original XML.

use std::path::PathBuf;

use hanji_core::{Engine, ImportOptions};
use hanji_format::sheet::{parse_spreadsheet, serialize_spreadsheet, SheetItem};
use hanji_xlsx::{package, XlsxEngine};

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

fn shape_lines(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| {
            let t = l.trim_start();
            t.starts_with("<shape ") || t.starts_with("<group ") || t.starts_with("</group")
        })
        .map(str::to_string)
        .collect()
}

fn drawing_part(bytes: &[u8]) -> (String, Vec<u8>) {
    let parts = package::read(bytes).unwrap();
    let p = parts
        .iter()
        .find(|p| {
            p.name.starts_with("xl/drawings/")
                && p.name.ends_with(".xml")
                && !p.name.contains("_rels")
        })
        .unwrap_or_else(|| {
            panic!(
                "drawing part: {:?}",
                parts.iter().map(|p| &p.name).collect::<Vec<_>>()
            )
        });
    (p.name.clone(), p.data.clone())
}

fn keep_kinds(text: &str) -> Vec<String> {
    let s = parse_spreadsheet(text, &hanji_format::Names::default()).unwrap();
    s.sheets
        .iter()
        .flat_map(|sh| sh.items.iter())
        .filter_map(|i| match i {
            SheetItem::Keep(k) => Some(k.kind.clone()),
            _ => None,
        })
        .collect()
}

fn xfrm_blob(xml_bytes: &[u8], name: &str) -> String {
    let text = String::from_utf8_lossy(xml_bytes);
    let marker = format!("name=\"{name}\"");
    let at = text
        .find(&marker)
        .unwrap_or_else(|| panic!("{marker} in drawing"));
    let rest = &text[at..];
    let start = rest.find("<a:xfrm").expect("xfrm after name");
    let end = rest[start..].find("</a:xfrm>").expect("</a:xfrm>") + start + "</a:xfrm>".len();
    rest[start..end].to_string()
}

#[test]
fn unedited_files_keep_their_drawing_xml() {
    for name in [
        "simple-monthly-budget.xlsx",
        "WithDrawing.xlsx",
        "ExcelWithAttachments.xlsm",
    ] {
        let bytes = corpus_file(name);
        let (_, orig) = drawing_part(&bytes);
        let imp = XlsxEngine
            .import(&bytes, &ImportOptions::default())
            .unwrap();
        let out = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
        let (_, drawing) = drawing_part(&out);
        assert_eq!(orig, drawing, "{name} drawing1.xml");
        if name.ends_with(".xlsm") {
            let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
            assert_eq!(again.text, imp.text, "{name}");
        } else {
            getput(&bytes, &out).unwrap_or_else(|e| panic!("{name}: {e}"));
        }
    }
}

#[test]
fn budget_callout_is_a_shape_line() {
    let bytes = corpus_file("simple-monthly-budget.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    assert_eq!(
        shape_lines(&imp.text),
        ["<shape name=\"Data Entry Tip\" anchor=\"B25:D27\">Need to add more entries? Start typing below the last entry and the table will automatically expand when you press Enter.</shape>"]
    );
    assert!(keep_kinds(&imp.text).contains(&"chart".into()));
    assert!(!keep_kinds(&imp.text).iter().any(|k| k == "shape"));
}

#[test]
fn withdrawing_textbox_and_picture_keeps() {
    let bytes = corpus_file("WithDrawing.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    assert_eq!(
        shape_lines(&imp.text),
        ["<shape name=\"TextBox 8\" anchor=\"D16\">Sheet with various pictures&#10;(jpeg, png, wmf, emf and pict)</shape>"]
    );
    let pictures = keep_kinds(&imp.text)
        .into_iter()
        .filter(|k| k == "picture")
        .count();
    assert_eq!(pictures, 5);
    assert!(!keep_kinds(&imp.text).iter().any(|k| k == "shape"));
}

#[test]
fn grouped_shapes_wrap_in_group() {
    let bytes = corpus_file("ExcelWithAttachments.xlsm");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let lines = shape_lines(&imp.text);
    assert_eq!(lines[0], "<group name=\"Group 1\">");
    assert_eq!(lines[1], "<shape name=\"AutoShape 2\" anchor=\"A18:B25\"/>");
    assert!(
        lines[2].starts_with("<shape name=\"Text Box 3\" anchor=\"A18:B25\">"),
        "{}",
        lines[2]
    );
    assert!(lines[2].contains("Словарь"));
    assert!(lines[2].contains("dictionary"));
    assert!(lines[2].contains("Луғат"));
    assert_eq!(lines[3], "</group>");
    assert!(lines.iter().any(|l| l.contains("WordArt 5")));
    assert!(lines.iter().any(|l| l.contains("WordArt 6")));
    assert!(lines.iter().any(|l| l.contains("WordArt 7")));
    assert!(!keep_kinds(&imp.text)
        .iter()
        .any(|k| k == "shape" || k == "group"));
}

#[test]
fn editing_shape_text_writes_it_and_keeps_geometry() {
    let bytes = corpus_file("simple-monthly-budget.xlsx");
    let (_, orig) = drawing_part(&bytes);
    let xfrm = xfrm_blob(&orig, "Data Entry Tip");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let edited = imp.text.replace(
        "Need to add more entries? Start typing below the last entry and the table will automatically expand when you press Enter.",
        "Type below the table.",
    );
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    assert_eq!(
        shape_lines(&a.text),
        ["<shape name=\"Data Entry Tip\" anchor=\"B25:D27\">Type below the table.</shape>"]
    );
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    let (_, drawing) = drawing_part(&out);
    let xml = String::from_utf8_lossy(&drawing);
    assert!(xml.contains("Type below the table."), "{xml}");
    assert!(!xml.contains("Need to add more entries?"), "{xml}");
    assert_eq!(xfrm_blob(&drawing, "Data Entry Tip"), xfrm);
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(again.text, a.text);
}

#[test]
fn moving_a_shape_rewrites_its_anchor() {
    let bytes = corpus_file("WithDrawing.xlsx");
    let (_, orig) = drawing_part(&bytes);
    let xfrm = xfrm_blob(&orig, "TextBox 8");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let edited = imp.text.replace("anchor=\"D16\"", "anchor=\"B2:D5\"");
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    assert!(
        shape_lines(&a.text)
            .iter()
            .any(|l| l.contains("name=\"TextBox 8\"") && l.contains("anchor=\"B2:D5\"")),
        "{:?}",
        shape_lines(&a.text)
    );
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    let (_, drawing) = drawing_part(&out);
    let xml = String::from_utf8_lossy(&drawing);
    assert!(xml.contains("<xdr:from>"), "{xml}");
    assert!(xml.contains("<xdr:col>1</xdr:col>"), "from col B: {xml}");
    assert!(xml.contains("<xdr:row>1</xdr:row>"), "from row 2: {xml}");
    assert!(xml.contains("<xdr:col>3</xdr:col>"), "to col D: {xml}");
    assert!(xml.contains("<xdr:row>4</xdr:row>"), "to row 5: {xml}");
    assert_eq!(xfrm_blob(&drawing, "TextBox 8"), xfrm);
    assert!(xml.contains("name=\"Picture 1\""), "pictures stay");
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(again.text, a.text);
}

#[test]
fn removing_a_shape_drops_it() {
    let bytes = corpus_file("ExcelWithAttachments.xlsm");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let line = imp
        .text
        .lines()
        .find(|l| l.contains("WordArt 5"))
        .unwrap()
        .to_string();
    let edited = imp.text.replace(&format!("{line}\n"), "");
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    assert!(
        !shape_lines(&a.text).iter().any(|l| l.contains("WordArt 5")),
        "{:?}",
        shape_lines(&a.text)
    );
    assert!(shape_lines(&a.text).iter().any(|l| l.contains("WordArt 6")));
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    let (_, drawing) = drawing_part(&out);
    let xml = String::from_utf8_lossy(&drawing);
    assert!(!xml.contains("WordArt 5"), "{xml}");
    assert!(xml.contains("WordArt 6"), "{xml}");
    assert!(xml.contains("Group 1"), "{xml}");
}

#[test]
fn adding_a_shape_writes_it_and_reads_back() {
    let bytes = corpus_file("rx_hyperlink01.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let edited = imp.text.replacen(
        "</sheet>",
        "<shape name=\"Callout\" anchor=\"B2:D5\">Hello from hanji.</shape>\n</sheet>",
        1,
    );
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    assert!(
        shape_lines(&a.text)
            .iter()
            .any(|l| l.contains("name=\"Callout\"") && l.contains("Hello from hanji.")),
        "{:?}",
        shape_lines(&a.text)
    );
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    let (_, drawing) = drawing_part(&out);
    let xml = String::from_utf8_lossy(&drawing);
    assert!(xml.contains("Callout"), "{xml}");
    assert!(xml.contains("Hello from hanji."), "{xml}");
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(again.text, a.text);
}

#[test]
fn retexting_one_grouped_shape_leaves_the_other() {
    let bytes = corpus_file("ExcelWithAttachments.xlsm");
    let (_, orig) = drawing_part(&bytes);
    let xfrm = xfrm_blob(&orig, "AutoShape 2");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let edited = imp.text.replace(
        "<shape name=\"AutoShape 2\" anchor=\"A18:B25\"/>",
        "<shape name=\"AutoShape 2\" anchor=\"A18:B25\">now labelled</shape>",
    );
    let a = XlsxEngine::apply(&edited, &imp.remainder, "[]").unwrap();
    assert!(
        shape_lines(&a.text)
            .iter()
            .any(|l| l.contains("AutoShape 2") && l.contains("now labelled")),
        "{:?}",
        shape_lines(&a.text)
    );
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    let (_, drawing) = drawing_part(&out);
    let xml = String::from_utf8_lossy(&drawing);
    assert!(xml.contains("now labelled"), "{xml}");
    assert!(
        xml.contains("Словарь") || xml.contains("dictionary"),
        "{xml}"
    );
    assert_eq!(xfrm_blob(&drawing, "AutoShape 2"), xfrm);
}

#[test]
fn structure_round_trip_keeps_shape_lines() {
    let bytes = corpus_file("WithDrawing.xlsx");
    let imp = XlsxEngine
        .import(&bytes, &ImportOptions::default())
        .unwrap();
    let parsed = parse_spreadsheet(&imp.text, &hanji_format::Names::default()).unwrap();
    assert_eq!(serialize_spreadsheet(&parsed), imp.text);
}
