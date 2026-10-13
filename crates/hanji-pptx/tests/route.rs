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

/// A small XML fixture exercises both endpoint directions without a corpus file.
fn synthetic_connector(end: &str) -> hanji_package::xml::Element {
    hanji_package::xml::fragment(&format!(
        "<p:cxnSp><p:nvCxnSpPr><p:cNvPr id=\"9\" name=\"Attached\"/><p:cNvCxnSpPr><a:{end}Cxn id=\"2\" idx=\"1\"/></p:cNvCxnSpPr></p:nvCxnSpPr><p:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"100\" cy=\"100\"/></a:xfrm><a:prstGeom prst=\"line\"/></p:spPr></p:cxnSp>"
    ))
}

#[test]
fn unchanged_attachments_require_an_existing_target_before_preserving_geometry() {
    use hanji_format::Attach;
    use hanji_package::xml::fragment;
    use hanji_pptx::{geom, route};

    for end in ["st", "end"] {
        let mut connector = synthetic_connector(end);
        let before = connector.clone();
        let mut want = geom::ends_of(&geom::own(&connector).unwrap());
        let attach = Some(Attach { group: false, id: 2, site: 1 });
        if end == "st" {
            want.from_at = attach;
        } else {
            want.to_at = attach;
        }
        let message = route::write(&mut connector, &want, &[], &[], &[], false).unwrap_err();
        assert!(message.contains("s2.1") && message.contains("no object s2"), "{message}");
        assert_eq!(connector, before, "missing-target refusal leaves the connector untouched");

        // Existing targets with unevaluated geometry retain their exact stored end.
        // Checking existence must not force a new site calculation on this path.
        let target = fragment(
            "<p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Custom\"/></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"100\" y=\"100\"/><a:ext cx=\"100\" cy=\"100\"/></a:xfrm><a:custGeom/></p:spPr></p:sp>",
        );
        assert!(!route::write(&mut connector, &want, &[&target], &[], &[2], false).unwrap());
        assert_eq!(connector, before);
        let message = route::write(&mut connector, &want, &[&target], &[2], &[2], false).unwrap_err();
        assert!(message.contains("custom geometry"), "{message}");
    }
}

#[test]
fn deleting_a_target_requires_removing_or_detaching_its_connector() {
    let imp = import(&deck("shapes.pptx"));
    let picture = imp.text.lines().find(|l| l.contains(PIC)).unwrap();
    let connector = imp.text.lines().find(|l| l.starts_with(ELBOW)).unwrap();
    let snapshot = imp.remainder.clone();
    let r = edit_in(&PptxModel, &imp.remainder, &imp.text, picture, "", CAPS).unwrap();
    let edited_snapshot = r.remainder.clone();
    let message = match PptxEngine.export(&r.text, &r.remainder) {
        Err(EngineError::Refused(message)) => message,
        other => panic!("expected missing-target refusal, got {:?}", other.map(|_| ())),
    };
    assert!(message.contains("Elbow Connector 9") && message.contains("no object s2"), "{message}");
    assert_eq!(r.remainder, edited_snapshot);
    assert_eq!(imp.remainder, snapshot);
    // No output package was returned, and the original import remains exportable.
    PptxEngine.export(&imp.text, &imp.remainder).unwrap();

    let r = edit_in(&PptxModel, &r.remainder, &r.text, connector, "", CAPS).unwrap();
    let out = PptxEngine.export(&r.text, &r.remainder).unwrap();
    assert_eq!(import(&out).text, canonical(&r.text, &r.remainder), "PutGet");
    let parts = package::read(&out).unwrap();
    let slide = slide1(&parts);
    assert!(!slide.contains("name=\"Picture 1\"") && !slide.contains("name=\"Elbow Connector 9\""));

    // Detach while the target exists, then export/reimport before deleting it.
    // The same-edit variant is covered below.
    let (_, detached) = edited(&imp, &[(ELBOW, &ELBOW.replace("s2.1", "300 300"))]);
    let picture = detached.text.lines().find(|l| l.contains(PIC)).unwrap();
    let r = edit_in(&PptxModel, &detached.remainder, &detached.text, picture, "", CAPS).unwrap();
    let out = PptxEngine.export(&r.text, &r.remainder).unwrap();
    assert_eq!(import(&out).text, canonical(&r.text, &r.remainder), "PutGet");
    let parts = package::read(&out).unwrap();
    assert!(!object(&slide1(&parts), "Elbow Connector 9").contains("endCxn"));
}

#[test]
fn a_connector_can_be_detached_while_deleting_its_target() {
    use hanji_format::shown_pt;
    use hanji_package::xml;
    use hanji_pptx::geom;

    for (id, target_name) in [(2, "Picture 1"), (4, "TextBox 3")] {
        for (end, attr) in [("st", "from"), ("end", "to")] {
            let mut original = package::read(&deck("shapes.pptx")).unwrap();
            let part = original.iter_mut().find(|p| p.name == "ppt/slides/slide1.xml").unwrap();
            let source = String::from_utf8(part.data.clone()).unwrap();
            part.data = source
                .replace("<a:endCxn id=\"2\" idx=\"1\"/>", &format!("<a:{end}Cxn id=\"{id}\" idx=\"1\"/>"))
                .into_bytes();
            let root = xml::parse(&part.data).unwrap().root;
            let tree = root.child("p:cSld").unwrap().child("p:spTree").unwrap();
            let connector = tree.elements().find(|e| geom::shape_id(e) == Some(10)).unwrap();
            let stored = geom::ends_of(&geom::own(connector).unwrap());
            let point = if end == "st" { stored.from } else { stored.to };
            let stored_point = format!("{} {}", shown_pt(point.0), shown_pt(point.1));
            let imp = import(&package::write(&original).unwrap());
            let target = imp.text.lines().find(|l| l.contains(&format!("name=\"{target_name}\""))).unwrap();
            let attached = format!("{attr}=\"s{id}.1\"");
            assert!(imp.text.contains(&attached));
            let snapshot = imp.remainder.clone();

            for point in [stored_point.as_str(), "300 300"] {
                let r = edit_in(&PptxModel, &imp.remainder, &imp.text, target, "", CAPS).unwrap();
                let r = edit_in(&PptxModel, &r.remainder, &r.text, &attached, &format!("{attr}=\"{point}\""), CAPS)
                    .unwrap();
                assert!(r.report.refused.is_empty());
                // Provenance must survive an edit and a serialized-remainder round trip.
                let rem = serde_json::from_slice(&serde_json::to_vec(&r.remainder).unwrap()).unwrap();
                let out = PptxEngine.export(&r.text, &rem).unwrap();
                let back = import(&out);
                assert_eq!(back.text, canonical(&r.text, &rem), "PutGet");
                let parts = package::read(&out).unwrap();
                let slide = slide1(&parts);
                assert!(!slide.contains(&format!("name=\"{target_name}\"")));
                let written = xml::parse(slide.as_bytes()).unwrap().root;
                let written_tree = written.child("p:cSld").unwrap().child("p:spTree").unwrap();
                let written_connector = written_tree.elements().find(|e| geom::shape_id(e) == Some(10)).unwrap();
                assert_eq!(geom::attachments(written_connector), [None, None], "dangling attachment");
                assert!(back.text.contains(&format!("{attr}=\"{point}\"")));
                if point == stored_point {
                    assert_eq!(written_connector, &hanji_pptx::route::without_attachments(connector));
                }
                for el in tree.elements().filter(|e| !matches!(geom::shape_id(e), Some(n) if n == id || n == 10)) {
                    assert!(written_tree.elements().any(|e| e == el), "untouched slide object: {}", el.to_xml());
                }
                assert_eq!(rem, r.remainder);
                assert_eq!(imp.remainder, snapshot);
            }
        }
    }
}

#[test]
fn attachments_hidden_by_slots_or_keep_objects_stay_byte_exact() {
    use hanji_package::xml::{self, fragment, Node};
    use hanji_pptx::geom;

    for (id, slot) in [(2, false), (4, true)] {
        for end in ["st", "end"] {
            let mut parts = package::read(&deck("shapes.pptx")).unwrap();
            if slot {
                let shell = hanji_pptx::DeckShell::of(&import(&package::write(&parts).unwrap()).remainder).unwrap();
                let layout = shell.deck.layouts.iter().find(|l| l.slots.iter().any(|s| s.name == "title")).unwrap();
                let rels = parts.iter_mut().find(|p| p.name == "ppt/slides/_rels/slide1.xml.rels").unwrap();
                let mut doc = xml::parse(&rels.data).unwrap();
                for rel in
                    doc.root.elements_mut().filter(|e| e.get("Type").is_some_and(|t| t.ends_with("/slideLayout")))
                {
                    rel.set("Target", &hanji_package::opc::relative_target("ppt/slides/slide1.xml", &layout.part));
                }
                rels.data = xml::write_doc(&doc);
            }
            let part = parts.iter_mut().find(|p| p.name == "ppt/slides/slide1.xml").unwrap();
            let mut doc = xml::parse(&part.data).unwrap();
            doc.root.walk_mut(&mut |el| {
                if !slot && el.is("a:blip") {
                    // An opaque picture shows as <keep/>, so its attachment is hidden.
                    el.children.push(Node::El(fragment(
                        "<a:duotone><a:srgbClr val=\"000000\"/><a:srgbClr val=\"FFFFFF\"/></a:duotone>",
                    )));
                }
                if el.is("a:endCxn") {
                    el.name = format!("a:{end}Cxn");
                    el.set("id", &id.to_string());
                }
                if slot && geom::shape_id(el) == Some(4) && el.is("p:sp") {
                    el.child_mut("p:nvSpPr")
                        .unwrap()
                        .child_mut("p:nvPr")
                        .unwrap()
                        .children
                        .push(Node::El(fragment("<p:ph type=\"title\"/>")));
                }
            });
            part.data = xml::write_doc(&doc);
            let before = part.data.clone();
            let imp = import(&package::write(&parts).unwrap());
            assert!(!imp.text.contains(&format!("s{id}.1")));
            if slot {
                assert!(imp.text.contains("::title"));
            }
            let out = PptxEngine.export(&imp.text, &imp.remainder).unwrap();
            let out_parts = package::read(&out).unwrap();
            assert_eq!(package::get(&out_parts, "ppt/slides/slide1.xml").unwrap(), before);
            let root = xml::parse(&before).unwrap().root;
            let tree = root.child("p:cSld").unwrap().child("p:spTree").unwrap();
            let connector = tree.elements().find(|e| geom::shape_id(e) == Some(10)).unwrap();
            // An unrelated text edit must also leave the hidden attachment untouched.
            let (out_parts, _) = edited(&imp, &[("Learning PPTX", "Still learning PPTX")]);
            let root = xml::parse(package::get(&out_parts, "ppt/slides/slide1.xml").unwrap()).unwrap().root;
            let tree = root.child("p:cSld").unwrap().child("p:spTree").unwrap();
            assert_eq!(tree.elements().find(|e| geom::shape_id(e) == Some(10)).unwrap(), connector);
        }
    }
}

#[test]
fn older_slide_metadata_still_exports_unchanged_attachments() {
    let imp = import(&deck("shapes.pptx"));
    let mut rem = imp.remainder.clone();
    for entry in rem.entries.iter_mut().filter(|e| e.kind == hanji_core::Kind::Slide && e.meta.tag == "slide") {
        let mut info: serde_json::Value = serde_json::from_str(&entry.meta.aux[0]).unwrap();
        info.as_object_mut().unwrap().remove("attachment_targets");
        entry.meta.aux[0] = serde_json::to_string(&info).unwrap();
    }
    let expected = PptxEngine.export(&imp.text, &imp.remainder).unwrap();
    assert_eq!(PptxEngine.export(&imp.text, &rem).unwrap(), expected);
}
