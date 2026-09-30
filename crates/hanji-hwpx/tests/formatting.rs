//! Formatting in an hwpx text (DESIGN.md §5.2, F2): the style section and
//! the direct formatting a file has, read from `header.xml` (GetPut), and
//! written back as new shapes cloned from the element's (PutGet), style
//! edits and new styles in `header.xml`, and what the vocabulary or hwpx
//! cannot hold, refused with the reason.

use hanji_core::{edit, rewrite, Capabilities, Engine, EngineError, ImportOptions, Refusal, Remainder};
use hanji_hwpx::{package, xml, HwpxEngine};

const CAPS: Capabilities =
    Capabilities { links: false, fields: false, footnotes: false, math: false, formatting: true };

fn corpus(name: &str) -> Vec<u8> {
    let path = format!("{}/corpus/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn part(pkg: &[u8], name: &str) -> String {
    String::from_utf8(package::get(&package::read(pkg).unwrap(), name).unwrap().to_vec()).unwrap()
}

/// The element `<tag…>…</tag>` (or `<tag…/>`) that holds `text` in `xml`.
fn around(xml: &str, tag: &str, text: &str) -> String {
    let at = xml.find(text).unwrap_or_else(|| panic!("{text:?} not in the part"));
    let open = [format!("<{tag}>"), format!("<{tag} ")]
        .iter()
        .filter_map(|o| xml[..at + text.len()].rfind(o.as_str()).filter(|k| *k <= at))
        .max()
        .unwrap_or_else(|| panic!("no <{tag}> before {text:?}"));
    let close = format!("</{tag}>");
    let end =
        xml[at..].find(&close).map_or_else(|| xml[open..].find("/>").unwrap() + open + 2, |k| k + at + close.len());
    xml[open..end].to_string()
}

/// The element of `header.xml` `<tag id="id" …>…</tag>`.
fn shape(header: &str, tag: &str, id: &str) -> String {
    around(header, tag, &format!("<{tag} id=\"{id}\""))
}

fn attr(el: &str, name: &str) -> String {
    let e = xml::fragment(el);
    e.get(name).unwrap_or_else(|| panic!("no {name} on {el}"))
}

/// Every list of `header.xml` counts its items, and their ids are distinct.
fn consistent(header: &str) {
    let root = xml::parse(header.as_bytes()).unwrap().root;
    root.walk(&mut |e| {
        if let Some(n) = e.get("itemCnt").or_else(|| e.get("fontCnt")) {
            let items: Vec<String> = e.elements().filter_map(|x| x.get("id")).collect();
            assert_eq!(n.parse::<usize>().unwrap(), e.elements().count(), "{}", e.name);
            let mut ids = items.clone();
            ids.sort();
            ids.dedup();
            assert_eq!(ids.len(), items.len(), "{}: {items:?}", e.name);
        }
    });
}

/// Exact edits `(old, new)` in turn, from the imported text.
fn edits(text: &str, rem: &Remainder, list: &[(&str, &str)]) -> hanji_core::Reanchored {
    let (mut t, mut r) = (text.to_string(), rem.clone());
    let mut last = None;
    for (old, new) in list {
        let e = edit(&r, &t, old, new, CAPS).unwrap_or_else(|e| panic!("{old:?} → {new:?}: {e}"));
        assert!(e.report.refused.is_empty() && e.report.removed.is_empty(), "{old:?}: {:?}", e.report);
        (t, r) = (e.text.clone(), e.remainder.clone());
        last = Some(e);
    }
    last.unwrap()
}

const BANNER: &str = "| {fill=#FFF0C3 border-top=\"0.34pt solid #000000\" border-bottom=\"2.83pt solid #7F7F7F\" valign=middle} **3D 프린팅 기술의 미래와 전망** {align=center font=나눔고딕 size=20pt} |";

#[test]
fn a_file_s_styles_and_direct_formatting_read_back_unchanged() {
    let pkg = corpus("footnote-01.hwpx");
    let imp = HwpxEngine.import(&pkg, &ImportOptions::default()).unwrap();
    let t = &imp.text;
    // The default style's line is complete, the others hold what differs.
    assert!(t.contains("---\n<style name=\"바탕글\" align=justify line-spacing=160% font=함초롬바탕 size=10pt color=#000000/>\n<style name=\"개요 2\" indent-left=10pt space-before=5pt font=휴먼명조 size=15pt bold/>\n"), "{t}");
    // A cell's box (its border fill and alignment) and its paragraph's own shapes.
    assert!(t.contains(BANNER), "{t}");
    // A list item names its style and what its paragraph shape sets beyond it.
    assert!(t.contains("  - 산업용 샘플을 찍어내던 것에서 발전해 시계, 신발, 휴대전화 케이스, 자동차 부속품까지 출력 {style=\"개요 3\" line-spacing=170%}\n"), "{t}");
    // GetPut: header.xml is not written, the section is the same XML.
    let out = HwpxEngine.export(t, &imp.remainder).unwrap();
    assert_eq!(part(&out, "Contents/header.xml"), part(&pkg, "Contents/header.xml"));
    let canon = |p: &[u8]| xml::canon_part(part(p, "Contents/section0.xml").as_bytes()).unwrap();
    assert_eq!(canon(&out), canon(&pkg));
}

#[test]
fn formatting_edits_add_shapes_cloned_from_the_element_s() {
    let pkg = corpus("footnote-01.hwpx");
    let imp = HwpxEngine.import(&pkg, &ImportOptions::default()).unwrap();
    let item = "출력 {style=\"개요 3\" line-spacing=170%}\n";
    let r = edits(
        &imp.text,
        &imp.remainder,
        &[
            // The banner's fill, and its text's colour and font (a font the file does not have).
            ("{fill=#FFF0C3 border-top", "{fill=#DDEBF7 border-top"),
            ("{align=center font=나눔고딕 size=20pt} |", "{align=center font=\"맑은 고딕\" size=20pt color=#1F4E79} |"),
            // Two list items' first-line indent.
            (item, "출력 {style=\"개요 3\" first-line=10pt line-spacing=170%}\n"),
            (
                "관련 산업에도 도입 {style=\"개요 3\" line-spacing=170%}\n",
                "관련 산업에도 도입 {style=\"개요 3\" first-line=10pt line-spacing=170%}\n",
            ),
            // A style line: every 개요 4 paragraph turns navy.
            (
                "<style name=\"개요 4\" indent-left=30pt space-before=3pt font=휴먼명조 size=14pt/>",
                "<style name=\"개요 4\" indent-left=30pt space-before=3pt font=휴먼명조 size=14pt color=#1F3864/>",
            ),
            // A new style, then a list item in it.
            (
                "size=14pt color=#1F3864/>\n",
                "size=14pt color=#1F3864/>\n<style name=\"Callout\" fill=#FFF2CC border-left=\"2.25pt solid #C00000\"/>\n",
            ),
            ("  - 전문인력 양성 및 기술 수용성 확산 {style=\"개요 3\"}", "  - 전문인력 양성 및 기술 수용성 확산 {style=\"Callout\"}"),
        ],
    );
    // The text as the store returns it: the new style's border snapped to Hancom's widths.
    let (_, mut blocks) = hanji_core::model_of(&r.text, &r.remainder, CAPS).unwrap();
    let mut rem = r.remainder.clone();
    HwpxEngine::complete(&mut blocks, &mut rem);
    let text = HwpxEngine::text_of(&blocks, &rem, None);
    assert!(text.contains("<style name=\"Callout\" fill=#FFF2CC border-left=\"1.98pt solid #C00000\"/>"), "{text}");
    let out = HwpxEngine.export(&text, &rem).unwrap();
    // PutGet: the export reads back as the returned text.
    let back = HwpxEngine.import(&out, &ImportOptions::default()).unwrap().text;
    if back != text {
        let (a, b): (Vec<&str>, Vec<&str>) = (back.lines().collect(), text.lines().collect());
        for (k, (x, y)) in a.iter().zip(&b).enumerate() {
            assert_eq!(x, y, "PutGet\n{}", a[k.saturating_sub(3)..(k + 3).min(a.len())].join("\n"));
        }
    }
    assert_eq!(back, text);
    let (h0, h1) = (part(&pkg, "Contents/header.xml"), part(&out, "Contents/header.xml"));
    let (s0, s1) = (part(&pkg, "Contents/section0.xml"), part(&out, "Contents/section0.xml"));
    consistent(&h1);
    // A paragraph no edit touched keeps its XML, layout cache and all.
    assert_eq!(around(&s1, "hp:p", "시제품 제작 시간과"), around(&s0, "hp:p", "시제품 제작 시간과"));
    // The banner cell points at a new border fill, a copy of its own with the fill changed.
    let (tc0, tc1) = (around(&s0, "hp:tc", "기술의 미래와 전망"), around(&s1, "hp:tc", "기술의 미래와 전망"));
    let (b0, b1) = (attr(&tc0, "borderFillIDRef"), attr(&tc1, "borderFillIDRef"));
    assert_ne!(b0, b1);
    let (f0, f1) = (shape(&h0, "hh:borderFill", &b0), shape(&h1, "hh:borderFill", &b1));
    assert_eq!(
        f1.replace(&format!("id=\"{b1}\""), &format!("id=\"{b0}\"")),
        f0.replace("faceColor=\"#FFF0C3\"", "faceColor=\"#DDEBF7\"")
    );
    // Its run: a copy of its character shape with colour and font changed; the font is added.
    let run = |s: &str| attr(&around(s, "hp:run", "기술의 미래와 전망"), "charPrIDRef");
    let c1 = shape(&h1, "hh:charPr", &run(&s1));
    assert!(c1.contains("textColor=\"#1F4E79\"") && c1.contains("height=\"2000\""), "{c1}");
    assert!(around(&h1, "hh:fontface", "lang=\"HANGUL\"").contains("face=\"맑은 고딕\""));
    let hangul = around(&h1, "hh:fontface", "lang=\"HANGUL\"");
    assert_eq!(
        attr(&hangul, "fontCnt"),
        (attr(&around(&h0, "hh:fontface", "lang=\"HANGUL\""), "fontCnt").parse::<u32>().unwrap() + 1).to_string()
    );
    // The indent: the HwpUnitChar branch holds 10pt, the default branch twice it.
    let p1 = around(&s1, "hp:p", "자동차 부속품까지 출력");
    let pp = shape(&h1, "hh:paraPr", &attr(&p1, "paraPrIDRef"));
    let case = around(&pp, "hp:case", "<hh:margin>");
    assert!(case.contains("<hc:intent value=\"1000\""), "{pp}");
    assert!(around(&pp, "hp:default", "<hp:default>").contains("<hc:intent value=\"2000\""), "{pp}");
    // Its layout cache is dropped (Hancom lays it out again); the rest of the paragraph stays.
    assert!(!p1.contains("linesegarray"), "{p1}");
    // The style line: 개요 4 points at a navy character shape, and so does each of its paragraphs.
    let style = around(&h1, "hh:style", "name=\"개요 4\"");
    let navy = attr(&style, "charPrIDRef");
    assert!(shape(&h1, "hh:charPr", &navy).contains("textColor=\"#1F3864\""), "{style}");
    let p4 = around(&s1, "hp:p", "3D 프린터로 만들 수 있는 물건은");
    assert!(shape(&h1, "hh:charPr", &attr(&around(&p4, "hp:run", "3D 프린터로"), "charPrIDRef")).contains("#1F3864"));
    // The new style: an hh:style with its own paragraph and character shapes, the item in it.
    let callout = around(&h1, "hh:style", "name=\"Callout\"");
    let bf = attr(
        &around(&shape(&h1, "hh:paraPr", &attr(&callout, "paraPrIDRef")), "hh:border", "borderFillIDRef"),
        "borderFillIDRef",
    );
    let fill = shape(&h1, "hh:borderFill", &bf);
    assert!(
        fill.contains("faceColor=\"#FFF2CC\"")
            && fill.contains("<hh:leftBorder type=\"SOLID\" width=\"0.7 mm\" color=\"#C00000\"/>"),
        "{fill}"
    );
    assert_eq!(attr(&around(&s1, "hp:p", "전문인력 양성"), "styleIDRef"), attr(&callout, "id"));
    // A shape two paragraphs ask for is added once: both items had the same one.
    let p2 = around(&s1, "hp:p", "관련 산업에도 도입");
    assert_eq!(attr(&p2, "paraPrIDRef"), attr(&p1, "paraPrIDRef"));
}

#[test]
fn what_the_vocabulary_or_hwpx_cannot_hold_is_refused() {
    let pkg = corpus("footnote-01.hwpx");
    let imp = HwpxEngine.import(&pkg, &ImportOptions::default()).unwrap();
    let (t, rem) = (&imp.text, &imp.remainder);
    let invalid = |old: &str, new: &str| match rewrite(rem, t, &t.replacen(old, new, 1), CAPS) {
        Err(Refusal::Invalid(d)) => d[0].message.clone(),
        other => panic!("{new}: {:?}", other.map(|r| r.text)),
    };
    let refused = |old: &str, new: &str| {
        let r = rewrite(rem, t, &t.replacen(old, new, 1), CAPS).unwrap_or_else(|e| panic!("{new}: {e}"));
        match HwpxEngine.export(&r.text, &r.remainder) {
            Err(EngineError::Refused(m)) => m,
            other => panic!("{new}: {:?}", other.map(|_| ())),
        }
    };
    // hwpx has no theme colours and no transparent colours.
    let m = refused("{fill=#FFF0C3 border-top", "{fill=accent1 border-top");
    assert!(m.contains("no theme colours"), "{m}");
    let m = refused("{fill=#FFF0C3 border-top", "{fill=#FFF0C3/50% border-top");
    assert!(m.contains("opacity"), "{m}");
    // A gradient is kept as written, never written.
    let m = refused("{fill=#FFF0C3 border-top", "{fill=gradient border-top");
    assert!(m.contains("gradient"), "{m}");
    // A mark is a mark: `bold=no` is not a property of running text.
    let m = invalid("{align=center font=나눔고딕 size=20pt} |", "{align=center font=나눔고딕 size=20pt bold=no} |");
    assert!(m.contains("bold"), "{m}");
    // A new style needs a name no style has; the line of a style the text
    // does not use (본문) was not shown, so it is not changed at once.
    let line = "<style name=\"개요 4\" indent-left=30pt space-before=3pt font=휴먼명조 size=14pt/>\n";
    let m = invalid(line, &format!("{line}<style name=\"본문\" size=9pt/>\n"));
    assert!(m.contains("\"본문\" is already a style of this file"), "{m}");
    // hwpx has no table styles.
    let m = invalid("| {fill=#FFF0C3", "{style=\"Grid\"}\n| {fill=#FFF0C3");
    assert!(m.contains("table style"), "{m}");
}
