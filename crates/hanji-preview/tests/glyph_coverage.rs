//! Real font outlines/cmaps, with explicitly labelled adverse metadata doubles.
use base64::Engine as _;
use hanji_preview::{
    fonts::{Fonts, Metrics, Script, Source},
    FontData, FontOptions, FontResolver, PageData, PageFormat, ResolvedFont,
};
use std::collections::BTreeSet;
#[path = "fixtures/xlsx_grid.rs"]
#[allow(dead_code)]
mod fixture;

fn named(data: &[u8], family: &str) -> Vec<u8> {
    hanji_preview::sfnt::build(
        data,
        0,
        &hanji_preview::sfnt::Adjust {
            family: family.into(),
            bold: false,
            italic: false,
            line_em: None,
            ea_advance: None,
        },
    )
    .unwrap()
}
fn carlito() -> &'static [u8] {
    oxml_layout::bundled_fonts::bundled_font_data()[0].1
}
fn sparse() -> Vec<u8> {
    named(&hanji_preview::subset::subset(carlito(), 0, &BTreeSet::from(['A'])).unwrap(), "Sparse Test")
}
fn embedded_covers(svg: &str, text: &str) {
    let fonts: Vec<_> = svg
        .split("url('data:font/")
        .skip(1)
        .filter_map(|s| s.split_once(";base64,").and_then(|(_, s)| s.split('\'').next()))
        .map(|s| base64::engine::general_purpose::STANDARD.decode(s).unwrap())
        .collect();
    for c in text.chars().filter(|c| !c.is_whitespace()) {
        assert!(
            fonts.iter().any(|f| ttf_parser::Face::parse(f, 0).unwrap().glyph_index(c).is_some_and(|g| g.0 != 0)),
            "{c:?} must survive in a drawing subset"
        );
    }
}

#[test]
fn worksheet_face_cache_checks_later_cells_and_embeds_their_actual_characters() {
    let sheet = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><cols><col min="1" max="1" width="20"/></cols><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>A</t></is></c></row><row r="2"><c r="A2"><v>42</v></c></row><row r="3"><c r="A3" t="inlineStr"><is><t>AZ</t></is></c></row></sheetData></worksheet>"#;
    let styles = fixture::STYLES.replace("Calibri", "Sparse Test");
    let bytes = fixture::build(sheet, "", &styles, &[]);
    let mut book = hanji_preview::xlsx::open_xlsx(&bytes, Default::default()).unwrap();
    let options = FontOptions { fonts: vec![FontData::new(sparse())], ..Default::default() };
    let window = book.render_window_with_fonts(0, "A1:A3", &options).unwrap();
    assert_eq!(window.fonts().missing_glyphs_total, 0);
    assert_eq!(window.cells.iter().map(|c| c.display.as_str()).collect::<Vec<_>>(), ["A", "42", "AZ"]);
    let PageData::Svg(svg) = window.render(PageFormat::Svg).unwrap().data else { panic!() };
    embedded_covers(&svg, "A42Z");
    embedded_covers(&window.html("cache"), "A42Z");
}

#[test]
fn complete_faces_and_styles_are_preserved_and_unavailable_private_symbols_are_not_rescued() {
    let f = Fonts::from_bytes(&[FontData::new(sparse())], &[], None).unwrap();
    let a = f.resolve_font_for_text("Sparse Test", Script::Latin, false, false, "A").unwrap();
    assert_eq!(a.family, "Sparse Test");
    assert_eq!(a.metrics, Metrics::Original);
    let bold = f.resolve_font_for_text("Calibri", Script::Latin, true, false, "Hello 42").unwrap();
    assert_eq!(bold.metrics.name(), "compatible:Carlito");
    assert!(ttf_parser::Face::parse(&bold.font.data, bold.font.face_index).unwrap().is_bold());
    let symbol = f.resolve_font_for_text("Symbol", Script::Latin, false, false, "\u{F0B7}").unwrap();
    assert!(ttf_parser::Face::parse(&symbol.font.data, symbol.font.face_index)
        .unwrap()
        .glyph_index('\u{F0B7}')
        .is_none());
}

#[test]
fn joiner_controls_keep_the_authored_face_and_source_text() {
    let data = named(&hanji_preview::subset::subset(carlito(), 0, &BTreeSet::from(['A', 'B'])).unwrap(), "Joiner Test");
    let face = ttf_parser::Face::parse(&data, 0).unwrap();
    for control in ['\u{200C}', '\u{200D}'] {
        assert!(face.glyph_index(control).is_none(), "the fixture has no standalone joiner glyph");
    }
    let options = FontOptions { fonts: vec![FontData::new(data)], ..Default::default() };
    let fonts = Fonts::from_bytes(&options.fonts, &[], None).unwrap();
    let render = |text: &str| {
        let sheet = format!(
            r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>{text}</t></is></c></row></sheetData></worksheet>"#
        );
        let styles = fixture::STYLES.replace("Calibri", "Joiner Test");
        let bytes = fixture::build(&sheet, "", &styles, &[]);
        let mut book = hanji_preview::xlsx::open_xlsx(&bytes, Default::default()).unwrap();
        book.render_window_with_fonts(0, "A1", &options).unwrap()
    };
    let plain = render("AB");
    let PageData::Png(plain_png) = plain.render(PageFormat::Png { dpi: 96.0 }).unwrap().data else { panic!() };
    for text in ["A\u{200C}B", "A\u{200D}B"] {
        let selected = fonts.resolve_font_for_text("Joiner Test", Script::Latin, false, false, text).unwrap();
        assert_eq!(selected.family, "Joiner Test", "joining controls must not force a different physical face");
        assert_eq!(selected.metrics, Metrics::Original);
        let preview = render(text);
        assert_eq!(preview.cells[0].display, text);
        let PageData::Svg(svg) = preview.render(PageFormat::Svg).unwrap().data else { panic!() };
        let root = hanji_package::xml::parse(svg.as_bytes()).unwrap().root;
        let mut source = Vec::new();
        root.walk(&mut |element| {
            if element.local() == "text" {
                source.push(element.text_of(&["text", "tspan"]));
            }
        });
        assert!(source.iter().any(|s| s == text), "joining controls remain searchable source text");
        assert!(preview.html("joiners").contains(text));
        // Neither A nor B has a joining variant in this font. A joiner is
        // invisible here, even though its scalar has no standalone cmap glyph.
        let PageData::Png(png) = preview.render(PageFormat::Png { dpi: 96.0 }).unwrap().data else { panic!() };
        assert!(png == plain_png, "joining controls must not paint an independent glyph");
        assert!(preview.fonts().drawn_as_requested.iter().any(|f| f.requested == "Joiner Test" && f.chars == 2));
        assert_eq!(preview.fonts().missing_glyphs_total, 0, "{text:?}");
        assert!(!preview
            .quality()
            .diagnostics
            .iter()
            .any(|d| { d.code == hanji_preview::quality::DiagnosticCode::MissingGlyphs }));
        assert!(preview.diagnostics().iter().any(|d| d.message.contains("complex-script shaping")));
    }
    // Numeric values are right-aligned and check glyph ink bounds. A trailing
    // joiner's absent cmap glyph must not turn a fitting value into overflow.
    let numeric = |text: &str| {
        let sheet = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData></worksheet>"#;
        let styles = format!(
            r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><numFmts count="1"><numFmt numFmtId="164" formatCode="&quot;{text}&quot;"/></numFmts><fonts count="1"><font><name val="Joiner Test"/><sz val="11"/></font></fonts><cellXfs count="1"><xf fontId="0" numFmtId="164"/></cellXfs></styleSheet>"#
        );
        let bytes = fixture::build(sheet, "", &styles, &[]);
        let mut book = hanji_preview::xlsx::open_xlsx(&bytes, Default::default()).unwrap();
        book.render_window_with_fonts(0, "A1", &options).unwrap()
    };
    let PageData::Png(plain_png) = numeric("AB").render(PageFormat::Png { dpi: 96.0 }).unwrap().data else { panic!() };
    for text in ["AB\u{200C}", "AB\u{200D}"] {
        let preview = numeric(text);
        assert_eq!(preview.cells[0].display, text);
        assert!(!preview.diagnostics().iter().any(|d| d.message.contains("overflow indicator")));
        let PageData::Svg(svg) = preview.render(PageFormat::Svg).unwrap().data else { panic!() };
        assert!(svg.contains(text), "numeric joining controls retain the drawn source text");
        let PageData::Png(png) = preview.render(PageFormat::Png { dpi: 96.0 }).unwrap().data else { panic!() };
        assert!(png == plain_png, "a trailing joining control has no ink bounds");
    }
    // Some format characters have visible glyphs; do not exempt the whole Cf category.
    assert!(!fonts
        .resolve_font_for_text("Joiner Test", Script::Latin, false, false, "A\u{0601}B")
        .is_some_and(|font| font.family == "Joiner Test"));
}

fn generic_metadata_double() -> Vec<u8> {
    let mut data = named(carlito(), "Generic Metadata Double");
    let face = ttf_parser::RawFace::parse(&data, 0).unwrap();
    let offset =
        face.table_records.into_iter().find(|r| r.tag == ttf_parser::Tag::from_bytes(b"head")).unwrap().offset as usize;
    let flags = u16::from_be_bytes([data[offset + 16], data[offset + 17]]) | (1 << 14);
    data[offset + 16..offset + 18].copy_from_slice(&flags.to_be_bytes());
    data
}

#[test]
fn last_resort_cmap_entries_do_not_count_as_character_coverage_even_for_old_custom_resolvers() {
    let data = generic_metadata_double();
    assert!(ttf_parser::Face::parse(&data, 0).unwrap().glyph_index('A').is_some());
    let f = Fonts::from_bytes(&[FontData::new(data.clone())], &[], None).unwrap();
    let selected = f.resolve_font_for_text("Generic Metadata Double", Script::Latin, false, false, "A42").unwrap();
    assert_ne!(selected.family, "Generic Metadata Double");
    struct Legacy(FontData);
    impl FontResolver for Legacy {
        fn resolve_font(&self, _: &str, _: Script, _: bool, _: bool) -> Option<ResolvedFont> {
            Some(ResolvedFont {
                family: "Generic Metadata Double".into(),
                source: Source::Supplied,
                metrics: Metrics::Original,
                ea_advance: None,
                font: self.0.clone(),
            })
        }
    }
    let bytes = fixture::build(
        r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>A</t></is></c></row></sheetData></worksheet>"#,
        "",
        fixture::STYLES,
        &[],
    );
    let mut book = hanji_preview::xlsx::open_xlsx(&bytes, Default::default()).unwrap();
    let window = book.render_window_with_resolver(0, "A1", &Legacy(FontData::new(data))).unwrap();
    assert!(window.fonts().missing_glyphs_total > 0);
    let quality = window.quality();
    assert!(quality.diagnostics.iter().any(|d| d.code == hanji_preview::quality::DiagnosticCode::FontGenericSymbols));
    assert!(quality.enforce(hanji_preview::quality::Strictness::CriticalLosses).is_err());
    assert_eq!(window.cells[0].display, "A");
}

#[test]
fn docx_passes_available_supplemental_faces_to_the_engine_without_replacing_latin_fonts() {
    let original = include_bytes!("../../../prototype/preview/baseline/08-docx-new-report-en.docx");
    let mut parts = hanji_package::package::read(original).unwrap();
    parts.iter_mut().find(|p|p.name=="word/document.xml").unwrap().data=br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:rPr><w:rFonts w:ascii="Arial" w:hAnsi="Arial"/></w:rPr><w:t>Latin 42</w:t></w:r></w:p><w:p><w:r><w:rPr><w:rFonts w:ascii="Arial" w:hAnsi="Arial"/></w:rPr><w:t>"#.iter().copied().chain("مرحبا".as_bytes().iter().copied()).chain(br#"</w:t></w:r></w:p></w:body></w:document>"#.iter().copied()).collect();
    let bytes = hanji_package::package::write(&parts).unwrap();
    let arabic =
        oxml_layout::bundled_fonts::bundled_font_data().into_iter().find(|(n, _)| *n == "Noto Sans Arabic").unwrap().1;
    let options = FontOptions { fonts: vec![FontData::new(named(arabic, "Noto Sans Arabic"))], ..Default::default() };
    let preview = hanji_preview::docx::render_with_fonts(&bytes, &options).unwrap();
    assert_eq!(preview.fonts.missing_glyphs_total, 0);
    assert!(preview
        .fonts
        .substituted
        .iter()
        .any(|f| f.drawn.as_deref() == Some("Noto Sans Arabic") && f.source == Some(Source::Supplied)));
    let svg = preview.slide_svg(0);
    // The renderer may split the Latin/digit spans, while retaining their text.
    assert!(svg.contains("Latin") && svg.contains("42") && svg.contains("مرحبا"));
    embedded_covers(&svg, "Latin 42مرحبا");
}
