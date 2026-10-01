//! Shared helpers: font loading, SVG size fix-up to 96-DPI px, resvg raster.
use std::collections::HashMap;
use std::sync::Arc;
use resvg::usvg;

pub struct CallerFont { pub family: String, pub data: Vec<u8>, pub path: String }

/// Load .ttf/.otf/.ttc from dirs (recursive). Family = the face's own first family name.
pub fn load_font_dirs(dirs: &[String]) -> Vec<CallerFont> {
    let mut out = Vec::new();
    fn walk(p: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        if let Ok(rd) = std::fs::read_dir(p) { for e in rd.flatten() { let p = e.path(); if p.is_dir() { walk(&p, out) } else { out.push(p) } } }
    }
    for d in dirs {
        let mut files = Vec::new(); walk(std::path::Path::new(d), &mut files); files.sort();
        for f in files {
            let ext = f.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if !matches!(ext.as_str(), "ttf" | "otf" | "ttc") { continue }
            let Ok(data) = std::fs::read(&f) else { continue };
            let mut db = usvg::fontdb::Database::new();
            db.load_font_data(data.clone());
            let fam = db.faces().next().and_then(|fc| fc.families.first().map(|(n, _)| n.clone()))
                .unwrap_or_else(|| f.file_stem().unwrap().to_string_lossy().into_owned());
            out.push(CallerFont { family: fam, data, path: f.display().to_string() });
        }
    }
    out
}

/// Rewrite the root <svg width="..pt" height="..pt"> to px at 96 DPI; viewBox (pt) unchanged.
pub fn svg_to_px(svg: &str, w_pt: f64, h_pt: f64) -> (String, u32, u32) {
    let wpx = (w_pt * 96.0 / 72.0).round() as u32;
    let hpx = (h_pt * 96.0 / 72.0).round() as u32;
    let end = svg.find('>').unwrap();
    let head = &svg[..end];
    let mut new_head = String::new();
    let mut rest = head;
    for (attr, val) in [("width=\"", wpx), ("height=\"", hpx)] {
        if let Some(i) = rest.find(attr) {
            let j = rest[i + attr.len()..].find('"').unwrap() + i + attr.len();
            new_head.push_str(&rest[..i]);
            new_head.push_str(&format!("{attr}{val}px\""));
            rest = &rest[j + 1..];
        }
    }
    new_head.push_str(rest);
    (format!("{}{}", new_head, &svg[end..]), wpx, hpx)
}

/// Rasterize with resvg; the layout's own font bytes are loaded and the SVG's
/// `rdocx-font-N` families map to them exactly (same approach as rdocx's resvg oracle).
pub fn raster(svg: &str, fonts: &[oxml_layout::FontData], wpx: u32, hpx: u32) -> Result<Vec<u8>, String> {
    let mut options = usvg::Options::default();
    let mut exact = HashMap::new();
    for f in fonts {
        let ids = options.fontdb_mut().load_font_source(usvg::fontdb::Source::Binary(Arc::new(f.data.to_vec())));
        if let Some(id) = ids.get(f.face_index as usize) { exact.insert(format!("rdocx-font-{}", f.id.0), *id); }
    }
    options.font_resolver = usvg::FontResolver {
        select_font: Box::new(move |font, _| font.families().iter().find_map(|fam| exact.get(fam.to_string().trim_matches('"')).copied())),
        select_fallback: Box::new(|_, _, _| None),
    };
    let tree = usvg::Tree::from_str(svg, &options).map_err(|e| format!("usvg: {e}"))?;
    let mut pm = resvg::tiny_skia::Pixmap::new(wpx, hpx).ok_or("pixmap")?;
    pm.fill(resvg::tiny_skia::Color::WHITE);
    let sx = wpx as f32 / tree.size().width(); let sy = hpx as f32 / tree.size().height();
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(sx, sy), &mut pm.as_mut());
    pm.encode_png().map_err(|e| format!("png: {e}"))
}

pub fn text_stats(svg: &str) -> (usize, usize) { (svg.matches("<text").count(), svg.matches("<path").count()) }
