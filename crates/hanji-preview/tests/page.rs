//! The display list (`hanji_preview::page`): what it draws matches the PNG
//! preview of the same layout, glyph for glyph. What this cannot see: a loss
//! the PNG preview shares, and anything this file's rasterizer does not draw
//! (shadows, grid paints).

use hanji_core::engine::{Engine, ImportOptions};
use hanji_preview::page::{
    self, Codec, FillRule, Item, Layout, LineCap, LineJoin, Paint, Path, PathCommand,
};
use hanji_preview::{render_document_with_fonts, DocumentKind, FontOptions, PageData, PageFormat};
use resvg::tiny_skia;

const SHAPES: &[u8] = include_bytes!("../../hanji-pptx/corpus/shapes.pptx");
const ENGLISH: &[u8] =
    include_bytes!("../../../prototype/preview/baseline/08-docx-new-report-en.docx");
const KOREAN: &[u8] =
    include_bytes!("../../../prototype/preview/baseline/01-docx-untouched-korean-report.docx");
const HWPX: &[u8] =
    include_bytes!("../../../prototype/preview/baseline/25-hwpx-untouched-hcar-001.hwpx");

#[test]
fn every_page_starts_white_and_its_glyphs_have_outlines() {
    let layout =
        page::layout_with_fonts(DocumentKind::Pptx, SHAPES, &FontOptions::default()).unwrap();
    assert_eq!(layout.page_count(), 6);
    let mut glyphs = 0;
    let mut drawn = 0;
    for k in 0..layout.page_count() {
        let page = layout.page(k).unwrap();
        let info = layout.page_info(k).unwrap();
        assert_eq!((page.width, page.height), (info.width, info.height));
        assert!(
            matches!(&page.items[0], Item::Fill { paint: Paint::Solid(c), .. }
            if (c.r, c.g, c.b, c.a) == (1.0, 1.0, 1.0, 1.0))
        );
        let mut depth = 0_i32;
        for item in &page.items {
            match item {
                Item::Glyphs(run) => {
                    assert!(run.size > 0.0 && !run.text.is_empty());
                    for g in &run.glyphs {
                        assert!(g.x.is_finite() && g.y.is_finite());
                        glyphs += 1;
                        drawn += usize::from(layout.outline(run.face, g.id).is_some());
                    }
                }
                Item::PushGroup(_) => depth += 1,
                Item::PopGroup => depth -= 1,
                _ => {}
            }
            assert!(depth >= 0);
        }
        assert_eq!(depth, 0, "groups close on page {k}");
    }
    // Spaces have no outline; everything else does.
    assert!(
        glyphs > 0 && drawn * 10 >= glyphs * 8,
        "{drawn} of {glyphs}"
    );
}

#[test]
fn a_revision_is_laid_out_through_its_export() {
    let engine = hanji_pptx::PptxEngine;
    let imported = engine.import(SHAPES, &ImportOptions::default()).unwrap();
    let layout = page::layout_revision(
        &engine,
        &imported.text,
        &imported.remainder,
        &FontOptions::default(),
    )
    .unwrap();
    let exported = engine.export(&imported.text, &imported.remainder).unwrap();
    let direct =
        page::layout_with_fonts(DocumentKind::Pptx, &exported, &FontOptions::default()).unwrap();
    assert_eq!(layout.page_count(), direct.page_count());
    for k in 0..layout.page_count() {
        assert_eq!(layout.page(k).unwrap(), direct.page(k).unwrap());
    }
}

#[test]
fn hwpx_glyphs_come_from_hanjis_faces() {
    let layout =
        page::layout_with_fonts(DocumentKind::Hwpx, HWPX, &FontOptions::default()).unwrap();
    let mut glyphs = 0;
    let mut drawn = 0;
    for k in 0..layout.page_count() {
        let page = layout.page(k).unwrap();
        let info = layout.page_info(k).unwrap();
        assert!((page.width - info.width).abs() < 0.01, "page {k} width");
        assert!((page.height - info.height).abs() < 0.01, "page {k} height");
        for item in &page.items {
            if let Item::Glyphs(run) = item {
                for g in &run.glyphs {
                    glyphs += 1;
                    drawn += usize::from(layout.outline(run.face, g.id).is_some());
                }
            }
        }
    }
    assert!(
        glyphs > 0 && drawn * 10 >= glyphs * 8,
        "{drawn} of {glyphs}"
    );
}

/// Mean absolute channel difference (0–255) between the display list drawn
/// here and the SVG preview's PNG, over the pixels either one inks (not
/// white), page by page: white margins would hide a missing line of text.
fn compare(name: &str, kind: DocumentKind, bytes: &[u8], pages: usize, limit: f64) {
    let fonts = fonts();
    let layout = page::layout_with_fonts(kind, bytes, &fonts).unwrap();
    let preview = render_document_with_fonts(kind, bytes, &fonts).unwrap();
    assert_eq!(layout.page_count(), preview.page_count());
    for k in 0..layout.page_count().min(pages) {
        let PageData::Png(png) = preview
            .render_page(k, PageFormat::Png { dpi: 96.0 })
            .unwrap()
            .data
        else {
            panic!("PNG")
        };
        let svg = tiny_skia::Pixmap::decode_png(&png).unwrap();
        let ours = draw(
            &layout,
            k,
            svg.width(),
            svg.height(),
            kind == DocumentKind::Hwpx,
        );
        let (mut total, mut inked) = (0_u64, 0_u64);
        for (a, b) in svg.data().chunks(4).zip(ours.data().chunks(4)) {
            if a.iter().chain(b).all(|v| *v == 255) {
                continue;
            }
            inked += 1;
            total += a
                .iter()
                .zip(b)
                .map(|(x, y)| u64::from(x.abs_diff(*y)))
                .sum::<u64>();
        }
        assert!(inked > 0, "{kind:?} page {k} draws nothing");
        if let Ok(dir) = std::env::var("HANJI_PAGE_DUMP") {
            std::fs::write(format!("{dir}/{name}-{k}-svg.png"), &png).unwrap();
            ours.save_png(format!("{dir}/{name}-{k}-list.png")).unwrap();
        }
        let mean = total as f64 / (inked * 4) as f64;
        eprintln!("{kind:?} page {k}: {inked} inked pixels, mean difference {mean:.2}");
        assert!(
            mean <= limit,
            "{kind:?} page {k}: mean difference {mean:.2} > {limit}"
        );
    }
}

/// The bundled faces have no Hangul, so Korean text draws as boxes unless
/// `HANJI_PAGE_FONTS` names a directory of font files to add.
fn fonts() -> FontOptions {
    let mut fonts = FontOptions::default();
    let Ok(dir) = std::env::var("HANJI_PAGE_FONTS") else {
        return fonts;
    };
    for entry in std::fs::read_dir(dir).unwrap() {
        let data = std::fs::read(entry.unwrap().path()).unwrap();
        fonts.fonts.push(hanji_preview::FontData::new(data));
    }
    fonts
}

#[test]
fn slides_draw_like_the_svg_preview() {
    compare("shapes", DocumentKind::Pptx, SHAPES, usize::MAX, 1.0);
}

#[test]
fn document_pages_draw_like_the_svg_preview() {
    compare("english", DocumentKind::Docx, ENGLISH, usize::MAX, 2.0);
    compare("korean", DocumentKind::Docx, KOREAN, usize::MAX, 2.0);
}

#[test]
fn hwpx_pages_draw_like_the_png_preview() {
    compare("hwpx", DocumentKind::Hwpx, HWPX, usize::MAX, 2.0);
}

/// Every PPTX, DOCX and HWPX of the preview baseline, first four pages.
/// Slower; run with `--ignored` when the lowering changes. The bound is loose
/// because the two outputs differ on purpose, and the measured differences
/// (2026-10-25, up to 7.6 on 13-pptx page 2; HWPX at most 1.5) were each one
/// of these: this file's
/// rasterizer draws no shadows; the SVG redraws a run whose glyphs are not one
/// per character from its text, losing ligatures the engine shaped (Lato's
/// "ti"), where the list draws the shaped glyphs; and usvg draws no synthetic
/// bold or italic, which the list carries as the engine asked.
#[test]
#[ignore]
fn the_baseline_draws_like_the_svg_preview() {
    let dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../prototype/preview/baseline"
    );
    let mut names: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    names.sort();
    for path in names {
        let kind = match path.extension().and_then(|e| e.to_str()) {
            Some("pptx") => DocumentKind::Pptx,
            Some("docx") => DocumentKind::Docx,
            Some("hwpx") => DocumentKind::Hwpx,
            _ => continue,
        };
        eprintln!("{}", path.display());
        let name = path.file_stem().unwrap().to_str().unwrap();
        compare(name, kind, &std::fs::read(&path).unwrap(), 4, 10.0);
    }
}

// A small rasterizer of the display list, enough to hold it to the SVG
// preview: solid and gradient paints, glyphs, PNG images, groups with clips
// and opacity. Shadows and grid paints are not drawn.

fn draw(layout: &Layout, index: usize, width: u32, height: u32, exact: bool) -> tiny_skia::Pixmap {
    let page = layout.page(index).unwrap();
    // The engines' PNG stretches the page to its rounded pixel size; HWPX's
    // (resvg) rounds the canvas but draws at 96 dpi exactly.
    let (sx, sy) = if exact {
        (96.0 / 72.0, 96.0 / 72.0)
    } else {
        (
            width as f32 / page.width as f32,
            height as f32 / page.height as f32,
        )
    };
    // White under the page, as the PNG is: a rounded-up canvas edge is
    // outside the page's own white.
    let mut canvas = tiny_skia::Pixmap::new(width, height).unwrap();
    canvas.fill(tiny_skia::Color::WHITE);
    let mut layers = vec![Layer {
        pixmap: Some(canvas),
        transform: tiny_skia::Transform::from_scale(sx, sy),
        mask: None,
        opacity: 1.0,
    }];
    for item in &page.items {
        let top = layers.last().unwrap();
        let (transform, mask) = (top.transform, top.mask.clone());
        let mask = mask.as_deref();
        let target = layers
            .iter_mut()
            .rev()
            .find_map(|l| l.pixmap.as_mut())
            .unwrap();
        match item {
            Item::Fill { path, paint } => {
                if let (Some(p), Some(paint)) = (skia_path(path), skia_paint(paint)) {
                    target.fill_path(&p, &paint, fill_rule(path.fill_rule), transform, mask);
                }
            }
            Item::Stroke { path, stroke } => {
                if let (Some(p), Some(paint)) = (skia_path(path), skia_paint(&stroke.paint)) {
                    let mut s = tiny_skia::Stroke {
                        width: stroke.width as f32,
                        line_cap: match stroke.cap {
                            LineCap::Butt => tiny_skia::LineCap::Butt,
                            LineCap::Round => tiny_skia::LineCap::Round,
                            LineCap::Square => tiny_skia::LineCap::Square,
                        },
                        line_join: match stroke.join {
                            LineJoin::Miter => tiny_skia::LineJoin::Miter,
                            LineJoin::Round => tiny_skia::LineJoin::Round,
                            LineJoin::Bevel => tiny_skia::LineJoin::Bevel,
                        },
                        ..Default::default()
                    };
                    if let Some(dash) = &stroke.dash {
                        s.dash = tiny_skia::StrokeDash::new(
                            dash.iter().map(|v| *v as f32).collect(),
                            0.0,
                        );
                    }
                    target.stroke_path(&p, &paint, &s, transform, mask);
                }
            }
            Item::Glyphs(run) => {
                let c = run.color;
                let mut paint = tiny_skia::Paint::default();
                paint.set_color(
                    tiny_skia::Color::from_rgba(c.r as f32, c.g as f32, c.b as f32, c.a as f32)
                        .unwrap(),
                );
                paint.anti_alias = true;
                for g in &run.glyphs {
                    let Some(outline) = layout.outline(run.face, g.id).and_then(|o| skia_path(&o))
                    else {
                        continue;
                    };
                    let mut t = transform;
                    if run.slant {
                        t = t
                            .pre_translate(run.origin.x as f32, run.origin.y as f32)
                            .pre_concat(tiny_skia::Transform::from_row(
                                1.0,
                                0.0,
                                -page::SLANT as f32,
                                1.0,
                                0.0,
                                0.0,
                            ))
                            .pre_translate(
                                (g.x - run.origin.x) as f32,
                                (g.y - run.origin.y) as f32,
                            );
                    } else {
                        t = t.pre_translate(g.x as f32, g.y as f32);
                    }
                    let m = run.matrix;
                    let em = tiny_skia::Transform::from_scale(run.size as f32, run.size as f32)
                        .pre_concat(tiny_skia::Transform::from_row(
                            m.a as f32, m.b as f32, m.c as f32, m.d as f32, 0.0, 0.0,
                        ));
                    target.fill_path(
                        &outline,
                        &paint,
                        tiny_skia::FillRule::Winding,
                        t.pre_concat(em),
                        mask,
                    );
                    if run.embolden {
                        let s = tiny_skia::Stroke {
                            width: (page::EMBOLDEN_EM * run.size) as f32,
                            line_cap: tiny_skia::LineCap::Round,
                            line_join: tiny_skia::LineJoin::Round,
                            ..Default::default()
                        };
                        if let Some(o) = outline.transform(em) {
                            target.stroke_path(&o, &paint, &s, t, mask);
                        }
                    }
                }
            }
            Item::Image { rect, image } => {
                let Some(img) = decode(image) else {
                    continue;
                };
                let t = transform
                    .pre_translate(rect.x as f32, rect.y as f32)
                    .pre_scale(
                        rect.width as f32 / img.width() as f32,
                        rect.height as f32 / img.height() as f32,
                    );
                let pattern = tiny_skia::Pattern::new(
                    img.as_ref(),
                    tiny_skia::SpreadMode::Pad,
                    tiny_skia::FilterQuality::Bilinear,
                    1.0,
                    tiny_skia::Transform::identity(),
                );
                let paint = tiny_skia::Paint {
                    shader: pattern,
                    ..Default::default()
                };
                let r =
                    tiny_skia::Rect::from_xywh(0.0, 0.0, img.width() as f32, img.height() as f32)
                        .unwrap();
                target.fill_rect(r, &paint, t, mask);
            }
            Item::PushGroup(group) => {
                let transform = transform.pre_concat(skia_transform(group.transform));
                let mut mask = layers.last().unwrap().mask.clone();
                if let Some(clip) = group.clip.as_ref().and_then(skia_path) {
                    let rule = fill_rule(group.clip.as_ref().unwrap().fill_rule);
                    let m = match mask {
                        Some(m) => {
                            let mut m = (*m).clone();
                            m.intersect_path(&clip, rule, true, transform);
                            m
                        }
                        None => {
                            let mut m = tiny_skia::Mask::new(width, height).unwrap();
                            m.fill_path(&clip, rule, true, transform);
                            m
                        }
                    };
                    mask = Some(std::rc::Rc::new(m));
                }
                // Only a group that fades needs a layer of its own.
                layers.push(Layer {
                    pixmap: (group.opacity < 1.0)
                        .then(|| tiny_skia::Pixmap::new(width, height).unwrap()),
                    transform,
                    mask,
                    opacity: group.opacity as f32,
                });
            }
            Item::PopGroup => {
                let layer = layers.pop().unwrap();
                let (Some(pixmap), opacity) = (layer.pixmap, layer.opacity) else {
                    continue;
                };
                let below = layers
                    .iter_mut()
                    .rev()
                    .find_map(|l| l.pixmap.as_mut())
                    .unwrap();
                below.draw_pixmap(
                    0,
                    0,
                    pixmap.as_ref(),
                    &tiny_skia::PixmapPaint {
                        opacity,
                        ..Default::default()
                    },
                    tiny_skia::Transform::identity(),
                    None,
                );
            }
            _ => {}
        }
    }
    layers.pop().unwrap().pixmap.unwrap()
}

/// PNG directly; JPEG through resvg, stretched to a square that the image's
/// rectangle stretches back.
fn decode(image: &page::Image) -> Option<tiny_skia::Pixmap> {
    use base64::Engine as _;
    if image.codec == Codec::Png {
        return tiny_skia::Pixmap::decode_png(&image.bytes).ok();
    }
    let side = 1024;
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{side}\" height=\"{side}\"><image width=\"{side}\" height=\"{side}\" preserveAspectRatio=\"none\" href=\"data:image/jpeg;base64,{}\"/></svg>",
        base64::engine::general_purpose::STANDARD.encode(&image.bytes)
    );
    let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).ok()?;
    let mut pixmap = tiny_skia::Pixmap::new(side, side)?;
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    Some(pixmap)
}

/// A group's state; `pixmap` is its own only when it fades, and otherwise
/// it draws into the nearest layer below that has one.
struct Layer {
    pixmap: Option<tiny_skia::Pixmap>,
    transform: tiny_skia::Transform,
    mask: Option<std::rc::Rc<tiny_skia::Mask>>,
    opacity: f32,
}

fn fill_rule(rule: FillRule) -> tiny_skia::FillRule {
    match rule {
        FillRule::NonZero => tiny_skia::FillRule::Winding,
        FillRule::EvenOdd => tiny_skia::FillRule::EvenOdd,
    }
}

fn skia_path(path: &Path) -> Option<tiny_skia::Path> {
    let mut b = tiny_skia::PathBuilder::new();
    for c in &path.commands {
        match *c {
            PathCommand::MoveTo(p) => b.move_to(p.x as f32, p.y as f32),
            PathCommand::LineTo(p) => b.line_to(p.x as f32, p.y as f32),
            PathCommand::QuadTo { control, to } => {
                b.quad_to(control.x as f32, control.y as f32, to.x as f32, to.y as f32)
            }
            PathCommand::CurveTo { c1, c2, to } => b.cubic_to(
                c1.x as f32,
                c1.y as f32,
                c2.x as f32,
                c2.y as f32,
                to.x as f32,
                to.y as f32,
            ),
            PathCommand::Close => b.close(),
        }
    }
    b.finish()
}

fn skia_transform(t: page::Transform) -> tiny_skia::Transform {
    tiny_skia::Transform::from_row(
        t.a as f32, t.b as f32, t.c as f32, t.d as f32, t.e as f32, t.f as f32,
    )
}

fn skia_color(c: page::Color) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba(c.r as f32, c.g as f32, c.b as f32, c.a as f32).unwrap()
}

fn skia_paint(paint: &Paint) -> Option<tiny_skia::Paint<'static>> {
    let mut out = tiny_skia::Paint {
        anti_alias: true,
        ..Default::default()
    };
    let stops = |stops: &[page::GradientStop]| {
        stops
            .iter()
            .map(|s| tiny_skia::GradientStop::new(s.offset as f32, skia_color(s.color)))
            .collect::<Vec<_>>()
    };
    match paint {
        Paint::Solid(c) => out.set_color(skia_color(*c)),
        Paint::Linear {
            start,
            end,
            stops: s,
            transform,
            ..
        } => {
            out.shader = tiny_skia::LinearGradient::new(
                tiny_skia::Point::from_xy(start.x as f32, start.y as f32),
                tiny_skia::Point::from_xy(end.x as f32, end.y as f32),
                stops(s),
                tiny_skia::SpreadMode::Pad,
                skia_transform(*transform),
            )?
        }
        Paint::Radial {
            center,
            radius,
            focal,
            stops: s,
            transform,
            ..
        } => {
            out.shader = tiny_skia::RadialGradient::new(
                tiny_skia::Point::from_xy(focal.x as f32, focal.y as f32),
                0.0,
                tiny_skia::Point::from_xy(center.x as f32, center.y as f32),
                *radius as f32,
                stops(s),
                tiny_skia::SpreadMode::Pad,
                skia_transform(*transform),
            )?
        }
        Paint::Grid { colors, .. } => out.set_color(skia_color(*colors.first()?)),
        _ => return None,
    }
    Some(out)
}
