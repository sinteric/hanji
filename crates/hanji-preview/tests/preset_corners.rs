use hanji_preview::{render_document_with_fonts, DocumentKind, FontOptions, PageData, PageFormat};

#[test]
fn default_two_corner_presets_keep_their_curved_and_square_corners() {
    let mut parts = hanji_package::package::read(include_bytes!("fixtures/images/owned.pptx")).unwrap();
    let shape = |id, preset, x, fill| {
        format!(
            r#"<p:sp><p:nvSpPr><p:cNvPr id="{id}" name="{preset}"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="{x}" y="127000"/><a:ext cx="1270000" cy="1270000"/></a:xfrm><a:prstGeom prst="{preset}"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="{fill}"/></a:solidFill><a:ln><a:noFill/></a:ln></p:spPr></p:sp>"#
        )
    };
    let same = shape(2, "round2SameRect", 127000, "0033FF");
    let diagonal = shape(3, "round2DiagRect", 1905000, "00AA33");
    let slide = format!(
        r#"<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{same}{diagonal}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>"#
    );
    parts.iter_mut().find(|part| part.name == "ppt/slides/slide1.xml").unwrap().data = slide.into_bytes();
    let presentation = parts.iter_mut().find(|part| part.name == "ppt/presentation.xml").unwrap();
    presentation.data = std::str::from_utf8(&presentation.data)
        .unwrap()
        .replace(r#"<p:sldSz cx="914400" cy="914400"/>"#, r#"<p:sldSz cx="3810000" cy="1905000"/>"#)
        .into_bytes();
    let bytes = hanji_package::package::write(&parts).unwrap();
    let preview = render_document_with_fonts(DocumentKind::Pptx, &bytes, &FontOptions::default()).unwrap();
    assert_eq!(preview.page_count(), 1);
    let PageData::Png(png) = preview.render_page(0, PageFormat::Png { dpi: 72.0 }).unwrap().data else { panic!("PNG") };
    let image = resvg::tiny_skia::Pixmap::decode_png(&png).unwrap();
    assert_eq!((image.width(), image.height()), (300, 150));
    let pixel = |x, y| image.pixel(x, y).unwrap();
    let same_fill = pixel(60, 60);
    let diagonal_fill = pixel(200, 60);
    assert_eq!((same_fill.red(), same_fill.green(), same_fill.blue()), (0, 51, 255));
    assert_eq!((diagonal_fill.red(), diagonal_fill.green(), diagonal_fill.blue()), (0, 170, 51));
    // Authored presets define two curved corners and two square corners.
    // These interior samples distinguish the preset paths from rectangular fallback.
    assert_ne!(pixel(12, 12), same_fill);
    assert_ne!(pixel(108, 12), same_fill);
    assert_eq!(pixel(12, 108), same_fill);
    assert_eq!(pixel(108, 108), same_fill);
    assert_ne!(pixel(152, 12), diagonal_fill);
    assert_eq!(pixel(248, 12), diagonal_fill);
    assert_eq!(pixel(152, 108), diagonal_fill);
    assert_ne!(pixel(248, 108), diagonal_fill);
}
