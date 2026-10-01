//! Shape fill (§5.3): each slot's and shape's fill shown on its tag or
//! marker, from its own `p:spPr`, its style or its layout, and written back
//! only where the text changes it.

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
    assert_eq!(without_new_ids(&back.text), without_new_ids(&canonical(&r.text, &r.remainder)), "PutGet");
    (package::read(&out).unwrap(), back)
}

/// A new text box read back without the id and name the write gave it.
fn without_new_ids(t: &str) -> String {
    t.lines()
        .map(|l| match (l.strip_prefix("<shape id=\""), l.find(" name=\"TextBox ")) {
            (Some(_), Some(k)) => {
                let rest = &l[k + 15..];
                format!("<shape{}", &rest[rest.find('"').unwrap() + 1..])
            }
            _ => l.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn slide(parts: &[Part], n: usize) -> String {
    let pres = String::from_utf8(package::get(parts, "ppt/presentation.xml").unwrap().to_vec()).unwrap();
    let list = &pres[pres.find("<p:sldIdLst>").unwrap()..pres.find("</p:sldIdLst>").unwrap()];
    let rid = list.split("r:id=\"").nth(n).unwrap();
    let part = opc::target_of(parts, "ppt/presentation.xml", &rid[..rid.find('"').unwrap()]).unwrap();
    String::from_utf8(package::get(parts, &part).unwrap().to_vec()).unwrap()
}

/// The `p:spPr` of the shape named `name`.
fn sp_pr(x: &str, name: &str) -> String {
    let at = x.find(&format!("name=\"{name}\"")).unwrap_or_else(|| panic!("no {name}"));
    let s = at + x[at..].find("<p:spPr").unwrap();
    x[s..s + x[s..].find("</p:spPr>").unwrap() + 9].to_string()
}

#[test]
fn every_fill_is_shown_from_where_it_comes() {
    let t = import(&deck("shapes.pptx")).text;
    // A style's fill reference, a shape's own theme colours with Office's tints.
    for want in [
        "<shape id=\"s2\" name=\"Rectangle 1\" box=\"66 72 72 72\" fill=accent1 border=\"2pt solid accent1*\"/>",
        "<shape id=\"s9\" name=\"Rectangle 8\" box=\"174 72 72 72\" fill=accent1+80% border=\"2pt solid accent1*\"/>",
        "<shape id=\"s12\" name=\"Rectangle 11\" box=\"468 72 72 72\" fill=accent1-25% border=\"2pt solid accent1*\"/>",
    ] {
        assert!(t.contains(want), "{want}\n{t}");
    }
    // No fill is nothing: a text box.
    assert!(t.contains("<shape id=\"s4\" name=\"TextBox 3\" box=\"72 72 180 29\" font=Calibri"), "{t}");
    // A gradient is shown, and kept.
    assert!(import(&deck("turns-deck.pptx")).text.contains("rot=\"270\" fill=gradient"));
}

#[test]
fn a_changed_fill_writes_only_the_fill() {
    let pkg = deck("shapes.pptx");
    let before = package::read(&pkg).unwrap();
    let imp = import(&pkg);
    let r8 = "name=\"Rectangle 8\" box=\"174 72 72 72\" fill=accent1+80% border=\"2pt solid accent1*\"/>";
    let (parts, _) =
        edited(&imp, r8, "name=\"Rectangle 8\" box=\"174 72 72 72\" fill=#FF7F50 border=\"2pt solid accent1*\"/>");
    let (a, b) = (slide(&before, 6), slide(&parts, 6));
    assert_eq!(
        sp_pr(&b, "Rectangle 8"),
        sp_pr(&a, "Rectangle 8").replace(
            "<a:solidFill><a:schemeClr val=\"accent1\"><a:lumMod val=\"20000\"/><a:lumOff val=\"80000\"/></a:schemeClr></a:solidFill>",
            "<a:solidFill><a:srgbClr val=\"FF7F50\"/></a:solidFill>"
        )
    );
    assert_eq!(b.replace(&sp_pr(&b, "Rectangle 8"), ""), a.replace(&sp_pr(&a, "Rectangle 8"), ""), "only that fill");
    // Taken back to what its style gives: its own fill goes.
    let (parts, _) =
        edited(&imp, r8, "name=\"Rectangle 8\" box=\"174 72 72 72\" fill=accent1 border=\"2pt solid accent1*\"/>");
    assert!(!sp_pr(&slide(&parts, 6), "Rectangle 8").contains("Fill"), "{}", sp_pr(&slide(&parts, 6), "Rectangle 8"));
}

#[test]
fn a_fill_left_out_is_none_and_written_back_it_is_the_style_s() {
    let pkg = deck("shapes.pptx");
    let before = package::read(&pkg).unwrap();
    let imp = import(&pkg);
    let r1 = "<shape id=\"s2\" name=\"Rectangle 1\" box=\"66 72 72 72\" fill=accent1 border=\"2pt solid accent1*\"/>";
    let bare = "<shape id=\"s2\" name=\"Rectangle 1\" box=\"66 72 72 72\" border=\"2pt solid accent1*\"/>";
    let (parts, imp2) = edited(&imp, r1, bare);
    assert!(sp_pr(&slide(&parts, 6), "Rectangle 1").contains("</a:prstGeom><a:noFill/></p:spPr>"));
    let (parts, _) = edited(&imp2, bare, r1);
    assert_eq!(slide(&parts, 6), slide(&before, 6));
}

#[test]
fn a_new_text_box_and_a_slot_take_a_fill() {
    let imp = import(&deck("korean-deck.pptx"));
    let title = "::title box=\"36 22 648 90\" font=\"맑은 고딕\" size=44pt color=tx1::\n핵심 지표\n";
    let (_, imp2) = edited(&imp, title, &title.replace("90\" font", "90\" fill=accent2-25% font"));
    let (parts, back) =
        edited(&imp2, "핵심 지표\n", "핵심 지표\n<shape box=\"36 112 648 28\" fill=bg2>새 상자</shape>\n");
    let x = slide(&parts, 2);
    assert!(
        x.contains("<a:solidFill><a:schemeClr val=\"accent2\"><a:lumMod val=\"75000\"/></a:schemeClr></a:solidFill>"),
        "{x}"
    );
    assert!(
        x.contains(
            "<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom><a:solidFill><a:schemeClr val=\"bg2\"/></a:solidFill>"
        ),
        "{x}"
    );
    assert!(back.text.contains(" fill=bg2 font=\"맑은 고딕\" size=18pt color=tx1>새 상자</shape>"), "{}", back.text);
}

#[test]
fn what_cannot_be_written_is_refused() {
    let imp = import(&deck("shapes.pptx"));
    let err = |text: String, rem: &Remainder| match PptxEngine.export(&text, rem) {
        Err(EngineError::Invalid(d)) => hanji_format::diag::render(&d),
        Err(EngineError::Refused(m)) => m,
        other => panic!("expected an error, got {:?}", other.map(|_| ())),
    };
    let r1 = "name=\"Rectangle 1\" box=\"66 72 72 72\" fill=accent1 border=\"2pt solid accent1*\"/>";
    let m = err(
        imp.text.replace(r1, "name=\"Rectangle 1\" box=\"66 72 72 72\" fill=gradient border=\"2pt solid accent1*\"/>"),
        &imp.remainder,
    );
    assert!(m.contains("fill=gradient") && m.contains("cannot be written"), "{m}");
    let m = err(
        imp.text.replace(r1, "name=\"Rectangle 1\" box=\"66 72 72 72\" fill=blue border=\"2pt solid accent1*\"/>"),
        &imp.remainder,
    );
    assert!(m.contains("not a fill"), "{m}");
    let m = err(
        imp.text
            .replace(r1, "name=\"Rectangle 1\" box=\"66 72 72 72\" background=accent2 border=\"2pt solid accent1*\"/>"),
        &imp.remainder,
    );
    assert!(m.contains("fill="), "{m}");
    // A gradient left as shown is kept.
    let t = import(&deck("turns-deck.pptx"));
    let out = PptxEngine.export(&t.text, &t.remainder).unwrap();
    assert_eq!(slide(&package::read(&out).unwrap(), 1), slide(&package::read(&deck("turns-deck.pptx")).unwrap(), 1));
}

const BAND: &str = "name=\"Rectangle 1\" box=\"0 0 960 180\" fill=\"linear 0 #14B8A6 #0EA5E9\"";

/// The `a:gradFill` of the shape named `name` on slide `n`.
fn grad_of(parts: &[Part], n: usize, name: &str) -> String {
    let sp = sp_pr(&slide(parts, n), name);
    sp[sp.find("<a:gradFill").unwrap()..sp.find("</a:gradFill>").unwrap() + 13].to_string()
}

#[test]
fn a_two_stop_linear_gradient_is_shown_and_written_by_its_parts() {
    let pkg = deck("audit/synth-modern-pitch.pptx");
    let before = package::read(&pkg).unwrap();
    let imp = import(&pkg);
    assert!(imp.text.contains(BAND), "{}", imp.text);
    let a = grad_of(&before, 4, "Rectangle 1");
    // Its angle alone: top to bottom.
    let (parts, back) = edited(&imp, BAND, &BAND.replace("linear 0", "linear 90"));
    assert_eq!(grad_of(&parts, 4, "Rectangle 1"), a.replace("<a:lin ang=\"0\"", "<a:lin ang=\"5400000\""));
    // Its end colour alone, a theme colour; the rest (rotWithShape) kept.
    let (parts, _) = edited(
        &back,
        &BAND.replace("linear 0", "linear 90"),
        &BAND.replace("linear 0 #14B8A6 #0EA5E9", "linear 90 #14B8A6 accent2"),
    );
    let g = grad_of(&parts, 4, "Rectangle 1");
    assert!(
        g.starts_with("<a:gradFill rotWithShape=\"1\">")
            && g.contains("<a:gs pos=\"100000\"><a:schemeClr val=\"accent2\"/></a:gs>"),
        "{g}"
    );
    // A solid shape made a gradient, and a gradient made solid.
    let s = import(&deck("shapes.pptx"));
    let r8 = "name=\"Rectangle 8\" box=\"174 72 72 72\" fill=accent1+80%";
    let (parts, _) = edited(&s, r8, &r8.replace("fill=accent1+80%", "fill=\"linear 45 accent1 #FFFFFF\""));
    assert_eq!(
        grad_of(&parts, 6, "Rectangle 8"),
        "<a:gradFill><a:gsLst><a:gs pos=\"0\"><a:schemeClr val=\"accent1\"/></a:gs><a:gs pos=\"100000\"><a:srgbClr val=\"FFFFFF\"/></a:gs></a:gsLst><a:lin ang=\"2700000\" scaled=\"0\"/></a:gradFill>"
    );
    let (parts, _) = edited(&imp, BAND, &BAND.replace("\"linear 0 #14B8A6 #0EA5E9\"", "#14B8A6"));
    assert!(sp_pr(&slide(&parts, 4), "Rectangle 1").contains("<a:solidFill><a:srgbClr val=\"14B8A6\"/></a:solidFill>"));
}

#[test]
fn a_gradient_the_text_cannot_write_is_refused() {
    let imp = import(&deck("audit/synth-modern-pitch.pptx"));
    let err = |text: String| match PptxEngine.export(&text, &imp.remainder) {
        Err(EngineError::Invalid(d)) => hanji_format::diag::render(&d),
        Err(EngineError::Refused(m)) => m,
        other => panic!("expected an error, got {:?}", other.map(|_| ())),
    };
    let m = err(imp.text.replace(BAND, &BAND.replace("#0EA5E9", "accent1*")));
    assert!(m.contains("cannot be written"), "{m}");
    let m = err(imp.text.replace(BAND, &BAND.replace("linear 0 #14B8A6 #0EA5E9", "linear up #14B8A6 #0EA5E9")));
    assert!(m.contains("not an angle") && m.contains("linear <angle> <from> <to>"), "{m}");
    let m = err(imp.text.replace(BAND, &BAND.replace("linear 0 #14B8A6 #0EA5E9", "linear 0 #14B8A6")));
    assert!(m.contains("linear <angle> <from> <to>"), "{m}");
}
