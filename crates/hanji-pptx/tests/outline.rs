//! Outlines (§5.3): each slot's, shape's and line's outline shown as
//! `border=`, a line's arrowheads as `start=` and `end=`, from its own
//! `a:ln`, its style or its layout, and written back only where the text
//! changes them.

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

/// A new object's line without the id and name the write gave it.
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

/// An exact edit, exported: the export reads back as the edited text.
fn edited(imp: &Imported, old: &str, new: &str) -> (Vec<Part>, Imported) {
    let r = edit_in(&PptxModel, &imp.remainder, &imp.text, old, new, CAPS).unwrap_or_else(|e| panic!("{old:?}: {e}"));
    assert!(r.report.removed.is_empty() && r.report.refused.is_empty(), "{:?}", r.report);
    let out = PptxEngine.export(&r.text, &r.remainder).unwrap_or_else(|e| panic!("{e}"));
    let back = import(&out);
    assert_eq!(without_new_ids(&back.text), without_new_ids(&canonical(&r.text, &r.remainder)), "PutGet");
    (package::read(&out).unwrap(), back)
}

fn slide(parts: &[Part], n: usize) -> String {
    let pres = String::from_utf8(package::get(parts, "ppt/presentation.xml").unwrap().to_vec()).unwrap();
    let list = &pres[pres.find("<p:sldIdLst>").unwrap()..pres.find("</p:sldIdLst>").unwrap()];
    let rid = list.split("r:id=\"").nth(n).unwrap();
    let part = opc::target_of(parts, "ppt/presentation.xml", &rid[..rid.find('"').unwrap()]).unwrap();
    String::from_utf8(package::get(parts, &part).unwrap().to_vec()).unwrap()
}

/// The `p:spPr` of the object named `name`.
fn sp_pr(x: &str, name: &str) -> String {
    let at = x.find(&format!("name=\"{name}\"")).unwrap_or_else(|| panic!("no {name}"));
    let s = at + x[at..].find("<p:spPr").unwrap();
    x[s..s + x[s..].find("</p:spPr>").unwrap() + 9].to_string()
}

const R8: &str = "name=\"Rectangle 8\" box=\"174 72 72 72\" fill=accent1+80% border=\"2pt solid accent1*\"/>";

#[test]
fn every_outline_is_shown_from_where_it_comes() {
    let t = import(&deck("shapes.pptx")).text;
    for want in [
        // A style's line reference: its width, and its colour with the theme's adjustments.
        "<line id=\"s6\" name=\"Straight Connector 5\" from=\"84 144\" to=\"252 144\" border=\"0.75pt solid accent1*\"/>",
        // A connector's own arrowheads over its style's line.
        "from=\"468 366\" to=\"468 216\" border=\"0.75pt solid accent1*\" start=arrow end=arrow/>",
        "fill=accent1 border=\"2pt solid accent1*\" font=Calibri",
        // A slot's outline, from its layout.
        "::title box=\"54 168 612 116\" border=\"0.75pt solid accent1\" font=Calibri",
    ] {
        assert!(t.contains(want), "{want}\n{t}");
    }
    // No outline is nothing: a text box.
    assert!(t.contains("<shape id=\"s4\" name=\"TextBox 3\" box=\"72 72 180 29\" font=Calibri"), "{t}");
    // A line's own, in a group.
    assert!(import(&deck("turns-deck.pptx")).text.contains("border=\"2pt solid accent1\" effects=\"shadow\"/>"));
}

#[test]
fn a_changed_border_writes_only_the_outline() {
    let pkg = deck("shapes.pptx");
    let before = package::read(&pkg).unwrap();
    let imp = import(&pkg);
    let dashed = R8.replace("2pt solid accent1*", "3pt dashed #FF0000");
    let (parts, back) = edited(&imp, R8, &dashed);
    let (a, b) = (slide(&before, 6), slide(&parts, 6));
    let ln =
        "<a:ln w=\"38100\"><a:solidFill><a:srgbClr val=\"FF0000\"/></a:solidFill><a:prstDash val=\"dash\"/></a:ln>";
    assert_eq!(sp_pr(&b, "Rectangle 8"), sp_pr(&a, "Rectangle 8").replace("</p:spPr>", &format!("{ln}</p:spPr>")));
    assert_eq!(b.replace(&sp_pr(&b, "Rectangle 8"), ""), a.replace(&sp_pr(&a, "Rectangle 8"), ""), "only that outline");
    // Taken back to what its style gives: its own outline goes, and the slide is as it was.
    let (parts, _) = edited(&back, &dashed, R8);
    assert_eq!(slide(&parts, 6), a);
    // Its width alone: the style's colour stays the style's.
    let (parts, _) = edited(&imp, R8, &R8.replace("2pt solid", "4pt solid"));
    assert!(sp_pr(&slide(&parts, 6), "Rectangle 8").ends_with("</a:solidFill><a:ln w=\"50800\"/></p:spPr>"));
}

#[test]
fn a_border_left_out_is_none_and_written_back_it_is_the_style_s() {
    let pkg = deck("shapes.pptx");
    let before = package::read(&pkg).unwrap();
    let imp = import(&pkg);
    let bare = R8.replace(" border=\"2pt solid accent1*\"", "");
    let (parts, imp2) = edited(&imp, R8, &bare);
    assert!(sp_pr(&slide(&parts, 6), "Rectangle 8").ends_with("</a:solidFill><a:ln><a:noFill/></a:ln></p:spPr>"));
    let (parts, _) = edited(&imp2, &bare, R8);
    assert_eq!(slide(&parts, 6), slide(&before, 6));
}

#[test]
fn arrowheads_and_a_slot_s_outline_are_written() {
    let pkg = deck("shapes.pptx");
    let imp = import(&pkg);
    // One arrowhead goes, the other changes.
    let (parts, _) = edited(&imp, "start=arrow end=arrow/>", "end=triangle/>");
    let c = sp_pr(&slide(&parts, 1), "Straight Arrow Connector 7");
    assert!(c.contains("<a:ln><a:headEnd type=\"none\"/><a:tailEnd type=\"triangle\"/></a:ln>"), "{c}");
    // A slot's colour over its layout's: only the colour is the slide's own.
    let title = "::title box=\"54 168 612 116\" border=\"0.75pt solid accent1\"";
    let (parts, _) = edited(&imp, title, &title.replace("accent1\"", "accent2\""));
    let x = slide(&parts, 2);
    assert!(x.contains("<a:ln><a:solidFill><a:schemeClr val=\"accent2\"/></a:solidFill></a:ln>"), "{x}");
    // A new line: drawn 1pt in the text colour, or as its text says.
    let tb = "<shape id=\"s4\" name=\"TextBox 3\"";
    let (parts, back) = edited(&imp, tb, &format!("<line from=\"72 110\" to=\"252 110\"/>\n{tb}"));
    assert!(back.text.contains("to=\"252 110\" border=\"1pt solid tx1\"/>"), "{}", back.text);
    assert!(slide(&parts, 1).contains("<a:ln w=\"12700\"><a:solidFill><a:schemeClr val=\"tx1\"/></a:solidFill></a:ln>"));
    let new = format!("<line from=\"72 110\" to=\"252 110\" border=\"2pt dotted accent1\" end=stealth/>\n{tb}");
    let (parts, _) = edited(&imp, tb, &new);
    assert!(
        slide(&parts, 1).contains("<a:ln w=\"25400\"><a:solidFill><a:schemeClr val=\"accent1\"/></a:solidFill><a:prstDash val=\"sysDot\"/><a:tailEnd type=\"stealth\"/></a:ln>"),
        "{}",
        slide(&parts, 1)
    );
}

#[test]
fn what_cannot_be_written_is_refused() {
    let imp = import(&deck("shapes.pptx"));
    let err = |text: String, rem: &Remainder| match PptxEngine.export(&text, rem) {
        Err(EngineError::Invalid(d)) => hanji_format::diag::render(&d),
        Err(EngineError::Refused(m)) => m,
        other => panic!("expected an error, got {:?}", other.map(|_| ())),
    };
    let m = err(imp.text.replace(R8, &R8.replace("solid accent1*", "solid accent2*")), &imp.remainder);
    assert!(m.contains("accent2*") && m.contains("cannot be written"), "{m}");
    let m = err(imp.text.replace(R8, &R8.replace("border=", "outline=")), &imp.remainder);
    assert!(m.contains("border="), "{m}");
    let m = err(imp.text.replace(R8, &R8.replace("/>", " end=arrow/>")), &imp.remainder);
    assert!(m.contains("arrowheads are a line's"), "{m}");
    let m = err(imp.text.replace("to=\"252 144\" border=", "to=\"252 144\" fill=accent1 border="), &imp.remainder);
    assert!(m.contains("a line has no fill"), "{m}");
    let m = err(imp.text.replace(R8, &R8.replace("2pt solid", "2pt wavy")), &imp.remainder);
    assert!(m.contains("wavy"), "{m}");
}
