//! Public package-to-output regressions; every fixture/image is source-owned.
use hanji_package::package;
use hanji_preview::{docx, render_pptx_with_fonts_and_limits, DocumentKind, FontOptions, ImageLimits, Preview};

const PNG: &[u8] = include_bytes!("fixtures/images/red.png");
const JPEG: &[u8] = include_bytes!("fixtures/images/gray.jpg");

fn bytes(kind: DocumentKind, data: &[u8], jpeg: bool) -> Vec<u8> {
    let base = match kind {
        DocumentKind::Pptx => include_bytes!("fixtures/images/owned.pptx").as_slice(),
        DocumentKind::Docx => include_bytes!("fixtures/images/owned.docx").as_slice(),
        _ => unreachable!(),
    };
    let mut parts = package::read(base).unwrap();
    for part in &mut parts {
        if part.name.ends_with("/media/image.png") {
            part.data = data.to_vec();
            if jpeg {
                part.name = part.name.replace("image.png", "image.jpg")
            }
        } else if jpeg && (part.name.ends_with(".rels") || part.name == "[Content_Types].xml") {
            part.data = String::from_utf8(part.data.clone())
                .unwrap()
                .replace("image.png", "image.jpg")
                .replace("Extension=\"png\"", "Extension=\"jpg\"")
                .replace("image/png", "image/jpeg")
                .into_bytes();
        }
    }
    package::write(&parts).unwrap()
}

fn preview(kind: DocumentKind, data: &[u8], jpeg: bool, limits: ImageLimits) -> Preview {
    let bytes = bytes(kind, data, jpeg);
    render(kind, &bytes, limits)
}

fn render(kind: DocumentKind, bytes: &[u8], limits: ImageLimits) -> Preview {
    match kind {
        DocumentKind::Pptx => render_pptx_with_fonts_and_limits(bytes, &FontOptions::default(), limits),
        DocumentKind::Docx => docx::render_with_fonts_and_limits(bytes, &FontOptions::default(), limits),
        _ => unreachable!(),
    }
    .unwrap()
}

#[test]
fn public_jobs_apply_page_image_aggregation() {
    for kind in [DocumentKind::Pptx, DocumentKind::Docx] {
        let mut parts = package::read(&bytes(kind, PNG, false)).unwrap();
        let (name, open, close, parent_close) = match kind {
            DocumentKind::Pptx => ("ppt/slides/slide1.xml", "<p:pic>", "</p:pic>", "</p:spTree>"),
            DocumentKind::Docx => ("word/document.xml", "<w:r>", "</w:r>", "</w:p>"),
            _ => unreachable!(),
        };
        let part = parts.iter_mut().find(|p| p.name == name).unwrap();
        let xml = String::from_utf8(part.data.clone()).unwrap();
        let start = xml.find(open).unwrap();
        let end = xml.find(close).unwrap() + close.len();
        let second = xml[start..end].replace("id=\"2\"", "id=\"3\"").replace("id=\"1\"", "id=\"2\"");
        part.data = xml.replace(parent_close, &format!("{second}{parent_close}")).into_bytes();
        let bytes = package::write(&parts).unwrap();
        let limits =
            ImageLimits { max_image_decoded_bytes: 4, max_page_decoded_bytes: 8, max_document_decoded_bytes: 8 };
        let p = render(kind, &bytes, limits);
        assert_eq!(p.page_count(), 1, "{kind:?}");
        assert_eq!(p.slide_svg(0).matches("<image").count(), 2, "{kind:?}");
        let p = render(kind, &bytes, ImageLimits { max_page_decoded_bytes: 4, ..limits });
        assert_eq!(p.slide_svg(0).matches("<image").count(), 1, "{kind:?}");
        assert!(p.diagnostics.iter().any(|d| d.message.contains("page decoded-image byte budget")));
    }
}

#[test]
fn public_jobs_preserve_owned_png_and_jpeg_pixels() {
    for kind in [DocumentKind::Pptx, DocumentKind::Docx] {
        for (data, jpeg, pixel) in [(PNG, false, [255, 0, 0, 255]), (JPEG, true, [128, 128, 128, 255])] {
            let p = preview(kind, data, jpeg, ImageLimits::default());
            assert!(!p.diagnostics.iter().any(|d| d.message.starts_with("embedded image")));
            assert!(p.html("owned image").contains("<image"), "{kind:?} jpeg={jpeg}: {:?}", p.diagnostics);
            let mut found = false;
            for page in 0..p.page_count() {
                let png = p.slide_png(page, 72.0).unwrap();
                let pixmap = resvg::tiny_skia::Pixmap::decode_png(&png).unwrap();
                found |= pixmap.data().as_chunks::<4>().0.contains(&pixel);
            }
            assert!(found, "{kind:?}: expected authored pixel");
        }
    }
}

#[test]
fn public_jobs_refuse_large_headers_before_output_decode() {
    for kind in [DocumentKind::Pptx, DocumentKind::Docx] {
        for (data, jpeg) in [
            (include_bytes!("fixtures/images/huge.png").as_slice(), false),
            (include_bytes!("fixtures/images/huge.jpg").as_slice(), true),
        ] {
            let p = preview(kind, data, jpeg, ImageLimits::default());
            match kind {
                DocumentKind::Pptx => {
                    // Pinned rpptx already performs a 64 MiB per-image check
                    // before its own compatibility raster. Preserve this guard.
                    assert!(p.diagnostics.iter().any(|d| d.path.starts_with("layout.diagnostics[")
                        && d.message.ends_with("exceeds the 64 MiB decoded-image render limit")));
                }
                DocumentKind::Docx => {
                    let diagnostics: Vec<_> =
                        p.diagnostics.iter().filter(|d| d.message.starts_with("embedded image")).collect();
                    assert_eq!(diagnostics.len(), 1, "{kind:?} jpeg={jpeg}: {:?}", p.diagnostics);
                    assert!(diagnostics[0].path.starts_with("pages["));
                    assert!(diagnostics[0].message.contains("decoded-image byte budget"));
                }
                _ => unreachable!(),
            }
            assert!(!p.html("refused image").contains("<image"));
            for page in 0..p.page_count() {
                assert!(!p.slide_svg(page).contains("<image"));
                let png = p.slide_png(page, 72.0).unwrap();
                let pixmap = resvg::tiny_skia::Pixmap::decode_png(&png).unwrap();
                if kind == DocumentKind::Docx {
                    assert!(
                        pixmap.data().as_chunks::<4>().0.iter().all(|p| *p == [255; 4]),
                        "{kind:?} jpeg={jpeg} page={page}"
                    );
                }
            }
        }
    }
}

#[test]
fn public_jobs_expose_image_budget_configuration() {
    for kind in [DocumentKind::Pptx, DocumentKind::Docx] {
        let limits =
            ImageLimits { max_image_decoded_bytes: 3, max_page_decoded_bytes: 4, max_document_decoded_bytes: 4 };
        let p = preview(kind, PNG, false, limits);
        assert_eq!(p.image_limits(), limits);
        assert!(!p.html("three byte budget").contains("<image"));
        let p = preview(kind, PNG, false, ImageLimits { max_image_decoded_bytes: 4, ..limits });
        assert!(p.html("four byte budget").contains("<image"));
    }
}
