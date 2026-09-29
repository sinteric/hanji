//! The Presentation grammar (DESIGN.md §5.3).

use hanji_format::*;

const FM: &str = "---\ntype: presentation\nformat: pptx\ntemplate: org/deck\nschema: 1\n---\n";

const EXAMPLE: &str = "---\ntype: presentation\nformat: pptx\ntemplate: org/deck\nschema: 1\n---\n\nlayout: Title and Content\n::title::\n핵심 지표\n::body::\n- 매출 **12% 증가**\n- 신규 고객 34곳\n::notes::\n전년 대비 강조\n\n---\n\nlayout: Two Content\n::title::\n지역별 현황\n::left::\n- 수도권 21곳\n::right::\n- 지방 13곳\n<shape id=\"s4\" name=\"출처\">출처: 내부 집계</shape>\n";

fn names() -> Names {
    let l = |n: &str, s: &[&str]| Layout { name: n.into(), slots: s.iter().map(|x| x.to_string()).collect() };
    Names {
        layouts: Some(vec![
            l("Title Slide", &["title", "subtitle"]),
            l("Title and Content", &["title", "body", "date", "footer", "number"]),
            l("Two Content", &["title", "left", "right"]),
            l("Blank", &[]),
        ]),
        shapes: Some(vec![("s4".into(), "출처".into())]),
        keeps: Some(vec![
            Keep { id: "k1".into(), kind: "picture".into(), summary: "logo".into() },
            Keep { id: "k5".into(), kind: "chart".into(), summary: "매출 추이".into() },
            Keep { id: "k6".into(), kind: "picture".into(), summary: "map".into() },
        ]),
        objects: Some(vec!["k5".into(), "k6".into()]),
        ..Default::default()
    }
}

fn parse(text: &str) -> Result<ParsedPresentation, Vec<Diagnostic>> {
    parse_presentation(text, &names())
}

fn roundtrip(text: &str) -> String {
    let p = parse(text).unwrap_or_else(|e| panic!("{}\n---\n{text}", diag::render(&e)));
    let s = serialize_presentation(&p.pres);
    let p2 = parse(&s).unwrap_or_else(|e| panic!("reparse: {}\n---\n{s}", diag::render(&e)));
    assert_eq!(p.pres, p2.pres, "parse → serialize → parse changed the AST\n{s}");
    assert_eq!(serialize_presentation(&p2.pres), s, "not idempotent");
    s
}

fn errors(body: &str) -> Vec<String> {
    match parse(&format!("{FM}{body}")) {
        Ok(_) => vec![],
        Err(e) => e.iter().map(|d| d.to_string()).collect(),
    }
}

#[test]
fn design_example_is_canonical() {
    assert_eq!(roundtrip(EXAMPLE), EXAMPLE);
    let p = parse(EXAMPLE).unwrap().pres;
    assert_eq!(p.front.doc_type, "presentation");
    assert_eq!(p.slides.len(), 2);
    assert_eq!(p.slides[0].layout, "Title and Content");
    let names: Vec<&str> = p.slides[0]
        .items
        .iter()
        .map(|i| match i {
            SlideItem::Slot(s) => s.name.as_str(),
            SlideItem::Shape(_) => "shape",
            SlideItem::Object(_) => "object",
        })
        .collect();
    assert_eq!(names, ["title", "body", "notes"]);
    let SlideItem::Slot(body) = &p.slides[0].items[1] else { panic!() };
    let Block::List(items) = &body.blocks[0] else { panic!("{:?}", body.blocks) };
    assert_eq!(items.len(), 2);
    assert_eq!(serialize_inline(&items[0].content), "매출 **12% 증가**");
    let SlideItem::Shape(sh) = &p.slides[1].items[3] else { panic!() };
    assert_eq!((sh.id.as_str(), sh.name.as_str()), ("s4", "출처"));
    assert_eq!(sh.paras, vec![Inline::plain("출처: 내부 집계")]);
}

#[test]
fn source_map_marks_heads_and_paragraphs() {
    let p = parse(EXAMPLE).unwrap();
    let s1 = &p.map.slides[1];
    assert_eq!(&EXAMPLE[s1.head.start..s1.head.mark], "layout: Two Content");
    let title = &s1.items[0];
    assert_eq!(&EXAMPLE[title.head.start..title.head.mark], "::title::");
    let BlockMapKind::Para(pm) = &title.blocks[0].kind else { panic!() };
    assert_eq!(&EXAMPLE[pm.units[0]..pm.mark], "지역별 현황");
    let shape = &s1.items[3];
    assert_eq!(&EXAMPLE[shape.head.mark..shape.head.end], "<shape id=\"s4\" name=\"출처\">");
    let BlockMapKind::Para(pm) = &shape.blocks[0].kind else { panic!() };
    assert_eq!(&EXAMPLE[pm.units[0]..pm.mark], "출처: 내부 집계");
    assert_eq!(&EXAMPLE[pm.mark..pm.mark + 8], "</shape>");
}

#[test]
fn slot_text_is_document_text() {
    let text = format!("{FM}\nlayout: Title and Content\n::title::\n$x^2$ *and* <u>more</u><br/>line\n::body::\nintro\n\n- a\n  - b\n\n1. one\n\n<p/>\n<p/>\n\n<keep id=\"k1\" kind=\"picture\" summary=\"logo\"/>\n");
    assert_eq!(roundtrip(&text), text);
    let p = parse(&text).unwrap().pres;
    let SlideItem::Slot(body) = &p.slides[0].items[1] else { panic!() };
    assert_eq!(body.blocks.len(), 6, "{:?}", body.blocks);
    assert!(matches!(&body.blocks[5], Block::Keep(k) if k.id == "k1"));
}

#[test]
fn a_multi_paragraph_shape_uses_p_tags() {
    let text = format!("{FM}\nlayout: Blank\n<shape id=\"s4\" name=\"출처\">first<p/>second<p/><p/>last</shape>\n");
    assert_eq!(roundtrip(&text), text);
    let p = parse(&text).unwrap();
    let SlideItem::Shape(sh) = &p.pres.slides[0].items[0] else { panic!() };
    assert_eq!(sh.paras.len(), 4);
    assert_eq!(p.map.slides[0].items[0].blocks.len(), 4);
}

#[test]
fn quoted_layouts_and_lines_that_look_like_markers() {
    let text = format!("{FM}\nlayout: \"Two Content\"\n::title::\nx\n");
    let s = roundtrip(&text);
    assert!(s.contains("\nlayout: Two Content\n"), "{s}");
    // Text that reads as a marker or a separator is escaped.
    let mut pres = parse(&text).unwrap().pres;
    let SlideItem::Slot(t) = &mut pres.slides[0].items[0] else { panic!() };
    t.blocks = vec![
        Block::Para(Para { style: ParaStyle::Plain, content: Inline::plain("::body::") }),
        Block::Para(Para { style: ParaStyle::Plain, content: Inline::plain("---") }),
        Block::Para(Para { style: ParaStyle::Plain, content: Inline::plain("<shape id=\"s4\">") }),
        Block::Para(Para { style: ParaStyle::Plain, content: Inline::plain("layout: Blank") }),
    ];
    let s = serialize_presentation(&pres);
    assert!(s.contains("\\::body::\n\n\\---\n\n\\<shape id=\"s4\">\n\nlayout: Blank\n"), "{s}");
    assert_eq!(parse(&s).unwrap().pres, pres);
}

#[test]
fn no_slides_is_a_presentation() {
    let text = FM.to_string();
    assert_eq!(roundtrip(&text), text);
    assert!(parse(&text).unwrap().pres.slides.is_empty());
}

#[test]
fn a_second_dash_line_is_an_error() {
    let e = errors("\nlayout: Title and Content\n---\n::title::\nx\n");
    assert!(e[0].starts_with("line 9, column 1: this --- starts a new slide"), "{e:?}");
    assert!(e[0].contains("Slidev closes a slide's front matter with a second ---"), "{e:?}");
    let e = errors("\n---\nlayout: Blank\n");
    assert!(e[0].contains("no slide before this one"), "{e:?}");
    let e = errors("\nlayout: Blank\n\n---\n");
    assert!(e[0].contains("no slide follows this one"), "{e:?}");
    let e = errors("\nlayout: Blank\n---\n---\nlayout: Blank\n");
    assert!(e[0].contains("two lines --- with no slide between them"), "{e:?}");
}

#[test]
fn every_slide_starts_with_a_layout_and_lists_the_layouts() {
    let e = errors("\n::title::\nx\n");
    assert_eq!(e, ["line 8, column 1: every slide begins with a line layout: Name, the name of one of the file's layouts. Layouts: \"Title Slide\", \"Title and Content\", \"Two Content\", \"Blank\"."]);
    let e = errors("\nlayout: Title and content\n::title::\nx\n");
    assert!(
        e[0].contains("layout: Title and content is not a layout of this file") && e[0].contains("\"Two Content\""),
        "{e:?}"
    );
    let e = errors("\nlayout: Blank\ntransition: fade\n");
    assert!(e[0].contains("\"transition:\" is not a slide setting"), "{e:?}");
    let e = errors("\nlayout: Blank\nlayout: Title Slide\n");
    assert!(e[0].contains("already has its layout: line"), "{e:?}");
}

#[test]
fn only_the_layouts_slots_and_all_text_in_a_slot() {
    let e = errors("\nlayout: Title and Content\n::left::\nx\n");
    assert_eq!(e, ["line 9, column 1: ::left:: is not a slot of layout \"Title and Content\": a slide has only its layout's slots. Its slots: ::title::, ::body::, ::date::, ::footer::, ::number::, ::notes::."]);
    let e = errors("\nlayout: Title and Content\nloose text\n::title::\nx\n");
    assert!(e[0].starts_with("line 9, column 1: text outside a slot"), "{e:?}");
    assert!(e[0].contains("This slide's slots: ::title::, ::body::"), "{e:?}");
    let e = errors("\nlayout: Title and Content\n::title::\nx\n::title::\ny\n");
    assert!(e[0].contains("::title:: appears twice on this slide"), "{e:?}");
    let e = errors("\nlayout: Title and Content\n::title::\n::body::\n- x\n");
    assert!(e[0].contains("::title:: has no text. An unfilled slot is left out"), "{e:?}");
    let e = errors("\nlayout: Title and Content\n::Title::\nx\n");
    assert!(e[0].contains("Slot names are lower case: ::title::"), "{e:?}");
    // Every slide may have notes.
    assert!(errors("\nlayout: Blank\n::notes::\nx\n").is_empty());
}

#[test]
fn slot_text_has_no_styles_headings_or_tables() {
    for (body, want) in [
        ("# Heading\n", "a heading (#) is not slot text"),
        ("<div style=\"Note\">x</div>\n", "a slide has no paragraph styles"),
        ("<p style=\"Note\"/>\n", "a slide has no paragraph styles"),
        ("| a |\n|---|\n", "a table cannot be written in a slot yet"),
        ("<pagebreak/>\n", "a slide has no page breaks"),
    ] {
        let e = errors(&format!("\nlayout: Title and Content\n::body::\n{body}"));
        assert!(e.first().is_some_and(|x| x.contains(want) && x.contains("Slot text is plain lines")), "{body}: {e:?}");
    }
}

#[test]
fn shapes_come_from_the_file() {
    let e = errors("\nlayout: Blank\n<shape id=\"s9\" name=\"new\">x</shape>\n");
    assert!(e[0].contains("is not a shape of this file") && e[0].contains("never create one"), "{e:?}");
    let e = errors("\nlayout: Blank\n<shape id=\"s4\" name=\"renamed\">x</shape>\n");
    assert!(e[0].contains("is named \"출처\" in the file"), "{e:?}");
    let e = errors("\nlayout: Blank\n<shape id=\"s4\" name=\"출처\"></shape>\n");
    assert!(e[0].contains("a shape without text is not shown"), "{e:?}");
    let e = errors("\nlayout: Blank\n<shape id=\"s4\" name=\"출처\">x\n");
    assert!(e[0].contains("not closed by </shape>"), "{e:?}");
    let e = errors("\nlayout: Blank\n<shape id=\"s4\" name=\"출처\">x</shape> tail\n");
    assert!(e[0].contains("nothing may follow </shape>"), "{e:?}");
    let e = errors("\nlayout: Blank\n<shape id=\"s4\" name=\"출처\" x=\"1\">x</shape>\n");
    assert!(e[0].contains("the attributes id and name only"), "{e:?}");
    let e =
        errors("\nlayout: Blank\n<shape id=\"s4\" name=\"출처\">x</shape>\n<shape id=\"s4\" name=\"출처\">y</shape>\n");
    assert!(e[0].contains("appears twice on this slide"), "{e:?}");
    // A shape ends the slot before it; a marker after it starts another.
    let text =
        format!("{FM}\nlayout: Two Content\n::left::\n- a\n<shape id=\"s4\" name=\"출처\">x</shape>\n::right::\n- b\n");
    assert_eq!(roundtrip(&text), text);
}

#[test]
fn placeholders_are_checked_as_in_a_document() {
    let e = errors("\nlayout: Blank\n::notes::\n<keep id=\"k2\" kind=\"picture\" summary=\"logo\"/>\n");
    assert!(e[0].contains("never create one"), "{e:?}");
    let e = errors("\nlayout: Blank\n::notes::\n<keep id=\"k1\" kind=\"picture\" summary=\"logo\"/>\n\n---\n\nlayout: Blank\n::notes::\n<keep id=\"k1\" kind=\"picture\" summary=\"logo\"/>\n");
    assert!(e[0].contains("appears twice"), "{e:?}");
}

#[test]
fn a_document_is_not_a_presentation() {
    let e = parse_presentation("---\ntype: document\nformat: docx\nschema: 1\n---\n", &Names::default()).unwrap_err();
    assert!(
        e[0].to_string().contains("type: document is not a Presentation; this parser reads type: presentation."),
        "{e:?}"
    );
    let e = hanji_format::parse("---\ntype: presentation\nformat: pptx\nschema: 1\n---\n").unwrap_err();
    assert!(e[0].to_string().contains("type: presentation is not a Document"), "{e:?}");
}

#[test]
fn objects_are_keep_lines_among_the_slots() {
    let text = format!(
        "{FM}\nlayout: Title and Content\n<keep id=\"k6\" kind=\"picture\" summary=\"map\"/>\n::title::\n매출\n<keep id=\"k5\" kind=\"chart\" summary=\"매출 추이\"/>\n::body::\n<keep id=\"k1\" kind=\"picture\" summary=\"logo\"/>\n"
    );
    assert_eq!(roundtrip(&text), text);
    let p = parse(&text).unwrap();
    let kinds: Vec<String> = p.pres.slides[0]
        .items
        .iter()
        .map(|i| match i {
            SlideItem::Slot(s) => s.name.clone(),
            SlideItem::Object(k) => k.id.clone(),
            SlideItem::Shape(_) => "shape".into(),
        })
        .collect();
    // k5 ends the title slot; k1, a slot's object, is the body's text.
    assert_eq!(kinds, ["k6", "title", "k5", "body"]);
    let SlideItem::Slot(body) = &p.pres.slides[0].items[3] else { panic!() };
    assert!(matches!(&body.blocks[..], [Block::Keep(k)] if k.id == "k1"));
    // Head and placeholder have distinct offsets on the line.
    let m = &p.map.slides[0].items[0];
    assert!(m.head.mark < m.blocks[0].start && m.blocks[0].end > m.blocks[0].start);
    // An object is kept exactly: never created or altered.
    let bad = errors("\nlayout: Blank\n<keep id=\"k5\" kind=\"chart\" summary=\"other\"/>\n");
    assert!(bad.iter().any(|e| e.contains("altered")), "{bad:?}");
    let bad = errors("\nlayout: Blank\n<keep id=\"k9\" kind=\"chart\" summary=\"x\"/>\n");
    assert!(bad.iter().any(|e| e.contains("not in this file")), "{bad:?}");
}

#[test]
#[ignore = "DESIGN.md §5.3 now writes geometry (size, box, <line>), which hanji-format reads once the geometry implementation lands; re-enable it then"]
fn the_design_md_example_parses() {
    // DESIGN.md §5.3's example, read from the spec itself so the two cannot drift.
    let spec = include_str!("../../../DESIGN.md");
    let body = &spec[spec.find("### 5.3 Presentation").unwrap()..];
    let a = body.find("```\n").unwrap() + 4;
    let b = a + body[a..].find("```\n").unwrap();
    let text = &body[a..b];
    let p = parse_presentation(text, &Names::default()).unwrap_or_else(|e| panic!("{}", diag::render(&e)));
    assert_eq!(serialize_presentation(&p.pres), text, "the example is in canonical form");
    let items: Vec<String> = p.pres.slides[1]
        .items
        .iter()
        .map(|i| match i {
            SlideItem::Slot(s) => format!("::{}::", s.name),
            SlideItem::Shape(sh) => format!("shape {} ({} paragraphs)", sh.id, sh.paras.len()),
            SlideItem::Object(k) => format!("object {}", k.kind),
        })
        .collect();
    assert_eq!(items, ["object picture", "::title::", "::left::", "::right::", "shape s4 (2 paragraphs)"]);
}
