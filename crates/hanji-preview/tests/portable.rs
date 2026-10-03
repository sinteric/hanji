//! The production byte-input path: no filesystem, system fonts, or CLI dependency.
use hanji_preview::{fonts, FontData, FontOptions, FontResolver, PageData, PageFormat, ResolvedFont};

const SHAPES: &[u8] = include_bytes!("../../hanji-pptx/corpus/shapes.pptx");

fn supplied() -> FontOptions {
    let data = hanji_preview::sfnt::build(
        oxml_layout::bundled_fonts::bundled_font_data()[0].1,
        0,
        &hanji_preview::sfnt::Adjust {
            family: "Caller Sans".into(),
            bold: false,
            italic: false,
            line_em: None,
            ea_advance: None,
        },
    )
    .unwrap();
    FontOptions {
        fonts: vec![FontData::new(data)],
        aliases: Some(
            fonts::Aliases::parse("[[family]]\nnames = [\"Calibri\"]\nmetric = [\"Caller Sans\"]\n").unwrap(),
        ),
    }
}

#[test]
fn supplied_bytes_and_aliases_draw_the_requested_face() {
    let p = hanji_preview::render_pptx_with_fonts(SHAPES, &supplied()).unwrap();
    assert_eq!(p.slide_count(), 6);
    assert_eq!(p.fonts.missing_glyphs_total, 0);
    assert!(p.fonts.substituted.iter().any(|f| f.requested == "Calibri"
        && f.drawn.as_deref() == Some("Caller Sans")
        && f.source == Some(fonts::Source::Supplied)));
    for k in 0..p.slide_count() {
        let svg = p.render_page(k, PageFormat::Svg).unwrap();
        assert_eq!(svg.page.index, k);
        assert!(matches!(svg.data, PageData::Svg(ref s) if s.starts_with("<svg")
            && (!s.contains("<text ") || s.contains("@font-face"))));
        let png = p.render_page(k, PageFormat::Png { dpi: 48.0 }).unwrap();
        assert!(matches!(png.data, PageData::Png(ref b) if b.starts_with(b"\x89PNG")));
    }
}

#[test]
fn invalid_supplied_bytes_or_face_index_are_configuration_errors() {
    let bad = FontOptions { fonts: vec![FontData::new(b"invalid font".as_slice())], ..Default::default() };
    assert!(hanji_preview::render_pptx_with_fonts(SHAPES, &bad).err().unwrap().contains("supplied font 0, face 0"));
    let mut bad = supplied();
    bad.fonts[0].face_index = 100;
    assert!(hanji_preview::render_pptx_with_fonts(SHAPES, &bad).err().unwrap().contains("face 100"));
}

#[test]
fn page_jobs_validate_indices_and_raster_resolution() {
    let p = hanji_preview::render_pptx_with_fonts(SHAPES, &FontOptions::default()).unwrap();
    assert!(p.page_info(6).is_none());
    assert!(p.render_page(6, PageFormat::Svg).is_err());
    assert!(p.slide_png(6, 96.0).is_err());
    for dpi in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(p.render_page(0, PageFormat::Png { dpi }).is_err());
    }
    let page = p.render_page(0, PageFormat::Png { dpi: 48.0 }).unwrap();
    let PageData::Png(bytes) = page.data else { panic!("PNG page") };
    let image = resvg::tiny_skia::Pixmap::decode_png(&bytes).unwrap();
    assert_eq!(image.width(), (page.page.width * 48.0 / 72.0).round() as u32);
    assert_eq!(image.height(), (page.page.height * 48.0 / 72.0).round() as u32);
    assert!(page.diagnostics.iter().any(|d| d.message.contains("unsupported slide hyperlink action")));
}

#[test]
fn a_custom_resolver_produces_the_same_buffers_and_library_diagnostics() {
    struct CallerPolicy {
        fonts: fonts::Fonts,
        warnings: Vec<String>,
    }
    impl FontResolver for CallerPolicy {
        fn resolve_font(
            &self,
            requested: &str,
            script: fonts::Script,
            bold: bool,
            italic: bool,
        ) -> Option<ResolvedFont> {
            self.fonts.resolve_font(requested, script, bold, italic)
        }
        fn warnings(&self) -> &[String] {
            &self.warnings
        }
    }
    let resolver = CallerPolicy {
        fonts: fonts::Fonts::from_bytes(&[], &[], None).unwrap(),
        warnings: vec!["caller font policy warning".into()],
    };
    let direct = hanji_preview::render_pptx_with_fonts(SHAPES, &FontOptions::default()).unwrap();
    let custom = hanji_preview::render_pptx_with_resolver(SHAPES, &resolver).unwrap();
    assert_eq!(direct.diagnostics, custom.diagnostics);
    assert!(custom.warnings.iter().any(|w| w == "caller font policy warning"));
    for k in 0..direct.slide_count() {
        assert_eq!(direct.slide_svg(k), custom.slide_svg(k));
        assert_eq!(direct.slide_png(k, 48.0).unwrap(), custom.slide_png(k, 48.0).unwrap());
    }
}

#[test]
fn per_style_resolved_faces_drive_reports_and_html_substitution_marks() {
    struct StylePolicy {
        regular: FontData,
        bold: FontData,
        italic: FontData,
        both: FontData,
    }
    impl FontResolver for StylePolicy {
        fn resolve_font(&self, requested: &str, _: fonts::Script, bold: bool, italic: bool) -> Option<ResolvedFont> {
            let (font, family, source, metrics) = match (bold, italic) {
                (false, false) => {
                    (&self.regular, requested.to_string(), fonts::Source::Supplied, fonts::Metrics::Original)
                }
                (true, false) => {
                    (&self.bold, "Bold Fallback".into(), fonts::Source::Embedded, fonts::Metrics::Substitute)
                }
                (false, true) => (
                    &self.italic,
                    "Italic Fallback".into(),
                    fonts::Source::Bundled,
                    fonts::Metrics::Compatible(requested.into()),
                ),
                (true, true) => {
                    (&self.both, "Both Fallback".into(), fonts::Source::Supplied, fonts::Metrics::Substitute)
                }
            };
            Some(ResolvedFont { family, source, metrics, ea_advance: None, font: font.clone() })
        }
    }
    let variant = |bold, italic| {
        let (_, data) = oxml_layout::bundled_fonts::bundled_font_data()
            .into_iter()
            .find(|(_, data)| {
                ttf_parser::Face::parse(data, 0).is_ok_and(|f| f.is_bold() == bold && f.is_italic() == italic)
            })
            .unwrap();
        FontData::new(
            hanji_preview::sfnt::build(
                data,
                0,
                &hanji_preview::sfnt::Adjust {
                    family: "Style Test".into(),
                    bold,
                    italic,
                    line_em: None,
                    ea_advance: None,
                },
            )
            .unwrap(),
        )
    };
    let policy = StylePolicy {
        regular: variant(false, false),
        bold: variant(true, false),
        italic: variant(false, true),
        both: variant(true, true),
    };
    let slide = br#"<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="2" name="Styles"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="914400" y="914400"/><a:ext cx="8000000" cy="3000000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr sz="2400" b="0" i="0"><a:latin typeface="Style Requested"/></a:rPr><a:t>Regular</a:t></a:r><a:r><a:rPr sz="2400" b="1" i="0"><a:latin typeface="Style Requested"/></a:rPr><a:t> Bold</a:t></a:r><a:r><a:rPr sz="2400" b="0" i="1"><a:latin typeface="Style Requested"/></a:rPr><a:t> Italic</a:t></a:r><a:r><a:rPr sz="2400" b="1" i="1"><a:latin typeface="Style Requested"/></a:rPr><a:t> Both</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#;
    let mut parts = hanji_package::package::read(SHAPES).unwrap();
    parts.iter_mut().find(|p| p.name == "ppt/slides/slide1.xml").unwrap().data = slide.to_vec();
    let bytes = hanji_package::package::write(&parts).unwrap();
    let p = hanji_preview::render_pptx_with_resolver(&bytes, &policy).unwrap();
    let original = p.fonts.drawn_as_requested.iter().find(|f| f.requested == "Style Requested").unwrap();
    assert_eq!(original.chars, 7, "only the regular run uses the requested face");
    assert_eq!(original.source, fonts::Source::Supplied);
    for (family, source, metrics, chars) in [
        ("Bold Fallback", fonts::Source::Embedded, fonts::Metrics::Substitute, 4),
        ("Italic Fallback", fonts::Source::Bundled, fonts::Metrics::Compatible("Style Requested".into()), 6),
        ("Both Fallback", fonts::Source::Supplied, fonts::Metrics::Substitute, 4),
    ] {
        let f = p
            .fonts
            .substituted
            .iter()
            .find(|f| f.requested == "Style Requested" && f.drawn.as_deref() == Some(family))
            .unwrap_or_else(|| panic!("missing {family}: {:#?}", p.fonts));
        assert_eq!(f.source, Some(source));
        assert_eq!(f.metrics, metrics);
        assert_eq!(f.chars, chars);
        let html = p.html("styles");
        assert!(html.contains(&format!("data-font-requested=\"Style Requested\" data-font-drawn=\"{family}\"")));
        assert!(html.contains(&format!("<title>Style Requested → {family} (metrics: {})</title>", metrics.name())));
        assert!(!html.contains("<title>Style Requested → Style Requested"));
    }
    // A resolver may offer only a regular physical face for styled requests.
    // FontManager's synthetic style flags must not erase that face's metadata.
    let regular_only = FontOptions {
        fonts: vec![policy.regular.clone()],
        aliases: Some(
            fonts::Aliases::parse("[[family]]\nnames = [\"Style Requested\"]\nmetric = [\"Style Test\"]\n").unwrap(),
        ),
    };
    let p = hanji_preview::render_pptx_with_fonts(&bytes, &regular_only).unwrap();
    assert_eq!(
        p.fonts.substituted.iter().filter(|f| f.requested == "Style Requested").map(|f| f.chars).sum::<usize>(),
        21
    );
    assert!(!p.fonts.drawn_as_requested.iter().any(|f| f.requested.starts_with("hanji-face-")));
    assert!(!p.fonts.substituted.iter().any(|f| f.requested.starts_with("hanji-face-")));
}

#[test]
#[cfg(all(feature = "host-fonts", not(target_family = "wasm")))]
fn the_native_adapter_and_byte_path_have_equivalent_outputs_without_host_fonts() {
    let bytes = hanji_preview::render_pptx_with_fonts(SHAPES, &FontOptions::default()).unwrap();
    let host =
        hanji_preview::render_pptx(SHAPES, &hanji_preview::Options { font_dirs: vec![], system_fonts: false }).unwrap();
    assert_eq!(bytes.diagnostics, host.diagnostics);
    assert_eq!(bytes.warnings, host.warnings);
    for k in 0..bytes.slide_count() {
        assert_eq!(bytes.slide_svg(k), host.slide_svg(k));
        assert_eq!(bytes.slide_png(k, 48.0).unwrap(), host.slide_png(k, 48.0).unwrap());
    }
}
