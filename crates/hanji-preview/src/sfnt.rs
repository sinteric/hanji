//! The face the preview hands rpptx for one requested family: the chosen
//! face cut out of its file (one face of a `.ttc`), renamed to the
//! requested family, and with the engine-compat metric changes below. Only
//! the `name`, `hhea`, `OS/2` and `hmtx` tables change; the outlines are the
//! source face's, so what is drawn is that face's glyphs.
//!
//! - **Renamed** to the requested family, so the layout's font list says
//!   which request each face serves (the substitution report and the
//!   viewer's marks read it back).
//! - **A 1.2 em line.** rpptx 0.12.1 makes a single line ascent + descent +
//!   line gap high; PowerPoint makes it 1.2 × the font size. The face's
//!   ascent and descent are scaled to 1.2 em (in their own proportion) and
//!   the line gap set to 0, so rpptx spaces lines as PowerPoint does. The
//!   baseline stays in the same place within the line, relative to its height.
//! - **East Asian advances** (only for a substitute whose alias entry gives
//!   the requested face's `ea_advance`, §7.1 "metric-adjusted substitute"):
//!   every Hangul and CJK glyph is given that advance, so lines break where
//!   the requested face breaks them. Glyphs keep their left side bearing.

use ttf_parser::{RawFace, Tag};

/// What to change in a face.
#[derive(Clone, Debug, PartialEq)]
pub struct Adjust {
    /// The family name the face is given.
    pub family: String,
    pub bold: bool,
    pub italic: bool,
    /// Line height in em (ascent + descent, no line gap); `None` keeps the face's.
    pub line_em: Option<f64>,
    /// Advance in em for Hangul and CJK glyphs; `None` keeps the face's.
    pub ea_advance: Option<f64>,
}

/// Hangul, Hangul jamo, CJK symbols, punctuation and ideographs, and full-width forms.
const EA_RANGES: &[(u32, u32)] =
    &[(0x1100, 0x11FF), (0x2E80, 0x9FFF), (0xAC00, 0xD7AF), (0xF900, 0xFAFF), (0xFF00, 0xFF60)];

fn be16(d: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([d[at], d[at + 1]])
}

fn put16(d: &mut [u8], at: usize, v: u16) {
    d[at..at + 2].copy_from_slice(&v.to_be_bytes());
}

/// The tables of face `index` of `data` (a font file or collection), in file order.
fn tables(data: &[u8], index: u32) -> Result<Vec<(Tag, Vec<u8>)>, String> {
    let raw = RawFace::parse(data, index).map_err(|e| format!("not a font: {e}"))?;
    let mut out = vec![];
    for r in raw.table_records {
        let (start, len) = (r.offset as usize, r.length as usize);
        let bytes = data.get(start..start + len).ok_or_else(|| format!("table {} is outside the file", r.tag))?;
        out.push((r.tag, bytes.to_vec()));
    }
    Ok(out)
}

fn utf16(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(u16::to_be_bytes).collect()
}

/// A `name` table (format 0, Windows Unicode, US English) with the family,
/// style, full and PostScript names.
fn name_table(family: &str, bold: bool, italic: bool) -> Vec<u8> {
    let style = match (bold, italic) {
        (false, false) => "Regular",
        (true, false) => "Bold",
        (false, true) => "Italic",
        (true, true) => "Bold Italic",
    };
    let full = if style == "Regular" { family.to_string() } else { format!("{family} {style}") };
    let ps: String = format!("{family}-{style}").chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
    let ps = if ps.trim_matches('-').is_empty() { format!("HanjiPreview-{style}") } else { ps };
    let records = [(1u16, utf16(family)), (2, utf16(style)), (4, utf16(&full)), (6, utf16(&ps))];
    let mut head = vec![];
    head.extend_from_slice(&0u16.to_be_bytes());
    head.extend_from_slice(&(records.len() as u16).to_be_bytes());
    head.extend_from_slice(&((6 + 12 * records.len()) as u16).to_be_bytes());
    let mut strings = vec![];
    for (id, s) in &records {
        for v in [3u16, 1, 0x409, *id, s.len() as u16, strings.len() as u16] {
            head.extend_from_slice(&v.to_be_bytes());
        }
        strings.extend_from_slice(s);
    }
    head.extend(strings);
    head
}

fn checksum(d: &[u8]) -> u32 {
    d.chunks(4).fold(0u32, |sum, c| {
        let mut w = [0u8; 4];
        w[..c.len()].copy_from_slice(c);
        sum.wrapping_add(u32::from_be_bytes(w))
    })
}

/// An sfnt file from tables (sorted by tag, 4-byte aligned, checksums and
/// `head.checkSumAdjustment` set).
fn write(mut tables: Vec<(Tag, Vec<u8>)>) -> Vec<u8> {
    tables.sort_by_key(|(t, _)| t.0);
    let n = tables.len() as u16;
    let flavor: u32 =
        if tables.iter().any(|(t, _)| *t == Tag::from_bytes(b"CFF ")) { 0x4F54_544F } else { 0x0001_0000 };
    let entry_selector = 15 - n.leading_zeros() as u16; // floor(log2 n)
    let search_range = (1u16 << entry_selector) * 16;
    let mut out = vec![];
    out.extend_from_slice(&flavor.to_be_bytes());
    for v in [n, search_range, entry_selector, n * 16 - search_range] {
        out.extend_from_slice(&v.to_be_bytes());
    }
    let mut offset = 12 + 16 * tables.len();
    let mut head_at = None;
    for (tag, data) in &mut tables {
        if *tag == Tag::from_bytes(b"head") && data.len() >= 12 {
            data[8..12].fill(0);
        }
        out.extend_from_slice(&tag.0.to_be_bytes());
        out.extend_from_slice(&checksum(data).to_be_bytes());
        out.extend_from_slice(&(offset as u32).to_be_bytes());
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        offset += data.len().next_multiple_of(4);
    }
    for (tag, data) in &tables {
        if *tag == Tag::from_bytes(b"head") {
            head_at = Some(out.len());
        }
        out.extend_from_slice(data);
        out.resize(out.len().next_multiple_of(4), 0);
    }
    if let Some(at) = head_at.filter(|at| out.len() >= at + 12) {
        let adj = 0xB1B0_AFBAu32.wrapping_sub(checksum(&out));
        out[at + 8..at + 12].copy_from_slice(&adj.to_be_bytes());
    }
    out
}

/// Face `index` of `data` as one standalone font with `adj` applied.
pub fn build(data: &[u8], index: u32, adj: &Adjust) -> Result<Vec<u8>, String> {
    let face = ttf_parser::Face::parse(data, index).map_err(|e| format!("not a font: {e}"))?;
    let upem = f64::from(face.units_per_em());
    let ea_glyphs: Vec<u16> = match adj.ea_advance {
        Some(_) => EA_RANGES
            .iter()
            .flat_map(|&(a, b)| a..=b)
            .filter_map(char::from_u32)
            .filter_map(|c| face.glyph_index(c).map(|g| g.0))
            .collect(),
        None => vec![],
    };
    let mut tables = tables(data, index)?;
    let num_h_metrics = tables.iter().find(|(t, _)| *t == Tag::from_bytes(b"hhea")).map(|(_, d)| be16(d, 34));
    for (tag, d) in &mut tables {
        match &tag.to_bytes() {
            b"name" => *d = name_table(&adj.family, adj.bold, adj.italic),
            b"hhea" if d.len() >= 36 => {
                if let Some(em) = adj.line_em {
                    let (asc, desc) = line_metrics(be16(d, 4) as i16, be16(d, 6) as i16, em * upem);
                    put16(d, 4, asc as u16);
                    put16(d, 6, (-desc) as u16);
                    put16(d, 8, 0);
                }
            }
            b"OS/2" if d.len() >= 78 => {
                if let Some(em) = adj.line_em {
                    let (asc, desc) = line_metrics(be16(d, 68) as i16, be16(d, 70) as i16, em * upem);
                    put16(d, 68, asc as u16);
                    put16(d, 70, (-desc) as u16);
                    put16(d, 72, 0);
                    put16(d, 74, asc.max(0) as u16);
                    put16(d, 76, desc.max(0) as u16);
                }
            }
            b"hmtx" => {
                if let (Some(em), Some(n)) = (adj.ea_advance, num_h_metrics) {
                    let adv = (em * upem).round() as u16;
                    for &g in &ea_glyphs {
                        let at = usize::from(g) * 4;
                        if g < n && at + 2 <= d.len() {
                            put16(d, at, adv);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(write(tables))
}

/// Ascent and descent (both positive) summing to `line` font units, in the
/// proportion of the face's own.
fn line_metrics(ascent: i16, descent: i16, line: f64) -> (i16, i16) {
    let (a, d) = (f64::from(ascent.max(0)), f64::from(descent.unsigned_abs()));
    let share = if a + d > 0.0 { a / (a + d) } else { 0.8 };
    let asc = (line * share).round();
    (asc as i16, (line - asc).round() as i16)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn carlito() -> &'static [u8] {
        oxml_layout::bundled_fonts::bundled_font_data().into_iter().find(|(f, _)| *f == "Carlito").unwrap().1
    }

    #[test]
    fn a_face_is_renamed_and_keeps_its_glyphs() {
        let adj =
            Adjust { family: "맑은 고딕".into(), bold: false, italic: false, line_em: None, ea_advance: None };
        let out = build(carlito(), 0, &adj).unwrap();
        let mut db = fontdb::Database::new();
        db.load_font_data(out.clone());
        let face = db.faces().next().unwrap();
        assert_eq!(face.families[0].0, "맑은 고딕");
        let (a, b) = (ttf_parser::Face::parse(carlito(), 0).unwrap(), ttf_parser::Face::parse(&out, 0).unwrap());
        for c in ['A', 'g', '1'] {
            let (ga, gb) = (a.glyph_index(c).unwrap(), b.glyph_index(c).unwrap());
            assert_eq!(ga, gb);
            assert_eq!(a.glyph_hor_advance(ga), b.glyph_hor_advance(gb));
        }
        assert_eq!((a.ascender(), a.descender()), (b.ascender(), b.descender()));
    }

    #[test]
    fn the_line_becomes_1_2_em_in_the_face_s_proportion() {
        let adj = Adjust { family: "X".into(), bold: true, italic: false, line_em: Some(1.2), ea_advance: None };
        let out = build(carlito(), 0, &adj).unwrap();
        let f = ttf_parser::Face::parse(&out, 0).unwrap();
        let em = f64::from(f.units_per_em());
        let line = f64::from(f.ascender()) - f64::from(f.descender()) + f64::from(f.line_gap());
        assert!((line / em - 1.2).abs() < 0.002, "{}", line / em);
        let src = ttf_parser::Face::parse(carlito(), 0).unwrap();
        let share = f64::from(src.ascender()) / f64::from(src.ascender() - src.descender());
        assert!((f64::from(f.ascender()) / (line) - share).abs() < 0.002);
    }

    #[test]
    fn east_asian_advances_change_only_east_asian_glyphs() {
        // Carlito has no Hangul: the Latin advances stay.
        let adj = Adjust { family: "X".into(), bold: false, italic: false, line_em: None, ea_advance: Some(1.0) };
        let out = build(carlito(), 0, &adj).unwrap();
        let (a, b) = (ttf_parser::Face::parse(carlito(), 0).unwrap(), ttf_parser::Face::parse(&out, 0).unwrap());
        let g = a.glyph_index('W').unwrap();
        assert_eq!(a.glyph_hor_advance(g), b.glyph_hor_advance(g));
    }

    #[test]
    fn line_metrics_keep_the_proportion() {
        assert_eq!(line_metrics(1160, -288, 1200.0), (961, 239));
        assert_eq!(line_metrics(0, 0, 1000.0), (800, 200));
    }
}
