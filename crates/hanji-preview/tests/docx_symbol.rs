//! Existing real Carlito outlines with an explicitly constructed legacy cmap.
//! This is an encoding fixture, not native Word or a substituted visual oracle.
use base64::Engine as _;
use hanji_preview::{FontData, FontOptions, PageData, PageFormat};
use ttf_parser::{Face, GlyphId, OutlineBuilder, Tag};

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

/// The same actual glyph is exposed in Unicode and Macintosh Symbol byte B7.
/// A format-6 table allows the real Carlito glyph ID to exceed 255.
fn encoding_fixture(legacy_char: char) -> Vec<u8> {
    let mut font = named(oxml_layout::bundled_fonts::bundled_font_data()[0].1, "Symbol");
    let face = Face::parse(&font, 0).unwrap();
    let glyph = face.glyph_index(legacy_char).unwrap();
    let raw = face.raw_face().table(Tag::from_bytes(b"cmap")).unwrap();
    let count = u16::from_be_bytes(raw[2..4].try_into().unwrap()) as usize;
    let mut records: Vec<_> = raw[4..4 + count * 8]
        .as_chunks::<8>()
        .0
        .iter()
        .filter_map(|r| {
            let p = u16::from_be_bytes(r[0..2].try_into().unwrap());
            let e = u16::from_be_bytes(r[2..4].try_into().unwrap());
            let o = u32::from_be_bytes(r[4..8].try_into().unwrap());
            ((p, e) != (1, 0)).then_some((p, e, o))
        })
        .collect();
    let header = 4 + 8 * (records.len() + 1);
    for (_, _, o) in &mut records {
        *o += header as u32;
    }
    records.push((1, 0, (header + raw.len()) as u32));
    records.sort_unstable();
    let mut cmap = vec![];
    for n in [0u16, records.len() as u16] {
        cmap.extend(n.to_be_bytes());
    }
    for (p, e, o) in records {
        cmap.extend(p.to_be_bytes());
        cmap.extend(e.to_be_bytes());
        cmap.extend(o.to_be_bytes());
    }
    cmap.extend(raw);
    for n in [6u16, 12, 0, 0xB7, 1, glyph.0] {
        cmap.extend(n.to_be_bytes());
    }
    let record = face.raw_face().table_records.into_iter().position(|r| r.tag == Tag::from_bytes(b"cmap")).unwrap();
    // Carlito's post format 3 has no glyph names. This explicit metadata
    // fixture names its actual Unicode bullet, as the native Symbol face does.
    let bullet = face.glyph_index('\u{2022}').unwrap();
    let mut post = face.raw_face().table(Tag::from_bytes(b"post")).unwrap()[..32].to_vec();
    post[..4].copy_from_slice(&0x0002_0000u32.to_be_bytes());
    post.extend(face.number_of_glyphs().to_be_bytes());
    for g in 0..face.number_of_glyphs() {
        post.extend(if g == bullet.0 { 258u16 } else { 0u16 }.to_be_bytes());
    }
    post.push(6);
    post.extend(b"bullet");
    let post_record =
        face.raw_face().table_records.into_iter().position(|r| r.tag == Tag::from_bytes(b"post")).unwrap();
    let at = 12 + record * 16;
    let offset = font.len() as u32;
    font[at + 8..at + 12].copy_from_slice(&offset.to_be_bytes());
    font[at + 12..at + 16].copy_from_slice(&(cmap.len() as u32).to_be_bytes());
    font.extend(cmap);
    let at = 12 + post_record * 16;
    let offset = font.len() as u32;
    font[at + 8..at + 12].copy_from_slice(&offset.to_be_bytes());
    font[at + 12..at + 16].copy_from_slice(&(post.len() as u32).to_be_bytes());
    font.extend(post);
    named(&font, "Symbol")
}

fn document(charset: &str, marker_font: &str, numbered: bool) -> Vec<u8> {
    // Retain valid package relationships from an existing fixture; only this
    // in-memory test package's body, numbering and font table are replaced.
    let original = include_bytes!("../../../prototype/preview/baseline/04-docx-untouched-testword-various.docx");
    let mut parts = hanji_package::package::read(original).unwrap();
    let (ppr, run_font, text, format, marker) = if numbered {
        (
            "<w:pPr><w:numPr><w:ilvl w:val=\"0\"/><w:numId w:val=\"1\"/></w:numPr></w:pPr>",
            "Calibri",
            "Latin 42",
            "bullet",
            "\u{F0B7}",
        )
    } else {
        ("", "Symbol", "\u{F0B7}", "decimal", "%1.")
    };
    let existing_symbol_text = if numbered {
        r#"<w:r><w:rPr><w:rFonts w:ascii="Symbol" w:hAnsi="Symbol"/></w:rPr><w:t>Α</w:t></w:r>"#
    } else {
        ""
    };
    let doc = format!(
        r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p>{ppr}<w:r><w:rPr><w:rFonts w:ascii="{run_font}" w:hAnsi="{run_font}"/></w:rPr><w:t>{text}</w:t></w:r>{existing_symbol_text}</w:p><w:sectPr><w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440"/></w:sectPr></w:body></w:document>"#
    );
    let numbering = format!(
        r#"<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:abstractNum w:abstractNumId="1"><w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="{format}"/><w:lvlText w:val="{marker}"/><w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr><w:rPr><w:rFonts w:ascii="{marker_font}" w:hAnsi="{marker_font}"/></w:rPr></w:lvl></w:abstractNum><w:num w:numId="1"><w:abstractNumId w:val="1"/></w:num></w:numbering>"#
    );
    let fonts = format!(
        r#"<w:fonts xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:font w:name="{marker_font}"><w:charset w:val="{charset}"/></w:font></w:fonts>"#
    );
    for (name, xml) in [("word/document.xml", doc), ("word/numbering.xml", numbering), ("word/fontTable.xml", fonts)] {
        parts.iter_mut().find(|p| p.name == name).unwrap().data = xml.into_bytes();
    }
    hanji_package::package::write(&parts).unwrap()
}

#[derive(Default, Debug, PartialEq)]
struct Outline(Vec<Vec<u32>>);
impl OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push(vec![0, x.to_bits(), y.to_bits()]);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push(vec![1, x.to_bits(), y.to_bits()]);
    }
    fn quad_to(&mut self, x: f32, y: f32, x1: f32, y1: f32) {
        self.0.push(vec![2, x.to_bits(), y.to_bits(), x1.to_bits(), y1.to_bits()]);
    }
    fn curve_to(&mut self, x: f32, y: f32, x1: f32, y1: f32, x2: f32, y2: f32) {
        self.0.push(vec![3, x.to_bits(), y.to_bits(), x1.to_bits(), y1.to_bits(), x2.to_bits(), y2.to_bits()]);
    }
    fn close(&mut self) {
        self.0.push(vec![4]);
    }
}
fn outline(face: &Face<'_>, glyph: GlyphId) -> Outline {
    let mut out = Outline::default();
    assert!(face.outline_glyph(glyph, &mut out).is_some());
    out
}

fn embedded_fonts(svg: &str) -> Vec<Vec<u8>> {
    svg.split("url('data:font/")
        .skip(1)
        .filter_map(|s| s.split_once(";base64,").and_then(|(_, s)| s.split('\'').next()))
        .map(|s| base64::engine::general_purpose::STANDARD.decode(s).unwrap())
        .collect()
}

#[test]
fn declared_symbol_numbering_retains_the_private_code_and_actual_legacy_glyph() {
    let font = encoding_fixture('\u{2022}');
    let source = Face::parse(&font, 0).unwrap();
    let bullet = source.glyph_index('\u{2022}').unwrap();
    assert_eq!(source.glyph_name(bullet), Some("bullet"));
    assert!(source.glyph_index('\u{F0B7}').is_none());
    let bytes = document("02", "Symbol", true);
    let preview = hanji_preview::docx::render_with_fonts(
        &bytes,
        &FontOptions { fonts: vec![FontData::new(font.clone())], ..Default::default() },
    )
    .unwrap();
    assert_eq!(preview.page_count(), 1);
    assert_eq!(preview.fonts.missing_glyphs_total, 0);
    let PageData::Svg(svg) = preview.render_page(0, PageFormat::Svg).unwrap().data else { panic!() };
    assert!(svg.contains('\u{F0B7}') && !svg.contains('\u{2022}'));
    assert!(svg.contains("Latin") && svg.contains("42"));
    assert!(svg.contains('Α'));
    for output in [&svg, &preview.html("symbols")] {
        let fonts = embedded_fonts(output);
        let (drawing, glyph) = fonts
            .iter()
            .find_map(|data| {
                let f = Face::parse(data, 0).unwrap();
                f.glyph_index('\u{F0B7}').map(|g| (f, g))
            })
            .expect("the original private code survives into the drawing cmap");
        assert_eq!(outline(&drawing, glyph), outline(&source, bullet));
        assert_eq!(drawing.glyph_hor_advance(glyph), source.glyph_hor_advance(bullet));
        let alpha = drawing.glyph_index('Α').expect("existing Symbol Unicode mappings survive");
        assert_eq!(outline(&drawing, alpha), outline(&source, source.glyph_index('Α').unwrap()));
    }
}

#[test]
fn absent_or_disagreeing_legacy_mapping_retains_a_critical_missing_character() {
    for font in [named(oxml_layout::bundled_fonts::bundled_font_data()[0].1, "Symbol"), encoding_fixture('A')] {
        let preview = hanji_preview::docx::render_with_fonts(
            &document("02", "Symbol", true),
            &FontOptions { fonts: vec![FontData::new(font)], ..Default::default() },
        )
        .unwrap();
        assert!(preview.fonts.missing_glyphs.iter().any(|g| g.char == "U+F0B7"));
        assert!(preview
            .quality(hanji_preview::DocumentKind::Docx)
            .diagnostics
            .iter()
            .any(|d| d.code == hanji_preview::quality::DiagnosticCode::MissingGlyphs
                && d.severity == hanji_preview::quality::Severity::Error));
        let PageData::Svg(svg) = preview.render_page(0, PageFormat::Svg).unwrap().data else { panic!() };
        assert!(svg.contains('\u{F0B7}'));
    }
}

#[test]
fn a_family_name_or_unrelated_private_text_is_insufficient_to_activate_the_bridge() {
    for (charset, family, numbered) in [("00", "Symbol", true), ("02", "Other Symbols", true), ("02", "Symbol", false)]
    {
        let preview = hanji_preview::docx::render_with_fonts(
            &document(charset, family, numbered),
            &FontOptions { fonts: vec![FontData::new(encoding_fixture('\u{2022}'))], ..Default::default() },
        )
        .unwrap();
        assert!(preview.fonts.missing_glyphs.iter().any(|g| g.char == "U+F0B7"));
        assert!(preview
            .quality(hanji_preview::DocumentKind::Docx)
            .diagnostics
            .iter()
            .any(|d| d.code == hanji_preview::quality::DiagnosticCode::MissingGlyphs
                && d.severity == hanji_preview::quality::Severity::Error));
    }
}
