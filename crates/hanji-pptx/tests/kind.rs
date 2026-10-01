//! Preset shapes (§5.3): each slot's, shape's and line's preset shown as
//! `kind="…"` and its adjustments as `adj="…"`, from its own `a:prstGeom`
//! or its layout, and written back only where the text changes them.

use hanji_core::edit::edit_in;
use hanji_core::{Capabilities, Engine, EngineError, ImportOptions, Imported, Part, Remainder};
use hanji_package::package;
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

/// A new object's line without the id and name the write gave it.
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

/// An exact edit, exported: the export reads back as the edited text.
fn edited(imp: &Imported, old: &str, new: &str) -> (Vec<Part>, Imported) {
    let r = edit_in(&PptxModel, &imp.remainder, &imp.text, old, new, CAPS).unwrap_or_else(|e| panic!("{old:?}: {e}"));
    assert!(r.report.removed.is_empty() && r.report.refused.is_empty(), "{:?}", r.report);
    let out = PptxEngine.export(&r.text, &r.remainder).unwrap_or_else(|e| panic!("{e}"));
    let back = import(&out);
    assert_eq!(without_new_ids(&back.text), without_new_ids(&canonical(&r.text, &r.remainder)), "PutGet");
    (package::read(&out).unwrap(), back)
}

/// The slide part holding the object named `name`.
fn slide_with(parts: &[Part], name: &str) -> String {
    parts
        .iter()
        .filter(|p| p.name.starts_with("ppt/slides/slide"))
        .map(|p| String::from_utf8(p.data.to_vec()).unwrap())
        .find(|x| x.contains(&format!("name=\"{name}\"")))
        .unwrap_or_else(|| panic!("no {name}"))
}

/// The `p:spPr` of the object named `name`.
fn sp_pr(parts: &[Part], name: &str) -> String {
    let x = slide_with(parts, name);
    let at = x.find(&format!("name=\"{name}\"")).unwrap();
    let s = at + x[at..].find("<p:spPr").unwrap();
    x[s..s + x[s..].find("</p:spPr>").unwrap() + 9].to_string()
}

const PITCH: &str = "audit/synth-modern-pitch.pptx";
const CARD: &str =
    "name=\"Rounded Rectangle 2\" box=\"64 130 260 320\" kind=\"roundRect\" adj=\"8000\" fill=#FFFFFF effects=\"shadow\"/>";

#[test]
fn every_preset_is_shown_and_a_rectangle_is_none() {
    let t = import(&deck("shapes.pptx")).text;
    for want in [
        "name=\"Oval 2\" box=\"306 150 72 72\" kind=\"ellipse\" fill=accent1",
        "name=\"Right Arrow 3\" box=\"144 222 77 38\" kind=\"rightArrow\" fill=accent1",
        // A connector: bent is shown, straight is not.
        "from=\"186 252\" to=\"s2.1\" kind=\"bentConnector3\" border=",
        "from=\"468 366\" to=\"468 216\" border=",
        // A rectangle shows no kind; custom geometry is custom.
        "name=\"Rectangle 8\" box=\"174 72 72 72\" fill=",
        "name=\"Freeform 6\" box=\"47 211 185 136\" kind=\"custom\" fill=",
    ] {
        assert!(t.contains(want), "{want}\n{t}");
    }
    // A preset's adjustments, in the file's units.
    assert!(import(&deck(PITCH)).text.contains(CARD));
    assert!(import(&deck("45545_Comment.pptx")).text.contains("kind=\"chevron\" adj=\"25000\" fill=tx1"));
}

#[test]
fn a_changed_kind_writes_only_the_preset() {
    let pkg = deck(PITCH);
    let before = package::read(&pkg).unwrap();
    let imp = import(&pkg);
    // The adjustment alone.
    let (parts, back) = edited(&imp, CARD, &CARD.replace("adj=\"8000\"", "adj=\"16667\""));
    let (a, b) = (sp_pr(&before, "Rounded Rectangle 2"), sp_pr(&parts, "Rounded Rectangle 2"));
    assert_eq!(b, a.replace("fmla=\"val 8000\"", "fmla=\"val 16667\""));
    let x = slide_with(&parts, "Rounded Rectangle 2");
    assert_eq!(x.replace(&b, ""), slide_with(&before, "Rounded Rectangle 2").replace(&a, ""), "only that preset");
    // Taken back, the slide is as it was.
    let (parts, _) = edited(&back, &CARD.replace("adj=\"8000\"", "adj=\"16667\""), CARD);
    assert_eq!(slide_with(&parts, "Rounded Rectangle 2"), slide_with(&before, "Rounded Rectangle 2"));
    // Another kind takes its own adjustments; a kind left out is a rectangle.
    let (parts, _) = edited(&imp, CARD, &CARD.replace("kind=\"roundRect\" adj=\"8000\"", "kind=\"snip1Rect\""));
    assert!(sp_pr(&parts, "Rounded Rectangle 2").contains("<a:prstGeom prst=\"snip1Rect\"><a:avLst/></a:prstGeom>"));
    let (parts, _) = edited(&imp, CARD, &CARD.replace(" kind=\"roundRect\" adj=\"8000\"", ""));
    assert!(sp_pr(&parts, "Rounded Rectangle 2").contains("<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom>"));
}

#[test]
fn a_connector_s_kind_a_new_shape_s_and_a_group_s_shape_s_are_written() {
    let imp = import(&deck("shapes.pptx"));
    // A bent connector made straight, and a straight one bent with its midpoint moved.
    let (parts, _) = edited(&imp, "to=\"s2.1\" kind=\"bentConnector3\" border=", "to=\"s2.1\" border=");
    assert!(
        sp_pr(&parts, "Elbow Connector 9").contains("<a:prstGeom prst=\"straightConnector1\"><a:avLst/></a:prstGeom>")
    );
    let (parts, _) =
        edited(&imp, "to=\"468 216\" border=", "to=\"468 216\" kind=\"bentConnector3\" adj=\"adj1=25000\" border=");
    let c = sp_pr(&parts, "Straight Arrow Connector 7");
    assert!(c.contains("<a:prstGeom prst=\"bentConnector3\"><a:avLst><a:gd name=\"adj1\" fmla=\"val 25000\"/></a:avLst></a:prstGeom>"), "{c}");
    // A new text box as an ellipse.
    let tb = "<shape id=\"s4\" name=\"TextBox 3\"";
    let (parts, back) = edited(&imp, tb, &format!("<shape box=\"72 400 144 72\" kind=\"ellipse\">새 원</shape>\n{tb}"));
    assert!(back.text.contains("box=\"72 400 144 72\" kind=\"ellipse\" font="), "{}", back.text);
    let x = slide_with(&parts, "TextBox 3");
    assert!(x.contains("<a:prstGeom prst=\"ellipse\"><a:avLst/></a:prstGeom>"), "{x}");
    // A group's shape.
    let oval = "name=\"Oval 2\" box=\"306 150 72 72\" kind=\"ellipse\"";
    let (parts, _) = edited(&imp, oval, &oval.replace("ellipse", "octagon"));
    assert!(sp_pr(&parts, "Oval 2").contains("<a:prstGeom prst=\"octagon\"><a:avLst/></a:prstGeom>"));
}

#[test]
fn what_cannot_be_written_is_refused() {
    let imp = import(&deck("shapes.pptx"));
    let err = |text: String, rem: &Remainder| match PptxEngine.export(&text, rem) {
        Err(EngineError::Invalid(d)) => hanji_format::diag::render(&d),
        Err(EngineError::Refused(m)) => m,
        other => panic!("expected an error, got {:?}", other.map(|_| ())),
    };
    // Custom geometry is not written anew.
    let r8 = "name=\"Rectangle 8\" box=\"174 72 72 72\" fill=";
    let m = err(imp.text.replace(r8, &r8.replace("fill=", "kind=\"custom\" fill=")), &imp.remainder);
    assert!(m.contains("Rectangle 8") && m.contains("cannot be written anew"), "{m}");
    let oval = "box=\"306 150 72 72\" kind=\"ellipse\"";
    let m = err(imp.text.replace(oval, "box=\"306 150 72 72\" kind=\"circle\""), &imp.remainder);
    assert!(m.contains("not a preset shape") && m.contains("ellipse"), "{m}");
    let m = err(imp.text.replace(oval, "box=\"306 150 72 72\" shape=\"ellipse\""), &imp.remainder);
    assert!(m.contains("kind="), "{m}");
    let m = err(imp.text.replace(oval, &format!("{oval} adj=\"25%\"")), &imp.remainder);
    assert!(m.contains("adjustments"), "{m}");
}

#[test]
fn custom_geometry_is_kept_and_a_preset_may_take_its_place() {
    let pkg = deck("shapes.pptx");
    let imp = import(&pkg);
    let ff = "name=\"Freeform 6\" box=\"47 211 185 136\" kind=\"custom\"";
    // Moved, it keeps its path.
    let (parts, _) = edited(&imp, ff, &ff.replace("47 211", "47 311"));
    let (a, b) = (sp_pr(&package::read(&pkg).unwrap(), "Freeform 6"), sp_pr(&parts, "Freeform 6"));
    let path = |x: &str| x[x.find("<a:custGeom>").unwrap()..x.find("</a:custGeom>").unwrap()].to_string();
    assert_eq!(path(&b), path(&a));
    // A preset in its place.
    let (parts, back) = edited(&imp, ff, &ff.replace("custom", "cloud"));
    let sp = sp_pr(&parts, "Freeform 6");
    assert!(!sp.contains("custGeom") && sp.contains("<a:prstGeom prst=\"cloud\"><a:avLst/></a:prstGeom>"), "{sp}");
    assert!(back.text.contains(&ff.replace("custom", "cloud")));
}
