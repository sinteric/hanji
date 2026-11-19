//! Spike: write PDF straight from Hanji's page display list with krilla, and
//! through the page SVG -> usvg -> svg2pdf route, for the same document.
//!
//! usage: pdf-krilla-spike <docx|pptx|hwpx> <input> <out-dir>
//!
//! Writes `<stem>.krilla.pdf` (tagged where the layout has structure, with
//! bookmarks and links), `<stem>.krilla-pdfa2a.pdf` / `-pdfa2b` / `-pdfua1`
//! when krilla's validator accepts them (the errors otherwise), and
//! `<stem>.svg2pdf.pdf`, plus `<stem>.report.json`.

use std::collections::HashMap;
use std::num::NonZeroU16;
use std::path::Path;
use std::time::Instant;

use hanji_preview::page::{
    self, FaceId, Item, Layout, Paint as HPaint, PathCommand, Role, EMBOLDEN_EM, SLANT,
};
use hanji_preview::{DocumentKind, FontOptions, PageData, PageFormat};
use krilla::color::rgb;
use krilla::configure::{Accessibility, Archival, ConfigurationBuilder, Validator};
use krilla::geom::{Path as KPath, PathBuilder, Point, Rect, Size, Transform};
use krilla::image::Image;
use krilla::metadata::Metadata;
use krilla::outline::{Outline, OutlineNode};
use krilla::tagging::{
    ArtifactType, ContentTag, Identifier, ListNumbering, Node, SpanTag, Tag, TableHeaderScope,
    TagGroup, TagKind, TagTree,
};
use krilla::action::LinkAction;
use krilla::annotation::{Annotation, LinkAnnotation, Target};
use krilla::destination::XyzDestination;
use krilla::num::NormalizedF32;
use krilla::page::PageSettings;
use krilla::paint::{
    Fill, FillRule, LineCap, LineJoin, LinearGradient, RadialGradient, SpreadMethod, Stop,
    Stroke, StrokeDash,
};
use krilla::surface::Surface;
use krilla::text::{Font, GlyphId, KrillaGlyph};
use krilla::{Document, SerializeSettings};
use serde_json::json;

#[derive(Default, Debug)]
struct Stats {
    glyph_runs: usize,
    glyphs: usize,
    runs_without_text: usize,
    runs_one_cluster: usize,
    transformed_runs: usize,
    synthetic_bold_runs: usize,
    synthetic_italic_runs: usize,
    fills: usize,
    strokes: usize,
    images: usize,
    images_failed: usize,
    groups: usize,
    shadows_dropped: usize,
    grid_paints: usize,
    links: usize,
    marked: usize,
    varied_faces: usize,
    font_failures: usize,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let kind = match args.get(1).map(String::as_str) {
        Some("docx") => DocumentKind::Docx,
        Some("pptx") => DocumentKind::Pptx,
        Some("hwpx") => DocumentKind::Hwpx,
        _ => panic!("usage: pdf-krilla-spike <docx|pptx|hwpx> <input> <out-dir>"),
    };
    let input = Path::new(&args[2]);
    let out = Path::new(&args[3]);
    std::fs::create_dir_all(out).unwrap();
    let stem = input.file_stem().unwrap().to_string_lossy().to_string();
    let bytes = std::fs::read(input).unwrap();
    // Caller fonts from SPIKE_FONT_DIR (every face of each file), as an app
    // would pass the user's fonts; bundled faces have no Hangul.
    let mut caller = Vec::new();
    if let Ok(dir) = std::env::var("SPIKE_FONT_DIR") {
        let mut paths: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        paths.sort();
        for path in paths {
            let data: std::sync::Arc<[u8]> = std::fs::read(&path).unwrap().into();
            let faces = if data.starts_with(b"ttcf") {
                u32::from_be_bytes([data[8], data[9], data[10], data[11]])
            } else {
                1
            };
            for index in 0..faces {
                let mut font = hanji_preview::FontData::new(data.clone());
                font.face_index = index;
                caller.push(font);
            }
        }
    }
    let fonts = FontOptions::from_fonts(caller);

    let t = Instant::now();
    let layout = page::layout_with_fonts(kind, &bytes, &fonts).expect("layout");
    let layout_ms = t.elapsed().as_millis();

    let mut report = json!({
        "input": input.display().to_string(),
        "pages": layout.page_count(),
        "layout_ms": layout_ms,
        "fonts_summary": layout.fonts().summary(),
        "layout_diagnostics": layout.diagnostics().len(),
    });

    // Plain krilla: tagged where structure exists, bookmarks, links.
    let variants: Vec<(&str, Option<Validator>, Option<Validator>)> = vec![
        ("krilla", None, None),
        ("krilla-pdfa2b", Some(Validator::A(Archival::A2_B)), None),
        ("krilla-pdfa2a", Some(Validator::A(Archival::A2_A)), None),
        ("krilla-pdfua1", None, Some(Validator::Ua(Accessibility::UA1))),
    ];
    for (name, a, ua) in variants {
        let t = Instant::now();
        let (result, stats) = write_krilla(&layout, &stem, a, ua);
        let ms = t.elapsed().as_millis();
        match result {
            Ok(pdf) => {
                std::fs::write(out.join(format!("{stem}.{name}.pdf")), &pdf).unwrap();
                report[name] = json!({"ok": true, "bytes": pdf.len(), "ms": ms, "stats": format!("{stats:?}")});
            }
            Err(e) => {
                report[name] = json!({"ok": false, "error": e, "ms": ms});
            }
        }
    }

    // SVG route: the preview's page SVGs, read by usvg with the subset faces
    // they embed, converted page by page by svg2pdf, joined with pdfunite.
    let t = Instant::now();
    match svg_route(kind, &bytes, &fonts, out, &stem) {
        Ok(size) => {
            report["svg2pdf"] = json!({"ok": true, "bytes": size, "ms": t.elapsed().as_millis()})
        }
        Err(e) => report["svg2pdf"] = json!({"ok": false, "error": e}),
    }

    std::fs::write(
        out.join(format!("{stem}.report.json")),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}

// ---------------------------------------------------------------- krilla

fn write_krilla(
    layout: &Layout,
    title: &str,
    archival: Option<Validator>,
    ua: Option<Validator>,
) -> (Result<Vec<u8>, String>, Stats) {
    let mut config = ConfigurationBuilder::new();
    if let Some(v) = archival {
        config = config.set_validator(v);
    }
    if let Some(v) = ua {
        config = config.set_validator(v);
    }
    let structure = layout.structure();
    let settings = SerializeSettings {
        configuration: config.finish().expect("configuration"),
        enable_tagging: true,
        ..SerializeSettings::default()
    };
    let mut document = Document::new_with(settings);
    document.set_metadata(
        Metadata::new()
            .title(title.to_string())
            .language("ko".into())
            .creator("Hanji PDF spike (krilla)".into())
            .creation_date(krilla::metadata::DateTime::new(2026).month(11).day(19)),
    );
    let interpolate = archival.is_none();

    let mut stats = Stats::default();
    let mut fonts: HashMap<FaceId, Option<Font>> = HashMap::new();
    // structure node -> its content identifiers, in reading order.
    let mut content: HashMap<u32, Vec<Identifier>> = HashMap::new();
    // structure node -> the text drawn in it, for heading titles.
    let mut node_text: HashMap<u32, String> = HashMap::new();

    for index in 0..layout.page_count() {
        let page = layout.page(index).expect("page");
        let mut kpage = document
            .start_page_with(PageSettings::new(
                Size::from_wh(page.width as f32, page.height as f32).unwrap(),
            ));
        let mut links = Vec::new();
        {
            let mut surface = kpage.surface();
            let mut painter = Painter {
                layout,
                fonts: &mut fonts,
                stats: &mut stats,
                ctm: vec![Transform::identity()],
                links: &mut links,
                content: &mut content,
                tagging: structure.is_some(),
                interpolate,
                node_text: &mut node_text,
                current_node: None,
                marked_depth: 0,
                group_pushes: Vec::new(),
            };
            painter.items(&mut surface, &page.items);
        }
        for (rect, url) in links {
            kpage.add_annotation(Annotation::new_link(
                LinkAnnotation::new(rect, Target::Action(LinkAction::new(url.clone()).into())),
                Some(url),
            ));
        }
        kpage.finish();
    }

    // Bookmarks from the engine's headings, nested by level.
    let headings = layout.headings();
    if !headings.is_empty() {
        let mut outline = Outline::new();
        let mut stack: Vec<(u32, OutlineNode)> = Vec::new();
        let flush = |stack: &mut Vec<(u32, OutlineNode)>, outline: &mut Outline, to: u32| {
            while stack.last().is_some_and(|(l, _)| *l >= to) {
                let (_, node) = stack.pop().unwrap();
                match stack.last_mut() {
                    Some((_, parent)) => parent.push_child(node),
                    None => outline.push_child(node),
                }
            }
        };
        for h in &headings {
            flush(&mut stack, &mut outline, h.level);
            let node = OutlineNode::new(
                h.title.clone(),
                XyzDestination::new(h.page, Point::from_xy(0.0, h.y as f32)),
            );
            stack.push((h.level, node));
        }
        flush(&mut stack, &mut outline, 0);
        document.set_outline(outline);
    }

    // The tag tree from the engine's structure.
    if let Some(structure) = &structure {
        let nodes: HashMap<u32, &page::StructureNode> =
            structure.nodes.iter().map(|n| (n.id, n)).collect();
        fn build(
            id: u32,
            nodes: &HashMap<u32, &page::StructureNode>,
            content: &mut HashMap<u32, Vec<Identifier>>,
            text: &HashMap<u32, String>,
        ) -> Option<TagGroup> {
            let node = nodes.get(&id)?;
            let mut children: Vec<Node> = content
                .remove(&id)
                .unwrap_or_default()
                .into_iter()
                .map(Node::from)
                .collect();
            for child in &node.children {
                if let Some(group) = build(*child, nodes, content, text) {
                    children.push(group.into());
                }
            }
            if children.is_empty() {
                return None;
            }
            let tag: TagKind = match node.role {
                Role::Document => Tag::Part.into(),
                Role::Paragraph => Tag::P.into(),
                Role::Heading(l) => {
                    Tag::Hn(
                    NonZeroU16::new(u16::from(l.max(1))).unwrap(),
                    Some(text.get(&id).cloned().unwrap_or_default().trim().to_string()),
                )
                .into()
                }
                Role::List => Tag::L(ListNumbering::None).into(),
                Role::ListItem => Tag::LI.into(),
                Role::Table => Tag::Table.into(),
                Role::TableRow => Tag::TR.into(),
                Role::TableHeaderCell => Tag::TH(TableHeaderScope::Both).into(),
                Role::TableCell => Tag::TD.into(),
                Role::Figure => Tag::Figure(node.alternate_text.clone()).into(),
                _ => Tag::P.into(),
            };
            Some(TagGroup::with_children(tag, children))
        }
        let mut tree = TagTree::new().with_lang(Some("ko".into()));
        if let Some(root) = build(structure.root, &nodes, &mut content, &node_text) {
            tree.push(root);
        }
        document.set_tag_tree(tree);
    }

    stats.varied_faces = fonts
        .keys()
        .filter(|f| layout.face_is_varied(**f))
        .count();
    let result = document.finish().map_err(|e| format!("{e:?}"));
    (result, stats)
}

struct Painter<'a> {
    layout: &'a Layout,
    fonts: &'a mut HashMap<FaceId, Option<Font>>,
    stats: &'a mut Stats,
    /// The current transform, page space, one entry per open group.
    ctm: Vec<Transform>,
    links: &'a mut Vec<(Rect, String)>,
    content: &'a mut HashMap<u32, Vec<Identifier>>,
    tagging: bool,
    marked_depth: usize,
    group_pushes: Vec<usize>,
    interpolate: bool,
    node_text: &'a mut HashMap<u32, String>,
    current_node: Option<u32>,
}

fn transform(t: page::Transform) -> Transform {
    Transform::from_row(
        t.a as f32, t.b as f32, t.c as f32, t.d as f32, t.e as f32, t.f as f32,
    )
}

fn path(p: &page::Path) -> Option<KPath> {
    let mut b = PathBuilder::new();
    for c in &p.commands {
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

fn rule(p: &page::Path) -> FillRule {
    match p.fill_rule {
        page::FillRule::NonZero => FillRule::NonZero,
        page::FillRule::EvenOdd => FillRule::EvenOdd,
    }
}

fn byte(v: f64) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn color(c: page::Color) -> rgb::Color {
    rgb::Color::new(byte(c.r), byte(c.g), byte(c.b))
}

fn norm(v: f64) -> NormalizedF32 {
    NormalizedF32::new(v.clamp(0.0, 1.0) as f32).unwrap()
}

/// A paint and its opacity. `None` for a grid paint, which the caller draws
/// cell by cell.
fn paint(p: &HPaint) -> Option<(krilla::paint::Paint, f64)> {
    let stops = |s: &[page::GradientStop]| {
        s.iter()
            .map(|s| Stop {
                offset: norm(s.offset),
                color: color(s.color).into(),
                opacity: norm(s.color.a),
            })
            .collect::<Vec<_>>()
    };
    Some(match p {
        HPaint::Solid(c) => (color(*c).into(), c.a),
        HPaint::Linear {
            start,
            end,
            stops: s,
            transform: t,
            ..
        } => (
            LinearGradient {
                x1: start.x as f32,
                y1: start.y as f32,
                x2: end.x as f32,
                y2: end.y as f32,
                transform: transform(*t),
                spread_method: SpreadMethod::Pad,
                stops: stops(s),
                anti_alias: false,
            }
            .into(),
            1.0,
        ),
        HPaint::Radial {
            center,
            radius,
            focal,
            stops: s,
            transform: t,
            ..
        } => (
            RadialGradient {
                fx: focal.x as f32,
                fy: focal.y as f32,
                fr: 0.0,
                cx: center.x as f32,
                cy: center.y as f32,
                cr: *radius as f32,
                transform: transform(*t),
                spread_method: SpreadMethod::Pad,
                stops: stops(s),
                anti_alias: false,
            }
            .into(),
            1.0,
        ),
        _ => return None,
    })
}

impl Painter<'_> {
    fn font(&mut self, face: FaceId) -> Option<Font> {
        let layout = self.layout;
        let stats = &mut *self.stats;
        self.fonts
            .entry(face)
            .or_insert_with(|| {
                let (data, index) = layout.face_data(face)?;
                let font = Font::new(data.to_vec().into(), index);
                if font.is_none() {
                    stats.font_failures += 1;
                }
                font
            })
            .clone()
    }


    fn items(&mut self, surface: &mut Surface, items: &[Item]) {
        for item in items {
            match item {
                Item::BeginMarked(node) => {
                    self.marked_depth += 1;
                    if !self.tagging || self.marked_depth > 1 {
                        continue;
                    }
                    self.stats.marked += 1;
                    match node {
                        Some(id) => {
                            let ident = surface.start_tagged(ContentTag::Span(SpanTag::empty()));
                            self.content.entry(*id).or_default().push(ident);
                            self.current_node = Some(*id);
                        }
                        None => {
                            surface.start_tagged(ContentTag::Artifact(
                                krilla::tagging::Artifact::with_kind(
                                    ArtifactType::Other,
                                ),
                            ));
                        }
                    }
                }
                Item::EndMarked => {
                    self.marked_depth = self.marked_depth.saturating_sub(1);
                    if self.tagging && self.marked_depth == 0 {
                        surface.end_tagged();
                        self.current_node = None;
                    }
                }
                Item::PushGroup(group) => {
                    self.stats.groups += 1;
                    self.stats.shadows_dropped += group.shadows.len();
                    let t = transform(group.transform);
                    let ctm = concat(*self.ctm.last().unwrap(), t);
                    self.ctm.push(ctm);
                    surface.push_transform(&t);
                    let mut pushes = 1;
                    if let Some(clip) = group.clip.as_ref().and_then(|c| path(c).map(|p| (p, rule(c)))) {
                        surface.push_clip_path(&clip.0, &clip.1);
                        pushes += 1;
                    }
                    if group.opacity < 1.0 {
                        surface.push_opacity(norm(group.opacity));
                        pushes += 1;
                    }
                    self.group_pushes.push(pushes);
                }
                Item::PopGroup => {
                    let pushes = self.group_pushes.pop().unwrap_or(0);
                    for _ in 0..pushes {
                        surface.pop();
                    }
                    self.ctm.pop();
                }
                Item::Link { rect, url } => {
                    self.stats.links += 1;
                    let ctm = *self.ctm.last().unwrap();
                    if let Some(r) = Rect::from_xywh(
                        rect.x as f32,
                        rect.y as f32,
                        rect.width as f32,
                        rect.height as f32,
                    )
                    .and_then(|r| r.transform(ctm))
                    {
                        self.links.push((r, url.clone()));
                    }
                }
                other => {
                    // With tagging on, content outside every marked region
                    // is an artifact, tagged draw by draw so marked content
                    // stays nested inside the groups.
                    let artifact = self.tagging && self.marked_depth == 0;
                    if artifact {
                        surface.start_tagged(ContentTag::Artifact(
                            krilla::tagging::Artifact::with_kind(
                                ArtifactType::Other,
                            ),
                        ));
                    }
                    self.draw(surface, other);
                    if artifact {
                        surface.end_tagged();
                    }
                }
            }
        }
    }

    fn draw(&mut self, surface: &mut Surface, item: &Item) {
        match item {
            Item::Fill { path: p, paint: pa } => {
                self.stats.fills += 1;
                let Some(kp) = path(p) else { return };
                match paint(pa) {
                    Some((paint, opacity)) => {
                        surface.set_stroke(None);
                        surface.set_fill(Some(Fill {
                            paint,
                            opacity: norm(opacity),
                            rule: rule(p),
                        }));
                        surface.draw_path(&kp);
                    }
                    None => self.grid(surface, &kp, rule(p), pa),
                }
            }
            Item::Stroke { path: p, stroke } => {
                self.stats.strokes += 1;
                let Some(kp) = path(p) else { return };
                let Some((paint, opacity)) = paint(&stroke.paint) else {
                    self.stats.grid_paints += 1;
                    return;
                };
                surface.set_fill(None);
                surface.set_stroke(Some(Stroke {
                    paint,
                    width: stroke.width as f32,
                    miter_limit: 10.0,
                    line_cap: match stroke.cap {
                        page::LineCap::Butt => LineCap::Butt,
                        page::LineCap::Round => LineCap::Round,
                        page::LineCap::Square => LineCap::Square,
                    },
                    line_join: match stroke.join {
                        page::LineJoin::Miter => LineJoin::Miter,
                        page::LineJoin::Round => LineJoin::Round,
                        page::LineJoin::Bevel => LineJoin::Bevel,
                    },
                    opacity: norm(opacity),
                    dash: stroke.dash.as_ref().map(|d| StrokeDash {
                        array: d.iter().map(|v| *v as f32).collect(),
                        offset: 0.0,
                    }),
                }));
                surface.draw_path(&kp);
                surface.set_stroke(None);
            }
            Item::Image { rect, image } => {
                self.stats.images += 1;
                let data: krilla::Data = image.bytes.to_vec().into();
                let decoded = match image.codec {
                    page::Codec::Png => Image::from_png(data, self.interpolate),
                    page::Codec::Jpeg => Image::from_jpeg(data, self.interpolate),
                    _ => Err("codec".into()),
                };
                let Ok(img) = decoded else {
                    self.stats.images_failed += 1;
                    return;
                };
                let Some(size) = Size::from_wh(rect.width as f32, rect.height as f32) else {
                    return;
                };
                surface.push_transform(&Transform::from_translate(rect.x as f32, rect.y as f32));
                surface.draw_image(img, size);
                surface.pop();
            }
            Item::Glyphs(run) => self.glyphs(surface, run),
            _ => {}
        }
    }

    /// A path-gradient grid, drawn as its cells clipped by the path.
    fn grid(&mut self, surface: &mut Surface, kp: &KPath, r: FillRule, pa: &HPaint) {
        self.stats.grid_paints += 1;
        let HPaint::Grid {
            bounds,
            columns,
            rows,
            colors,
        } = pa
        else {
            return;
        };
        surface.push_clip_path(kp, &r);
        let (cw, ch) = (
            bounds.width / f64::from(*columns),
            bounds.height / f64::from(*rows),
        );
        for row in 0..*rows {
            for col in 0..*columns {
                let c = colors[usize::from(row) * usize::from(*columns) + usize::from(col)];
                let mut b = PathBuilder::new();
                if let Some(rect) = Rect::from_xywh(
                    (bounds.x + f64::from(col) * cw) as f32,
                    (bounds.y + f64::from(row) * ch) as f32,
                    cw as f32 + 0.01,
                    ch as f32 + 0.01,
                ) {
                    b.push_rect(rect);
                }
                if let Some(cell) = b.finish() {
                    surface.set_fill(Some(Fill {
                        paint: color(c).into(),
                        opacity: norm(c.a),
                        rule: FillRule::NonZero,
                    }));
                    surface.draw_path(&cell);
                }
            }
        }
        surface.pop();
    }

    fn glyphs(&mut self, surface: &mut Surface, run: &page::Glyphs) {
        if run.glyphs.is_empty() {
            return;
        }
        self.stats.glyph_runs += 1;
        if let Some(node) = self.current_node {
            self.node_text.entry(node).or_default().push_str(&run.text);
        }
        self.stats.glyphs += run.glyphs.len();
        let Some(font) = self.font(run.face) else {
            return;
        };
        let size = run.size as f32;
        if size <= 0.0 {
            return;
        }
        // The outline map: glyph positions are page points, `matrix` applies
        // to outlines only, so positions are mapped back through it.
        let m = run.matrix;
        let o = run.origin;
        let identity = [m.a - 1.0, m.b, m.c, m.d - 1.0, m.e, m.f]
            .iter()
            .all(|v| v.abs() < 1e-6);
        let about = concat(
            concat(
                Transform::from_translate(o.x as f32, o.y as f32),
                transform(m),
            ),
            Transform::from_translate(-o.x as f32, -o.y as f32),
        );
        let mut outer = Transform::identity();
        if run.slant {
            self.stats.synthetic_italic_runs += 1;
            // x' = x - SLANT * (y - origin.y)
            outer = Transform::from_row(
                1.0,
                0.0,
                -SLANT as f32,
                1.0,
                (SLANT * o.y) as f32,
                0.0,
            );
        }
        let to_local = if identity {
            Transform::identity()
        } else {
            self.stats.transformed_runs += 1;
            about.invert().unwrap_or_default()
        };
        let pts: Vec<(f32, f32)> = run
            .glyphs
            .iter()
            .map(|g| {
                let mut p = tiny_point(g.x as f32, g.y as f32);
                p = apply(to_local, p);
                p
            })
            .collect();

        // Text ranges: one character per glyph when the counts agree;
        // otherwise the whole run is one cluster, so copy and search still
        // see its text.
        let text = &run.text;
        let chars: Vec<(usize, char)> = text.char_indices().collect();
        let one_to_one = chars.len() == run.glyphs.len();
        if text.is_empty() {
            self.stats.runs_without_text += 1;
        } else if !one_to_one {
            self.stats.runs_one_cluster += 1;
        }
        let (x0, y0) = pts[0];
        let mut kglyphs = Vec::with_capacity(pts.len());
        for (i, g) in run.glyphs.iter().enumerate() {
            let (x, y) = pts[i];
            let next_x = pts.get(i + 1).map(|p| p.0);
            let advance = match next_x {
                Some(nx) => (nx - x) / size,
                None => 0.0,
            };
            let range = if one_to_one {
                let start = chars[i].0;
                let end = chars.get(i + 1).map(|c| c.0).unwrap_or(text.len());
                start..end
            } else {
                0..text.len()
            };
            kglyphs.push(KrillaGlyph::new(
                GlyphId::new(u32::from(g.id)),
                advance,
                0.0,
                // krilla's y offset is y up; page y is down.
                -(y - y0) / size,
                0.0,
                range,
                None,
            ));
        }
        let c = run.color;
        surface.set_fill(Some(Fill {
            paint: color(c).into(),
            opacity: norm(c.a),
            rule: FillRule::NonZero,
        }));
        let stroke_width = match (run.stroke, run.embolden) {
            (Some(w), _) => Some(w),
            (None, true) => Some(EMBOLDEN_EM * run.size),
            (None, false) => None,
        };
        if let Some(width) = stroke_width {
            self.stats.synthetic_bold_runs += 1;
            // Outline units are em and the run is drawn at `size`, but a
            // transformed run's stroke is scaled by its matrix; close enough
            // for a spike.
            surface.set_stroke(Some(Stroke {
                paint: color(c).into(),
                width: width as f32,
                line_join: LineJoin::Round,
                line_cap: LineCap::Round,
                opacity: norm(c.a),
                ..Stroke::default()
            }));
        } else {
            surface.set_stroke(None);
        }
        let mut pushed = 0;
        if run.slant {
            surface.push_transform(&outer);
            pushed += 1;
        }
        if !identity {
            surface.push_transform(&about);
            pushed += 1;
        }
        surface.draw_glyphs(
            Point::from_xy(x0, y0),
            &kglyphs,
            font,
            text,
            size,
            false,
        );
        for _ in 0..pushed {
            surface.pop();
        }
        surface.set_stroke(None);
    }
}

/// `a` after `b`: a point goes through `b`, then `a`.
fn concat(a: Transform, b: Transform) -> Transform {
    Transform::from_row(
        a.sx() * b.sx() + a.kx() * b.ky(),
        a.ky() * b.sx() + a.sy() * b.ky(),
        a.sx() * b.kx() + a.kx() * b.sy(),
        a.ky() * b.kx() + a.sy() * b.sy(),
        a.sx() * b.tx() + a.kx() * b.ty() + a.tx(),
        a.ky() * b.tx() + a.sy() * b.ty() + a.ty(),
    )
}

fn tiny_point(x: f32, y: f32) -> (f32, f32) {
    (x, y)
}

fn apply(t: Transform, (x, y): (f32, f32)) -> (f32, f32) {
    (
        t.sx() * x + t.kx() * y + t.tx(),
        t.ky() * x + t.sy() * y + t.ty(),
    )
}

// ---------------------------------------------------------------- SVG route

fn svg_route(
    kind: DocumentKind,
    bytes: &[u8],
    fonts: &FontOptions,
    out: &Path,
    stem: &str,
) -> Result<usize, String> {
    use base64::Engine as _;
    let preview = hanji_preview::render_document_with_fonts(kind, bytes, fonts)?;
    let dir = out.join(format!("{stem}.svgpages"));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut files = Vec::new();
    for index in 0..preview.page_count() {
        let rendered = preview.render_page(index, PageFormat::Svg)?;
        let PageData::Svg(svg) = rendered.data else {
            return Err("not svg".into());
        };
        std::fs::write(dir.join(format!("p{index:03}.svg")), &svg).map_err(|e| e.to_string())?;
        // The preview's own PNG, the reference for the visual comparison.
        if let PageData::Png(png) = preview
            .render_page(index, PageFormat::Png { dpi: 96.0 })?
            .data
        {
            std::fs::write(dir.join(format!("p{index:03}.preview.png")), png)
                .map_err(|e| e.to_string())?;
        }
        // The page embeds its faces as @font-face data URLs; usvg reads no
        // CSS fonts, so they go to its font database by family name.
        let mut options = usvg::Options::default();
        let mut exact = HashMap::new();
        for block in svg.split("@font-face").skip(1) {
            let block = &block[..block.find('}').unwrap_or(block.len())];
            let family = block
                .split("font-family")
                .nth(1)
                .and_then(|s| s.split(['"', '\'']).nth(1))
                .map(str::to_string);
            let data = block
                .split("base64,")
                .nth(1)
                .and_then(|s| s.split([')', '"', '\'']).next());
            if let (Some(family), Some(data)) = (family, data) {
                if let Ok(font) = base64::engine::general_purpose::STANDARD.decode(data) {
                    let ids = options
                        .fontdb_mut()
                        .load_font_source(usvg::fontdb::Source::Binary(std::sync::Arc::new(font)));
                    if let Some(id) = ids.first() {
                        exact.insert(family, *id);
                    }
                }
            }
        }
        options.font_resolver = usvg::FontResolver {
            select_font: Box::new(move |font, _| {
                font.families()
                    .iter()
                    .find_map(|f| exact.get(f.to_string().trim_matches('"')).copied())
            }),
            select_fallback: Box::new(|_, _, _| None),
        };
        let tree = usvg::Tree::from_str(&svg, &options).map_err(|e| format!("usvg: {e}"))?;
        let pdf = svg2pdf::to_pdf(
            &tree,
            svg2pdf::ConversionOptions::default(),
            svg2pdf::PageOptions { dpi: 96.0 },
        )
        .map_err(|e| format!("svg2pdf p{index}: {e:?}"))?;
        let file = dir.join(format!("p{index:03}.pdf"));
        std::fs::write(&file, pdf).map_err(|e| e.to_string())?;
        files.push(file);
    }
    let target = out.join(format!("{stem}.svg2pdf.pdf"));
    let status = std::process::Command::new("pdfunite")
        .args(&files)
        .arg(&target)
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("pdfunite failed".into());
    }
    Ok(std::fs::metadata(&target).map_err(|e| e.to_string())?.len() as usize)
}
