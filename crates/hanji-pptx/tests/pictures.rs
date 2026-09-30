//! Pictures in the Presentation text (DESIGN.md §5.3): `<picture id name box
//! src crop mask alt/>`, each value left as shown keeping the stored XML, a
//! changed one written into its own XML child, a new image from the package
//! or from a file the host hands over, and the pictures kept whole.

use std::collections::BTreeMap;

use hanji_core::edit::edit_in;
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

fn canonical(text: &str, rem: &Remainder) -> String {
    let (blocks, _) = PptxModel.resolve(text, rem, CAPS).unwrap_or_else(|e| panic!("{e:?}"));
    PptxEngine::text_of(&blocks, rem, None)
}

/// Exports `text` (with the host's `files`); the export reads back as its
/// canonical form (PutGet), `fix` mapping what the write assigns (a new
/// picture's id, name and image part) back to what the text wrote.
fn export_with(text: &str, rem: &Remainder, files: &BTreeMap<String, Vec<u8>>, fix: &[(&str, &str)]) -> Vec<Part> {
    let out = PptxEngine.export_with_files(text, rem, files).unwrap_or_else(|e| panic!("{e}"));
    let mut back = import(&out).text;
    for (a, b) in fix {
        back = back.replacen(a, b, 1);
    }
    let want = canonical(text, rem);
    if back != want {
        let d = back.lines().zip(want.lines()).find(|(a, b)| a != b);
        panic!("PutGet: first difference (read back, canonical): {d:?}");
    }
    package::read(&out).unwrap()
}

fn export(text: &str, rem: &Remainder) -> Vec<Part> {
    export_with(text, rem, &BTreeMap::new(), &[])
}

fn xml(parts: &[Part], name: &str) -> String {
    String::from_utf8(package::get(parts, name).unwrap_or_else(|| panic!("no {name}")).to_vec()).unwrap()
}

fn exact(text: &str, rem: &Remainder, old: &str, new: &str) -> (String, Remainder) {
    let r = edit_in(&PptxModel, rem, text, old, new, CAPS).unwrap_or_else(|e| panic!("{old:?}: {e}"));
    assert!(r.report.refused.is_empty() && r.report.removed.is_empty(), "{:?}", r.report);
    (r.text, r.remainder)
}

/// The `<p:pic>` named `name` in a slide's XML.
fn pic<'x>(x: &'x str, name: &str) -> &'x str {
    let at = x.find(&format!("name=\"{name}\"")).unwrap_or_else(|| panic!("no {name}"));
    let start = x[..at].rfind("<p:pic>").unwrap();
    &x[start..start + x[start..].find("</p:pic>").unwrap() + 8]
}

/// A package with one part's text changed.
fn with_part(pkg: &[u8], name: &str, f: &dyn Fn(&str) -> String) -> Vec<u8> {
    let mut parts = package::read(pkg).unwrap();
    let p = parts.iter_mut().find(|p| p.name == name).unwrap();
    p.data = f(std::str::from_utf8(&p.data).unwrap()).into_bytes();
    package::write(&parts).unwrap()
}

const S5: &str = "ppt/slides/slide5.xml";

#[test]
fn pictures_read_with_their_image_crop_mask_and_alt_and_getput_keeps_them() {
    let pkg = deck("audit/synth-modern-pitch.pptx");
    let imp = import(&pkg);
    for line in [
        "<picture id=\"s3\" name=\"Picture 2\" box=\"64 120 400 300\" src=\"media/image2.jpg\" crop=\"10 0 5 0\" alt=\"image.jpg\"/>",
        "<picture id=\"s4\" name=\"Picture 3\" box=\"500 120 180 180\" src=\"media/image3.jpg\" mask=\"ellipse\" alt=\"image.jpg\"/>",
        "<picture id=\"s5\" name=\"Picture 4\" box=\"710 120 200 150\" src=\"media/image4.jpg\" mask=\"roundRect\" alt=\"image.jpg\"/>",
    ] {
        assert!(imp.text.contains(line), "{line}\n{}", imp.text);
    }
    assert_eq!(canonical(&imp.text, &imp.remainder), imp.text);
    let out = PptxEngine.export(&imp.text, &imp.remainder).unwrap();
    for p in package::read(&pkg).unwrap() {
        assert_eq!(package::get(&package::read(&out).unwrap(), &p.name), Some(&p.data[..]), "{} changed", p.name);
    }
}

#[test]
fn a_crop_is_written_to_its_src_rect_each_value_left_as_shown_kept_exact() {
    // Picture 2's crop stored as 10.432% on the left shows 10.4.
    let pkg = with_part(&deck("audit/synth-modern-pitch.pptx"), S5, &|x| x.replace("l=\"10000\"", "l=\"10432\""));
    let imp = import(&pkg);
    assert!(imp.text.contains("crop=\"10.4 0 5 0\""), "{}", imp.text);
    let (text, rem) = exact(&imp.text, &imp.remainder, "crop=\"10.4 0 5 0\"", "crop=\"10.4 2 8 0\"");
    let x = xml(&export(&text, &rem), S5);
    assert!(
        pic(&x, "Picture 2").contains("<a:blip r:embed=\"rId2\"/><a:srcRect l=\"10432\" r=\"8000\" t=\"2000\"/>"),
        "{x}"
    );
    // Moved and resized, a picture keeps its crop.
    let (text, rem) = exact(&imp.text, &imp.remainder, "box=\"64 120 400 300\"", "box=\"72 120 300 225\"");
    let x = xml(&export(&text, &rem), S5);
    assert!(pic(&x, "Picture 2").contains("<a:srcRect l=\"10432\" r=\"5000\"/>"), "{x}");
    // No crop: the a:srcRect goes; a new one is written after a:blip.
    let (text, rem) = exact(&imp.text, &imp.remainder, " crop=\"10.4 0 5 0\"", "");
    let x = xml(&export(&text, &rem), S5);
    assert!(pic(&x, "Picture 2").contains("<a:blip r:embed=\"rId2\"/><a:stretch>"), "{x}");
    let (text, rem) =
        exact(&imp.text, &imp.remainder, "src=\"media/image3.jpg\"", "src=\"media/image3.jpg\" crop=\"0 12.5 0 12.5\"");
    let x = xml(&export(&text, &rem), S5);
    assert!(
        pic(&x, "Picture 3").contains("<a:blip r:embed=\"rId3\"/><a:srcRect t=\"12500\" b=\"12500\"/><a:stretch>"),
        "{x}"
    );
}

#[test]
fn a_mask_and_alt_text_are_written_to_their_own_xml() {
    let pkg = deck("audit/synth-modern-pitch.pptx");
    let imp = import(&pkg);
    let before = xml(&package::read(&pkg).unwrap(), S5);
    let (text, rem) =
        exact(&imp.text, &imp.remainder, "mask=\"ellipse\" alt=\"image.jpg\"", "mask=\"hexagon\" alt=\"배터리 공장\"");
    let x = xml(&export(&text, &rem), S5);
    let p = pic(&x, "Picture 3");
    assert!(p.contains("name=\"Picture 3\" descr=\"배터리 공장\"/>"), "{p}");
    assert!(p.contains("<a:prstGeom prst=\"hexagon\"><a:avLst/></a:prstGeom>"), "{p}");
    // Everything else on the slide is as it was.
    assert_eq!(x.replace(p, ""), before.replace(pic(&before, "Picture 3"), ""));
    // No mask is a rectangle; no alt removes the alternative text.
    let (text, rem) = exact(&imp.text, &imp.remainder, " mask=\"roundRect\" alt=\"image.jpg\"", "");
    let x = xml(&export(&text, &rem), S5);
    let p = pic(&x, "Picture 4");
    assert!(p.contains("<p:cNvPr id=\"5\" name=\"Picture 4\"/>") && p.contains("<a:prstGeom prst=\"rect\">"), "{p}");
    assert!(p.contains("<a:effectLst><a:outerShdw"), "its shadow stays: {p}");
}

#[test]
fn a_new_src_names_another_image_and_the_old_one_goes_when_nothing_names_it() {
    let pkg = deck("audit/synth-modern-pitch.pptx");
    let imp = import(&pkg);
    // Picture 4 shows slide 1's photo: slide 5 gains a relationship to it,
    // and image4.jpg, which only Picture 4 showed, goes with its relationship.
    let (text, rem) = exact(&imp.text, &imp.remainder, "src=\"media/image4.jpg\"", "src=\"media/image1.jpg\"");
    let parts = export(&text, &rem);
    let rels = opc::rels_of(&parts, S5);
    let r = rels.iter().find(|r| r.target == "../media/image1.jpg").expect("a relationship to image1.jpg");
    assert!(pic(&xml(&parts, S5), "Picture 4").contains(&format!("<a:blip r:embed=\"{}\"/>", r.id)));
    assert!(!rels.iter().any(|r| r.target.ends_with("image4.jpg")), "{rels:?}");
    assert!(package::get(&parts, "ppt/media/image4.jpg").is_none());
    assert!(package::get(&parts, "ppt/media/image1.jpg").is_some());
    // Two pictures showing one image share its relationship.
    let (text, rem) = exact(&imp.text, &imp.remainder, "src=\"media/image4.jpg\"", "src=\"media/image3.jpg\"");
    let parts = export(&text, &rem);
    let x = xml(&parts, S5);
    assert!(pic(&x, "Picture 4").contains("<a:blip r:embed=\"rId3\"/>"), "{x}");
    assert_eq!(opc::rels_of(&parts, S5).len(), 3, "the layout's, image2's and image3's");
}

/// A 1 × 1 PNG.
const PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0D, 0x49,
    0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8, 0xCF, 0xC0, 0xF0, 0x1F, 0x00, 0x05, 0x00, 0x01, 0xFF, 0x89, 0x99, 0x3D,
    0x1D, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

#[test]
fn a_picture_from_a_file_the_host_hands_over_is_added() {
    let pkg = deck("korean-deck.pptx");
    let imp = import(&pkg);
    let line = "<picture box=\"600 20 96 96\" src=\"logo.png\" mask=\"ellipse\" alt=\"회사 로고\"/>\n";
    let at = imp.text.find("<shape id=\"s5\" name=\"출처\"").unwrap();
    let text = format!("{}{line}{}", &imp.text[..at], &imp.text[at..]);
    let r = hanji_core::edit::rewrite_in(&PptxModel, &imp.remainder, &imp.text, &text, CAPS).unwrap();
    // Without the file, the write is refused with the reason.
    match PptxEngine.export(&r.text, &r.remainder) {
        Err(EngineError::Refused(m)) => {
            assert!(m.contains("the new picture: src=\"logo.png\" is neither an image of this file"), "{m}")
        }
        other => panic!("expected a refusal, got {:?}", other.map(|_| ())),
    }
    let files = BTreeMap::from([("logo.png".to_string(), PNG.to_vec())]);
    let parts = export_with(
        &r.text,
        &r.remainder,
        &files,
        &[(
            "<picture id=\"s6\" name=\"Picture 5\" box=\"600 20 96 96\" src=\"media/image2.png\"",
            "<picture box=\"600 20 96 96\" src=\"logo.png\"",
        )],
    );
    let s3 = "ppt/slides/slide3.xml";
    let x = xml(&parts, s3);
    let p = pic(&x, "Picture 5");
    let rel = opc::rels_of(&parts, s3)
        .into_iter()
        .find(|r| r.target == "../media/image2.png")
        .expect("the new image's relationship");
    assert!(p.starts_with("<p:pic><p:nvPicPr><p:cNvPr id=\"6\" name=\"Picture 5\" descr=\"회사 로고\"/>"), "{p}");
    assert!(p.contains(&format!("<a:blip r:embed=\"{}\"/>", rel.id)) && p.contains("prst=\"ellipse\""), "{p}");
    assert_eq!(package::get(&parts, "ppt/media/image2.png"), Some(PNG));
    assert!(xml(&parts, "[Content_Types].xml")
        .contains("<Override PartName=\"/ppt/media/image2.png\" ContentType=\"image/png\"/>"));
    // A file that is not an image is refused.
    let files = BTreeMap::from([("logo.png".to_string(), b"<svg/>".to_vec())]);
    match PptxEngine.export_with_files(&r.text, &r.remainder, &files) {
        Err(EngineError::Refused(m)) => assert!(m.contains("logo.png is not a PNG, JPEG, GIF or BMP image"), "{m}"),
        other => panic!("expected a refusal, got {:?}", other.map(|_| ())),
    }
}

#[test]
fn what_a_picture_cannot_be_is_refused_with_the_reason() {
    let imp = import(&deck("audit/synth-modern-pitch.pptx"));
    let invalid = |old: &str, new: &str, want: &str| {
        let text = imp.text.replacen(old, new, 1);
        match PptxEngine.export(&text, &imp.remainder) {
            Err(EngineError::Invalid(d)) => assert!(d.iter().any(|x| x.message.contains(want)), "{d:?}"),
            other => panic!("{new}: expected invalid, got {:?}", other.map(|_| ())),
        }
    };
    invalid("mask=\"ellipse\"", "mask=\"circle\"", "mask=\"circle\" is not a preset shape");
    invalid("mask=\"ellipse\"", "mask=\"elipse\"", "is not a preset shape");
    invalid("crop=\"10 0 5 0\"", "crop=\"60 0 50 0\"", "cuts off the whole image");
    invalid("crop=\"10 0 5 0\"", "crop=\"10%\"", "is not a crop");
    invalid("name=\"Picture 2\"", "name=\"Photo\"", "is not a picture of this file");
    invalid(" src=\"media/image2.jpg\"", "", "a picture names its image");
    invalid("<picture id=\"s3\"", "<picture id=\"s3\" fill=\"red\"", "<picture> has no attribute \"fill\"");
    // A picture in a group changes only its box (§5.3).
    let imp = import(&deck("audit/o09_Performance_typical.pptx"));
    let line =
        imp.text.lines().find(|l| l.starts_with("<picture id=\"s19474\"")).expect("a picture in a group").to_string();
    let text = imp.text.replacen(&line, &line.replace("/>", " alt=\"새 설명\"/>"), 1);
    let r = hanji_core::edit::rewrite_in(&PptxModel, &imp.remainder, &imp.text, &text, CAPS).unwrap();
    match PptxEngine.export(&r.text, &r.remainder) {
        Err(EngineError::Refused(m)) => assert!(m.contains("a group's objects can be moved and resized here"), "{m}"),
        other => panic!("expected a refusal, got {:?}", other.map(|_| ())),
    }
}

#[test]
fn pictures_with_artistic_effects_a_duotone_or_a_video_are_kept_whole() {
    let base = deck("korean-deck.pptx");
    let s4 = "ppt/slides/slide4.xml";
    let shown = import(&base);
    assert!(shown.text.contains("<picture id=\"s4\" name=\"Picture 3\""), "{}", shown.text);
    for (what, blip) in [
        (
            "artistic effects",
            "<a:blip r:embed=\"rId2\"><a:extLst><a:ext uri=\"{BEBA8EAE-BF5A-486C-A8C5-ECC9F3942E4B}\"><a14:imgProps xmlns:a14=\"http://schemas.microsoft.com/office/drawing/2010/main\"><a14:imgLayer r:embed=\"rId2\"/></a14:imgProps></a:ext></a:extLst></a:blip>",
        ),
        ("a duotone", "<a:blip r:embed=\"rId2\"><a:duotone><a:prstClr val=\"black\"/><a:srgbClr val=\"D9C3A5\"/></a:duotone></a:blip>"),
    ] {
        let pkg = with_part(&base, s4, &|x| x.replacen("<a:blip r:embed=\"rId2\"/>", blip, 1));
        let imp = import(&pkg);
        assert!(!imp.text.contains("<picture"), "{what}: {}", imp.text);
        assert!(imp.text.contains("kind=\"picture\" summary=\"image.png\" box=\"576 396 72 36\"/>"), "{what}: {}", imp.text);
        // Kept whole, it still moves (as any <keep/> does) and GetPut holds.
        let out = PptxEngine.export(&imp.text, &imp.remainder).unwrap();
        assert_eq!(xml(&package::read(&out).unwrap(), s4), xml(&package::read(&pkg).unwrap(), s4));
    }
    let video = import(&deck("EmbeddedVideo.pptx"));
    assert!(!video.text.contains("<picture") && video.text.contains("kind=\"picture\""), "{}", video.text);
}
