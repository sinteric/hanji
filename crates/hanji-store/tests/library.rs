//! The operations end to end, per format, through the library API: open →
//! read → edit → export → reopen with PutGet holding (the reopened text is the
//! text written), new files from the blank packages, and the refusals: a
//! stale revision, an ambiguous `old`, validator errors, the surfaced-before-
//! export gate, the rule-7 re-import of a file a person changed, and partial
//! reads of a large file.

use hanji_core::Part;
use hanji_package::package;
use hanji_store::*;

fn corpus(path: &str) -> (String, Vec<u8>) {
    let full = format!("{}/../../{path}", env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(&full).unwrap_or_else(|e| panic!("{full}: {e}"));
    (path.rsplit('/').next().unwrap().to_string(), bytes)
}

fn ws() -> Workspace<MemStorage> {
    Workspace::new(MemStorage::new())
}

fn open(ws: &mut Workspace<MemStorage>, path: &str) -> Opened {
    let (name, bytes) = corpus(path);
    ws.open_bytes(&name, &bytes, None).unwrap_or_else(|e| panic!("{e}"))
}

fn text(ws: &Workspace<MemStorage>, id: &str) -> (u32, String) {
    let r = ws.read(id, None, &Window::default()).unwrap();
    assert!(!r.partial);
    (r.revision, r.text)
}

fn one(old: &str, new: &str) -> Vec<TextEdit> {
    vec![TextEdit { old: old.into(), new: new.into() }]
}

const ACK: ExportOptions = ExportOptions { acknowledge_surfaced: true, tracked_changes: false, format: None };

/// Export the current revision, open the bytes as a new document, and check
/// PutGet: the reopened text is the revision's text. Returns the bytes.
fn putget(ws: &mut Workspace<MemStorage>, id: &str, name: &str) -> Vec<u8> {
    let (_, written) = text(ws, id);
    let (out, bytes) = ws.export_bytes(id, None, &ACK).unwrap_or_else(|e| panic!("{e}"));
    let (again, _) = ws.export_bytes(id, None, &ACK).unwrap();
    assert_eq!(out.digest, again.digest, "§8: the same revision exports the same bytes");
    let back = ws.open_bytes(name, &bytes, None).unwrap_or_else(|e| panic!("{e}"));
    let (_, reread) = text(ws, &back.doc_id);
    assert_eq!(reread, written, "PutGet");
    bytes
}

/// The package with `from` replaced by `to` in part `part`, as a person's
/// edit in the native application would change it.
fn person_edits(pkg: &[u8], part: &str, from: &str, to: &str) -> Vec<u8> {
    let mut parts: Vec<Part> = package::read(pkg).unwrap();
    let p = parts.iter_mut().find(|p| p.name == part).unwrap();
    let s = String::from_utf8(p.data.clone()).unwrap();
    assert!(s.contains(from), "{part} has no {from:?}");
    p.data = s.replacen(from, to, 1).into_bytes();
    package::write(&parts).unwrap()
}

// ------------------------------------------------------------ per format

#[test]
fn docx_open_read_edit_export_reopen() {
    let mut ws = ws();
    let o = open(&mut ws, "prototype/remainder/corpus/korean-report.docx");
    assert_eq!((o.format, o.doc_type, o.revision), (Format::Docx, DocType::Document, 1));
    assert_eq!(o.report.surfaced[0].kind, "comment");
    let (rev, t) = text(&ws, &o.doc_id);
    assert!(t.contains("| ^^ | 종로 | 95 |"));
    let c = ws.edit(&o.doc_id, rev, &one("| ^^ | 종로 | 95 |", "| ^^ | 종로 | 97 |")).unwrap();
    assert_eq!((c.parent, c.revision), (1, 2));
    assert!(c.removed.is_empty() && c.placed.unwrap() > 0);
    // A list of edits applies in order, all or nothing.
    let c = ws
        .edit(
            &o.doc_id,
            2,
            &[
                TextEdit { old: "4분기에는".into(), new: "4분기부터".into() },
                TextEdit {
                    old: "4분기부터 [부산]{color=#C00000}과 ".into(),
                    new: "4분기부터 [부산]{color=#C00000}·".into(),
                },
            ],
        )
        .unwrap();
    assert_eq!(c.revision, 3);
    let (_, t) = text(&ws, &o.doc_id);
    assert!(t.contains("| ^^ | 종로 | 97 |") && t.contains("4분기부터 [부산]{color=#C00000}·[대구]"), "{t}");
    putget(&mut ws, &o.doc_id, "korean-report.docx");
    let h = ws.history(&o.doc_id).unwrap();
    assert_eq!(h.revisions.iter().map(|r| r.op).collect::<Vec<_>>(), [RevOp::Open, RevOp::Edit, RevOp::Edit]);
    assert!(ws.diff(&o.doc_id, 1, 3).unwrap().diff.contains("-| ^^ | 종로 | 95 |\n+| ^^ | 종로 | 97 |"));
}

#[test]
fn hwpx_open_read_edit_export_reopen() {
    let mut ws = ws();
    let o = open(&mut ws, "crates/hanji-hwpx/corpus/basic-table-01.hwpx");
    let (rev, _) = text(&ws, &o.doc_id);
    ws.edit(&o.doc_id, rev, &one("| 5 | 6 |", "| 5 | 6.5 |")).unwrap();
    let c = ws.edit(&o.doc_id, 2, &one("<p/>", "표 아래 문단.")).unwrap();
    assert_eq!(c.revision, 3);
    putget(&mut ws, &o.doc_id, "basic-table-01.hwpx");
}

#[test]
fn pptx_open_read_edit_export_reopen() {
    let mut ws = ws();
    let o = open(&mut ws, "crates/hanji-pptx/corpus/korean-deck.pptx");
    let (rev, _) = text(&ws, &o.doc_id);
    ws.edit(&o.doc_id, rev, &one("핵심 지표", "핵심 성과 지표")).unwrap();
    // A new slide from a layout, with notes.
    ws.edit(
        &o.doc_id,
        2,
        &one(
            "::title box=\"36 22 648 90\" font=\"맑은 고딕\" size=44pt color=tx1::\n감사합니다\n",
            "::title box=\"36 22 648 90\" font=\"맑은 고딕\" size=44pt color=tx1::\n감사합니다\n\n---\n\nlayout: Title and Content\n::title::\n질의응답\n::body::\n- 질문 받기\n",
        ),
    )
    .unwrap();
    let (_, t) = text(&ws, &o.doc_id);
    // Stored in canonical form: the new slide's slots with their layout's
    // boxes and the text formatting they inherit (§5.3).
    let tail = "::body box=\"36 126 648 356\" font=\"맑은 고딕\" size=32pt color=tx1::\n- 질문 받기\n";
    assert!(t.contains("핵심 성과 지표") && t.ends_with(tail), "{t}");
    // This deck has no notes master: a new slide cannot get notes (refused, with the reason).
    let e = ws.edit(&o.doc_id, 3, &one("- 질문 받기\n", "- 질문 받기\n::notes::\n5분\n")).unwrap_err();
    assert!(e.code == Code::Refused && e.message.contains("notes master"), "{e:?}");
    putget(&mut ws, &o.doc_id, "korean-deck.pptx");
}

#[test]
fn xlsx_open_read_ops_edit_export_reopen() {
    let mut ws = ws();
    let o = open(&mut ws, "crates/hanji-xlsx/corpus/korean-sales.xlsx");
    let r = ws.read(&o.doc_id, None, &Window::default()).unwrap();
    assert!(
        r.text.contains("<table name=\"Sales\"")
            && r.data.as_deref().unwrap().contains("| 4 | 2026-01 | 강남 | 가전 | 10,269,000 |")
    );
    let c = ws.ops(&o.doc_id, 1, r#"[{"op": "set", "range": "매출!D4", "values": [[11111111]]}]"#).unwrap();
    assert_eq!((c.revision, c.applied), (2, Some(1)));
    let w = Window { table: Some("Sales".into()), rows: Some("4:5".into()), ..Default::default() };
    let r = ws.read(&o.doc_id, None, &w).unwrap();
    assert!(r.data.as_deref().unwrap().contains("| 4 | 2026-01 | 강남 | 가전 | 11,111,111 |"), "{:?}", r.data);
    // The structure text is edited like any text: a column's number format.
    let c = ws.edit(&o.doc_id, 2, &one("| 원가 | number | #,##0 |", "| 원가 | number | #,##0.0 |")).unwrap();
    assert_eq!(c.revision, 3);
    let r = ws.read(&o.doc_id, None, &w).unwrap();
    assert!(r.data.as_deref().unwrap().contains("| 7,410,000.0 |"), "{:?}", r.data);
    // Operations that do not fit the structure are refused, written for the model.
    let e = ws
        .ops(&o.doc_id, 3, r#"[{"op": "set_type", "table": "Sales", "column": "수량", "type": "number"}]"#)
        .unwrap_err();
    assert_eq!(e.code, Code::Invalid);
    assert!(e.message.contains("operation 1 (set_type)") && e.message.contains("\"매출\""), "{}", e.message);
    // Range operations are a Spreadsheet's.
    let doc = open(&mut ws, "prototype/remainder/corpus/korean-report.docx");
    assert_eq!(ws.ops(&doc.doc_id, 1, "[]").unwrap_err().code, Code::BadRequest);
    let bytes = putget(&mut ws, &o.doc_id, "korean-sales.xlsx");
    let back = ws.open_bytes("again.xlsx", &bytes, None).unwrap();
    let r = ws.read(&back.doc_id, None, &w).unwrap();
    assert!(r.data.as_deref().unwrap().contains("| 11,111,111 |"));
}

// ------------------------------------------------------------ new files

#[test]
fn new_files_from_the_blank_packages() {
    let mut ws = ws();
    let cases: [(DocType, Format, &str); 4] = [
        (DocType::Document, Format::Docx, "---\ntype: document\nformat: docx\nschema: 1\n---\n# 3분기 보고\n\n매출은 **12%** 늘었다.\n\n- 신규 고객 34곳\n  - 수도권 21곳\n\n1. 다음 분기 목표\n1. 채용\n\n| 지역 | 매출 |\n|---|---|\n| 서울 | 120 |\n| 합계 | 215 |\n\n<div style=\"Quote\">고객이 있는 곳에 지점이 있다.</div>\n\n<p/>\n"),
        (DocType::Document, Format::Hwpx, "---\ntype: document\nformat: hwpx\nschema: 1\n---\n# 3분기 보고\n\n매출은 **12%** 늘었다.\n\n- 신규 고객 34곳\n- 수도권 21곳\n\n1. 다음 분기 목표\n1. 채용\n\n| 지역 | 매출 |\n|---|---|\n| 서울 | 120 |\n| 합계 | 215 |\n\n<div style=\"본문\">본문 스타일 문단.</div>\n"),
        (DocType::Presentation, Format::Pptx, "---\ntype: presentation\nformat: pptx\nschema: 1\n---\n\nlayout: Title Slide\n::title::\n3분기 보고\n::subtitle::\n영업본부\n\n---\n\nlayout: Title and Content\n::title::\n핵심 지표\n::body::\n- 매출 **12%** 증가\n  - 수도권 21곳\n::notes::\n강조할 것\n\n---\n\nlayout: Two Content\n::title::\n지역\n::left::\n- 수도권\n::right::\n- 지방\n"),
        (DocType::Spreadsheet, Format::Xlsx, ""),
    ];
    for (ty, f, body) in cases {
        let o = ws.create(ty, Some(f), None).unwrap_or_else(|e| panic!("{f:?}: {e}"));
        assert!(o.report.surfaced.is_empty() && o.report.neutralised.is_empty(), "a blank has nothing to surface");
        if f == Format::Xlsx {
            let ops = r##"[{"op": "add_table", "sheet": "Sheet1", "name": "Sales", "anchor": "A1", "columns": [{"name": "지점", "type": "text"}, {"name": "매출", "type": "number", "format": "#,##0"}, {"name": "원가", "type": "number", "format": "#,##0"}, {"name": "이익", "type": "number", "format": "#,##0", "formula": "=[@매출]-[@원가]"}], "rows": [{"지점": "강남", "매출": 1200, "원가": 800}, {"지점": "서초", "매출": 950, "원가": 700}]}]"##;
            ws.ops(&o.doc_id, 1, ops).unwrap_or_else(|e| panic!("{e}"));
            let r = ws.read(&o.doc_id, None, &Window::default()).unwrap();
            assert!(r.data.as_deref().unwrap().contains("| 2 | 강남 | 1,200 | 800 | 400 |"), "{:?}", r.data);
        } else {
            let c = ws.write(&o.doc_id, 1, body).unwrap_or_else(|e| panic!("{f:?}: {e}"));
            if f == Format::Pptx {
                // A deck's canonical text adds its slide size, each slot's box and
                // the text formatting it inherits (§5.3).
                let t = text(&ws, &o.doc_id).1;
                assert!(c.canonicalized && t.contains("size: 960 x 540 pt\n") && t.contains("::title box=\""), "{t}");
                assert!(t.contains("\n- 매출 **12%** 증가 {size=28pt}\n"), "{t}");
                let bare = t.replace("size: 960 x 540 pt\n", "");
                let bare: String = bare
                    .split_inclusive('\n')
                    .map(|l| match (l.starts_with("::"), l.find(" box=\""), l.rfind(" {")) {
                        (true, Some(k), _) => format!("{}::\n", &l[..k]),
                        (false, _, Some(k)) if l.ends_with("}\n") => format!("{}\n", &l[..k]),
                        _ => l.to_string(),
                    })
                    .collect();
                assert_eq!(bare, body);
            } else if f == Format::Docx {
                // A docx text's canonical form adds the style lines of the
                // styles it uses (§5.2).
                let t = text(&ws, &o.doc_id).1;
                assert!(c.canonicalized && t.contains("\n<style name=\"Normal\" "), "{t}");
                let bare: String = t.split_inclusive('\n').filter(|l| !l.starts_with("<style ")).collect();
                assert_eq!(bare.replacen("---\n\n", "---\n", 1), body);
            } else {
                // An hwpx text's canonical form adds the style lines too, and
                // the look a new table takes (Hancom's, in a file without one).
                let t = text(&ws, &o.doc_id).1;
                assert!(c.canonicalized && t.contains("\n<style name=\"바탕글\" "), "{t}");
                let look = "{border=\"0.34pt solid #000000\" valign=middle}\n";
                assert!(t.contains(&format!("{look}| 지역 | 매출 |")), "{t}");
                let bare: String =
                    t.split_inclusive('\n').filter(|l| !l.starts_with("<style ") && *l != look).collect();
                assert_eq!(bare.replacen("---\n\n", "---\n", 1), body);
            }
        }
        // Nothing to surface: exports without acknowledging.
        ws.export_bytes(&o.doc_id, None, &ExportOptions::default()).unwrap_or_else(|e| panic!("{f:?}: {e}"));
        let bytes = putget(&mut ws, &o.doc_id, &format!("new.{}", f.name()));
        if f == Format::Hwpx {
            // Next to the hwpx corpus exports, so the rhwp check (hanji-hwpx/validate) re-opens it too.
            let d = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("hwpx-corpus-out/store-blank");
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("ORIGINAL.hwpx"), blank::package(Format::Hwpx)).unwrap();
            std::fs::write(d.join("new.hwpx"), &bytes).unwrap();
        }
    }
}

#[test]
fn a_new_file_takes_its_type_s_formats_only() {
    let e = ws().create(DocType::Presentation, Some(Format::Docx), None).unwrap_err();
    assert_eq!(e.code, Code::BadRequest);
    assert!(e.message.contains("pptx"), "{}", e.message);
}

#[test]
fn a_template_s_reference_is_in_the_front_matter() {
    let mut ws = ws();
    let (_, bytes) = corpus("prototype/remainder/corpus/korean-report.docx");
    let o = ws.create(DocType::Document, None, Some(("org/report", &bytes))).unwrap();
    assert_eq!(o.template.as_deref(), Some("org/report"));
    let (_, t) = text(&ws, &o.doc_id);
    assert!(t.starts_with("---\ntype: document\nformat: docx\ntemplate: org/report\nschema: 1\n---\n"), "{t}");
    // Edits keep it.
    ws.edit(&o.doc_id, 1, &one("4분기에는", "4분기부터")).unwrap();
    assert!(text(&ws, &o.doc_id).1.contains("template: org/report"));
}

// ------------------------------------------------------------ refusals

#[test]
fn a_missing_old_names_invisible_and_private_use_characters() {
    // Korean government documents: U+2007 FIGURE SPACE after □, and a
    // Hancom symbol font's private-use U+F076 in a banner. The text keeps
    // them (PutGet); the error names them where `old` would match.
    for f in [Format::Hwpx, Format::Docx] {
        let mut ws = ws();
        let o = ws.create(DocType::Document, Some(f), None).unwrap();
        let fmt = format!("{f:?}").to_lowercase();
        let body = format!(
            "---\ntype: document\nformat: {fmt}\nschema: 1\n---\n\u{f076}2025년 사업 계획\n\n□\u{2007}추진 배경\n"
        );
        ws.write(&o.doc_id, 1, &body).unwrap_or_else(|e| panic!("{f:?}: {e}"));
        let (rev, t) = text(&ws, &o.doc_id);
        assert!(t.contains("\u{f076}2025년 사업 계획\n") && t.contains("□\u{2007}추진 배경\n"), "{f:?}: {t}");
        let e = ws.edit(&o.doc_id, rev, &one("□ 추진 배경", "□ 추진 경과")).unwrap_err();
        assert_eq!(e.code, Code::NoMatch);
        assert!(e.message.contains("\"□⟨U+2007 FIGURE SPACE⟩추진 배경\""), "{f:?}: {}", e.message);
        let e = ws.edit(&o.doc_id, rev, &one(" 2025년 사업 계획\n", "2026년 사업 계획\n")).unwrap_err();
        assert!(e.message.contains("⟨U+F076 private use⟩"), "{f:?}: {}", e.message);
        // Written with the characters, the edit applies.
        ws.edit(&o.doc_id, rev, &one("□\u{2007}추진 배경", "□\u{2007}추진 경과")).unwrap_or_else(|e| panic!("{e}"));
    }
}

#[test]
fn a_stale_revision_is_refused() {
    let mut ws = ws();
    let o = open(&mut ws, "prototype/remainder/corpus/korean-report.docx");
    ws.edit(&o.doc_id, 1, &one("4분기에는", "4분기부터")).unwrap();
    for e in [
        ws.edit(&o.doc_id, 1, &one("부산과 대구", "부산·대구")).unwrap_err(),
        ws.write(&o.doc_id, 1, "---\ntype: document\nformat: docx\nschema: 1\n---\nx\n").unwrap_err(),
    ] {
        assert_eq!((e.code, e.detail.head), (Code::StaleRevision, Some(2)));
        assert!(e.message.contains("Read revision 2"), "{}", e.message);
    }
    assert_eq!(ws.edit(&o.doc_id, 9, &one("a", "b")).unwrap_err().code, Code::NotFound);
    assert_eq!(ws.history(&o.doc_id).unwrap().head, 2, "a refusal makes no revision");
}

#[test]
fn an_ambiguous_or_missing_old_is_refused_with_its_count() {
    let mut ws = ws();
    let o = open(&mut ws, "prototype/remainder/corpus/korean-report.docx");
    let e = ws.edit(&o.doc_id, 1, &one("수도권", "서울·경기")).unwrap_err();
    assert_eq!((e.code, e.detail.matches), (Code::AmbiguousMatch, Some(2)));
    assert_eq!(e.detail.lines, [14, 16]);
    assert!(e.message.contains("occurs 2 times (lines 14, 16)"), "{}", e.message);
    let e = ws.edit(&o.doc_id, 1, &one("5분기", "6분기")).unwrap_err();
    assert_eq!((e.code, e.detail.matches), (Code::NoMatch, Some(0)));
    // In a list, the error names the edit, and nothing is applied.
    let e = ws
        .edit(
            &o.doc_id,
            1,
            &[
                TextEdit { old: "4분기에는".into(), new: "4분기부터".into() },
                TextEdit { old: "수도권".into(), new: "x".into() },
            ],
        )
        .unwrap_err();
    assert_eq!((e.code, e.detail.edit), (Code::AmbiguousMatch, Some(2)));
    assert!(e.message.starts_with("edit 2 of 2: "), "{}", e.message);
    assert_eq!(ws.history(&o.doc_id).unwrap().head, 1);
}

#[test]
fn validator_errors_have_line_column_and_the_allowed_names() {
    let mut ws = ws();
    let o = open(&mut ws, "prototype/remainder/corpus/korean-report.docx");
    let e = ws.edit(&o.doc_id, 1, &one("<div style=\"Note\">", "<div style=\"Callout\">")).unwrap_err();
    assert_eq!(e.code, Code::Invalid);
    let d = &e.detail.diagnostics[0];
    assert_eq!((d.line, d.col), (16, 6), "{d:?}");
    assert!(d.message.contains("\"Callout\"") && d.message.contains("\"Note\""), "{}", d.message);
    // validate: the grammar alone, or against a document's names.
    let bad = "---\ntype: document\nformat: docx\nschema: 1\n---\n| a | b |\n|---|---|\n| 1 |\n";
    let v = ws.validate(bad, None, None).unwrap();
    assert!(!v.valid && v.diagnostics[0].line == 8, "{v:?}");
    let styled = "---\ntype: document\nformat: docx\nschema: 1\n---\n<div style=\"Callout\">x</div>\n";
    assert!(ws.validate(styled, Some(DocType::Document), None).unwrap().valid);
    let v = ws.validate(styled, None, Some(&o.doc_id)).unwrap();
    assert!(!v.valid && v.diagnostics[0].message.contains("\"Note\""), "{v:?}");
    let deck = "---\ntype: presentation\nformat: pptx\nschema: 1\n---\n\nlayout: Title Slide\n::title::\n";
    let v = ws.validate(deck, None, None).unwrap();
    assert_eq!(v.doc_type, DocType::Presentation);
    assert!(!v.valid, "an empty marker is an error");
    assert_eq!(ws.validate("hello", None, None).unwrap_err().code, Code::BadRequest);
}

#[test]
fn export_shows_what_it_would_surface_and_needs_an_acknowledgement() {
    let mut ws = ws();
    let o = open(&mut ws, "prototype/remainder/corpus/korean-report.docx");
    let e = ws.export_bytes(&o.doc_id, None, &ExportOptions::default()).unwrap_err();
    assert_eq!(e.code, Code::SurfacedNotAcknowledged);
    assert!(
        e.detail.surfaced.iter().any(|s| s.kind == "comment" && s.detail.contains("지방 수치 재확인")),
        "{:?}",
        e.detail.surfaced
    );
    assert!(e.message.contains("--acknowledge-surfaced"), "{}", e.message);
    let (out, _) = ws.export_bytes(&o.doc_id, None, &ACK).unwrap();
    assert_eq!(out.surfaced, e.detail.surfaced);
    // Tracked changes (§10.2): with no edit since the file was opened, the same bytes.
    let tracked = ExportOptions { tracked_changes: true, ..ACK };
    let (_, a) = ws.export_bytes(&o.doc_id, None, &ACK).unwrap();
    let (_, b) = ws.export_bytes(&o.doc_id, None, &tracked).unwrap();
    assert_eq!(a, b);
    // docx only.
    let h = open(&mut ws, "crates/hanji-hwpx/corpus/basic-table-01.hwpx");
    let e = ws.export_bytes(&h.doc_id, None, &tracked).unwrap_err();
    assert_eq!(e.code, Code::Unsupported);
}

#[test]
fn a_person_s_edits_come_back_in_and_a_pending_edit_is_merged_or_refused() {
    let mut ws = ws();
    let o = open(&mut ws, "prototype/remainder/corpus/korean-report.docx");
    let (_, exported) = ws.export_bytes(&o.doc_id, None, &ACK).unwrap();
    // The person changes a word in Word and saves.
    let polished = person_edits(&exported, "word/document.xml", ">분기에는 <", ">분기부터는 <");
    let r = ws.reimport_bytes_at_revision(&o.doc_id, 1, "korean-report.docx", &polished).unwrap();
    assert_eq!((r.parent, r.revision), (1, 2));
    assert!(
        r.diff.contains("-4분기에는 [부산]{color=#C00000}과 [대구]{color=#C00000}에 지점을 연다.\n+4분기부터는 [부산]{color=#C00000}과 [대구]{color=#C00000}에 지점을 연다."),
        "{}",
        r.diff
    );
    assert_eq!(ws.history(&o.doc_id).unwrap().revisions[1].op, RevOp::Reimport);
    // A model edit made against revision 1, elsewhere: merged over the person's.
    let c = ws.edit(&o.doc_id, 1, &one("| ^^ | 종로 | 95 |", "| ^^ | 종로 | 97 |")).unwrap();
    assert_eq!((c.revision, c.rebased_over.clone()), (3, vec![2]));
    let (_, t) = text(&ws, &o.doc_id);
    assert!(t.contains("4분기부터는") && t.contains("| ^^ | 종로 | 97 |"));
    // One against revision 1 on the line the person changed: refused.
    let e = ws
        .edit(&o.doc_id, 1, &one("4분기에는 [부산]{color=#C00000}과", "4분기에는 [광주]{color=#C00000}와"))
        .unwrap_err();
    assert_eq!(e.code, Code::StaleRevision, "revision 3 is a model edit: stale, not merged");
    let (_, exported) = ws.export_bytes(&o.doc_id, None, &ACK).unwrap();
    let polished = person_edits(&exported, "word/document.xml", ">대구<", ">대구, 광주<");
    ws.reimport_bytes_at_revision(&o.doc_id, 3, "korean-report.docx", &polished).unwrap();
    let e = ws
        .edit(
            &o.doc_id,
            3,
            &one("[부산]{color=#C00000}과 [대구]{color=#C00000}에 지점을", "[부산]{color=#C00000}에 지점을"),
        )
        .unwrap_err();
    assert_eq!(e.code, Code::MergeConflict, "{}", e.message);
    assert_eq!(e.detail.conflicts[0].first_line, e.detail.conflicts[0].last_line);
    assert_eq!(e.detail.head, Some(4));
    // A whole-file rewrite against revision 3 is merged too.
    let (_, base) = ws.read(&o.doc_id, Some(3), &Window::default()).map(|r| (r.revision, r.text)).unwrap();
    let c = ws.write(&o.doc_id, 3, &base.replace("부록: 지점 목록", "부록: 지점 목록 (2026년)")).unwrap();
    assert_eq!(c.rebased_over, [4]);
    let (_, t) = text(&ws, &o.doc_id);
    assert!(t.contains("[부산]{color=#C00000}과 [대구, 광주]{color=#C00000}에") && t.contains("(2026년)"));
    // Re-importing the same file again changes nothing.
    let (_, exported) = ws.export_bytes(&o.doc_id, None, &ACK).unwrap();
    let r = ws.reimport_bytes_at_revision(&o.doc_id, c.revision, "korean-report.docx", &exported).unwrap();
    assert!(r.unchanged && r.diff.is_empty());
    assert_eq!(ws.reimport_bytes(&o.doc_id, "x.pptx", &exported).unwrap_err().code, Code::BadRequest);
}

#[test]
fn a_large_document_is_read_a_part_at_a_time() {
    let mut ws = ws();
    let o = ws.create(DocType::Document, None, None).unwrap();
    let mut body = String::from("---\ntype: document\nformat: docx\nschema: 1\n---\n");
    for s in 1..=30 {
        body.push_str(&format!("# 제{s}장\n"));
        for p in 1..=100 {
            body.push_str(&format!("제{s}장의 {p}번째 문단. 매출과 원가, 이익을 지점별로 정리한다.\n\n"));
        }
    }
    ws.write(&o.doc_id, 1, &body).unwrap();
    let r = ws.read(&o.doc_id, None, &Window::default()).unwrap();
    assert!(r.partial && r.text.len() <= 60_000 && r.first_line == 1, "{} bytes", r.text.len());
    let stored = ws.read(&o.doc_id, None, &Window { lines: Some("1:100000".into()), ..Default::default() }).unwrap();
    assert_eq!(r.total_lines, stored.total_lines);
    assert_eq!(r.outline.len(), 30);
    assert_eq!(r.outline[29].text, "# 제30장");
    assert!(r.next.as_deref().unwrap().starts_with(&format!("lines {}:", r.last_line + 1)));
    let s = ws.read(&o.doc_id, None, &Window { section: Some("제7장".into()), ..Default::default() }).unwrap();
    assert!(
        s.text.starts_with("# 제7장\n")
            && s.text.ends_with("제7장의 100번째 문단. 매출과 원가, 이익을 지점별로 정리한다.\n\n")
    );
    assert_eq!(s.first_line, r.outline[6].line);
    // An edit copies old from the part it read.
    let old = "제7장의 50번째 문단.";
    ws.edit(&o.doc_id, 2, &one(old, "제7장의 오십 번째 문단.")).unwrap();
    let l = ws
        .read(
            &o.doc_id,
            None,
            &Window { lines: Some(format!("{}:{}", s.first_line, s.first_line + 3)), ..Default::default() },
        )
        .unwrap();
    assert_eq!(l.text.lines().count(), 4);
    let e = ws.read(&o.doc_id, None, &Window { section: Some("제99장".into()), ..Default::default() }).unwrap_err();
    assert!(e.message.contains("# 제1장 (line 9)"), "{}", e.message);
}

#[test]
fn a_deck_is_read_by_slides() {
    let mut ws = ws();
    let o = open(&mut ws, "crates/hanji-pptx/corpus/korean-deck.pptx");
    let r = ws.read(&o.doc_id, None, &Window { slides: Some("2:3".into()), ..Default::default() }).unwrap();
    assert!(r.partial && r.text.starts_with(
        "layout: Title and Content\n::title box=\"36 22 648 90\" font=\"맑은 고딕\" size=44pt color=tx1::\n핵심 지표\n"
    ));
    assert!(r.text.contains("layout: Two Content") && !r.text.contains("분기별 매출"));
    assert_eq!(r.outline[3].text, "slide 4: layout: Title Only · 분기별 매출");
    let e = ws.read(&o.doc_id, None, &Window { section: Some("x".into()), ..Default::default() }).unwrap_err();
    assert_eq!(e.code, Code::BadRequest);
}

#[test]
fn an_edit_that_would_lose_content_is_refused_with_the_reason() {
    let mut ws = ws();
    let o = open(&mut ws, "crates/hanji-hwpx/corpus/basic-table-01.hwpx");
    // A merge over an existing cell: hwpx keeps no cell there, so what it held would go (refused on export).
    let e = ws.edit(&o.doc_id, 1, &one("| 9 | 10 |", "| 9 | ^^ |")).unwrap_err();
    assert_eq!(e.code, Code::Refused, "{e:?}");
    assert!(e.message.contains("row 3, column 2 is now covered by a merge"), "{}", e.message);
}

/// The guide's docx formatting example, written into a new docx, reads back
/// as written and exports with PutGet: its style edits, new style,
/// paragraph, run, cell, row and table formatting all land. The blank's
/// Heading 1 is not shown before the text uses it, so its line changes in a
/// second write (§5.2).
#[test]
fn the_guide_s_formatting_example_is_a_docx_text() {
    let from = GUIDE.find("<style name=\"Normal\" line-spacing=115%").unwrap();
    let body = &GUIDE[from..from + GUIDE[from..].find("\nPresentation (pptx)").unwrap()];
    // The blank's Heading 1 is named as Word names it.
    let body = body.trim_end().replace("\"Heading 1\"", "\"heading 1\"") + "\n";
    let text = format!("---\ntype: document\nformat: docx\nschema: 1\n---\n{body}");
    let heading = text.lines().find(|l| l.starts_with("<style name=\"heading 1\"")).unwrap().to_string() + "\n";
    let mut ws = ws();
    let o = ws.create(DocType::Document, Some(Format::Docx), None).unwrap();
    let c = ws.write(&o.doc_id, 1, &text.replace(&heading, "")).unwrap_or_else(|e| panic!("{e}"));
    assert!(c.canonicalized, "the text lists Heading 1's line");
    let (rev, shown) = self::text(&ws, &o.doc_id);
    let line = shown.lines().find(|l| l.starts_with("<style name=\"heading 1\"")).unwrap().to_string() + "\n";
    ws.write(&o.doc_id, rev, &shown.replace(&line, &heading)).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(self::text(&ws, &o.doc_id).1, text);
    let bytes = putget(&mut ws, &o.doc_id, "guide.docx");
    let parts = package::read(&bytes).unwrap();
    let styles = String::from_utf8(package::get(&parts, "word/styles.xml").unwrap().to_vec()).unwrap();
    assert!(styles.contains("<w:name w:val=\"Callout\"/>"), "{styles}");
}
