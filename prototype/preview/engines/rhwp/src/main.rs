//! render-rhwp --out DIR [--font-dir D]... [--alias REQ=LOADED]... [--path layer|legacy] FILES...
use resvg::usvg;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mut out, mut dirs, mut aliases, mut files, mut path) = (String::new(), vec![], HashMap::new(), vec![], "layer".to_string());
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => { out = args[i + 1].clone(); i += 2 }
            "--font-dir" => { dirs.push(args[i + 1].clone()); i += 2 }
            "--alias" => { let (a, b) = args[i + 1].split_once('=').unwrap(); aliases.insert(a.to_lowercase(), b.to_string()); i += 2 }
            "--path" => { path = args[i + 1].clone(); i += 2 }
            f => { files.push(f.to_string()); i += 1 }
        }
    }
    std::fs::create_dir_all(&out).unwrap();
    // fontdb for resvg: only the given dirs (no system fonts), so results are reproducible.
    let mut db = usvg::fontdb::Database::new();
    for d in &dirs { db.load_fonts_dir(d); }
    db.set_sans_serif_family("Noto Sans CJK KR");
    db.set_serif_family("Noto Serif CJK KR");
    db.set_monospace_family("Liberation Mono");
    let db = Arc::new(db);
    let families: std::collections::HashSet<String> = db.faces().flat_map(|f| f.families.iter().map(|(n, _)| n.to_lowercase())).collect();
    for file in &files {
        let name = std::path::Path::new(file).file_name().unwrap().to_string_lossy().into_owned();
        let nn = &name[..2];
        let t0 = Instant::now();
        let res: Result<serde_json::Value, String> = (|| {
            let bytes = std::fs::read(file).map_err(|e| e.to_string())?;
            let doc = rhwp::wasm_api::HwpDocument::from_bytes(&bytes).map_err(|e| format!("open: {e}"))?;
            let t_open = t0.elapsed().as_secs_f64();
            let n = doc.page_count();
            let mut pages = vec![];
            let mut requested = std::collections::BTreeMap::<String, (usize, String)>::new();
            let mut overflow = 0u64;
            let mut render_s = 0.0;
            let mut raster_s = 0.0;
            for k in 0..n {
                let t1 = Instant::now();
                let svg = if path == "legacy" { doc.render_page_svg_legacy_native(k) } else {
                    doc.render_page_svg_layer_with_profile_native(k, rhwp::paint::RenderProfile::Screen) }.map_err(|e| format!("render page {}: {e}", k + 1))?;
                overflow += doc.take_overflow_cell_lines() as u64;
                render_s += t1.elapsed().as_secs_f64();
                std::fs::write(format!("{out}/{nn}-page-{}.svg", k + 1), &svg).map_err(|e| e.to_string())?;
                let t2 = Instant::now();
                let mut opt = usvg::Options::default();
                opt.fontdb = db.clone();
                let al = aliases.clone();
                let fam2 = families.clone();
                let req = std::sync::Mutex::new(HashMap::<String, String>::new());
                let req = Arc::new(req);
                let req2 = req.clone();
                opt.font_resolver = usvg::FontResolver {
                    select_font: Box::new(move |font, db| {
                        // first family in the chain that exists, after alias mapping
                        let mut chosen = None;
                        for fam in font.families() {
                            let s = fam.to_string(); let s = s.trim_matches('"').to_string();
                            let mapped = al.get(&s.to_lowercase()).cloned().unwrap_or(s.clone());
                            let generic = matches!(s.as_str(), "serif" | "sans-serif" | "monospace" | "cursive" | "fantasy");
                            if generic || fam2.contains(&mapped.to_lowercase()) {
                                let q_fam = match s.as_str() { "serif" => usvg::fontdb::Family::Serif, "sans-serif" => usvg::fontdb::Family::SansSerif, "monospace" => usvg::fontdb::Family::Monospace, _ => usvg::fontdb::Family::Name(&mapped) };
                                let q = usvg::fontdb::Query { families: &[q_fam], weight: usvg::fontdb::Weight(font.weight()), stretch: font.stretch().into(), style: font.style().into() };
                                if let Some(id) = db.query(&q) { chosen = Some((id, s.clone(), mapped.clone())); break; }
                            }
                        }
                        let first = font.families().first().map(|f| f.to_string().trim_matches('"').to_string()).unwrap_or_default();
                        let r = chosen.as_ref().map(|(_, s, m)| if s == m { s.clone() } else { format!("{s} -> {m}") }).unwrap_or("NONE".into());
                        req2.lock().unwrap().insert(first, r);
                        chosen.map(|c| c.0)
                    }),
                    select_fallback: usvg::FontResolver::default_fallback_selector(),
                };
                let tree = usvg::Tree::from_str(&svg, &opt).map_err(|e| format!("usvg page {}: {e}", k + 1))?;
                let (w, h) = (tree.size().width().round() as u32, tree.size().height().round() as u32);
                let mut pm = resvg::tiny_skia::Pixmap::new(w, h).ok_or("pixmap")?;
                pm.fill(resvg::tiny_skia::Color::WHITE);
                resvg::render(&tree, resvg::tiny_skia::Transform::default(), &mut pm.as_mut());
                std::fs::write(format!("{out}/{nn}-page-{}.png", k + 1), pm.encode_png().map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
                raster_s += t2.elapsed().as_secs_f64();
                for (a, b) in req.lock().unwrap().iter() { let e = requested.entry(a.clone()).or_insert((0, b.clone())); e.0 += 1; }
                let head = &svg[..svg.find('>').unwrap_or(0).min(400)];
                pages.push(serde_json::json!({"page": k + 1, "w_px": w, "h_px": h, "svg_head": head, "svg_bytes": svg.len(),
                    "text_elems": svg.matches("<text").count(), "path_elems": svg.matches("<path").count()}));
            }
            let fonts: serde_json::Value = requested.into_iter().map(|(k, (c, v))| (k, serde_json::json!({"resolved": v, "pages": c}))).collect::<serde_json::Map<_, _>>().into();
            Ok(serde_json::json!({"open_s": t_open, "svg_s": render_s, "png_s": raster_s, "page_count": n, "pages": pages, "font_resolution": fonts, "overflow_cell_lines": overflow}))
        })();
        let secs = t0.elapsed().as_secs_f64();
        let line = match res {
            Ok(mut v) => { v["file"] = name.clone().into(); v["ok"] = true.into(); v["total_s"] = secs.into(); v }
            Err(e) => serde_json::json!({"file": name, "ok": false, "error": e, "total_s": secs}),
        };
        println!("{}", line);
        eprintln!("{name}: ok={} pages={} {:.2}s {}", line["ok"], line["page_count"], secs, line["error"].as_str().unwrap_or(""));
    }
}
