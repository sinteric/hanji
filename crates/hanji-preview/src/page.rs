//! Laid-out pages as a display list (DESIGN.md §7, "Pages as a display list").
//!
//! A [`Layout`] is one document laid out by its engine, with hanji's fonts,
//! and [`Layout::page`] answers one page as [`Item`]s a caller draws in order:
//! glyphs, filled and stroked paths, images, and groups that transform, clip,
//! fade or shadow what they hold. The types are hanji's own, so a caller never
//! links an engine and an engine can be replaced behind them.
//!
//! Glyphs are glyph ids, not text to shape again: [`Layout::outline`] gives
//! each glyph's outline, so the page draws exactly the glyphs the engine
//! placed, in the face hanji chose. The text of each run comes with it for
//! search and selection.
//!
//! What a page cannot carry: a link (a viewer of a preview follows none), a
//! tile paint (the layout carries no bytes for it), and an image that is
//! neither PNG nor JPEG after GIF normalisation. Each is left out with a
//! diagnostic, as the SVG output does. HWPX is not laid out here yet: rhwp's
//! page layer tree is the planned source.

use std::collections::HashMap;
use std::sync::Arc;

use oxml_layout::{FontId, LayoutResult, PositionedElement};

use crate::{images, Diagnostic, DocumentKind, FontOptions, FontsReport, PageInfo, Preview};

/// A face of the layout, named by [`Glyphs::face`] and [`Layout::outline`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FaceId(pub u32);

/// A point in page space: points, y down, origin at the page's top left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Straight (not premultiplied) RGBA, each channel in 0..=1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

/// An affine map: x' = a·x + c·y + e, y' = b·x + d·y + f.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathCommand {
    MoveTo(Point),
    LineTo(Point),
    QuadTo { control: Point, to: Point },
    CurveTo { c1: Point, c2: Point, to: Point },
    Close,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    pub commands: Vec<PathCommand>,
    pub fill_rule: FillRule,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientStop {
    pub offset: f64,
    pub color: Color,
}

/// How a fill or a stroke is painted. Gradient coordinates are in the space
/// of the path they paint.
#[derive(Clone, Debug, PartialEq)]
pub enum Paint {
    Solid(Color),
    /// `extend` says whether the end colours continue past the start and the
    /// end; where they do not, nothing is painted there.
    Linear {
        start: Point,
        end: Point,
        stops: Vec<GradientStop>,
        extend: (bool, bool),
    },
    Radial {
        center: Point,
        radius: f64,
        focal: Point,
        stops: Vec<GradientStop>,
        extend: (bool, bool),
    },
    /// A grid of `columns` × `rows` colour cells, row by row, filling
    /// `bounds` and repeated beyond it (a path gradient's approximation).
    Grid {
        bounds: Rect,
        columns: u16,
        rows: u16,
        colors: Vec<Color>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineCap {
    Butt,
    Round,
    Square,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stroke {
    pub paint: Paint,
    pub width: f64,
    pub cap: LineCap,
    pub join: LineJoin,
    /// Alternating dash and gap lengths in points, starting with a dash.
    pub dash: Option<Vec<f64>>,
}

/// A glyph placed on the page: `x`, `y` is its baseline origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glyph {
    pub id: u16,
    pub x: f64,
    pub y: f64,
}

/// A run of glyphs in one face, size and colour.
#[derive(Clone, Debug, PartialEq)]
pub struct Glyphs {
    pub face: FaceId,
    /// The em size in points: an outline from [`Layout::outline`] is scaled
    /// by it.
    pub size: f64,
    pub color: Color,
    /// The baseline origin of the run, which `slant` shears about.
    pub origin: Point,
    pub glyphs: Vec<Glyph>,
    /// The run's text in logical order, for search and selection. It need not
    /// have one character per glyph.
    pub text: String,
    /// The face has no bold of its own: also stroke each outline, centred,
    /// [`EMBOLDEN_EM`] of the size wide, with round joins and caps.
    pub embolden: bool,
    /// The face has no italic of its own: shear the run about `origin`,
    /// x' = x − [`SLANT`] · (y − origin.y).
    pub slant: bool,
}

/// Stroke width of a synthetic bold, in em (oxml-layout's rasterizer's).
pub const EMBOLDEN_EM: f64 = 1.0 / 24.0;
/// Shear of a synthetic italic: tan 12°, as oxml-layout's rasterizer draws it.
pub const SLANT: f64 = 0.212_556_56;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Codec {
    Png,
    Jpeg,
}

/// Encoded image bytes, drawn stretched to fill their rectangle.
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub codec: Codec,
    pub bytes: Arc<[u8]>,
}

/// A drop shadow of a group's content: its alpha, blurred by a Gaussian of
/// standard deviation `blur / 2`, offset by `dx`, `dy` and filled with
/// `color`, all in the group's own space, drawn under the content.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub dx: f64,
    pub dy: f64,
    pub blur: f64,
    pub color: Color,
}

/// What a group does to the items until its [`Item::PopGroup`]. The clip is
/// in the group's own space, after `transform`.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub transform: Transform,
    pub clip: Option<Path>,
    pub opacity: f64,
    pub shadows: Vec<Shadow>,
}

/// One drawing step. Coordinates are in the space of the innermost open
/// group, and page space outside every group.
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Glyphs(Glyphs),
    Fill { path: Path, paint: Paint },
    Stroke { path: Path, stroke: Stroke },
    Image { rect: Rect, image: Image },
    PushGroup(Group),
    PopGroup,
}

/// One page: its size in points and its items in drawing order. The first
/// item paints the page white.
#[derive(Clone, Debug, PartialEq)]
pub struct Page {
    pub width: f64,
    pub height: f64,
    pub items: Vec<Item>,
}

/// One document laid out, held until dropped. Pages are made on demand.
pub struct Layout {
    preview: Preview,
    kind: DocumentKind,
    faces: HashMap<FontId, Face>,
}

struct Face {
    data: Arc<[u8]>,
    index: u32,
    variations: Vec<([u8; 4], f32)>,
}

/// Lay out a PPTX or DOCX package with caller fonts, package fonts and the
/// bundled fallbacks, as [`crate::render_document_with_fonts`] does. No host
/// fonts are searched.
pub fn layout_with_fonts(
    kind: DocumentKind,
    package: &[u8],
    fonts: &FontOptions,
) -> Result<Layout, String> {
    layout_with_fonts_and_limits(kind, package, fonts, images::ImageLimits::default())
}

/// [`layout_with_fonts`] with explicit embedded-image budgets.
pub fn layout_with_fonts_and_limits(
    kind: DocumentKind,
    package: &[u8],
    fonts: &FontOptions,
    limits: images::ImageLimits,
) -> Result<Layout, String> {
    let preview = match kind {
        DocumentKind::Pptx => crate::layout_pptx(package, fonts, limits)?,
        DocumentKind::Docx => crate::docx::layout(package, fonts, limits)?,
        DocumentKind::Hwpx => {
            return Err("HWPX is not laid out as a display list yet; use its SVG preview".into())
        }
    };
    Ok(Layout::new(preview, kind))
}

/// Lay out one revision: `engine` exports `text` and `remainder` to their home
/// format, which is laid out as [`layout_with_fonts`] does. The package is the
/// one export writes (DESIGN.md §2 rule 4) and stays inside this call.
pub fn layout_revision(
    engine: &dyn hanji_core::engine::Engine,
    text: &str,
    remainder: &hanji_core::remainder::Remainder,
    fonts: &FontOptions,
) -> Result<Layout, String> {
    let kind = match engine.format() {
        "pptx" => DocumentKind::Pptx,
        "docx" => DocumentKind::Docx,
        "hwpx" => DocumentKind::Hwpx,
        other => return Err(format!("{other} has no paged layout")),
    };
    let package = engine.export(text, remainder).map_err(|e| e.to_string())?;
    layout_with_fonts(kind, &package, fonts)
}

impl Layout {
    fn new(preview: Preview, kind: DocumentKind) -> Self {
        let faces = preview
            .layout
            .fonts
            .iter()
            .map(|f| {
                (
                    f.id,
                    Face {
                        data: f.data.clone(),
                        index: f.face_index,
                        variations: f.variations.clone(),
                    },
                )
            })
            .collect();
        Self {
            preview,
            kind,
            faces,
        }
    }

    pub fn page_count(&self) -> usize {
        self.preview.page_count()
    }

    pub fn page_info(&self, index: usize) -> Option<PageInfo> {
        self.preview.page_info(index)
    }

    /// Page `index` (0-based) as items. Its diagnostics are already in
    /// [`Layout::diagnostics`], made when the layout was.
    pub fn page(&self, index: usize) -> Result<Page, String> {
        let mut lowering =
            Lowering::new(&self.preview.layout, &self.preview.image_plan, index, true);
        lowering
            .page()
            .ok_or_else(|| format!("page index {index} is out of range"))
    }

    /// Glyph `glyph` of `face` as a path in em: 1.0 is the run's size, y down,
    /// origin at the glyph's baseline origin. `None` for a face the layout
    /// does not have, or a glyph with no outline (a space).
    pub fn outline(&self, face: FaceId, glyph: u16) -> Option<Path> {
        let face = self.faces.get(&FontId(face.0))?;
        let mut parsed = ttf_parser::Face::parse(&face.data, face.index).ok()?;
        for (tag, value) in &face.variations {
            parsed.set_variation(ttf_parser::Tag::from_bytes(tag), *value);
        }
        let scale = 1.0 / f64::from(parsed.units_per_em());
        let mut builder = OutlineBuilder {
            scale,
            commands: Vec::new(),
        };
        parsed.outline_glyph(ttf_parser::GlyphId(glyph), &mut builder)?;
        Some(Path {
            commands: builder.commands,
            fill_rule: FillRule::NonZero,
        })
    }

    pub fn fonts(&self) -> &FontsReport {
        &self.preview.fonts
    }

    pub fn warnings(&self) -> &[String] {
        &self.preview.warnings
    }

    /// The layout's own diagnostics and every page's, with zero-based
    /// `pages[i]` paths, as [`crate::Preview::diagnostics`] has them.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.preview.diagnostics
    }

    pub fn quality(&self) -> crate::quality::QualityReport {
        self.preview.quality(self.kind)
    }
}

/// Page `index`'s diagnostics for the display list, without copying images.
pub(crate) fn page_diagnostics(
    layout: &LayoutResult,
    plan: &images::ImagePlan,
    index: usize,
) -> Option<Vec<Diagnostic>> {
    let mut lowering = Lowering::new(layout, plan, index, false);
    lowering.page()?;
    Some(lowering.diagnostics)
}

struct OutlineBuilder {
    scale: f64,
    commands: Vec<PathCommand>,
}

impl OutlineBuilder {
    fn point(&self, x: f32, y: f32) -> Point {
        Point {
            x: f64::from(x) * self.scale,
            y: -f64::from(y) * self.scale,
        }
    }
}

impl ttf_parser::OutlineBuilder for OutlineBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        let p = self.point(x, y);
        self.commands.push(PathCommand::MoveTo(p));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.point(x, y);
        self.commands.push(PathCommand::LineTo(p));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (control, to) = (self.point(x1, y1), self.point(x, y));
        self.commands.push(PathCommand::QuadTo { control, to });
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (c1, c2, to) = (self.point(x1, y1), self.point(x2, y2), self.point(x, y));
        self.commands.push(PathCommand::CurveTo { c1, c2, to });
    }

    fn close(&mut self) {
        self.commands.push(PathCommand::Close);
    }
}

/// One page's lowering. Element paths match the SVG lowering's
/// (`pages[k].elements[i].children[j]`), which the image plan is keyed by.
struct Lowering<'a> {
    layout: &'a LayoutResult,
    plan: &'a images::ImagePlan,
    index: usize,
    /// False: diagnostics only, and image bytes are not copied.
    output: bool,
    items: Vec<Item>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Lowering<'a> {
    fn new(
        layout: &'a LayoutResult,
        plan: &'a images::ImagePlan,
        index: usize,
        output: bool,
    ) -> Self {
        Self {
            layout,
            plan,
            index,
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

    fn page(&mut self) -> Option<Page> {
        let frame = self.layout.pages.get(self.index)?.clone();
        let bounds = Rect {
            x: 0.0,
            y: 0.0,
            width: frame.width,
            height: frame.height,
        };
        self.items.push(Item::Fill {
            path: rect_path(bounds),
            paint: Paint::Solid(Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            }),
        });
        if let Some(background) = &frame.background {
            let path = format!("pages[{}].background", self.index);
            if let Some(paint) = self.paint(background, &path) {
                self.items.push(Item::Fill {
                    path: rect_path(bounds),
                    paint,
                });
            }
        }
        self.elements(&frame.elements, &format!("pages[{}].elements", self.index));
        Some(Page {
            width: frame.width,
            height: frame.height,
            items: std::mem::take(&mut self.items),
        })
    }

    fn elements(&mut self, elements: &[PositionedElement], path: &str) {
        for (index, element) in elements.iter().enumerate() {
            let path = format!("{path}[{index}]");
            match element {
                PositionedElement::Text(run) => self.text(run, None, &path),
                PositionedElement::MultilingualText(run) => {
                    if !run.is_valid() {
                        self.diagnose(&path, "invalid multilingual glyph positioning was omitted");
                        continue;
                    }
                    let offsets = (&run.x_offsets[..], &run.y_offsets[..], &run.y_advances[..]);
                    self.text(&run.legacy_projection(), Some(offsets), &path);
                }
                PositionedElement::Line {
                    start,
                    end,
                    width,
                    color,
                    dash_pattern,
                } => self.items.push(Item::Stroke {
                    path: Path {
                        commands: vec![
                            PathCommand::MoveTo(point(*start)),
                            PathCommand::LineTo(point(*end)),
                        ],
                        fill_rule: FillRule::NonZero,
                    },
                    stroke: Stroke {
                        paint: Paint::Solid(color_of(*color)),
                        width: *width,
                        cap: LineCap::Butt,
                        join: LineJoin::Miter,
                        dash: dash_pattern.map(|(on, off)| vec![on, off]),
                    },
                }),
                PositionedElement::FilledRect { rect, color } => self.items.push(Item::Fill {
                    path: rect_path(rect_of(*rect)),
                    paint: Paint::Solid(color_of(*color)),
                }),
                PositionedElement::Image { rect, data, .. } => self.image(*rect, data, &path),
                PositionedElement::LinkAnnotation { .. } => {
                    self.diagnose(&path, "link was omitted: a page follows no links")
                }
                PositionedElement::Path(element) => {
                    let shape = path_of(&element.path);
                    if let Some(fill) = &element.fill {
                        if let Some(paint) = self.paint(fill, &format!("{path}.fill")) {
                            self.items.push(Item::Fill {
                                path: shape.clone(),
                                paint,
                            });
                        }
                    }
                    if let Some(stroke) = &element.stroke {
                        if let Some(paint) = self.paint(&stroke.paint, &format!("{path}.stroke")) {
                            self.items.push(Item::Stroke {
                                path: shape,
                                stroke: Stroke {
                                    paint,
                                    width: stroke.width,
                                    cap: match stroke.cap {
                                        oxml_layout::LineCap::Butt => LineCap::Butt,
                                        oxml_layout::LineCap::Round => LineCap::Round,
                                        oxml_layout::LineCap::Square => LineCap::Square,
                                    },
                                    join: match stroke.join {
                                        oxml_layout::LineJoin::Miter => LineJoin::Miter,
                                        oxml_layout::LineJoin::Round => LineJoin::Round,
                                        oxml_layout::LineJoin::Bevel => LineJoin::Bevel,
                                    },
                                    dash: stroke.dash.clone(),
                                },
                            });
                        }
                    }
                }
                PositionedElement::Group(group) => {
                    let mut shadows = Vec::new();
                    for (index, effect) in group.effects.iter().enumerate() {
                        match effect {
                            oxml_layout::Effect::OuterShadow {
                                dx,
                                dy,
                                blur,
                                color,
                            } => shadows.push(Shadow {
                                dx: *dx,
                                dy: *dy,
                                blur: blur.max(0.0),
                                color: color_of(*color),
                            }),
                            #[allow(unreachable_patterns)]
                            _ => self.diagnose(
                                &format!("{path}.effects[{index}]"),
                                "unsupported group effect was omitted while its children were preserved",
                            ),
                        }
                    }
                    let t = group.transform;
                    self.items.push(Item::PushGroup(Group {
                        transform: Transform {
                            a: t.a,
                            b: t.b,
                            c: t.c,
                            d: t.d,
                            e: t.e,
                            f: t.f,
                        },
                        clip: group.clip.as_ref().map(path_of),
                        opacity: group.opacity.clamp(0.0, 1.0),
                        shadows,
                    }));
                    self.elements(&group.children, &format!("{path}.children"));
                    self.items.push(Item::PopGroup);
                }
                PositionedElement::MarkedContent { children, .. } => {
                    self.elements(children, &format!("{path}.children"))
                }
                #[allow(unreachable_patterns)]
                _ => self.diagnose(&path, "unsupported positioned element was omitted"),
            }
        }
    }

    /// A run, placed as oxml-layout's rasterizer places it: x advances along
    /// the run, `x_offsets` and `y_offsets` (y up) shift single glyphs, and
    /// `y_advances` (y up) move the pen.
    fn text(
        &mut self,
        run: &oxml_layout::GlyphRun,
        offsets: Option<(&[f64], &[f64], &[f64])>,
        path: &str,
    ) {
        if run.glyph_ids.is_empty() {
            return;
        }
        let Some(face) = self.layout.fonts.iter().find(|f| f.id == run.font_id) else {
            self.diagnose(
                path,
                "text references font data that is absent from the layout result",
            );
            return;
        };
        let (synthetic_bold, synthetic_italic) = (face.synthetic_bold, face.synthetic_italic);
        if run.advances.len() != run.glyph_ids.len() {
            self.diagnose(
                path,
                "glyphs without an advance of their own share the run's last position",
            );
        }
        let (x_offsets, y_offsets, y_advances) = offsets.unwrap_or((&[], &[], &[]));
        let (mut x, mut y) = (run.origin.x, run.origin.y);
        let mut glyphs = Vec::with_capacity(run.glyph_ids.len());
        for (i, id) in run.glyph_ids.iter().enumerate() {
            glyphs.push(Glyph {
                id: *id,
                x: x + x_offsets.get(i).copied().unwrap_or(0.0),
                y: y - y_offsets.get(i).copied().unwrap_or(0.0),
            });
            x += run.advances.get(i).copied().unwrap_or(0.0);
            y -= y_advances.get(i).copied().unwrap_or(0.0);
        }
        if !self.output {
            return;
        }
        self.items.push(Item::Glyphs(Glyphs {
            face: FaceId(run.font_id.0),
            size: run.font_size,
            color: color_of(run.color),
            origin: point(run.origin),
            glyphs,
            text: run.text.clone(),
            embolden: synthetic_bold,
            slant: synthetic_italic,
        }));
    }

    fn image(&mut self, rect: oxml_layout::Rect, data: &[u8], path: &str) {
        let plan = self.plan;
        if let Some(message) = plan.omission(path) {
            let message = message.to_owned();
            self.diagnose(path, &message);
            return;
        }
        if let Some(message) = plan.approximation(path) {
            self.diagnose(path, message);
        }
        let data = plan.normalized(path).unwrap_or(data);
        let codec = if data.starts_with(b"\x89PNG\r\n\x1a\n") {
            Codec::Png
        } else if data.starts_with(&[0xff, 0xd8, 0xff]) {
            Codec::Jpeg
        } else {
            self.diagnose(
                path,
                "image bytes are neither PNG nor JPEG and were omitted",
            );
            return;
        };
        if !self.output {
            return;
        }
        self.items.push(Item::Image {
            rect: rect_of(rect),
            image: Image {
                codec,
                bytes: Arc::from(data),
            },
        });
    }

    fn paint(&mut self, paint: &oxml_layout::Paint, path: &str) -> Option<Paint> {
        let stops = |stops: &[oxml_layout::GradientStop]| {
            stops
                .iter()
                .map(|s| GradientStop {
                    offset: if s.offset.is_nan() {
                        0.0
                    } else {
                        s.offset.clamp(0.0, 1.0)
                    },
                    color: color_of(s.color),
                })
                .collect()
        };
        Some(match paint {
            oxml_layout::Paint::Solid(color) => Paint::Solid(color_of(*color)),
            oxml_layout::Paint::Linear {
                start,
                end,
                stops: s,
                extend,
            } => Paint::Linear {
                start: point(*start),
                end: point(*end),
                stops: stops(s),
                extend: *extend,
            },
            oxml_layout::Paint::Radial {
                center,
                radius,
                focal,
                stops: s,
                extend,
            } => Paint::Radial {
                center: point(*center),
                radius: *radius,
                focal: point(*focal),
                stops: stops(s),
                extend: *extend,
            },
            oxml_layout::Paint::Grid {
                bounds,
                columns,
                rows,
                colors,
            } => {
                oxml_layout::Paint::grid(*bounds, *columns, *rows, colors.clone())?;
                Paint::Grid {
                    bounds: rect_of(*bounds),
                    columns: *columns,
                    rows: *rows,
                    colors: colors.iter().copied().map(color_of).collect(),
                }
            }
            oxml_layout::Paint::Tile { .. } => {
                self.diagnose(
                    path,
                    "tile paint was omitted because its carrier has no media bytes",
                );
                return None;
            }
            #[allow(unreachable_patterns)]
            _ => {
                self.diagnose(path, "unsupported paint was omitted");
                return None;
            }
        })
    }
}

fn point(p: oxml_layout::Point) -> Point {
    Point { x: p.x, y: p.y }
}

fn rect_of(r: oxml_layout::Rect) -> Rect {
    Rect {
        x: r.x,
        y: r.y,
        width: r.width,
        height: r.height,
    }
}

fn color_of(c: oxml_layout::Color) -> Color {
    let channel = |v: f64| if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) };
    Color {
        r: channel(c.r),
        g: channel(c.g),
        b: channel(c.b),
        a: channel(c.a),
    }
}

fn rect_path(r: Rect) -> Path {
    let (x0, y0, x1, y1) = (r.x, r.y, r.x + r.width, r.y + r.height);
    Path {
        commands: vec![
            PathCommand::MoveTo(Point { x: x0, y: y0 }),
            PathCommand::LineTo(Point { x: x1, y: y0 }),
            PathCommand::LineTo(Point { x: x1, y: y1 }),
            PathCommand::LineTo(Point { x: x0, y: y1 }),
            PathCommand::Close,
        ],
        fill_rule: FillRule::NonZero,
    }
}

fn path_of(path: &oxml_layout::Path) -> Path {
    Path {
        commands: path
            .commands
            .iter()
            .map(|c| match c {
                oxml_layout::PathCommand::MoveTo(p) => PathCommand::MoveTo(point(*p)),
                oxml_layout::PathCommand::LineTo(p) => PathCommand::LineTo(point(*p)),
                oxml_layout::PathCommand::CurveTo { c1, c2, to } => PathCommand::CurveTo {
                    c1: point(*c1),
                    c2: point(*c2),
                    to: point(*to),
                },
                oxml_layout::PathCommand::Close => PathCommand::Close,
            })
            .collect(),
        fill_rule: match path.fill_rule {
            oxml_layout::FillRule::NonZero => FillRule::NonZero,
            oxml_layout::FillRule::EvenOdd => FillRule::EvenOdd,
        },
    }
}
