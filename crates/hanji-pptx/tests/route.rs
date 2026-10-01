//! Connectors (§5.3): an end attached to an object shows as that object's
//! connection site (`to="s2.1"`), and is put on the site again when the
//! object moves or is resized, or the text attaches it elsewhere.

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

/// A new line's tag without the id and name the write gave it.
fn without_new_ids(t: &str) -> String {
    t.lines()
        .map(|l| match (l.strip_prefix("<line id=\""), l.find(" name=\"Straight Connector ")) {
            (Some(_), Some(k)) => {
                let rest = &l[k + 26..];
                format!("<line{}", &rest[rest.find('"').unwrap() + 1..])
            }
            _ => l.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

/// Exact edits one after another, exported: the export reads back as the edited text.
fn edited(imp: &Imported, edits: &[(&str, &str)]) -> (Vec<Part>, Imported) {
    let (mut text, mut rem) = (imp.text.clone(), imp.remainder.clone());
    for (old, new) in edits {
        let r = edit_in(&PptxModel, &rem, &text, old, new, CAPS).unwrap_or_else(|e| panic!("{old:?}: {e}"));
        assert!(r.report.removed.is_empty() && r.report.refused.is_empty(), "{:?}", r.report);
        (text, rem) = (r.text, r.remainder);
    }
    let out = PptxEngine.export(&text, &rem).unwrap_or_else(|e| panic!("{e}"));
    let back = import(&out);
    assert_eq!(without_new_ids(&back.text), without_new_ids(&canonical(&text, &rem)), "PutGet");
    (package::read(&out).unwrap(), back)
}

fn refused(imp: &Imported, edits: &[(&str, &str)]) -> String {
    let (mut text, mut rem) = (imp.text.clone(), imp.remainder.clone());
    for (old, new) in edits {
        let r = edit_in(&PptxModel, &rem, &text, old, new, CAPS).unwrap_or_else(|e| panic!("{old:?}: {e}"));
        (text, rem) = (r.text, r.remainder);
    }
    match PptxEngine.export(&text, &rem) {
        Err(EngineError::Refused(m)) => m,
        Err(EngineError::Invalid(d)) => hanji_format::diag::render(&d),
        other => panic!("expected a refusal, got {:?}", other.map(|_| ())),
    }
}

fn slide1(parts: &[Part]) -> String {
    String::from_utf8(package::get(parts, "ppt/slides/slide1.xml").unwrap().to_vec()).unwrap()
}

/// The element of the object named `name`, from its `p:cNvPr` to its end.
fn object(x: &str, name: &str) -> String {
    let at = x.find(&format!("name=\"{name}\"")).unwrap_or_else(|| panic!("no {name}"));
    let end = at + x[at..].find("</p:cxnSp>").unwrap();
    x[at..end].to_string()
}

// Elbow Connector 9 ends on Picture 1's left side (its endCxn: shape 2, site 1).
const ELBOW: &str = "<line id=\"s10\" name=\"Elbow Connector 9\" from=\"186 252\" to=\"s2.1\" kind=\"bentConnector3\"";
const PIC: &str = "name=\"Picture 1\" box=\"402 78 144 132\"";

#[test]
fn an_attached_end_shows_its_site() {
    let t = import(&deck("shapes.pptx")).text;
    assert!(t.contains(ELBOW), "{t}");
    // An end attached to nothing shows its point.
    assert!(t.contains("name=\"Straight Arrow Connector 7\" from=\"468 366\" to=\"468 216\""), "{t}");
}

#[test]
fn a_moved_object_takes_its_connectors_with_it() {
    let imp = import(&deck("shapes.pptx"));
    let (parts, back) = edited(&imp, &[(PIC, "name=\"Picture 1\" box=\"402 178 144 132\"")]);
    let c = object(&slide1(&parts), "Elbow Connector 9");
    // From (186, 252) to the picture's left side, now at (402, 244): the connector stays flipped up.
    assert!(
        c.contains(
            "<a:xfrm flipV=\"1\"><a:off x=\"2362200\" y=\"3098800\"/><a:ext cx=\"2743200\" cy=\"101600\"/></a:xfrm>"
        ),
        "{c}"
    );
    assert!(c.contains("<a:endCxn id=\"2\" idx=\"1\"/>"), "{c}");
    assert!(back.text.contains(ELBOW), "{}", back.text);
    // Resized, too.
    let (parts, _) = edited(&imp, &[(PIC, "name=\"Picture 1\" box=\"402 78 144 200\"")]);
    let c = object(&slide1(&parts), "Elbow Connector 9");
    assert!(c.contains("<a:off x=\"2362200\" y=\"2260600\"/><a:ext cx=\"2743200\" cy=\"939800\"/>"), "{c}");
}

#[test]
fn a_connector_is_attached_elsewhere_detached_or_drawn_attached() {
    let imp = import(&deck("shapes.pptx"));
    // To the picture's right side.
    let (parts, back) = edited(&imp, &[(ELBOW, &ELBOW.replace("s2.1", "s2.3"))]);
    let c = object(&slide1(&parts), "Elbow Connector 9");
    assert!(c.contains("<a:endCxn id=\"2\" idx=\"3\"/>"), "{c}");
    assert!(c.contains("<a:off x=\"2362200\" y=\"1828800\"/><a:ext cx=\"4572000\" cy=\"1371600\"/>"), "{c}");
    assert!(back.text.contains(&ELBOW.replace("s2.1", "s2.3")));
    // Written as a point: it comes loose.
    let (parts, back) = edited(&imp, &[(ELBOW, &ELBOW.replace("s2.1", "300 300"))]);
    assert!(!object(&slide1(&parts), "Elbow Connector 9").contains("endCxn"));
    assert!(back.text.contains("from=\"186 252\" to=\"300 300\" kind=\"bentConnector3\""), "{}", back.text);
    // A new line from the text box's bottom to the picture's left side.
    let tb = "<shape id=\"s4\" name=\"TextBox 3\"";
    let (parts, back) = edited(&imp, &[(tb, &format!("<line from=\"s4.2\" to=\"s2.1\"/>\n{tb}"))]);
    let c = object(&slide1(&parts), "Straight Connector 10");
    assert!(c.contains("<a:stCxn id=\"4\" idx=\"2\"/><a:endCxn id=\"2\" idx=\"1\"/>"), "{c}");
    assert!(back.text.contains("from=\"s4.2\" to=\"s2.1\" border=\"1pt solid tx1\"/>"), "{}", back.text);
}

#[test]
fn a_turned_line_shows_its_ends_where_they_are() {
    // Line turned: a vertical line turned a quarter back, so it runs right to left.
    let t = import(&deck("turns-deck.pptx")).text;
    assert!(t.contains("name=\"Line turned\" from=\"424 330\" to=\"284 330\""), "{t}");
}

#[test]
fn what_cannot_be_rerouted_is_refused() {
    let imp = import(&deck("shapes.pptx"));
    let moved = (PIC, "name=\"Picture 1\" box=\"402 178 144 132\"");
    // A curved connector.
    let m = refused(&imp, &[(ELBOW, &ELBOW.replace("bentConnector3", "curvedConnector3")), moved]);
    assert!(m.contains("Elbow Connector 9") && m.contains("curvedConnector3") && m.contains("rerouted"), "{m}");
    // Custom geometry, an object that is not there.
    let m = refused(&imp, &[(ELBOW, &ELBOW.replace("s2.1", "s7.0"))]);
    assert!(m.contains("s7.0") && m.contains("custom geometry"), "{m}");
    let m = refused(&imp, &[(ELBOW, &ELBOW.replace("s2.1", "s99.0"))]);
    assert!(m.contains("no object s99"), "{m}");
    let m = refused(&imp, &[(ELBOW, &ELBOW.replace("s2.1", "s2.9"))]);
    assert!(m.contains("no connection site 9"), "{m}");
}
