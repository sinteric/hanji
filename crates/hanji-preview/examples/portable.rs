//! Reproducible byte-only native/WASM smoke job. JSON goes to stdout, never to files.
//! Run with `--no-default-features`; the document and font are compiled-in test inputs.
use base64::Engine as _;
use hanji_preview::{fonts, FontData, FontOptions, PageData, PageFormat};

fn main() -> Result<(), String> {
    let bytes = include_bytes!("../../hanji-pptx/corpus/shapes.pptx");
    let font = hanji_preview::sfnt::build(
        oxml_layout::bundled_fonts::bundled_font_data()[0].1,
        0,
        &hanji_preview::sfnt::Adjust {
            family: "Caller Sans".into(),
            bold: false,
            italic: false,
            line_em: None,
            ea_advance: None,
        },
    )?;
    let options = FontOptions {
        fonts: vec![FontData::new(font)],
        aliases: Some(fonts::Aliases::parse("[[family]]\nnames = [\"Calibri\"]\nmetric = [\"Caller Sans\"]\n")?),
    };
    let preview = hanji_preview::render_pptx_with_fonts(bytes, &options)?;
    assert_eq!(preview.slide_count(), 6);
    assert_eq!(preview.fonts.missing_glyphs_total, 0);
    assert!(preview.fonts.substituted.iter().any(|f| f.source == Some(fonts::Source::Supplied)));
    let mut pages = vec![];
    for index in 0..preview.slide_count() {
        let svg = preview.render_page(index, PageFormat::Svg)?;
        let png = preview.render_page(index, PageFormat::Png { dpi: 48.0 })?;
        let (PageData::Svg(svg_data), PageData::Png(png_data)) = (svg.data, png.data) else { unreachable!() };
        assert!(png_data.starts_with(b"\x89PNG"));
        pages.push(serde_json::json!({"page":svg.page, "svg":svg_data,
            "png":base64::engine::general_purpose::STANDARD.encode(png_data), "diagnostics":svg.diagnostics}));
    }
    println!(
        "{}",
        serde_json::json!({"fonts":preview.fonts, "warnings":preview.warnings,
        "diagnostics":preview.diagnostics, "pages":pages})
    );
    Ok(())
}
