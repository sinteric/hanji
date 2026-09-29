//! Engine behaviour on corpus decks: exact edits, slides added, deleted and
//! moved, notes, layout changes, refusals, and §8 neutralisation.

use hanji_core::edit::{edit_in, reanchor_span_in, rewrite_in};
use hanji_core::{Capabilities, Engine, EngineError, ImportOptions, Imported, Part, Remainder};
use hanji_package::{opc, package};
use hanji_pptx::{PptxEngine, PptxModel};

const CAPS: Capabilities = Capabilities { links: false, fields: false, footnotes: false, math: false };

fn deck(name: &str) -> Vec<u8> {
    let path = format!("{}/corpus/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn import(pkg: &[u8]) -> Imported {
    PptxEngine.import(pkg, &ImportOptions::default()).unwrap_or_else(|e| panic!("{e}"))
}

/// Exports `text`, checks that the export reads back as `text`, and returns its parts.
fn export(text: &str, rem: &Remainder) -> Vec<Part> {
    let out = PptxEngine.export(text, rem).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(import(&out).text, text, "PutGet");
    let parts = package::read(&out).unwrap();
    for p in &parts {
        if p.name.ends_with(".xml") || p.name.ends_with(".rels") {
            hanji_package::xml::parse(&p.data).unwrap_or_else(|e| panic!("{}: {e}", p.name));
        }
    }
    parts
}

fn xml(parts: &[Part], name: &str) -> String {
    String::from_utf8(package::get(parts, name).unwrap_or_else(|| panic!("no {name}")).to_vec()).unwrap()
}

/// The slide parts in `p:sldIdLst` order.
fn slides(parts: &[Part]) -> Vec<String> {
    let pres = xml(parts, "ppt/presentation.xml");
    let list = &pres[pres.find("<p:sldIdLst>").unwrap()..pres.find("</p:sldIdLst>").unwrap()];
    list.split("r:id=\"")
        .skip(1)
        .map(|s| opc::target_of(parts, "ppt/presentation.xml", &s[..s.find('"').unwrap()]).unwrap())
        .collect()
}

fn rel_targets(parts: &[Part], part: &str, short: &str) -> Vec<String> {
    opc::rels_of(parts, part)
        .iter()
        .filter(|r| r.short_type() == short && !r.external)
        .map(|r| opc::resolve_target(part, &r.target))
        .collect()
}

fn span_edit(imp: &Imported, start: usize, end: usize, new: &str) -> (String, Remainder) {
    let r = reanchor_span_in(&PptxModel, &imp.remainder, &imp.text, start, end, new, CAPS)
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert!(r.report.refused.is_empty(), "{:?}", r.report);
    (r.text, r.remainder)
}

/// The byte range of slide `k` (0-based) in a text, from its `layout:` line
/// to the next separator (`b + 5` is past the separator and its blank line).
fn slide_span(text: &str, k: usize) -> (usize, usize) {
    let starts: Vec<usize> = text.match_indices("layout: ").map(|(i, _)| i).collect();
    let end = starts.get(k + 1).map_or(text.len(), |&n| text[..n].rfind("---\n").unwrap());
    (starts[k], end)
}

#[test]
fn a_title_edit_keeps_the_run_formatting_and_everything_else() {
    let pkg = deck("korean-deck.pptx");
    let imp = import(&pkg);
    let r = edit_in(&PptxModel, &imp.remainder, &imp.text, "핵심 지표", "핵심 성과 지표", CAPS).unwrap();
    assert!(r.report.refused.is_empty() && r.report.removed.is_empty(), "{:?}", r.report);
    let parts = export(&r.text, &r.remainder);
    let before = package::read(&pkg).unwrap();
    let s2 = &slides(&parts)[1];
    assert!(xml(&parts, s2).contains(">핵심 성과 지표</a:t>"));
    // Every other part is byte-equal.
    for p in &before {
        if &p.name != s2 {
            assert_eq!(package::get(&parts, &p.name), Some(&p.data[..]), "{} changed", p.name);
        }
    }
    let r = edit_in(&PptxModel, &imp.remainder, &imp.text, "12% 증가", "15% 증가", CAPS).unwrap();
    let parts = export(&r.text, &r.remainder);
    let s = xml(&parts, &slides(&parts)[1]);
    assert!(s.contains(" b=\"1\"") && s.contains(">15% 증가</a:t>"), "the bold run keeps its bold");
}

#[test]
fn a_new_slide_comes_from_its_layout() {
    let imp = import(&deck("korean-deck.pptx"));
    let n = imp.text.len();
    let (text, rem) =
        span_edit(&imp, n, n, "\n---\n\nlayout: Title and Content\n::title::\n새 슬라이드\n::body::\n- 하나\n  - 둘\n");
    let parts = export(&text, &rem);
    let list = slides(&parts);
    assert_eq!(list.len(), 8);
    let new = list.last().unwrap();
    assert!(!slides(&package::read(&deck("korean-deck.pptx")).unwrap()).contains(new));
    let layout = rel_targets(&parts, new, "slideLayout");
    assert_eq!(layout.len(), 1);
    assert!(xml(&parts, &layout[0]).contains("name=\"Title and Content\""));
    let s = xml(&parts, new);
    assert!(s.contains("<p:ph type=\"title\"/>") && s.contains(">새 슬라이드</a:t>"), "{s}");
    assert!(
        s.contains("<a:pPr lvl=\"1\"/>") && s.matches("<p:spPr/>").count() == 2,
        "levels, and geometry from the layout: {s}"
    );
    let ct = xml(&parts, "[Content_Types].xml");
    assert!(ct.contains(&format!("PartName=\"/{new}\"")), "{ct}");
    // Slide ids stay unique.
    let pres = xml(&parts, "ppt/presentation.xml");
    let mut ids: Vec<&str> = pres.split("<p:sldId id=\"").skip(1).map(|s| &s[..s.find('"').unwrap()]).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 8);
}

#[test]
fn deleting_a_slide_removes_its_parts_and_moving_one_reorders_the_list() {
    let pkg = deck("korean-deck.pptx");
    let before = package::read(&pkg).unwrap();
    let old = slides(&before);
    let imp = import(&pkg);
    // Delete the second slide, which has notes.
    let (a, b) = slide_span(&imp.text, 1);
    let (text, rem) = span_edit(&imp, a, b + 5, "");
    let parts = export(&text, &rem);
    let now = slides(&parts);
    assert_eq!(now.len(), 6);
    assert!(!now.contains(&old[1]) && package::get(&parts, &old[1]).is_none());
    let notes = rel_targets(&before, &old[1], "notesSlide");
    assert_eq!(notes.len(), 1);
    assert!(package::get(&parts, &notes[0]).is_none(), "its notes page goes too");
    let ct = xml(&parts, "[Content_Types].xml");
    assert!(!ct.contains(&format!("/{}\"", old[1])) && !ct.contains(&format!("/{}\"", notes[0])));
    // Move the last slide first: the same parts, in a new order.
    let body = slide_span(&imp.text, 0).0;
    let mut chunks: Vec<&str> = imp.text[body..].trim_end().split("\n\n---\n\n").collect();
    chunks.rotate_right(1);
    let text = format!("{}{}\n", &imp.text[..body], chunks.join("\n\n---\n\n"));
    let r = rewrite_in(&PptxModel, &imp.remainder, &imp.text, &text, CAPS).unwrap_or_else(|e| panic!("{e:?}"));
    let parts = export(&r.text, &r.remainder);
    let now = slides(&parts);
    assert_eq!(now[0], old[6]);
    assert_eq!(&now[1..], &old[..6]);
    for s in &old {
        assert_eq!(package::get(&parts, s), package::get(&before, s), "{s} is unchanged");
    }
}

#[test]
fn notes_are_edited_and_added() {
    let pkg = deck("sld-notes.pptx");
    let imp = import(&pkg);
    assert!(imp.text.contains("::notes::\nNotes\n"));
    let r = edit_in(&PptxModel, &imp.remainder, &imp.text, "Notes", "발표 메모", CAPS).unwrap();
    let parts = export(&r.text, &r.remainder);
    let n = rel_targets(&parts, &slides(&parts)[0], "notesSlide");
    assert!(xml(&parts, &n[0]).contains(">발표 메모</a:t>"));
    // The second slide has no notes page: one is made from the notes master.
    let n = imp.text.len();
    let (text, rem) = span_edit(&imp, n, n, "::notes::\n새 메모\n");
    let parts = export(&text, &rem);
    let second = &slides(&parts)[1];
    let notes = rel_targets(&parts, second, "notesSlide");
    assert_eq!(notes.len(), 1, "the slide links its new notes page");
    let x = xml(&parts, &notes[0]);
    assert!(x.contains(">새 메모</a:t>") && x.contains("type=\"body\""), "{x}");
    assert_eq!(rel_targets(&parts, &notes[0], "slide"), vec![second.clone()]);
    assert_eq!(rel_targets(&parts, &notes[0], "notesMaster").len(), 1);
    assert!(xml(&parts, "[Content_Types].xml").contains(&format!("/{}\"", notes[0])));
}

#[test]
fn shape_text_and_a_layout_change() {
    let imp = import(&deck("korean-deck.pptx"));
    let r = edit_in(&PptxModel, &imp.remainder, &imp.text, "출처: 내부 집계", "출처: 영업본부", CAPS).unwrap();
    let parts = export(&r.text, &r.remainder);
    let s = xml(&parts, &slides(&parts)[2]);
    assert!(s.contains(">출처: 영업본부</a:t>") && s.contains("name=\"출처\""));
    // The section header becomes a title-and-content slide: its placeholders
    // take the new layout's type and index, and the slide its relationship.
    let r = edit_in(&PptxModel, &imp.remainder, &imp.text, "layout: Section Header", "layout: Title and Content", CAPS)
        .unwrap();
    let parts = export(&r.text, &r.remainder);
    let s6 = &slides(&parts)[5];
    let layout = rel_targets(&parts, s6, "slideLayout");
    assert!(xml(&parts, &layout[0]).contains("name=\"Title and Content\""));
    assert!(xml(&parts, s6).contains("<p:ph idx=\"1\"/>"), "{}", xml(&parts, s6));
}

#[test]
fn invalid_texts_are_refused_with_what_is_allowed() {
    let imp = import(&deck("korean-deck.pptx"));
    let bad = imp.text.replace("layout: Title Only", "layout: Nope");
    match PptxEngine.export(&bad, &imp.remainder) {
        Err(EngineError::Invalid(d)) => {
            let m = hanji_format::diag::render(&d);
            assert!(m.contains("Nope") && m.contains("Title and Content"), "{m}");
        }
        other => panic!("{other:?}"),
    }
    let bad = imp.text.replace("::title::\n분기별 매출", "::title::\n분기별 매출\n::body::\n표");
    match PptxEngine.export(&bad, &imp.remainder) {
        Err(EngineError::Invalid(d)) => assert!(hanji_format::diag::render(&d).contains("body"), "{d:?}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_slide_another_slide_links_to_is_not_deleted() {
    let imp = import(&deck("shapes.pptx"));
    let at = imp.text.find("layout: Title Slide").unwrap();
    let k = imp.text[..at].matches("layout: ").count();
    let (a, b) = slide_span(&imp.text, k);
    let r = reanchor_span_in(&PptxModel, &imp.remainder, &imp.text, a, b + 5, "", CAPS).unwrap();
    match PptxEngine.export(&r.text, &r.remainder) {
        Err(EngineError::Refused(m)) => assert!(m.contains("slide"), "{m}"),
        other => panic!("expected a refusal, got {:?}", other.map(|_| ())),
    }
}

#[test]
fn macros_ole_objects_and_program_actions_are_neutralised() {
    let pkg = deck("act-props.pptm");
    let imp = import(&pkg);
    let kinds: Vec<&str> = imp.report.neutralised.iter().map(|n| n.kind.as_str()).collect();
    for k in ["macros", "ole-object", "click-action"] {
        assert!(kinds.contains(&k), "{k}: {:?}", imp.report.neutralised);
    }
    let parts = export(&imp.text, &imp.remainder);
    assert!(parts.iter().all(|p| !p.name.ends_with("vbaProject.bin") && !p.name.contains("embeddings/")));
    let ct = xml(&parts, "[Content_Types].xml");
    assert!(!ct.contains("macroEnabled") && ct.contains("presentationml.presentation.main+xml"), "{ct}");
    let all: String = parts.iter().filter(|p| p.name.ends_with(".xml")).map(|p| xml(&parts, &p.name)).collect();
    assert!(!all.contains("ppaction://macro") && !all.contains("ppaction://program") && !all.contains("oleObj"));
    assert!(all.contains("ppaction://hlinkshowjump?jump=firstslide"), "slide jumps stay");
    assert!(PptxEngine
        .import(&package::write(&parts).unwrap(), &ImportOptions::default())
        .unwrap()
        .report
        .neutralised
        .is_empty());
}

#[test]
fn linked_media_and_external_objects_are_cut() {
    let pkg = deck("ext-rels.pptx");
    let mut parts = package::read(&pkg).unwrap();
    let s1 = slides(&parts)[0].clone();
    let rels = opc::rels_part(&s1);
    let r = parts.iter_mut().find(|p| p.name == rels).unwrap();
    let x = String::from_utf8(r.data.clone()).unwrap().replace(
        "</Relationships>",
        "<Relationship Id=\"rId90\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/video\" Target=\"file:///C:/talk.mp4\" TargetMode=\"External\"/><Relationship Id=\"rId91\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"http://example.com/x.png\" TargetMode=\"External\"/></Relationships>",
    );
    r.data = x.into_bytes();
    let s = parts.iter_mut().find(|p| p.name == s1).unwrap();
    let x = String::from_utf8(s.data.clone()).unwrap().replace(
        "</p:spTree>",
        "<p:pic><p:nvPicPr><p:cNvPr id=\"90\" name=\"Linked\"/><p:cNvPicPr/><p:nvPr><a:videoFile r:link=\"rId90\"/></p:nvPr></p:nvPicPr><p:blipFill><a:blip r:link=\"rId91\"/></p:blipFill><p:spPr/></p:pic></p:spTree>",
    );
    s.data = x.into_bytes();
    let imp = import(&package::write(&parts).unwrap());
    let kinds: Vec<&str> = imp.report.neutralised.iter().map(|n| n.kind.as_str()).collect();
    assert!(kinds.iter().any(|k| k.contains("linked")), "{:?}", imp.report.neutralised);
    let out = export(&imp.text, &imp.remainder);
    let x = xml(&out, &rels);
    assert!(!x.contains("talk.mp4") && !x.contains("example.com/x.png"), "{x}");
    assert!(x.contains("github.com"), "hyperlinks stay: {x}");
    assert!(!xml(&out, &s1).contains("r:link"));
}
