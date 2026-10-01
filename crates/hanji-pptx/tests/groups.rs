//! The text and fill of a group's shapes (§5.3): shown as a slide shape's
//! are, and written back in place, the rest of the group as it was.

use hanji_core::edit::edit_in;
use hanji_core::{Capabilities, Engine, EngineError, ImportOptions, Imported, Part, Remainder};
use hanji_package::{opc, package};
use hanji_pptx::{PptxEngine, PptxModel};

const CAPS: Capabilities =
    Capabilities { links: false, fields: false, footnotes: false, math: false, formatting: false, table_place: false };

fn deck(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/corpus/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn import(pkg: &[u8]) -> Imported {
    PptxEngine.import(pkg, &ImportOptions::default()).unwrap_or_else(|e| panic!("{e}"))
}

fn canonical(text: &str, rem: &Remainder) -> String {
    use hanji_core::TextModel;
    let (blocks, _) = PptxModel.resolve(text, rem, CAPS).unwrap_or_else(|e| panic!("{e:?}"));
    PptxEngine::text_of(&blocks, rem, None)
}

/// An exact edit, exported: the export reads back as the edited text.
fn edited(imp: &Imported, old: &str, new: &str) -> (Vec<Part>, Imported) {
    let r = edit_in(&PptxModel, &imp.remainder, &imp.text, old, new, CAPS).unwrap_or_else(|e| panic!("{old:?}: {e}"));
    assert!(r.report.removed.is_empty() && r.report.refused.is_empty(), "{:?}", r.report);
    let out = PptxEngine.export(&r.text, &r.remainder).unwrap_or_else(|e| panic!("{e}"));
    let back = import(&out);
    assert_eq!(back.text, canonical(&r.text, &r.remainder), "PutGet");
    (package::read(&out).unwrap(), back)
}

fn slide(parts: &[Part], n: usize) -> String {
    let pres = String::from_utf8(package::get(parts, "ppt/presentation.xml").unwrap().to_vec()).unwrap();
    let list = &pres[pres.find("<p:sldIdLst>").unwrap()..pres.find("</p:sldIdLst>").unwrap()];
    let rid = list.split("r:id=\"").nth(n).unwrap();
    let part = opc::target_of(parts, "ppt/presentation.xml", &rid[..rid.find('"').unwrap()]).unwrap();
    String::from_utf8(package::get(parts, &part).unwrap().to_vec()).unwrap()
}

/// The `p:sp` named `name`.
fn sp<'a>(x: &'a str, name: &str) -> &'a str {
    let at = x.find(&format!("name=\"{name}\"")).unwrap_or_else(|| panic!("no {name}"));
    let s = x[..at].rfind("<p:sp>").unwrap();
    &x[s..at + x[at..].find("</p:sp>").unwrap() + 7]
}

const PITCH: &str = "audit/synth-modern-pitch.pptx";
const BODY: &str = "<shape id=\"s6\" name=\"TextBox 5\" box=\"88 260 212 120\" font=Pretendard size=16pt color=#555555>Low-emission zones in 40 cities</shape>";

#[test]
fn a_group_s_shapes_show_their_text_formatting_and_fill() {
    let t = import(&deck(PITCH)).text;
    assert!(t.contains(BODY), "{t}");
    let groups = import(&deck("audit/slide-section-test.pptx")).text;
    assert!(groups.contains("fill=#7F59AE font=\"DejaVu Sans\" size=18pt color=#FFFFFF>**1**</shape>"), "{groups}");
}

#[test]
fn a_group_s_text_is_written_in_place() {
    let pkg = deck(PITCH);
    let before = package::read(&pkg).unwrap();
    let imp = import(&pkg);
    let a = slide(&before, 3);
    // A figure changed: the run keeps its formatting.
    let (parts, _) = edited(&imp, "zones in 40 cities", "zones in 45 cities");
    let x = slide(&parts, 3);
    assert_eq!(sp(&x, "TextBox 5"), sp(&a, "TextBox 5").replace("in 40 cities", "in 45 cities"));
    assert_eq!(
        x.replace(sp(&x, "TextBox 5"), ""),
        a.replace(sp(&a, "TextBox 5"), ""),
        "the rest of the slide as it was"
    );
    // A word restyled, a paragraph added, the shape filled.
    let (parts, back) = edited(
        &imp,
        BODY,
        "<shape id=\"s6\" name=\"TextBox 5\" box=\"88 260 212 120\" fill=accent2 font=Pretendard size=16pt color=#555555>Low-emission zones in [40]{color=#FF7F50} cities<p/>**Since 2024**</shape>",
    );
    let s = sp(&slide(&parts, 3), "TextBox 5").to_string();
    assert!(
        !s.contains("<a:noFill/>") && s.contains("<a:solidFill><a:schemeClr val=\"accent2\"/></a:solidFill>"),
        "{s}"
    );
    assert!(
        s.contains("<a:srgbClr val=\"FF7F50\"/></a:solidFill><a:latin typeface=\"Pretendard\"/></a:rPr><a:t>40 </a:t>"),
        "{s}"
    );
    assert!(s.contains("b=\"1\"") && s.contains("<a:t>Since 2024</a:t>"), "{s}");
    let shown = "fill=accent2 font=Pretendard size=16pt>[Low-emission zones in]{color=#555555} [40]{color=#FF7F50} [cities]{color=#555555}<p/>**Since 2024** {color=#555555}</shape>";
    assert!(back.text.contains(shown), "{}", back.text);
}

#[test]
fn what_a_group_s_shapes_cannot_take_is_refused() {
    let imp = import(&deck(PITCH));
    let err = |text: String| match PptxEngine.export(&text, &imp.remainder) {
        Err(EngineError::Invalid(d)) => hanji_format::diag::render(&d),
        Err(EngineError::Refused(m)) => m,
        other => panic!("expected an error, got {:?}", other.map(|_| ())),
    };
    let m = err(imp.text.replace("name=\"TextBox 5\"", "name=\"Body\""));
    assert!(m.contains("keep its id and name"), "{m}");
    let m = err(imp.text.replace(BODY, &BODY.replace("font=Pretendard", "fill=gradient font=Pretendard")));
    assert!(m.contains("fill=gradient") && m.contains("cannot be written"), "{m}");
}

const TURNED: &str = "<group id=\"g300278322\" name=\"\" box=\"648 122 249 151\" rot=\"347\">";
const INNER: &str = "<shape id=\"s1461313824\" name=\"\" box=\"682 215 49 32\" fill=bg1";

/// The `a:xfrm` of the group with shape id `id`.
fn group_xfrm(parts: &[Part], id: &str) -> String {
    let x = parts
        .iter()
        .filter(|p| p.name.starts_with("ppt/slides/slide"))
        .map(|p| String::from_utf8(p.data.to_vec()).unwrap())
        .find(|x| x.contains(&format!("<p:cNvPr id=\"{id}\"")))
        .unwrap();
    let at = x.find(&format!("<p:cNvPr id=\"{id}\"")).unwrap();
    let s = at + x[at..].find("<a:xfrm").unwrap();
    // Empty elements written as the export writes them.
    let mut out = x[s..s + x[s..].find("</a:xfrm>").unwrap() + 9].to_string();
    for n in ["a:off", "a:ext", "a:chOff", "a:chExt"] {
        out = out.replace(&format!("></{n}>"), "/>");
    }
    out
}

#[test]
fn a_turned_group_shows_its_turn_and_moves_or_turns_whole() {
    let imp = import(&deck("audit/onlyoffice-sample.pptx"));
    // Its objects as they are in it before its turn.
    assert!(
        imp.text.contains(&format!("{TURNED}\n<group id=\"g1156312507\" name=\"\" box=\"648 122 249 151\">")),
        "{}",
        imp.text
    );
    let before = group_xfrm(&package::read(&deck("audit/onlyoffice-sample.pptx")).unwrap(), "300278322");
    // Moved: its offset changes, its turn stays, its objects follow.
    let (parts, back) = edited(&imp, TURNED, &TURNED.replace("648 122", "600 122"));
    let after = group_xfrm(&parts, "300278322");
    assert!(after.starts_with("<a:xfrm rot=\"") && after != before, "{after}");
    assert_eq!(
        after.replace(&after[after.find("<a:off").unwrap()..after.find("<a:ext").unwrap()], ""),
        before.replace(&before[before.find("<a:off").unwrap()..before.find("<a:ext").unwrap()], "")
    );
    assert!(back.text.contains("<group id=\"g300278322\" name=\"\" box=\"600 122 249 151\" rot=\"347\">"));
    // Turned: only its rotation changes.
    let (parts, _) = edited(&imp, TURNED, &TURNED.replace("rot=\"347\"", "rot=\"10\""));
    let after = group_xfrm(&parts, "300278322");
    assert!(after.starts_with("<a:xfrm rot=\"600000\" flipH=\"0\" flipV=\"0\">"), "{after}");
    assert_eq!(after[after.find('>').unwrap()..], before[before.find('>').unwrap()..]);
}

#[test]
fn a_turned_group_s_objects_are_edited_in_place_and_not_moved_out() {
    let imp = import(&deck("audit/onlyoffice-sample.pptx"));
    // Their formatting, in place.
    let (parts, _) = edited(&imp, INNER, &INNER.replace("fill=bg1", "fill=accent2"));
    assert!(
        group_xfrm(&parts, "300278322")
            == group_xfrm(&package::read(&deck("audit/onlyoffice-sample.pptx")).unwrap(), "300278322")
    );
    // Moved beyond the box the objects fill: refused, the reason named.
    let r = edit_in(&PptxModel, &imp.remainder, &imp.text, INNER, &INNER.replace("682 215", "682 415"), CAPS).unwrap();
    match PptxEngine.export(&r.text, &r.remainder) {
        Err(EngineError::Refused(m)) => assert!(m.contains("turned or flipped") && m.contains("whole group"), "{m}"),
        other => panic!("expected a refusal, got {:?}", other.map(|_| ())),
    }
}
