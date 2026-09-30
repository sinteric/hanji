//! Effects (§5.3): each slot's, shape's and line's effects shown as a
//! summary, `effects="shadow"`, from its own `a:effectLst`, its style or its
//! layout; left out they are removed, and `effects="shadow"` writes
//! PowerPoint's preset shadow.

use hanji_core::edit::edit_in;
use hanji_core::{Capabilities, Engine, EngineError, ImportOptions, Imported, Part, Remainder};
use hanji_package::package;
use hanji_pptx::{PptxEngine, PptxModel};

const CAPS: Capabilities =
    Capabilities { links: false, fields: false, footnotes: false, math: false, formatting: false };

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
// Its own shadow.
const CARD: &str =
    "name=\"Rounded Rectangle 2\" box=\"64 130 260 320\" kind=\"roundRect\" adj=\"8000\" fill=#FFFFFF effects=\"shadow\"/>";
// Its style's shadow (effectRef 2 into the theme's effect styles).
const DOT: &str = "name=\"Oval 5\" box=\"64 64 36 36\" kind=\"ellipse\" fill=#14B8A6 effects=\"shadow\"/>";

#[test]
fn effects_are_shown_from_where_they_come() {
    let t = import(&deck(PITCH)).text;
    assert!(t.contains(CARD) && t.contains(DOT), "{t}");
    // No effects is nothing: a text box.
    let s = import(&deck("shapes.pptx")).text;
    assert!(s.contains("<shape id=\"s4\" name=\"TextBox 3\" box=\"72 72 180 29\" font=Calibri"), "{s}");
}

#[test]
fn effects_left_out_are_removed_and_a_shadow_is_added() {
    let pkg = deck(PITCH);
    let before = package::read(&pkg).unwrap();
    let imp = import(&pkg);
    // Its own shadow goes, over its style's shadow: an empty list in its place.
    let plain = CARD.replace(" effects=\"shadow\"", "");
    let (parts, back) = edited(&imp, CARD, &plain);
    let (a, b) = (sp_pr(&before, "Rounded Rectangle 2"), sp_pr(&parts, "Rounded Rectangle 2"));
    assert_eq!(b, a[..a.find("<a:effectLst>").unwrap()].to_string() + "<a:effectLst/></p:spPr>");
    let x = slide_with(&parts, "Rounded Rectangle 2");
    assert_eq!(x.replace(&b, ""), slide_with(&before, "Rounded Rectangle 2").replace(&a, ""), "only its effects");
    // Written back, the shadow is its style's again: the empty list goes.
    let (parts, _) = edited(&back, &plain, CARD);
    assert_eq!(sp_pr(&parts, "Rounded Rectangle 2"), a[..a.find("<a:effectLst>").unwrap()].to_string() + "</p:spPr>");
    // Over its style's: an empty list; back, the style's again.
    let (parts, back) = edited(&imp, DOT, &DOT.replace(" effects=\"shadow\"", ""));
    assert!(sp_pr(&parts, "Oval 5").ends_with("<a:effectLst/></p:spPr>"), "{}", sp_pr(&parts, "Oval 5"));
    let (parts, _) = edited(&back, &DOT.replace(" effects=\"shadow\"", ""), DOT);
    assert_eq!(slide_with(&parts, "Oval 5"), slide_with(&before, "Oval 5"));
    // A shape without effects gets the preset shadow.
    let s = import(&deck("shapes.pptx"));
    let tb = "box=\"72 72 180 29\" font=Calibri";
    let (parts, back) = edited(&s, tb, "box=\"72 72 180 29\" effects=\"shadow\" font=Calibri");
    let sp = sp_pr(&parts, "TextBox 3");
    assert!(sp.ends_with(&format!("<a:noFill/>{}</p:spPr>", hanji_pptx::effects::SHADOW)), "{sp}");
    assert!(back.text.contains("box=\"72 72 180 29\" effects=\"shadow\" font=Calibri"));
}

#[test]
fn what_cannot_be_written_is_refused() {
    let imp = import(&deck(PITCH));
    let err = |text: String, rem: &Remainder| match PptxEngine.export(&text, rem) {
        Err(EngineError::Invalid(d)) => hanji_format::diag::render(&d),
        Err(EngineError::Refused(m)) => m,
        other => panic!("expected an error, got {:?}", other.map(|_| ())),
    };
    let m = err(imp.text.replace(CARD, &CARD.replace("effects=\"shadow\"", "effects=\"shadow glow\"")), &imp.remainder);
    assert!(m.contains("Rounded Rectangle 2") && m.contains("cannot be written"), "{m}");
    let m = err(imp.text.replace(CARD, &CARD.replace("effects=\"shadow\"", "effects=\"sparkle\"")), &imp.remainder);
    assert!(m.contains("not an effect"), "{m}");
    let m = err(imp.text.replace(CARD, &CARD.replace("effects=\"shadow\"", "shadow=\"yes\"")), &imp.remainder);
    assert!(m.contains("effects="), "{m}");
}
