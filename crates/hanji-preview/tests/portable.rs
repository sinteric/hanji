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
    assert!(page.diagnostics.iter().any(|d| d.message.contains("unsupported connector line style")));
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
