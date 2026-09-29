//! The Presentation model (DESIGN.md §5.3): a presentation's text resolved
//! to one flat list of blocks, a [`Head`] for each slide, slot, shape and
//! object followed by the blocks it holds (an object: its placeholder). Re-anchoring (design C, exact spans)
//! then treats slides and shapes as blocks like any other: remainder entries
//! anchored to a slide or a shape sit on its head.

use hanji_format::{self as fmt, Diagnostic, Names};

use crate::model::{self, Block, BlockSrc, Capabilities, Head, Para, SrcKind, StyleSet};

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
}

pub fn kind(h: &Head) -> HeadKind<'_> {
    if let Some(name) = h.key.strip_prefix("slot:") {
        HeadKind::Slot { name }
    } else if let Some(id) = h.key.strip_prefix("shape:") {
        HeadKind::Shape { id, name: &h.label }
    } else if h.key == OBJECT {
        HeadKind::Object
    } else {
        HeadKind::Slide { layout: &h.label }
    }
}

pub fn slide_head(layout: &str) -> Block {
    Block::Head(Head { level: 0, key: SLIDE.into(), label: layout.into() })
}

pub fn slot_head(name: &str) -> Block {
    Block::Head(Head { level: 1, key: slot_key(name), label: String::new() })
}

pub fn shape_head(id: &str, name: &str) -> Block {
    Block::Head(Head { level: 1, key: shape_key(id), label: name.into() })
}

pub fn object_head() -> Block {
    Block::Head(Head { level: 1, key: OBJECT.into(), label: String::new() })
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
                    blocks.push(slot_head(&slot.name));
                    slot.blocks.clone()
                }
                fmt::SlideItem::Shape(sh) => {
                    blocks.push(shape_head(&sh.id, &sh.name));
                    let para = |c: &fmt::Inline| {
                        fmt::Block::Para(fmt::Para { style: fmt::ParaStyle::Plain, content: c.clone() })
                    };
                    sh.paras.iter().map(para).collect()
                }
                // `Names::objects` lets only an object's placeholder stand here.
                fmt::SlideItem::Object(k) => {
                    blocks.push(object_head());
                    vec![fmt::Block::Keep(k.clone())]
                }
            };
            maps.push(head_src(&im.head));
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
        match kind(h) {
            HeadKind::Slide { layout } => pres.slides.push(fmt::Slide { layout: layout.into(), items: vec![] }),
            HeadKind::Slot { name } => {
                let blocks = model::unresolve(body, &styles, front.clone(), keep).blocks;
                if let Some(s) = pres.slides.last_mut() {
                    s.items.push(fmt::SlideItem::Slot(fmt::Slot { name: name.into(), blocks }));
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
                    s.items.push(fmt::SlideItem::Shape(fmt::ShapeText { id: id.into(), name: name.into(), paras }));
                }
            }
            HeadKind::Object => {
                if let (Some(s), Some(Block::Keep(id))) = (pres.slides.last_mut(), body.first()) {
                    s.items.push(fmt::SlideItem::Object(keep(id)));
                }
            }
        }
        k = end;
    }
    pres.normalize();
    pres
}
