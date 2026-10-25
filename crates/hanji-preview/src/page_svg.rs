//! An SVG page, as usvg reads it, lowered to display-list items: how HWPX
//! reaches [`crate::page`]. rhwp draws a page as SVG, hanji gives it fonts,
//! and usvg resolves it the way hanji's PNG preview draws it (resvg), so the
//! list shows the same picture: text as glyphs usvg laid out, in the faces
//! hanji chose.
//!
//! What it leaves out, each with a diagnostic: masks, filters, pattern paints,
//! GIF/WebP/nested-SVG images, a nested clip path and a clip made of more than
//! paths. A gradient's reflect or repeat spread is drawn padded, and a radial
//! gradient's focal radius as zero.

use std::collections::HashMap;
use std::sync::Arc;

use resvg::usvg;

use crate::page::{
    Codec, Color, FillRule, Glyph, Glyphs, GradientStop, Group, Image, Item, LineCap, LineJoin,
    Paint, Path, PathCommand, Point, Rect, Stroke, Transform,
};
use crate::Diagnostic;

/// usvg's font ids mapped to the face numbers that [`Glyphs::face`] carries
/// and each face's units per em.
pub(crate) type Faces = HashMap<usvg::fontdb::ID, (usize, u16)>;

/// Lowers one page.
pub(crate) struct SvgLowering<'a> {
    faces: &'a Faces,
    page: usize,
    /// False: diagnostics only, and image bytes are not copied.
    output: bool,
    pub(crate) items: Vec<Item>,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

impl<'a> SvgLowering<'a> {
    pub(crate) fn new(faces: &'a Faces, page: usize, output: bool) -> Self {
        Self {
            faces,
            page,
            output,
            items: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn diagnose(&mut self, path: &str, message: &str) {
        self.diagnostics.push(Diagnostic {
            path: path.to_owned(),
            message: message.to_owned(),
        });
    }

    /// The tree's root, in its own units, under `transform` (CSS pixels to
    /// points for an rhwp page).
    pub(crate) fn tree(&mut self, tree: &usvg::Tree, transform: Transform) {
        self.items.push(Item::PushGroup(Group {
            transform,
            clip: None,
            opacity: 1.0,
            shadows: Vec::new(),
        }));
        self.group_children(tree.root(), &format!("pages[{}].svg", self.page));
        self.items.push(Item::PopGroup);
    }

    fn group_children(&mut self, group: &usvg::Group, path: &str) {
        for (index, node) in group.children().iter().enumerate() {
            let path = format!("{path}[{index}]");
            match node {
                usvg::Node::Group(g) => self.group(g, &path),
                usvg::Node::Path(p) => self.path(p, &path),
                usvg::Node::Image(i) => self.image(i, &path),
                usvg::Node::Text(t) => self.text(t, &path),
            }
        }
    }

    fn group(&mut self, group: &usvg::Group, path: &str) {
        if group.mask().is_some() {
            self.diagnose(path, "mask was omitted; the masked content is drawn whole");
        }
        if !group.filters().is_empty() {
            self.diagnose(path, "filter was omitted; the content is drawn unfiltered");
        }
        let clip = group.clip_path().and_then(|clip| self.clip(clip, path));
        let opacity = f64::from(group.opacity().get());
        let transform = transform_of(group.transform());
        let plain = clip.is_none() && opacity >= 1.0 && transform == Transform::IDENTITY;
        if !plain {
            self.items.push(Item::PushGroup(Group {
                transform,
                clip,
                opacity,
                shadows: Vec::new(),
            }));
        }
        self.group_children(group, &format!("{path}.children"));
        if !plain {
            self.items.push(Item::PopGroup);
        }
    }

    /// A clip path made of paths, as one path in the group's space.
    fn clip(&mut self, clip: &usvg::ClipPath, path: &str) -> Option<Path> {
        if clip.clip_path().is_some() {
            self.diagnose(
                path,
                "nested clip path was omitted; the outer clip still applies",
            );
        }
        let outer = clip.transform();
        let mut commands = Vec::new();
        let mut rules = Vec::new();
        let mut complete = true;
        collect_clip(clip.root(), outer, &mut commands, &mut rules, &mut complete);
        if !complete {
            self.diagnose(path, "clip path holds more than paths; only its paths clip");
        }
        let fill_rule = match rules.as_slice() {
            [rule] => *rule,
            _ => {
                if rules.contains(&FillRule::EvenOdd) {
                    self.diagnose(
                        path,
                        "clip paths with differing rules were joined as non-zero",
                    );
                }
                FillRule::NonZero
            }
        };
        Some(Path {
            commands,
            fill_rule,
        })
    }

    fn path(&mut self, node: &usvg::Path, path: &str) {
        if !node.is_visible() {
            return;
        }
        let shape = path_of(node.data(), usvg::Transform::identity());
        let fill = node.fill().and_then(|fill| {
            let rule = match fill.rule() {
                usvg::FillRule::NonZero => FillRule::NonZero,
                usvg::FillRule::EvenOdd => FillRule::EvenOdd,
            };
            let paint = self.paint(fill.paint(), fill.opacity().get(), &format!("{path}.fill"))?;
            Some(Item::Fill {
                path: Path {
                    commands: shape.commands.clone(),
                    fill_rule: rule,
                },
                paint,
            })
        });
        let stroke = node.stroke().and_then(|stroke| {
            let paint = self.paint(
                stroke.paint(),
                stroke.opacity().get(),
                &format!("{path}.stroke"),
            )?;
            Some(Item::Stroke {
                path: shape.clone(),
                stroke: Stroke {
                    paint,
                    width: f64::from(stroke.width().get()),
                    cap: match stroke.linecap() {
                        usvg::LineCap::Butt => LineCap::Butt,
                        usvg::LineCap::Round => LineCap::Round,
                        usvg::LineCap::Square => LineCap::Square,
                    },
                    join: match stroke.linejoin() {
                        usvg::LineJoin::Miter | usvg::LineJoin::MiterClip => LineJoin::Miter,
                        usvg::LineJoin::Round => LineJoin::Round,
                        usvg::LineJoin::Bevel => LineJoin::Bevel,
                    },
                    dash: stroke
                        .dasharray()
                        .map(|dash| dash.iter().map(|v| f64::from(*v)).collect()),
                },
            })
        });
        let (first, second) = match node.paint_order() {
            usvg::PaintOrder::FillAndStroke => (fill, stroke),
            usvg::PaintOrder::StrokeAndFill => (stroke, fill),
        };
        self.items.extend(first);
        self.items.extend(second);
    }

    fn image(&mut self, node: &usvg::Image, path: &str) {
        if !node.is_visible() {
            return;
        }
        let (codec, bytes) = match node.kind() {
            usvg::ImageKind::PNG(bytes) => (Codec::Png, bytes),
            usvg::ImageKind::JPEG(bytes) => (Codec::Jpeg, bytes),
            _ => {
                self.diagnose(path, "image that is neither PNG nor JPEG was omitted");
                return;
            }
        };
        if !self.output {
            return;
        }
        let size = node.size();
        self.items.push(Item::Image {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: f64::from(size.width()),
                height: f64::from(size.height()),
            },
            image: Image {
                codec,
                bytes: Arc::from(bytes.as_slice()),
            },
        });
    }

    /// Text as usvg laid it out: one run per span and linear map, with the
    /// span's decorations as paths. A span painted other than in one colour,
    /// or stroked, is drawn from usvg's flattened outlines instead.
    fn text(&mut self, node: &usvg::Text, path: &str) {
        let solid = node.layouted().iter().all(|span| {
            span.stroke.is_none()
                && span
                    .fill
                    .as_ref()
                    .is_none_or(|f| matches!(f.paint(), usvg::Paint::Color(_)))
        });
        if !solid {
            self.group(node.flattened(), &format!("{path}.flattened"));
            return;
        }
        for (index, span) in node.layouted().iter().enumerate() {
            let path = format!("{path}.spans[{index}]");
            if !span.visible {
                continue;
            }
            let Some(fill) = &span.fill else { continue };
            let usvg::Paint::Color(c) = fill.paint() else {
                continue;
            };
            let color = Color {
                r: f64::from(c.red) / 255.0,
                g: f64::from(c.green) / 255.0,
                b: f64::from(c.blue) / 255.0,
                a: f64::from(fill.opacity().get()),
            };
            for decoration in [&span.underline, &span.overline].into_iter().flatten() {
                self.path(decoration, &format!("{path}.decoration"));
            }
            let size = f64::from(span.font_size.get());
            let mut run: Option<Glyphs> = None;
            for glyph in &span.positioned_glyphs {
                let Some(&(face, units)) = self.faces.get(&glyph.font) else {
                    self.diagnose(&path, "glyph from a face hanji did not supply was omitted");
                    continue;
                };
                // An sfnt numbers glyphs in 16 bits; usvg's ids are wider.
                let Ok(id) = u16::try_from(glyph.id.0) else {
                    self.diagnose(&path, "glyph id beyond 16 bits was omitted");
                    continue;
                };
                let t = glyph.outline_transform();
                let units = f64::from(units);
                // Outline space is em, y down; usvg's is font units, y up.
                let matrix = Transform {
                    a: f64::from(t.sx) * units / size,
                    b: f64::from(t.ky) * units / size,
                    c: -f64::from(t.kx) * units / size,
                    d: -f64::from(t.sy) * units / size,
                    e: 0.0,
                    f: 0.0,
                };
                let at = Glyph {
                    id,
                    x: f64::from(t.tx),
                    y: f64::from(t.ty),
                };
                match &mut run {
                    Some(r)
                        if r.face.0 as usize == face
                            && close(&r.matrix, &matrix)
                            && (r.size - size).abs() < 1e-9 =>
                    {
                        r.glyphs.push(at);
                        r.text.push_str(&glyph.text);
                    }
                    _ => {
                        if let Some(done) = run.take() {
                            self.push_glyphs(done);
                        }
                        run = Some(Glyphs {
                            face: crate::page::FaceId(face as u32),
                            size,
                            color,
                            origin: Point { x: at.x, y: at.y },
                            matrix,
                            glyphs: vec![at],
                            text: glyph.text.clone(),
                            embolden: false,
                            slant: false,
                        });
                    }
                }
            }
            if let Some(done) = run.take() {
                self.push_glyphs(done);
            }
            if let Some(line) = &span.line_through {
                self.path(line, &format!("{path}.decoration"));
            }
        }
    }

    fn push_glyphs(&mut self, glyphs: Glyphs) {
        if self.output {
            self.items.push(Item::Glyphs(glyphs));
        }
    }

    fn paint(&mut self, paint: &usvg::Paint, opacity: f32, path: &str) -> Option<Paint> {
        let opacity = f64::from(opacity);
        let stops = |stops: &[usvg::Stop]| -> Vec<GradientStop> {
            stops
                .iter()
                .map(|s| GradientStop {
                    offset: f64::from(s.offset().get()),
                    color: Color {
                        r: f64::from(s.color().red) / 255.0,
                        g: f64::from(s.color().green) / 255.0,
                        b: f64::from(s.color().blue) / 255.0,
                        a: f64::from(s.opacity().get()) * opacity,
                    },
                })
                .collect()
        };
        match paint {
            usvg::Paint::Color(c) => Some(Paint::Solid(Color {
                r: f64::from(c.red) / 255.0,
                g: f64::from(c.green) / 255.0,
                b: f64::from(c.blue) / 255.0,
                a: opacity,
            })),
            usvg::Paint::LinearGradient(g) => {
                self.spread(g.spread_method(), path);
                Some(Paint::Linear {
                    start: Point {
                        x: f64::from(g.x1()),
                        y: f64::from(g.y1()),
                    },
                    end: Point {
                        x: f64::from(g.x2()),
                        y: f64::from(g.y2()),
                    },
                    stops: stops(g.stops()),
                    extend: (true, true),
                    transform: transform_of(g.transform()),
                })
            }
            usvg::Paint::RadialGradient(g) => {
                self.spread(g.spread_method(), path);
                if g.fr().get() > 0.0 {
                    self.diagnose(path, "radial gradient focal radius was drawn as zero");
                }
                Some(Paint::Radial {
                    center: Point {
                        x: f64::from(g.cx()),
                        y: f64::from(g.cy()),
                    },
                    radius: f64::from(g.r().get()),
                    focal: Point {
                        x: f64::from(g.fx()),
                        y: f64::from(g.fy()),
                    },
                    stops: stops(g.stops()),
                    extend: (true, true),
                    transform: transform_of(g.transform()),
                })
            }
            usvg::Paint::Pattern(_) => {
                self.diagnose(path, "pattern paint was omitted");
                None
            }
        }
    }

    fn spread(&mut self, spread: usvg::SpreadMethod, path: &str) {
        if spread != usvg::SpreadMethod::Pad {
            self.diagnose(path, "reflected or repeated gradient was drawn padded");
        }
    }
}

fn close(a: &Transform, b: &Transform) -> bool {
    [(a.a, b.a), (a.b, b.b), (a.c, b.c), (a.d, b.d)]
        .iter()
        .all(|(x, y)| (x - y).abs() < 1e-9)
}

fn transform_of(t: usvg::Transform) -> Transform {
    Transform {
        a: f64::from(t.sx),
        b: f64::from(t.ky),
        c: f64::from(t.kx),
        d: f64::from(t.sy),
        e: f64::from(t.tx),
        f: f64::from(t.ty),
    }
}

fn collect_clip(
    group: &usvg::Group,
    transform: usvg::Transform,
    commands: &mut Vec<PathCommand>,
    rules: &mut Vec<FillRule>,
    complete: &mut bool,
) {
    let transform = transform.pre_concat(group.transform());
    if group.clip_path().is_some() || group.mask().is_some() || !group.filters().is_empty() {
        *complete = false;
    }
    for node in group.children() {
        match node {
            usvg::Node::Path(p) => {
                commands.extend(path_of(p.data(), transform).commands);
                rules.push(match p.fill().map(|f| f.rule()) {
                    Some(usvg::FillRule::EvenOdd) => FillRule::EvenOdd,
                    _ => FillRule::NonZero,
                });
            }
            usvg::Node::Group(g) => collect_clip(g, transform, commands, rules, complete),
            usvg::Node::Text(t) => {
                collect_clip(t.flattened(), transform, commands, rules, complete)
            }
            usvg::Node::Image(_) => *complete = false,
        }
    }
}

fn path_of(data: &usvg::tiny_skia_path::Path, t: usvg::Transform) -> Path {
    use usvg::tiny_skia_path::PathSegment;
    let p = |mut point: usvg::tiny_skia_path::Point| {
        t.map_point(&mut point);
        Point {
            x: f64::from(point.x),
            y: f64::from(point.y),
        }
    };
    Path {
        commands: data
            .segments()
            .map(|segment| match segment {
                PathSegment::MoveTo(a) => PathCommand::MoveTo(p(a)),
                PathSegment::LineTo(a) => PathCommand::LineTo(p(a)),
                PathSegment::QuadTo(c, a) => PathCommand::QuadTo {
                    control: p(c),
                    to: p(a),
                },
                PathSegment::CubicTo(c1, c2, a) => PathCommand::CurveTo {
                    c1: p(c1),
                    c2: p(c2),
                    to: p(a),
                },
                PathSegment::Close => PathCommand::Close,
            })
            .collect(),
        fill_rule: FillRule::NonZero,
    }
}
