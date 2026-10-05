//! A Document exported in its other home format (DESIGN.md §3): docx ↔
//! hwpx through the library API. What crosses is the text, so the corpus
//! tests check that every file's text survives both ways; the others check
//! that the report names each thing that did not: placeholders by kind,
//! properties the target cannot write (with its reason), styles it has under
//! its own definition, theme colours written as RGB, and what the remainder
//! held. Across types the export is refused.

use hanji_format::vocab::{Border, Color, Fill, Tint};
use hanji_format::{Key, Value};
use hanji_store::convert::{StyleFate, StyleNote};
use hanji_store::*;

fn corpus(path: &str) -> (String, Vec<u8>) {
    let full = format!("{}/../../{path}", env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(&full).unwrap_or_else(|e| panic!("{full}: {e}"));
    (path.rsplit('/').next().unwrap().to_string(), bytes)
}

fn files(dir: &str, ext: &str) -> Vec<String> {
    let full = format!("{}/../../{dir}", env!("CARGO_MANIFEST_DIR"));
    let mut v: Vec<String> = std::fs::read_dir(&full)
        .unwrap_or_else(|e| panic!("{full}: {e}"))
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(ext))
        .map(|n| format!("{dir}/{n}"))
        .collect();
    v.sort();
    assert!(!v.is_empty(), "{full} has no {ext} files");
    v
}

fn ws() -> Workspace<MemStorage> {
    Workspace::new(MemStorage::new())
}

fn open(ws: &mut Workspace<MemStorage>, path: &str) -> Opened {
    let (name, bytes) = corpus(path);
    ws.open_bytes(&name, &bytes, None).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The whole text of the current revision (a read stops at 60 KB).
fn full_text(ws: &Workspace<MemStorage>, id: &str) -> String {
    let doc = ws.doc(id).unwrap();
    ws.text(&doc, doc.head).unwrap()
}

fn to(format: Format) -> ExportOptions {
    ExportOptions { acknowledge_surfaced: true, format: Some(format), ..Default::default() }
}

/// The words of a text, whatever the markup: its front matter, style
/// section, tags (placeholders included), braces and punctuation left out.
fn plain(text: &str) -> String {
    let body = text.strip_prefix("---\n").map_or(text, |rest| rest.split_once("\n---\n").map_or(rest, |x| x.1));
    let mut out = String::new();
    for line in body.lines().filter(|l| !l.starts_with("<style ")) {
        let mut rest = line;
        while !rest.is_empty() {
            if rest.starts_with('<') {
                let end = if rest.starts_with("<keep ") {
                    rest.find("/>").map(|k| k + 2)
                } else {
                    rest.find('>').map(|k| k + 1)
                };
                rest = &rest[end.unwrap_or(rest.len())..];
            } else if rest.starts_with('{') {
                rest = &rest[rest.find('}').map_or(rest.len(), |k| k + 1)..];
            } else {
                let c = rest.chars().next().unwrap();
                if c.is_alphanumeric() {
                    out.push(c);
                }
                rest = &rest[c.len_utf8()..];
            }
        }
    }
    out
}

/// The ids of the placeholders a text has, in order.
fn keep_ids(text: &str) -> Vec<String> {
    text.split("<keep id=\"").skip(1).map(|s| s.split('"').next().unwrap().to_string()).collect()
}

/// A channel-wise distance of two `#RRGGBB` colours.
fn near(got: &str, want: &str, tol: i32) -> bool {
    let ch = |s: &str, k: usize| i32::from(u8::from_str_radix(&s[1 + 2 * k..3 + 2 * k], 16).unwrap());
    (0..3).all(|k| (ch(got, k) - ch(want, k)).abs() <= tol)
}

fn convert(path: &str, target: Format) -> (Workspace<MemStorage>, String, Exported, Vec<u8>) {
    let mut ws = ws();
    let o = open(&mut ws, path);
    let (out, bytes) = ws.export_bytes(&o.doc_id, None, &to(target)).unwrap_or_else(|e| panic!("{path}: {e}"));
    (ws, o.doc_id, out, bytes)
}

// ------------------------------------------------------------ the corpus

#[test]
fn every_document_of_the_corpus_crosses_both_ways_with_its_text() {
    let docx = files("prototype/remainder/corpus", ".docx");
    let hwpx = files("crates/hanji-hwpx/corpus", ".hwpx");
    assert!(docx.len() >= 13 && hwpx.len() >= 16);
    for (path, home, other) in
        docx.iter().map(|p| (p, Format::Docx, Format::Hwpx)).chain(hwpx.iter().map(|p| (p, Format::Hwpx, Format::Docx)))
    {
        let mut ws = ws();
        let o = open(&mut ws, path);
        let source = full_text(&ws, &o.doc_id);
        let (out, bytes) = ws.export_bytes(&o.doc_id, None, &to(other)).unwrap_or_else(|e| panic!("{path}: {e}"));
        let c = out.conversion.as_ref().expect("a conversion report");
        assert_eq!((out.format, c.from, c.to), (other, home, other), "{path}");
        // The converted file opens in its own format, with the text and none of the placeholders.
        let name = format!("converted.{}", other.name());
        let back = ws.open_bytes(&name, &bytes, None).unwrap_or_else(|e| panic!("{path}: {e}"));
        assert_eq!(back.format, other, "{path}");
        let converted = full_text(&ws, &back.doc_id);
        assert!(!converted.contains("<keep "), "{path}: a placeholder crossed");
        assert_eq!(plain(&converted), plain(&source), "{path}: the text changed");
        // Every placeholder is in the report, each once.
        let mut ids = keep_ids(&source);
        let mut named: Vec<String> = c.placeholders.iter().map(|p| p.id.clone()).collect();
        ids.sort();
        named.sort();
        assert_eq!(named, ids, "{path}: the report names the placeholders");
        assert_eq!(c.placeholder_kinds.values().sum::<usize>(), ids.len(), "{path}");
        // And back again: the converted document converts to the home format, its text still the same.
        let (_, home_bytes) =
            ws.export_bytes(&back.doc_id, None, &to(home)).unwrap_or_else(|e| panic!("{path} back: {e}"));
        let again = ws.open_bytes(&format!("again.{}", home.name()), &home_bytes, None).unwrap();
        assert_eq!(plain(&full_text(&ws, &again.doc_id)), plain(&source), "{path}: and back");
    }
}

// ------------------------------------------------------------ the report

#[test]
fn the_report_names_each_placeholder_by_kind_with_its_line() {
    let (ws, id, out, _) = convert("prototype/remainder/corpus/testWORD_2006ml.docx", Format::Hwpx);
    let c = out.conversion.unwrap();
    let source = full_text(&ws, &id);
    assert_eq!(c.placeholders.len(), 53);
    let kinds: Vec<(&str, usize)> = c.placeholder_kinds.iter().map(|(k, n)| (k.as_str(), *n)).collect();
    assert_eq!(
        kinds,
        [
            ("comment", 1),
            ("content-control", 9),
            ("drawing", 5),
            ("endnote", 1),
            ("field", 4),
            ("field-part", 12),
            ("footnote", 1),
            ("math", 1),
            ("object", 3),
            ("table", 1),
            ("tracked-delete", 11),
            ("tracked-insert", 2),
            ("tracked-move", 2),
        ]
    );
    // What a placeholder held is in its summary, and where it was is its line.
    let ins = c.placeholders.iter().find(|p| p.kind == "tracked-insert" && p.summary.contains("dog")).unwrap();
    assert!(ins.summary.contains("Allison, Timothy B."), "{ins:?}");
    let line: usize = ins.location.strip_prefix("line ").unwrap().parse().unwrap();
    assert!(source.lines().nth(line - 1).unwrap().contains(&format!("id=\"{}\"", ins.id)), "{ins:?}");
    let footnote = c.placeholders.iter().find(|p| p.kind == "footnote").unwrap();
    assert!(footnote.summary.contains("And this is the footnote"), "{footnote:?}");
}

#[test]
fn nothing_hidden_leaves_in_a_converted_file_and_the_report_says_what_was_left() {
    let mut ws = ws();
    let o = open(&mut ws, "prototype/remainder/corpus/testWORD_2006ml.docx");
    assert!(!o.report.surfaced.is_empty());
    // The docx cannot leave unacknowledged; its hwpx can: no comment, tracked change or author is in it.
    assert_eq!(
        ws.export_bytes(&o.doc_id, None, &ExportOptions::default()).unwrap_err().code,
        Code::SurfacedNotAcknowledged
    );
    let opts = ExportOptions { format: Some(Format::Hwpx), ..Default::default() };
    let (out, _) = ws.export_bytes(&o.doc_id, None, &opts).unwrap_or_else(|e| panic!("{e}"));
    assert!(out.surfaced.is_empty());
    let c = out.conversion.unwrap();
    let kinds: Vec<&str> = c.placeholder_kinds.keys().map(String::as_str).collect();
    assert!(["comment", "tracked-delete", "tracked-insert"].iter().all(|k| kinds.contains(k)), "{kinds:?}");
    let left: Vec<&str> = c.not_carried.iter().map(|n| n.what.as_str()).collect();
    for what in [
        "document properties (author, dates, statistics)",
        "headers and footers",
        "bookmarks",
        "comment ranges",
        "pictures, charts and embedded objects",
    ] {
        assert!(left.contains(&what), "{what}: {left:?}");
    }
    let props = c.not_carried.iter().find(|n| n.what.starts_with("document properties")).unwrap();
    assert_eq!(props.parts, ["docProps/app.xml", "docProps/core.xml"]);
    let sections = c.not_carried.iter().find(|n| n.what.starts_with("section properties")).unwrap();
    assert_eq!(sections.count, 1);
}

#[test]
fn page_setup_of_an_hwpx_is_named_as_left_behind() {
    let (_, _, out, _) = convert("crates/hanji-hwpx/corpus/landscape-001.hwpx", Format::Docx);
    let c = out.conversion.unwrap();
    let s = c.not_carried.iter().find(|n| n.what.starts_with("section properties")).expect("section properties");
    assert_eq!(s.count, 1);
    assert!(s.what.contains("orientation"), "{}", s.what);
    // A document with three sections names three.
    let (_, _, out, _) = convert("crates/hanji-hwpx/corpus/hcar-001.hwpx", Format::Docx);
    let c = out.conversion.unwrap();
    assert_eq!(c.not_carried.iter().find(|n| n.what.starts_with("section properties")).unwrap().count, 3);
}

#[test]
fn a_table_style_and_position_hwpx_cannot_write_are_dropped_with_the_reason() {
    let (ws, _, out, bytes) = convert("prototype/remainder/corpus/fdo76098.docx", Format::Hwpx);
    let c = out.conversion.unwrap();
    let style = c.properties.iter().find(|p| p.property.starts_with("table style")).expect("table style");
    assert_eq!((style.property.as_str(), style.count), ("table style \"Normal Table\"", 1));
    assert!(style.reason.contains("hwpx has no table style"), "{}", style.reason);
    assert!(style.at[0].starts_with("line "), "{style:?}");
    let place = c.properties.iter().find(|p| p.property.starts_with("table-align")).expect("table position");
    assert_eq!(place.property, "table-align=left table-indent=0pt");
    assert!(place.reason.contains("hwpx has no table position"), "{}", place.reason);
    let mut ws = ws;
    let back = ws.open_bytes("c.hwpx", &bytes, None).unwrap();
    let t = full_text(&ws, &back.doc_id);
    assert!(!t.contains("table-align") && !t.contains("table-indent"), "{t}");
}

#[test]
fn a_fill_the_target_cannot_write_is_dropped_by_name_and_the_theme_colours_beside_it_are_not() {
    // docx → hwpx: a pattern fill goes; the theme fills become RGB.
    let (mut ws, _, out, bytes) = convert("prototype/remainder/corpus/docx4j-tables.docx", Format::Hwpx);
    let c = out.conversion.unwrap();
    assert_eq!(c.properties.len(), 1, "{:?}", c.properties);
    let p = &c.properties[0];
    assert_eq!((p.property.as_str(), p.count), ("fill=pattern", 13));
    assert!(p.reason.contains("shown as the file has it and cannot be written anew"), "{}", p.reason);
    // The theme here is custom: its background is E2F0D1 (the document's own w:fill says so too).
    let r = c.resolved_colours.iter().find(|r| r.from == "bg1").expect("bg1 resolved");
    assert_eq!((r.to.as_str(), r.count), ("#E2F0D1", 1));
    let r = c.resolved_colours.iter().find(|r| r.from == "tx2+90%").expect("tx2+90% resolved");
    assert!(near(&r.to, "#D2E3FF", 2), "Word's own value for it is D2E3FF: {}", r.to);
    let back = ws.open_bytes("c.hwpx", &bytes, None).unwrap();
    let t = full_text(&ws, &back.doc_id);
    assert!(t.contains("#E2F0D1"), "{t}");
    assert!(!t.contains("accent") && !t.contains("tx2") && !t.contains("bg1"), "{t}");
    // hwpx → docx: a gradient goes, named.
    let (_, _, out, _) = convert("crates/hanji-hwpx/corpus/mel-001.hwpx", Format::Docx);
    let c = out.conversion.unwrap();
    let g = c.properties.iter().find(|p| p.property == "fill=gradient").expect("gradient");
    assert_eq!(g.count, 3);
    assert!(g.reason.contains("cannot be written anew"), "{}", g.reason);
}

#[test]
fn a_theme_colour_in_a_style_is_written_as_rgb() {
    let mut ws = ws();
    let o = open(&mut ws, "prototype/remainder/corpus/korean-report.docx");
    let source = full_text(&ws, &o.doc_id);
    // A new style with a theme colour, and a paragraph that uses it (an unused style's line is not kept).
    let edited = source.replacen("# 3분기", "<div style=\"Accent\">3분기</div>\n\n# 3분기", 1).replacen(
        "\n\n",
        "\n<style name=\"Accent\" color=accent1-25% size=13pt/>\n\n",
        1,
    );
    ws.write(&o.doc_id, 1, &edited).unwrap_or_else(|e| panic!("{e}"));
    let stored = full_text(&ws, &o.doc_id);
    let line = stored.lines().find(|l| l.starts_with("<style name=\"Accent\"")).expect("the style is there");
    assert!(line.contains("color=accent1-25%"), "{line}");
    let (out, bytes) = ws.export_bytes(&o.doc_id, None, &to(Format::Hwpx)).unwrap_or_else(|e| panic!("{e}"));
    let c = out.conversion.unwrap();
    assert!(c.styles.iter().any(|s| s.name == "Accent" && s.fate == StyleFate::Created), "{:?}", c.styles);
    let r = c.resolved_colours.iter().find(|r| r.from == "accent1-25%").expect("resolved");
    assert!(r.to.starts_with('#') && r.to.len() == 7 && r.count == 1, "{r:?}");
    assert!(c.properties.iter().all(|p| !p.reason.contains("theme")), "{:?}", c.properties);
    let back = ws.open_bytes("c.hwpx", &bytes, None).unwrap();
    let t = full_text(&ws, &back.doc_id);
    let line = t.lines().find(|l| l.starts_with("<style name=\"Accent\"")).expect("created in the hwpx");
    assert!(line.contains(&format!("color={}", r.to)) && line.contains("size=13pt"), "{line}");
    assert!(t.contains("<div style=\"Accent\">3분기</div>"), "{t}");
}

#[test]
fn styles_the_target_has_keep_its_definition_and_the_others_are_created() {
    let (mut ws, _, out, bytes) = convert("prototype/remainder/corpus/korean-report.docx", Format::Hwpx);
    let c = out.conversion.unwrap();
    let note = |name: &str| -> &StyleNote {
        c.styles.iter().find(|s| s.name == name).unwrap_or_else(|| panic!("{name}: {:?}", c.styles))
    };
    let normal = note("Normal");
    assert_eq!((normal.fate, normal.target.as_deref()), (StyleFate::Replaced, Some("바탕글")));
    assert!(normal.detail.contains("font Noto Sans CJK KR → 함초롬바탕"), "{}", normal.detail);
    let h1 = note("Heading 1");
    assert_eq!((h1.fate, h1.target.as_deref()), (StyleFate::Replaced, Some("개요 1")));
    // A style's own line is what is compared, not the default's it builds on.
    assert!(h1.detail.contains("size") && !h1.detail.contains("font"), "{}", h1.detail);
    assert_eq!(note("Note").fate, StyleFate::Created);
    assert_eq!(note("Block Quotation").fate, StyleFate::Created);
    let back = ws.open_bytes("c.hwpx", &bytes, None).unwrap();
    let t = full_text(&ws, &back.doc_id);
    assert!(t.contains("<style name=\"Note\"") && t.contains("<div style=\"Note\">"), "{t}");
    assert!(t.lines().any(|l| l.starts_with("# ")), "a heading stays a heading:\n{t}");
    // The other way: the hwpx default is the docx Normal.
    let (_, _, out, _) = convert("crates/hanji-hwpx/corpus/para-001.hwpx", Format::Docx);
    let c = out.conversion.unwrap();
    let d = c.styles.iter().find(|s| s.name == "바탕글").unwrap();
    assert_eq!((d.fate, d.target.as_deref()), (StyleFate::Replaced, Some("Normal")));
}

#[test]
fn a_report_has_a_text_for_people() {
    let (_, _, out, _) = convert("prototype/remainder/corpus/testWORD_2006ml.docx", Format::Hwpx);
    let t = out.conversion.unwrap().to_string();
    assert!(t.starts_with("converted docx → hwpx. Not carried across:"), "{t}");
    assert!(t.contains("placeholders dropped, 53 (comment ×1, content-control ×9,"), "{t}");
    assert!(t.contains("Normal → 바탕글:"), "{t}");
    assert!(t.contains("theme colours written as #RRGGBB"), "{t}");
    assert!(t.contains("headers and footers (word/footer1.xml"), "{t}");
}

// ------------------------------------------------------------ refusals

#[test]
fn another_type_is_refused_and_so_is_tracking_across_formats() {
    let mut ws = ws();
    let deck = open(&mut ws, "crates/hanji-pptx/corpus/60810.pptx");
    for target in [Format::Docx, Format::Hwpx, Format::Xlsx] {
        let e = ws.export_bytes(&deck.doc_id, None, &to(target)).unwrap_err();
        assert_eq!(e.code, Code::Unsupported, "{target:?}");
        assert!(e.message.contains("a presentation is exported as pptx"), "{}", e.message);
        assert!(e.message.contains("within a type only"), "{}", e.message);
    }
    let doc = open(&mut ws, "prototype/remainder/corpus/korean-report.docx");
    for target in [Format::Pptx, Format::Xlsx] {
        let e = ws.export_bytes(&doc.doc_id, None, &to(target)).unwrap_err();
        assert_eq!(e.code, Code::Unsupported, "{target:?}");
        assert!(e.message.contains("a document is exported as docx or hwpx"), "{}", e.message);
    }
    let sheet = open(&mut ws, "crates/hanji-xlsx/corpus/50867_with_table.xlsx");
    let e = ws.export_bytes(&sheet.doc_id, None, &to(Format::Docx)).unwrap_err();
    assert_eq!(e.code, Code::Unsupported);
    // Tracked changes are the edits since the file was opened, written into it.
    let opts = ExportOptions { tracked_changes: true, ..to(Format::Hwpx) };
    let e = ws.export_bytes(&doc.doc_id, None, &opts).unwrap_err();
    assert_eq!(e.code, Code::BadRequest);
    assert!(e.message.contains("has no file to track them in"), "{}", e.message);
}

#[test]
fn the_same_revision_converts_to_the_same_bytes() {
    // §8: the digest identifies the export; nothing in a conversion varies.
    for (path, other) in [
        ("prototype/remainder/corpus/testWORD_2006ml.docx", Format::Hwpx),
        ("crates/hanji-hwpx/corpus/hcar-001.hwpx", Format::Docx),
    ] {
        let mut ws = ws();
        let o = open(&mut ws, path);
        let (a, bytes_a) = ws.export_bytes(&o.doc_id, None, &to(other)).unwrap();
        let (b, bytes_b) = ws.export_bytes(&o.doc_id, None, &to(other)).unwrap();
        assert_eq!((a.digest, bytes_a), (b.digest, bytes_b), "{path}");
        assert_eq!(a.conversion, b.conversion, "{path}: the report is the same too");
    }
}

#[test]
fn naming_the_home_format_is_an_ordinary_export() {
    let mut ws = ws();
    let o = open(&mut ws, "prototype/remainder/corpus/korean-report.docx");
    let (plain_out, a) =
        ws.export_bytes(&o.doc_id, None, &ExportOptions { acknowledge_surfaced: true, ..Default::default() }).unwrap();
    let (named, b) = ws.export_bytes(&o.doc_id, None, &to(Format::Docx)).unwrap();
    assert!(named.conversion.is_none() && plain_out.conversion.is_none());
    assert_eq!((named.digest, a), (plain_out.digest, b));
}

#[test]
fn a_conversion_does_not_touch_the_stored_document() {
    let mut ws = ws();
    let o = open(&mut ws, "prototype/remainder/corpus/korean-report.docx");
    let before = (ws.doc(&o.doc_id).unwrap(), full_text(&ws, &o.doc_id));
    ws.export_bytes(&o.doc_id, None, &to(Format::Hwpx)).unwrap();
    let after = (ws.doc(&o.doc_id).unwrap(), full_text(&ws, &o.doc_id));
    assert_eq!(before, after);
    assert_eq!(ws.list().unwrap().len(), 1);
}

// ------------------------------------------------------------ what a format writes

#[test]
fn a_format_says_what_it_cannot_write_and_why() {
    let theme = Value::Color(Color::theme("tx2"));
    let e = Format::Hwpx.writes(Key::Color, &theme).unwrap_err();
    assert!(e.contains("no theme colours"), "{e}");
    assert!(Format::Docx.writes(Key::Color, &theme).is_ok());
    let faded = Value::Fill(Fill::Color(Color { alpha: Some(50), ..Color::rgb(255, 0, 0) }));
    assert!(Format::Hwpx.writes(Key::Fill, &faded).unwrap_err().contains("opacity"));
    assert!(Format::Docx.writes(Key::Fill, &faded).unwrap_err().contains("transparent"));
    let shown = Value::Fill(Fill::Gradient);
    for f in [Format::Docx, Format::Hwpx] {
        assert!(f.writes(Key::Fill, &shown).unwrap_err().contains("cannot be written anew"), "{f:?}");
    }
    let tinted = Value::Border(Border::Line {
        width: 100,
        style: "solid".into(),
        color: Color { tint: Tint::Lighter(40), ..Color::theme("accent1") },
    });
    assert!(Format::Hwpx.writes(Key::BorderTop, &tinted).is_err());
    assert!(Format::Docx.writes(Key::BorderTop, &tinted).is_ok());
    assert!(Format::Hwpx.writes(Key::Size, &Value::Len(1000)).is_ok());
}
