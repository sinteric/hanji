//! render-rpptx --out DIR [--font-dir D]... [--alias REQ=LOADED]... [--mode caller|bundled] FILES...
mod common;
mod svg;
use rpptx::Presentation;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mut out, mut dirs, mut aliases, mut files, mut mode) = (String::new(), vec![], vec![], vec![], "caller".to_string());
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => { out = args[i + 1].clone(); i += 2 }
            "--font-dir" => { dirs.push(args[i + 1].clone()); i += 2 }
            "--alias" => { let (a, b) = args[i + 1].split_once('=').unwrap(); aliases.push((a.to_string(), b.to_string())); i += 2 }
            "--mode" => { mode = args[i + 1].clone(); i += 2 }
            f => { files.push(f.to_string()); i += 1 }
        }
    }
    std::fs::create_dir_all(&out).unwrap();
    let fonts = common::load_font_dirs(&dirs);
    let font_files: Vec<oxml_layout::font::FontFile> = fonts.iter().map(|f| oxml_layout::font::FontFile { family: f.family.clone(), data: f.data.clone() }).collect();
    eprintln!("caller fonts: {:?}", fonts.iter().map(|f| &f.family).collect::<Vec<_>>());
    for file in &files {
        let name = std::path::Path::new(file).file_name().unwrap().to_string_lossy().into_owned();
        let nn = &name[..2];
        let t0 = Instant::now();
        let res: Result<serde_json::Value, String> = (|| {
            let bytes = std::fs::read(file).map_err(|e| e.to_string())?;
            let pres = Presentation::from_bytes(&bytes).map_err(|e| format!("open: {e}"))?;
            let t_open = t0.elapsed().as_secs_f64();
            let (mut input, stock) = pres.render_deterministic().map_err(|e| format!("render_deterministic: {e}"))?;
            let layout = if mode == "bundled" { stock } else {
                input.fonts.extend(font_files.iter().cloned());
                let mut fm = oxml_layout::font::FontManager::new_deterministic().map_err(|e| format!("fonts: {e}"))?;
                fm.load_additional_fonts(&input.fonts);
                fm.set_caller_aliases(&aliases);
                rpptx_render::layout_presentation_with_font_manager(&input, fm).map_err(|e| format!("relayout: {e}"))?
            };
            let t_layout = t0.elapsed().as_secs_f64();
            let mut pages = vec![];
            let mut svg_diags = std::collections::BTreeMap::<String, usize>::new();
            for (k, page) in layout.pages.iter().enumerate() {
                let r = svg::render_page(&layout, k).ok_or("no page")?;
                for d in &r.diagnostics { if !d.path.starts_with("layout.") { *svg_diags.entry(d.message.clone()).or_default() += 1; } }
                let (s, w, h) = common::svg_to_px(&r.svg, page.width, page.height);
                std::fs::write(format!("{out}/{nn}-page-{}.svg", k + 1), &s).map_err(|e| e.to_string())?;
                let png = common::raster(&s, &layout.fonts, w, h)?;
                std::fs::write(format!("{out}/{nn}-page-{}.png", k + 1), &png).map_err(|e| e.to_string())?;
                let (texts, paths) = common::text_stats(&s);
                pages.push(serde_json::json!({"page": k + 1, "w_pt": page.width, "h_pt": page.height, "w_px": w, "h_px": h, "svg_bytes": s.len(), "text_elems": texts, "path_elems": paths}));
            }
            let mut diags = std::collections::BTreeMap::<String, usize>::new();
            for d in &layout.diagnostics { *diags.entry(d.message.clone()).or_default() += 1; }
            for s in &input.slides { for d in &s.diagnostics { *diags.entry(format!("resolve: {}", d.message)).or_default() += 1; } }
            let fonts_used: Vec<String> = layout.fonts.iter().map(|f| format!("{}{}{}", f.family, if f.bold {" bold"} else {""}, if f.italic {" italic"} else {""})).collect();
            Ok(serde_json::json!({"open_s": t_open, "layout_s": t_layout - t_open, "page_count": layout.pages.len(), "slide_count": pres.slides().len(), "pages": pages,
                "fonts_used": fonts_used, "layout_diagnostics": diags, "svg_diagnostics": svg_diags}))
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
