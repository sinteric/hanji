//! Fonts cut to the characters a page (or a document) draws, for
//! `@font-face` data URIs (DESIGN.md §7.1): allsorts keeps a Unicode `cmap`,
//! which browsers need to draw text from characters.

use std::collections::BTreeSet;

use allsorts::binary::read::ReadScope;
use allsorts::font_data::FontData;
use allsorts::subset::{subset as cut, CmapTarget, SubsetProfile};
use allsorts::tag;

/// `font`'s face `index` with only the glyphs `chars` map to (and `.notdef`).
pub fn subset(font: &[u8], index: u32, chars: &BTreeSet<char>) -> Result<Vec<u8>, String> {
    let face = ttf_parser::Face::parse(font, index).map_err(|e| format!("not a font: {e}"))?;
    let mut glyphs: Vec<u16> =
        chars.iter().filter_map(|&c| face.glyph_index(c)).map(|g| g.0).filter(|&g| g != 0).collect();
    glyphs.sort_unstable();
    glyphs.dedup();
    glyphs.insert(0, 0);
    let data = ReadScope::new(font).read::<FontData<'_>>().map_err(|e| format!("not a font: {e}"))?;
    let provider = data.table_provider(index as usize).map_err(|e| format!("not a font: {e}"))?;
    // Some valid Apple fonts have no OS/2 table. Keep it and the TrueType
    // hinting programs when present, without requiring optional tables.
    let mut tables = vec![tag::OS_2, tag::CVT, tag::FPGM, tag::PREP];
    tables.retain(|t| allsorts::tables::FontTableProvider::has_table(&provider, *t));
    let profile = SubsetProfile::Custom(
        [tag::CMAP, tag::HEAD, tag::HHEA, tag::HMTX, tag::MAXP, tag::NAME, tag::POST]
            .into_iter()
            .chain(tables)
            .collect(),
    );
    cut(&provider, &glyphs, &profile, CmapTarget::Unicode).map_err(|e| format!("cannot subset the font: {e}"))
}

/// The `@font-face` rule for a subset font named `hanji-font-{id}`.
pub fn font_face(id: u32, data: &[u8], bold: bool, italic: bool) -> String {
    use base64::Engine as _;
    let (mime, format) = if data.starts_with(b"OTTO") { ("font/otf", "opentype") } else { ("font/ttf", "truetype") };
    format!(
        "@font-face{{font-family:'hanji-font-{id}';src:url('data:{mime};base64,{}') format('{format}');font-weight:{};font-style:{}}}",
        base64::engine::general_purpose::STANDARD.encode(data),
        if bold { "bold" } else { "normal" },
        if italic { "italic" } else { "normal" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // Remove a directory entry without moving the font data its offsets address.
    fn without_table(font: &[u8], tag: &[u8; 4]) -> Vec<u8> {
        let mut out = font.to_vec();
        let n = u16::from_be_bytes([out[4], out[5]]) as usize;
        let records: Vec<u8> = out[12..12 + n * 16]
            .as_chunks::<16>()
            .0
            .iter()
            .filter(|record| &record[..4] != tag)
            .flatten()
            .copied()
            .collect();
        assert_eq!(records.len(), (n - 1) * 16, "the fixture contains the table");
        out[12..12 + records.len()].copy_from_slice(&records);
        out[12 + records.len()..12 + n * 16].fill(0);
        let n = (n - 1) as u16;
        let selector = 15 - n.leading_zeros() as u16;
        let search_range = (1u16 << selector) * 16;
        for (at, value) in [(4, n), (6, search_range), (8, selector), (10, n * 16 - search_range)] {
            out[at..at + 2].copy_from_slice(&value.to_be_bytes());
        }
        out
    }

    #[test]
    fn a_font_without_optional_os2_keeps_its_glyphs_and_cmap() {
        let original = oxml_layout::bundled_fonts::bundled_font_data()[0].1;
        let font = without_table(original, b"OS/2");
        assert!(ttf_parser::Face::parse(&font, 0).unwrap().tables().os2.is_none());
        let chars: BTreeSet<char> = "Hello, 2026".chars().collect();
        let out = subset(&font, 0, &chars).unwrap();
        let face = ttf_parser::Face::parse(&out, 0).unwrap();
        assert!(face.tables().os2.is_none());
        assert!(face.tables().cmap.is_some());
        for c in chars {
            assert!(face.glyph_index(c).is_some(), "{c:?} survives subsetting without OS/2");
        }
        assert!(face.glyph_index('Z').is_none());
    }

    #[test]
    fn a_missing_required_table_is_still_an_error() {
        let original = oxml_layout::bundled_fonts::bundled_font_data()[0].1;
        let font = without_table(original, b"glyf");
        let chars = "Hello".chars().collect();
        assert!(subset(&font, 0, &chars).is_err());
    }

    #[test]
    fn a_subset_keeps_the_characters_and_their_cmap() {
        let carlito = oxml_layout::bundled_fonts::bundled_font_data()[0].1;
        let chars: BTreeSet<char> = "Hello, 2026".chars().collect();
        let out = subset(carlito, 0, &chars).unwrap();
        assert!(out.len() * 10 < carlito.len(), "{} of {}", out.len(), carlito.len());
        let f = ttf_parser::Face::parse(&out, 0).unwrap();
        for c in chars {
            assert!(f.glyph_index(c).is_some(), "{c:?} is in the cmap");
        }
        assert!(f.glyph_index('Z').is_none());
        assert!(font_face(3, &out, true, false)
            .starts_with("@font-face{font-family:'hanji-font-3';src:url('data:font/ttf;base64,"));
    }
}
