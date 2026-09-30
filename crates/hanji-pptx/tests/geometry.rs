//! Geometry in the Presentation text (DESIGN.md §5.3): every object's box in
//! points, a box left as shown kept exact, boxes edited, objects added, and
//! the edits that are refused (attached connectors, groups, the slide size).

use hanji_core::edit::{edit_in, rewrite_in};
use hanji_core::{Capabilities, Engine, EngineError, ImportOptions, Imported, Part, Remainder, TextModel};
use hanji_package::{opc, package};
use hanji_pptx::{PptxEngine, PptxModel};

const CAPS: Capabilities =
    Capabilities { links: false, fields: false, footnotes: false, math: false, formatting: false };

fn deck(name: &str) -> Vec<u8> {
    let path = format!("{}/corpus/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn import(pkg: &[u8]) -> Imported {
    PptxEngine.import(pkg, &ImportOptions::default()).unwrap_or_else(|e| panic!("{e}"))
}

/// The canonical form of `text`: what the write returns.
fn canonical(text: &str, rem: &Remainder) -> String {
    let (blocks, _) = PptxModel.resolve(text, rem, CAPS).unwrap_or_else(|e| panic!("{e:?}"));
    PptxEngine::text_of(&blocks, rem, None)
}

/// The ids and names the write gave new objects (`(id, name)`), left out to compare texts.
fn without_new_ids(text: &str, new: &[(&str, &str)]) -> String {
    let mut t = text.to_string();
    for (id, name) in new {
        for tag in ["<shape", "<line"] {
            t = t.replacen(&format!("{tag} id=\"{id}\" name=\"{name}\""), tag, 1);
        }
    }
    t
}

/// Placeholder ids (made from each object's content) as `?`.
fn keep_ids_out(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(k) = rest.find("<keep id=\"") {
        out.push_str(&rest[..k + 10]);
        rest = &rest[k + 10..];
        out.push('?');
        rest = &rest[rest.find('"').unwrap()..];
    }
    out + rest
}

/// Exports `text`; the export reads back as its canonical form (PutGet), `new` naming the ids new objects got.
fn export_new(text: &str, rem: &Remainder, new: &[(&str, &str)]) -> (Vec<Part>, String) {
    let out = PptxEngine.export(text, rem).unwrap_or_else(|e| panic!("{e}"));
    let back = import(&out).text;
    let (got, want) = (without_new_ids(&back, new), canonical(text, rem));
    if got != want {
        let d = got.lines().zip(want.lines()).find(|(a, b)| a != b);
        panic!("PutGet: first difference (read back, canonical): {d:?}");
    }
    (package::read(&out).unwrap(), back)
}

fn export(text: &str, rem: &Remainder) -> Vec<Part> {
    export_new(text, rem, &[]).0
}

fn xml(parts: &[Part], name: &str) -> String {
    String::from_utf8(package::get(parts, name).unwrap_or_else(|| panic!("no {name}")).to_vec()).unwrap()
}

fn slides(parts: &[Part]) -> Vec<String> {
    let pres = xml(parts, "ppt/presentation.xml");
    let list = &pres[pres.find("<p:sldIdLst>").unwrap()..pres.find("</p:sldIdLst>").unwrap()];
    list.split("r:id=\"")
        .skip(1)
        .map(|s| opc::target_of(parts, "ppt/presentation.xml", &s[..s.find('"').unwrap()]).unwrap())
        .collect()
}

/// An exact edit that must place every entry (none removed, none refused).
fn exact(imp_text: &str, rem: &Remainder, old: &str, new: &str) -> (String, Remainder) {
    let r = edit_in(&PptxModel, rem, imp_text, old, new, CAPS).unwrap_or_else(|e| panic!("{old:?}: {e}"));
    assert!(r.report.refused.is_empty() && r.report.removed.is_empty(), "{:?}", r.report);
    (r.text, r.remainder)
}

/// Every `a:xfrm` and `p:xfrm` of a part, in order.
fn xfrms(x: &str) -> Vec<String> {
    let mut out = vec![];
    for (open, close) in [("<a:xfrm", "</a:xfrm>"), ("<p:xfrm", "</p:xfrm>")] {
        let mut rest = x;
        while let Some(k) = rest.find(open) {
            let e = rest[k..].find(close).map_or(k + open.len(), |e| k + e + close.len());
            out.push(rest[k..e].to_string());
            rest = &rest[e..];
        }
    }
    out
}

/// The `a:xfrm` of the shape named `name` in slide XML.
fn xfrm_of(x: &str, name: &str) -> String {
    let at = x.find(&format!("name=\"{name}\"")).unwrap_or_else(|| panic!("no {name}"));
    let rest = &x[at..];
    let end = rest.find("</p:sp>").or_else(|| rest.find("</p:pic>")).unwrap();
    xfrms(&rest[..end]).into_iter().next().unwrap_or_default()
}

#[test]
fn every_object_shows_its_box_and_getput_keeps_every_xfrm() {
    let dir = format!("{}/corpus", env!("CARGO_MANIFEST_DIR"));
    let mut n = 0;
    for f in std::fs::read_dir(&dir).unwrap() {
        let path = f.unwrap().path();
        if !matches!(path.extension().and_then(|e| e.to_str()), Some("pptx" | "pptm")) {
            continue;
        }
        let pkg = std::fs::read(&path).unwrap();
        let before = package::read(&pkg).unwrap();
        let raw = ImportOptions { neutralise: false, template: None };
        let imp = PptxEngine.import(&pkg, &raw).unwrap_or_else(|e| panic!("{path:?}: {e}"));
        let out = PptxEngine.export(&imp.text, &imp.remainder).unwrap_or_else(|e| panic!("{path:?}: {e}"));
        let after = package::read(&out).unwrap();
        for s in slides(&before) {
            // Every stored box unchanged, and no box added where there was none.
            assert_eq!(xfrms(&xml(&after, &s)), xfrms(&xml(&before, &s)), "{path:?} {s}");
        }
        n += 1;
    }
    assert_eq!(n, 22);
    // shapes.pptx slide 1 reads as round 5's rendering of it: connectors and
    // textless shapes are shown (rule 8), every object with its box.
    let imp = import(&deck("shapes.pptx"));
    let s1 = [
        "layout: Blank",
        "<shape id=\"s4\" name=\"TextBox 3\" box=\"72 72 180 29\" font=Calibri size=18pt color=tx1>Learning PPTX</shape>",
        "<line id=\"s6\" name=\"Straight Connector 5\" from=\"84 144\" to=\"252 144\" border=\"0.75pt solid accent1*\"/>",
        "<shape id=\"s7\" name=\"Freeform 6\" box=\"47 211 185 136\" fill=accent1 border=\"2pt solid accent1*\" font=Calibri size=18pt color=lt1>Cloud</shape>",
        "<picture id=\"s2\" name=\"Picture 1\" box=\"402 78 144 132\" src=\"media/image1.jpg\"/>",
        "<keep id=\"?\" kind=\"table\" summary=\"Table 2: Column1 Column2 Column3 data1 data2 data3\" box=\"300 372 372 96\"/>",
        "<line id=\"s8\" name=\"Straight Arrow Connector 7\" from=\"468 366\" to=\"468 216\" border=\"0.75pt solid accent1*\" start=arrow end=arrow/>",
        "<line id=\"s10\" name=\"Elbow Connector 9\" from=\"186 252\" to=\"402 144\" border=\"0.75pt solid accent1*\" end=arrow/>",
    ]
    .join("\n");
    assert!(keep_ids_out(&imp.text).contains(&s1), "{}", imp.text);
    assert!(imp.text.contains("<group id=\"g5\" name=\"Group 4\" box=\"120 108 258 152\">\n<shape id=\"s2\" name=\"Rectangle 1\" box=\"120 108 138 60\" fill=accent1 border=\"2pt solid accent1*\"/>\n"), "{}", imp.text);
    assert!(imp.text.contains(
        "<shape id=\"s13\" name=\"Rectangle 12\" box=\"594 72 72 72\" fill=accent1-50% border=\"2pt solid accent1*\"/>"
    ));
    assert!(imp.text.starts_with("---\ntype: presentation\nformat: pptx\nschema: 1\nsize: 720 x 540 pt\n---\n"));
    // Every slot shows its box, inherited from the layout or the master.
    let k = import(&deck("korean-deck.pptx"));
    assert!(
        k.text.contains("layout: Title Slide\n::title box=\"54 168 612 116\" font=Calibri size=44pt color=tx1::\n"),
        "{}",
        k.text
    );
    assert!(k.text.contains("::body box=\"36 126 648 356\" font=Calibri color=tx1::\n"));
}

#[test]
fn a_box_left_as_shown_keeps_its_exact_emu() {
    let imp = import(&deck("korean-deck.pptx"));
    let before = package::read(&deck("korean-deck.pptx")).unwrap();
    let s3 = slides(&before)[2].clone();
    // x changes; y (475.2 pt, shown 475) keeps its stored EMU.
    let (text, rem) = exact(
        &imp.text,
        &imp.remainder,
        "box=\"36 475 288 29\" font=Calibri size=18pt color=tx1>출처",
        "box=\"396 475 288 29\" font=Calibri size=18pt color=tx1>출처",
    );
    let parts = export(&text, &rem);
    assert_eq!(
        xfrm_of(&xml(&parts, &s3), "출처"),
        "<a:xfrm><a:off x=\"5029200\" y=\"6035040\"/><a:ext cx=\"3657600\" cy=\"365760\"/></a:xfrm>"
    );
    // A slot's inherited box: a changed height gives it an a:xfrm of its own,
    // the other numbers the master's exact ones (y 274638 EMU shows as 22 pt).
    let s2 = slides(&before)[1].clone();
    assert!(!xml(&before, &s2).contains("<a:xfrm"));
    let old = "::title box=\"36 22 648 90\" font=Calibri size=44pt color=tx1::\n핵심 지표";
    let (text, rem) = exact(
        &imp.text,
        &imp.remainder,
        old,
        "::title box=\"36 22 648 60\" font=Calibri size=44pt color=tx1::\n핵심 지표",
    );
    let parts = export(&text, &rem);
    assert_eq!(
        xfrms(&xml(&parts, &s2)),
        ["<a:xfrm><a:off x=\"457200\" y=\"274638\"/><a:ext cx=\"8229600\" cy=\"762000\"/></a:xfrm>"]
    );
    // Written back at the layout's box, or without a box, the slot sits where its layout puts it again.
    let imp2 = import(&PptxEngine.export(&text, &rem).unwrap());
    for back in [old, "::title::\n핵심 지표"] {
        let (t2, r2) = exact(
            &imp2.text,
            &imp2.remainder,
            "::title box=\"36 22 648 60\" font=Calibri size=44pt color=tx1::\n핵심 지표",
            back,
        );
        let parts = export(&t2, &r2);
        assert!(!xml(&parts, &s2).contains("<a:xfrm"), "{back}");
    }
}

#[test]
fn objects_move_resize_align_and_a_text_box_is_added() {
    let pkg = deck("korean-deck.pptx");
    let imp = import(&pkg);
    let s4 = slides(&package::read(&pkg).unwrap())[3].clone();
    // Resize the picture, add a text box under the title.
    let (text, rem) = exact(
        &imp.text,
        &imp.remainder,
        "name=\"Picture 3\" box=\"576 396 72 36\"",
        "name=\"Picture 3\" box=\"504 360 144 72\"",
    );
    let (text, rem) = exact(
        &text,
        &rem,
        "::title box=\"36 22 648 90\" font=Calibri size=44pt color=tx1::\n분기별 매출\n",
        "::title box=\"36 22 648 90\" font=Calibri size=44pt color=tx1::\n분기별 매출\n<shape box=\"36 112 648 28\">단위: 억 원</shape>\n",
    );
    let (parts, back) = export_new(&text, &rem, &[("s5", "TextBox 4")]);
    let x = xml(&parts, &s4);
    assert!(x.contains("<a:off x=\"6400800\" y=\"4572000\"/><a:ext cx=\"1828800\" cy=\"914400\"/>"), "{x}");
    let tb = &x[x.find("name=\"TextBox 4\"").expect("the new text box is named for its id")..];
    assert!(tb.starts_with("name=\"TextBox 4\"/><p:cNvSpPr txBox=\"1\"/>"), "{tb}");
    assert!(tb.contains("<a:off x=\"457200\" y=\"1422400\"/><a:ext cx=\"8229600\" cy=\"355600\"/>"));
    assert!(tb.contains(">단위: 억 원</a:t>"));
    // The text read back shows the new box's id and name, after the title, before the table.
    assert!(
        back.contains(
            "분기별 매출\n<shape id=\"s5\" name=\"TextBox 4\" box=\"36 112 648 28\" font=Calibri size=18pt color=tx1>단위: 억 원</shape>\n<keep id=\""
        ),
        "{back}"
    );
    // Align two shapes' left edges (shapes.pptx slide 6), by a whole-file rewrite.
    let imp = import(&deck("shapes.pptx"));
    let new =
        imp.text.replace("name=\"Rectangle 8\" box=\"174 72 72 72\"", "name=\"Rectangle 8\" box=\"66 180 72 72\"");
    let r = rewrite_in(&PptxModel, &imp.remainder, &imp.text, &new, CAPS).unwrap_or_else(|e| panic!("{e}"));
    assert!(r.report.refused.is_empty() && r.report.removed.is_empty(), "{:?}", r.report);
    let parts = export(&r.text, &r.remainder);
    let s6 = slides(&parts)[5].clone();
    let x = xml(&parts, &s6);
    let left = |name: &str| {
        let f = xfrm_of(&x, name);
        f[f.find("<a:off x=\"").unwrap() + 11..].split('"').next().unwrap().to_string()
    };
    assert_eq!(left("Rectangle 8"), left("Rectangle 1"));
    assert!(xfrm_of(&x, "Rectangle 8").contains("<a:off x=\"838200\" y=\"2286000\"/>"), "{x}");
}

#[test]
fn a_resized_table_scales_its_grid() {
    let imp = import(&deck("korean-deck.pptx"));
    let (text, rem) = exact(&imp.text, &imp.remainder, "box=\"72 144 576 144\"", "box=\"72 144 288 72\"");
    let parts = export(&text, &rem);
    let x = xml(&parts, &slides(&parts)[3]);
    assert!(x.contains("<a:off x=\"914400\" y=\"1828800\"/><a:ext cx=\"3657600\" cy=\"914400\"/>"), "{x}");
    assert_eq!(x.matches("<a:gridCol w=\"1219200\"/>").count(), 3, "{x}");
    assert_eq!(x.matches("<a:tr h=\"228600\">").count(), 4, "{x}");
}

#[test]
fn lines_move_by_their_ends_and_new_lines_are_drawn() {
    let imp = import(&deck("shapes.pptx"));
    let old = "name=\"Straight Connector 5\" from=\"84 144\" to=\"252 144\"";
    // Reversed: the line now runs right to left and up, both flips.
    let (text, rem) =
        exact(&imp.text, &imp.remainder, old, "name=\"Straight Connector 5\" from=\"252 180\" to=\"84 144\"");
    let (text, rem) = exact(
        &text,
        &rem,
        "<shape id=\"s4\" name=\"TextBox 3\"",
        "<line from=\"72 110\" to=\"252 110\"/>\n<shape id=\"s4\" name=\"TextBox 3\"",
    );
    let (parts, back) = export_new(&text, &rem, &[("s11", "Straight Connector 10")]);
    let x = xml(&parts, &slides(&parts)[0]);
    let c = &x[x.find("name=\"Straight Connector 5\"").unwrap()..];
    assert!(
        c.contains(
            "<a:xfrm flipH=\"1\" flipV=\"1\"><a:off x=\"1066800\" y=\"1828800\"/><a:ext cx=\"2133600\" cy=\"457200\"/>"
        ),
        "{c}"
    );
    assert!(x.contains("name=\"Straight Connector 10\""), "{x}");
    assert!(
        back.contains(
            "layout: Blank\n<line id=\"s11\" name=\"Straight Connector 10\" from=\"72 110\" to=\"252 110\" border=\"1pt solid tx1\"/>"
        ),
        "{back}"
    );
}

#[test]
fn an_object_a_connector_is_attached_to_moves_only_with_the_connector() {
    // Elbow Connector 9 ends on Picture 1 (its endCxn names shape 2).
    let imp = import(&deck("shapes.pptx"));
    let pic = ("name=\"Picture 1\" box=\"402 78 144 132\"", "name=\"Picture 1\" box=\"402 178 144 132\"");
    let (text, rem) = exact(&imp.text, &imp.remainder, pic.0, pic.1);
    match PptxEngine.export(&text, &rem) {
        Err(EngineError::Refused(m)) => assert!(
            m.starts_with("<picture id=\"s2\" name=\"Picture 1\">")
                && m.contains("is moved or resized, and connector <line id=\"s10\" name=\"Elbow Connector 9\"> has its end attached to it")
                && m.contains("move that end of the <line> in the same edit"),
            "{m}"
        ),
        other => panic!("expected a refusal, got {:?}", other.map(|_| ())),
    }
    // Resizing is refused too; moving the connector's end with it is not.
    let (text, rem) = exact(&imp.text, &imp.remainder, pic.0, "name=\"Picture 1\" box=\"402 78 72 132\"");
    assert!(matches!(PptxEngine.export(&text, &rem), Err(EngineError::Refused(_))));
    let (text, rem) = exact(&imp.text, &imp.remainder, pic.0, pic.1);
    let (text, rem) = exact(&text, &rem, "from=\"186 252\" to=\"402 144\"", "from=\"186 252\" to=\"402 244\"");
    export(&text, &rem);
    // An object nothing is attached to moves alone.
    let (text, rem) = exact(&imp.text, &imp.remainder, "box=\"47 211 185 136\"", "box=\"47 311 185 136\"");
    export(&text, &rem);
}

#[test]
fn groups_move_whole_or_by_their_objects() {
    let pkg = deck("shapes.pptx");
    let imp = import(&pkg);
    let s3 = slides(&package::read(&pkg).unwrap())[2].clone();
    let orig = xml(&package::read(&pkg).unwrap(), &s3);
    // The group's box: the whole group moves; its objects stay in its child coordinates.
    let g = "<group id=\"g5\" name=\"Group 4\" box=\"120 108 258 152\">";
    let (text, rem) = exact(&imp.text, &imp.remainder, g, "<group id=\"g5\" name=\"Group 4\" box=\"220 108 258 152\">");
    let c = canonical(&text, &rem);
    assert!(
        c.contains(
            "<shape id=\"s2\" name=\"Rectangle 1\" box=\"220 108 138 60\" fill=accent1 border=\"2pt solid accent1*\"/>"
        ),
        "{c}"
    );
    let parts = export(&text, &rem);
    let x = xml(&parts, &s3);
    let (a, b) = (xfrms(&x), xfrms(&orig));
    // [0] is the slide's shape tree, [1] the group.
    assert_eq!(a[2..], b[2..], "the objects' own boxes do not change");
    assert!(a[1].contains("<a:chOff") && a[1].contains("<a:off x=\"2794000\""), "{}", a[1]);
    // One of its objects: it moves, and the group's box follows.
    let (text, rem) = exact(
        &imp.text,
        &imp.remainder,
        "name=\"Oval 2\" box=\"306 150 72 72\" fill",
        "name=\"Oval 2\" box=\"406 150 72 72\" fill",
    );
    let c = canonical(&text, &rem);
    assert!(c.contains("<group id=\"g5\" name=\"Group 4\" box=\"120 108 358 152\">"), "{c}");
    export(&text, &rem);
    // Both, disagreeing: refused with the reason.
    let both = imp
        .text
        .replace(g, "<group id=\"g5\" name=\"Group 4\" box=\"0 0 100 100\">")
        .replace("name=\"Oval 2\" box=\"306 150 72 72\"", "name=\"Oval 2\" box=\"406 150 72 72\"");
    let r = rewrite_in(&PptxModel, &imp.remainder, &imp.text, &both, CAPS).unwrap_or_else(|e| panic!("{e}"));
    match PptxEngine.export(&r.text, &r.remainder) {
        Err(EngineError::Refused(m)) => assert!(m.contains("both changed and disagree"), "{m}"),
        other => panic!("expected a refusal, got {:?}", other.map(|_| ())),
    }
    // Its objects' text is written (§5.3); which objects it has, and their names, never change here.
    let (text, rem) = exact(
        &imp.text,
        &imp.remainder,
        "name=\"Oval 2\" box=\"306 150 72 72\" fill=accent1 border=\"2pt solid accent1*\"/>",
        "name=\"Oval 2\" box=\"306 150 72 72\" fill=accent1 border=\"2pt solid accent1*\">text</shape>",
    );
    let x = xml(&export(&text, &rem), &s3);
    assert!(x.contains("<a:t>text</a:t>"), "{x}");
    let (text, rem) = exact(&imp.text, &imp.remainder, "name=\"Oval 2\"", "name=\"Oval 3\"");
    match PptxEngine.export(&text, &rem) {
        Err(EngineError::Refused(m)) => {
            assert!(m.contains("never added, deleted, reordered, renamed"), "{m}")
        }
        other => panic!("expected a refusal, got {:?}", other.map(|_| ())),
    }
}

#[test]
fn a_layout_change_keeps_each_slot_where_its_box_says() {
    let pkg = deck("korean-deck.pptx");
    let imp = import(&pkg);
    // The section header's slots stay where the text puts them.
    let (text, rem) = exact(&imp.text, &imp.remainder, "layout: Section Header", "layout: Title and Content");
    let parts = export(&text, &rem);
    let x = xml(&parts, &slides(&parts)[5]);
    assert_eq!(xfrms(&x).len(), 2, "{x}");
    // Bare markers put them where the new layout does.
    let t2 = text
        .replace("::title box=\"57 347 612 107\" font=Calibri size=40pt color=tx1::\n부록", "::title::\n부록")
        .replace("::body box=\"57 229 612 118\" font=Calibri size=20pt color=tx1*::", "::body::");
    let r = rewrite_in(&PptxModel, &rem, &text, &t2, CAPS).unwrap_or_else(|e| panic!("{e}"));
    let parts = export(&r.text, &r.remainder);
    assert!(xfrms(&xml(&parts, &slides(&parts)[5])).is_empty());
    assert!(
        canonical(&t2, &r.remainder).contains("::title box=\"36 22 648 90\" font=Calibri size=44pt color=tx1::\n부록")
    );
}

#[test]
fn what_cannot_change_is_refused_with_the_reason() {
    let imp = import(&deck("korean-deck.pptx"));
    let bad = imp.text.replace("size: 720 x 540 pt", "size: 960 x 540 pt");
    match PptxEngine.export(&bad, &imp.remainder) {
        Err(EngineError::Invalid(d)) => {
            assert!(hanji_format::diag::render(&d).contains("size is the deck's slide size, size: 720 x 540 pt"))
        }
        other => panic!("{:?}", other.map(|_| ())),
    }
    let bad = imp.text.replace("box=\"36 475 288 29\"", "box=\"36 475 -288 29\"");
    assert!(matches!(PptxEngine.export(&bad, &imp.remainder), Err(EngineError::Invalid(_))));
    let bad = imp.text.replace("<shape id=\"s5\" name=\"출처\"", "<shape id=\"s99\" name=\"출처\"");
    assert!(matches!(PptxEngine.export(&bad, &imp.remainder), Err(EngineError::Invalid(_))));
}

#[test]
fn a_changed_number_another_object_shows_takes_its_exact_emu() {
    // Kit file 40 (txt-font-props.pptx, P15): TextBox 1's left edge is
    // 2952093 EMU (232.45 pt, shown 232). TextBox 2 written at x 232 lands
    // exactly on it, not at 232 pt (2946400 EMU); its other numbers keep their own.
    let pkg = deck("txt-font-props.pptx");
    let imp = import(&pkg);
    let s5 = slides(&package::read(&pkg).unwrap())[4].clone();
    let (text, rem) = exact(
        &imp.text,
        &imp.remainder,
        "name=\"TextBox 2\" box=\"206 255 308 29\"",
        "name=\"TextBox 2\" box=\"232 255 308 29\"",
    );
    let parts = export(&text, &rem);
    let x = xml(&parts, &s5);
    let f = xfrm_of(&x, "TextBox 2");
    assert!(f.contains("<a:off x=\"2952093\" y=\"3244334\"/>"), "{f}");
    assert!(f.contains("<a:ext cx=\"3917095\" cy=\"369332\"/>"), "{f}");
    // A new text box as wide as TextBox 1 (255 pt shown, 3239814 EMU) is exactly as wide,
    // and as tall as the others (29 pt shown, 369332 EMU).
    let (text, rem) = exact(
        &text,
        &rem,
        "Shape 2 – MSO_LANGUAGE_ID.POLISH</shape>",
        "Shape 2 – MSO_LANGUAGE_ID.POLISH</shape>\n<shape box=\"232 440 255 29\">new</shape>",
    );
    let (parts, _) = export_new(&text, &rem, &[("s5", "TextBox 4")]);
    let f = xfrm_of(&xml(&parts, &s5), "TextBox 4");
    assert!(f.contains("<a:off x=\"2952093\" y=\"5588000\"/><a:ext cx=\"3239814\" cy=\"369332\"/>"), "{f}");
    // TextBox 1 moved in the same edit: its old x is nowhere, and 232 is 232 pt.
    let (text, rem) = exact(
        &imp.text,
        &imp.remainder,
        "name=\"TextBox 1\" box=\"232 113 255 29\" font=Calibri size=18pt color=tx1>Shape 0",
        "name=\"TextBox 1\" box=\"100 113 255 29\" font=Calibri size=18pt color=tx1>Shape 0",
    );
    let (text, rem) = exact(&text, &rem, "box=\"206 255 308 29\"", "box=\"232 255 308 29\"");
    let f = xfrm_of(&xml(&export(&text, &rem), &s5), "TextBox 2");
    assert!(f.contains("<a:off x=\"2946400\" y=\"3244334\"/>"), "{f}");
}

#[test]
fn a_stored_rotation_left_as_shown_keeps_its_exact_value() {
    // turns-deck.pptx slide 1 stores rotations as Google Slides and older
    // PowerPoint files do: negative, past a full turn, flipH="true".
    let pkg = deck("turns-deck.pptx");
    let before = package::read(&pkg).unwrap();
    let s1 = slides(&before)[0].clone();
    let imp = import(&pkg);
    for shown in [
        "name=\"Minus ninety\" box=\"60 160 200 40\" rot=\"270\" fill=gradient border=\"0.75pt solid accent1*\" font=Calibri size=18pt color=lt1>",
        "name=\"Minus fifteen flipped\" box=\"300 160 160 60\" rot=\"345\" flip=\"h\" fill=gradient border=\"0.75pt solid accent1*\" font=Calibri size=18pt color=lt1>",
        "name=\"Past a turn\" box=\"500 160 160 60\" rot=\"60\" fill=gradient border=\"0.75pt solid accent1*\" font=Calibri size=18pt color=lt1>",
        "name=\"Turned in group\" box=\"80 420 80 80\" rot=\"315\" fill=gradient border=\"0.75pt solid accent1*\"/>",
    ] {
        assert!(imp.text.contains(shown), "{shown}\n{}", imp.text);
    }
    // GetPut: nothing edited, every part as it was (the text is its own canonical form).
    assert_eq!(canonical(&imp.text, &imp.remainder), imp.text);
    let parts = export(&imp.text, &imp.remainder);
    for p in &before {
        assert_eq!(package::get(&parts, &p.name), Some(&p.data[..]), "{} changed", p.name);
    }
    // The same turn written another way (-90 for 270, -300 for 60): still unchanged.
    let (text, rem) = exact(&imp.text, &imp.remainder, "rot=\"270\" fill=", "rot=\"-90\" fill=");
    let (text, rem) = exact(&text, &rem, "rot=\"60\"", "rot=\"-300\"");
    let out = PptxEngine.export(&text, &rem).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(xml(&package::read(&out).unwrap(), &s1), xml(&before, &s1));
    // A box moved: its rotation, left as shown, keeps its stored value and sign.
    let (text, rem) = exact(&imp.text, &imp.remainder, "box=\"60 160 200 40\"", "box=\"72 160 200 40\"");
    let x = xml(&export(&text, &rem), &s1);
    assert!(xfrm_of(&x, "Minus ninety").starts_with("<a:xfrm rot=\"-5400000\"><a:off x=\"914400\""), "{x}");
    assert!(xfrm_of(&x, "Past a turn").starts_with("<a:xfrm rot=\"25200000\">"));
    // A changed rotation is written from 0 up to a full turn; a changed flip
    // goes, an unchanged one stays as the file writes it.
    let (text, rem) = exact(&imp.text, &imp.remainder, "rot=\"270\" fill=", "rot=\"90\" fill=");
    let (text, rem) = exact(&text, &rem, "rot=\"60\"", "rot=\"-30\"");
    let (text, rem) = exact(&text, &rem, "rot=\"345\" flip=\"h\"", "rot=\"345\"");
    let x = xml(&export(&text, &rem), &s1);
    assert!(xfrm_of(&x, "Minus ninety").starts_with("<a:xfrm rot=\"5400000\">"), "{x}");
    assert!(xfrm_of(&x, "Past a turn").starts_with("<a:xfrm rot=\"19800000\">"), "{x}");
    assert!(xfrm_of(&x, "Minus fifteen flipped").starts_with("<a:xfrm rot=\"-900000\">"), "{x}");
    let (text, rem) = exact(&imp.text, &imp.remainder, "rot=\"60\"", "rot=\"-30\"");
    let x = xml(&export(&text, &rem), &s1);
    assert!(xfrm_of(&x, "Minus fifteen flipped").starts_with("<a:xfrm rot=\"-900000\" flipH=\"true\">"), "{x}");
    // A group's object moved: the turned one beside it keeps its stored rotation.
    let (text, rem) = exact(&imp.text, &imp.remainder, "box=\"200 430 120 60\"", "box=\"200 440 120 60\"");
    let x = xml(&export(&text, &rem), &s1);
    assert!(xfrm_of(&x, "Turned in group").starts_with("<a:xfrm rot=\"-2700000\">"), "{x}");
}

#[test]
fn an_alternate_content_object_left_as_shown_is_kept() {
    // turns-deck.pptx slide 2: a picture in mc:AlternateContent at the top
    // of the slide, its box not whole points (as Open XML SDK's 3dtestdash
    // stores a 3D model). Left as shown, the export keeps it; moved, it is
    // refused with the reason.
    let pkg = deck("turns-deck.pptx");
    let before = package::read(&pkg).unwrap();
    let s2 = slides(&before)[1].clone();
    let imp = import(&pkg);
    let line = "kind=\"picture\" summary=\"Smiling face\" box=\"363 151 234 237\"/>";
    assert!(imp.text.contains(line), "{}", imp.text);
    assert_eq!(xml(&export(&imp.text, &imp.remainder), &s2), xml(&before, &s2));
    // An edit elsewhere on the slide leaves it alone too.
    let (text, rem) = exact(&imp.text, &imp.remainder, "One object in two forms", "One object, two forms");
    let x = xml(&export(&text, &rem), &s2);
    assert_eq!(xfrms(&x), xfrms(&xml(&before, &s2)));
    let moved = imp.text.replace("box=\"363 151 234 237\"", "box=\"300 151 234 237\"");
    match PptxEngine.export(&moved, &imp.remainder) {
        Err(e) => assert!(e.to_string().contains("stored in more than one form"), "{e}"),
        Ok(_) => panic!("an alternate-content object was moved"),
    }
}
