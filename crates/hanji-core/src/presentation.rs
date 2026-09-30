//! The Presentation model (DESIGN.md §5.3): a presentation's text resolved
//! to one flat list of blocks, a [`Head`] for each slide, slot, shape,
//! object, line and group followed by the blocks it holds (an object: its
//! placeholder; a line or group: none). Each head's geometry is its
//! [`Place`]. Re-anchoring (design C, exact spans) then treats slides and
//! shapes as blocks like any other: remainder entries anchored to a slide or
//! a shape sit on its head.

use hanji_format::{self as fmt, Diagnostic, Names};

use crate::model::{self, Block, BlockSrc, Capabilities, Head, Para, Place, SrcKind, StyleSet};

/// Key of every slide head.
pub const SLIDE: &str = "slide";

/// Key of every object head: which object is its placeholder's id.
pub const OBJECT: &str = "object";

pub fn slot_key(name: &str) -> String {
    format!("slot:{name}")
}

pub fn shape_key(id: &str) -> String {
    format!("shape:{id}")
}

pub fn line_key(id: &str) -> String {
    format!("line:{id}")
}

pub fn group_key(id: &str) -> String {
    format!("group:{id}")
}

pub fn picture_key(id: &str) -> String {
    format!("picture:{id}")
}

/// What a head stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeadKind<'a> {
    Slide {
        layout: &'a str,
    },
    Slot {
        name: &'a str,
    },
    Shape {
        id: &'a str,
        name: &'a str,
    },
    /// A slide object (rule 8): the head is followed by its `Block::Keep`.
    Object,
    /// A line or connector; an empty id is a new one.
    Line {
        id: &'a str,
        name: &'a str,
    },
    /// A group, its objects in its place.
    Group {
        id: &'a str,
        name: &'a str,
    },
    /// A picture, its image, crop, mask and alternative text in its place;
    /// an empty id is a new one.
    Picture {
        id: &'a str,
        name: &'a str,
    },
}

pub fn kind(h: &Head) -> HeadKind<'_> {
    if let Some(name) = h.key.strip_prefix("slot:") {
        HeadKind::Slot { name }
    } else if let Some(id) = h.key.strip_prefix("shape:") {
        HeadKind::Shape { id, name: &h.label }
    } else if h.key == OBJECT {
        HeadKind::Object
    } else if let Some(id) = h.key.strip_prefix("line:") {
        HeadKind::Line { id, name: &h.label }
    } else if let Some(id) = h.key.strip_prefix("group:") {
        HeadKind::Group { id, name: &h.label }
    } else if let Some(id) = h.key.strip_prefix("picture:") {
        HeadKind::Picture { id, name: &h.label }
    } else {
        HeadKind::Slide { layout: &h.label }
    }
}

pub fn slide_head(layout: &str) -> Block {
    Block::Head(Head { level: 0, key: SLIDE.into(), label: layout.into(), place: None })
}

pub fn slot_head(name: &str, geom: Option<fmt::Geom>) -> Block {
    Block::Head(Head { level: 1, key: slot_key(name), label: String::new(), place: geom.map(Place::Box) })
}

pub fn shape_head(id: &str, name: &str, geom: Option<fmt::Geom>) -> Block {
    Block::Head(Head { level: 1, key: shape_key(id), label: name.into(), place: geom.map(Place::Box) })
}

pub fn object_head(geom: Option<fmt::Geom>) -> Block {
    Block::Head(Head { level: 1, key: OBJECT.into(), label: String::new(), place: geom.map(Place::Box) })
}

pub fn line_head(id: &str, name: &str, ends: fmt::Ends) -> Block {
    Block::Head(Head { level: 1, key: line_key(id), label: name.into(), place: Some(Place::Line(ends)) })
}

pub fn group_head(g: &fmt::GroupItem) -> Block {
    Block::Head(Head { level: 1, key: group_key(&g.id), label: g.name.clone(), place: Some(Place::Group(g.clone())) })
}

pub fn picture_head(p: &fmt::PictureItem) -> Block {
    Block::Head(Head {
        level: 1,
        key: picture_key(&p.id),
        label: p.name.clone(),
        place: Some(Place::Picture(p.clone())),
    })
}

/// A head's box, when it has one.
pub fn geom_of(h: &Head) -> Option<fmt::Geom> {
    match &h.place {
        Some(Place::Box(g)) => Some(*g),
        Some(Place::Group(g)) => g.geom,
        Some(Place::Picture(p)) => p.geom,
        _ => None,
    }
}

/// A presentation's paragraphs have no style: formatting comes from the layout.
fn no_styles() -> StyleSet {
    StyleSet::default()
}

fn head_src(h: &fmt::HeadMap) -> BlockSrc {
    BlockSrc { start: h.start, end: h.end, kind: SrcKind::Para(fmt::ParaMap { units: vec![], mark: h.mark }) }
}

/// Parse and check a presentation's text against `names`, and resolve it:
/// its blocks and where each is in the text.
pub fn model_of(
    text: &str,
    names: &Names,
    caps: Capabilities,
    is_block_keep: &dyn Fn(&str) -> Option<bool>,
) -> Result<(Vec<Block>, Vec<BlockSrc>), Vec<Diagnostic>> {
    let parsed = fmt::parse_presentation(text, names)?;
    resolve(&parsed, text, caps, is_block_keep)
}

/// Parsed presentation → flat blocks and their sources.
pub fn resolve(
    parsed: &fmt::ParsedPresentation,
    text: &str,
    caps: Capabilities,
    is_block_keep: &dyn Fn(&str) -> Option<bool>,
) -> Result<(Vec<Block>, Vec<BlockSrc>), Vec<Diagnostic>> {
    let styles = no_styles();
    let (mut blocks, mut maps, mut errs) = (vec![], vec![], vec![]);
    for (s, sm) in parsed.pres.slides.iter().zip(&parsed.map.slides) {
        blocks.push(slide_head(&s.layout));
        maps.push(head_src(&sm.head));
        for (it, im) in s.items.iter().zip(&sm.items) {
            let body = match it {
                fmt::SlideItem::Slot(slot) => {
                    blocks.push(slot_head(&slot.name, slot.geom));
                    slot.blocks.clone()
                }
                fmt::SlideItem::Shape(sh) => {
                    blocks.push(shape_head(&sh.id, &sh.name, sh.geom));
                    let para = |c: &fmt::Inline| {
                        fmt::Block::Para(fmt::Para { style: fmt::ParaStyle::Plain, content: c.clone() })
                    };
                    sh.paras.iter().map(para).collect()
                }
                // `Names::objects` lets only an object's placeholder stand here.
                fmt::SlideItem::Object(o) => {
                    blocks.push(object_head(o.geom));
                    vec![fmt::Block::Keep(o.keep.clone())]
                }
                fmt::SlideItem::Line(l) => {
                    blocks.push(line_head(&l.id, &l.name, l.ends));
                    vec![]
                }
                fmt::SlideItem::Group(g) => {
                    blocks.push(group_head(g));
                    vec![]
                }
                fmt::SlideItem::Picture(pic) => {
                    blocks.push(picture_head(pic));
                    vec![]
                }
            };
            maps.push(head_src(&im.head));
            if body.is_empty() {
                continue;
            }
            // The item's blocks resolve as a document body would.
            let doc = fmt::Parsed {
                doc: fmt::Document { front: parsed.pres.front.clone(), blocks: body },
                map: fmt::SourceMap { blocks: im.blocks.clone() },
            };
            match model::resolve(&doc, text, &styles, caps, is_block_keep) {
                Ok(b) => {
                    blocks.extend(b);
                    maps.extend(model::block_maps(&doc));
                }
                Err(e) => errs.extend(e),
            }
        }
    }
    if errs.is_empty() {
        Ok((blocks, maps))
    } else {
        errs.sort_by_key(|d| (d.line, d.col));
        Err(errs)
    }
}

/// Flat blocks → the Presentation AST, in canonical shape.
pub fn unresolve(blocks: &[Block], front: fmt::FrontMatter, keep: &dyn Fn(&str) -> fmt::Keep) -> fmt::Presentation {
    let styles = no_styles();
    let mut pres = fmt::Presentation { front: front.clone(), slides: vec![] };
    let mut k = 0;
    while k < blocks.len() {
        let Block::Head(h) = &blocks[k] else {
            k += 1; // not held by a head: a malformed model
            continue;
        };
        let end = (k + 1..blocks.len()).find(|&x| blocks[x].head().is_some()).unwrap_or(blocks.len());
        let body = &blocks[k + 1..end];
        let geom = geom_of(h);
        match kind(h) {
            HeadKind::Slide { layout } => pres.slides.push(fmt::Slide { layout: layout.into(), items: vec![] }),
            HeadKind::Slot { name } => {
                let blocks = model::unresolve(body, &styles, front.clone(), keep).blocks;
                if let Some(s) = pres.slides.last_mut() {
                    s.items.push(fmt::SlideItem::Slot(fmt::Slot { name: name.into(), geom, blocks }));
                }
            }
            HeadKind::Shape { id, name } => {
                let paras = body
                    .iter()
                    .filter_map(|b| match b {
                        Block::Para(Para { content, .. }) => Some(content.clone()),
                        _ => None,
                    })
                    .collect();
                if let Some(s) = pres.slides.last_mut() {
                    let sh = fmt::ShapeText { id: id.into(), name: name.into(), geom, paras };
                    s.items.push(fmt::SlideItem::Shape(sh));
                }
            }
            HeadKind::Object => {
                if let (Some(s), Some(Block::Keep(id))) = (pres.slides.last_mut(), body.first()) {
                    s.items.push(fmt::SlideItem::Object(fmt::ObjectItem { keep: keep(id), geom }));
                }
            }
            HeadKind::Line { id, name } => {
                if let (Some(s), Some(Place::Line(ends))) = (pres.slides.last_mut(), &h.place) {
                    s.items.push(fmt::SlideItem::Line(fmt::LineItem { id: id.into(), name: name.into(), ends: *ends }));
                }
            }
            HeadKind::Group { .. } => {
                if let (Some(s), Some(Place::Group(g))) = (pres.slides.last_mut(), &h.place) {
                    s.items.push(fmt::SlideItem::Group(g.clone()));
                }
            }
            HeadKind::Picture { .. } => {
                if let (Some(s), Some(Place::Picture(p))) = (pres.slides.last_mut(), &h.place) {
                    s.items.push(fmt::SlideItem::Picture(p.clone()));
                }
            }
        }
        k = end;
    }
    pres.normalize();
    pres
}
