//! Slides and notes pages → resolved model blocks + remainder entries.
//!
//! A slide is a `slide` head (its layout the label), then, in the slide's
//! z-order, a head and its paragraphs for each slot and shape with text, then
//! the notes slot. What becomes what:
//!
//! - The slide part and its notes page: a [`Kind::Slide`] entry each on the
//!   slide head, holding the part as it is but for the modelled shapes, which
//!   are stand-ins (`hanji-item`). Transitions and animations stay there.
//! - A chart, table or other object that is not a layout placeholder, a
//!   picture the text cannot hold (see [`picture_item`]), and a rotated or
//!   flipped group: an `object` head and its
//!   `<keep/>` line (rule 8; a `Bkeep` entry tagged `object`), in z-order
//!   among the slots and shapes.
//! - A shape without text, a connector (`<line/>`), a picture (`<picture/>`,
//!   its image, crop, mask and alternative text in its head's place) and a
//!   group (`<group>`, its objects in its head's place): a head and a
//!   [`Kind::Shape`] entry holding the element whole.
//! - Every head but the slide's and the notes' carries its geometry (§5.3):
//!   a slot its own box, else the one it inherits from its layout.
//! - A placeholder with text: a slot (`::title::`); its element without its
//!   paragraphs is a [`Kind::Shape`] entry on the slot's head. One without
//!   text is left out of the text and kept in the slide entry (`hanji-latent`)
//!   for a slot that fills it later.
//! - A picture, chart or table placeholder that holds its object: a slot
//!   whose text is the object's `<keep/>` (a `Bkeep` entry).
//! - A shape with text that is not a layout placeholder: `<shape>`.
//! - `a:p`: a paragraph (`Ppr`: its shell, `a:pPr`, `a:endParaRPr`); a list
//!   item when its bullet (own, else inherited) is a character or a number.
//! - `a:r`, `a:br`: `Run` (shell and `a:rPr`; bold, italic, underline and
//!   strike from its attributes). `a:fld` and other paragraph content: inline `Keep`.

use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;

use hanji_core::presentation::{
    group_head, kind, line_head, object_head, picture_head, shape_head, slide_head, slot_head, HeadKind,
};
use hanji_core::{Block, Entry, KeepIds, Kind, ListItem, Meta, Para};
use hanji_format::{
    Atom, Crop, Geom, GroupItem, Inline, Keep, LineItem, Marks, ObjectItem, PictureItem, ShapeText, SlideItem, Unit,
};
use hanji_package::clip;
use hanji_package::xml::{canon, fp, is_blank, Element, Node, Scope};

use crate::deck::{LayoutInfo, SlotInfo};
use crate::effects;
use crate::fill;
use crate::geom::{self, Frame};
use crate::kind;
use crate::members::{self, Styling};
use crate::outline;
use crate::pml::*;
use crate::route;
use crate::text::{self, RunStyle, ThemeFonts};
use hanji_format::inline_style::{self as istyle, TextStyle};

/// Where a slide came from, stored on its `Slide` entry.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SlideInfo {
    pub part: String,
    pub prolog: String,
    pub epilog: String,
    /// Its `p:sldId` in `presentation.xml`.
    pub sld_id: String,
    /// Its layout part at import.
    pub layout: String,
    /// Every shape id (`p:cNvPr id`) the slide had.
    pub ids: Vec<u32>,
    /// Every relationship id the slide's XML named (`r:embed`, `r:id`, …):
    /// one the export no longer names goes, with what only it reached.
    #[serde(default)]
    pub named: Vec<String>,
}

/// Where a notes page came from, stored on its `Slide` entry (tag `notes`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NotesInfo {
    pub part: String,
    pub prolog: String,
    pub epilog: String,
}

#[derive(Default, Debug, Clone)]
pub struct Stats {
    pub slides: usize,
    pub slots: usize,
    pub keep_slots: usize,
    pub shapes: usize,
    pub latent: usize,
    pub notes: usize,
    /// Objects shown as `<keep/>` lines (pictures, charts, tables, groups, …).
    pub objects: usize,
    /// Shape-tree children kept in the skeleton (empty placeholders the layout does not have, …).
    pub kept: usize,
    /// Shapes without text, lines and groups shown with their geometry.
    pub bare: usize,
    /// Pictures shown as `<picture/>`.
    pub pictures: usize,
    pub lines: usize,
    pub groups: usize,
    pub list_items: usize,
}

pub struct Importer<'a> {
    scope: Scope,
    pub blocks: Vec<Block>,
    pub entries: Vec<Entry>,
    pub next_id: u64,
    next_seq: u64,
    keep_ids: KeepIds,
    buf: Vec<Unit>,
    pub stats: Stats,
    /// Shapes shown as `<shape>`, `<line/>` or `<group>`: `(id, name)`.
    pub shapes: Vec<(String, String)>,
    /// The part being read.
    part: String,
    /// The open list of the slot being read: the `lvl` of each open level.
    list: Option<Vec<u32>>,
    /// What the text of the shape being read inherits, per level (none for notes).
    base: Option<[RunStyle; 9]>,
    fonts: ThemeFonts,
    /// Each unit's formatting as shown, beside `buf`.
    ustyle: Vec<TextStyle>,
    /// The relationships of the slide being read: id → the `src` its target is.
    pub rels: Rels,
    _deck: std::marker::PhantomData<&'a ()>,
}

/// What one shape-tree child is to the text.
enum What<'s> {
    Slot(&'s SlotInfo),
    KeepSlot(&'s SlotInfo),
    Latent(&'s SlotInfo),
    Shape(String, String),
    /// A shape without text that is not a placeholder.
    Bare(String, String),
    Line(String, String),
    Group(String, String),
    Picture(PictureItem),
    Object,
    Other,
}

/// The tag of an object's `Bkeep` entry (a slot's object has none).
pub const OBJECT_TAG: &str = "object";

impl Importer<'_> {
    pub fn new(scope: Scope) -> Self {
        Importer {
            scope,
            blocks: vec![],
            entries: vec![],
            next_id: 1,
            next_seq: 1000,
            keep_ids: KeepIds::default(),
            buf: vec![],
            stats: Stats::default(),
            shapes: vec![],
            part: String::new(),
            list: None,
            base: None,
            fonts: ThemeFonts::default(),
            ustyle: vec![],
            rels: Rels::new(),
            _deck: std::marker::PhantomData,
        }
    }

    fn seq(&mut self) -> u64 {
        let s = self.next_seq;
        self.next_seq += 1000;
        s
    }

    #[allow(clippy::too_many_arguments)]
    fn entry(
        &mut self,
        kind: Kind,
        xml: Vec<String>,
        fp: String,
        path: &[usize],
        start: Option<usize>,
        end: Option<usize>,
        mut meta: Meta,
    ) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        let seq = self.seq();
        if meta.part.is_none() && xml.iter().any(|x| refers(x)) {
            meta.part = Some(self.part.clone());
        }
        self.entries.push(Entry { id, kind, xml, fp, path: path.to_vec(), start, end, seq, meta });
        self.entries.len() - 1
    }

    fn fp(&self, els: &[Option<&Element>]) -> String {
        fp(&self.scope, els)
    }

    /// One slide and its notes page.
    pub fn slide(
        &mut self,
        info: SlideInfo,
        root: &Element,
        layout: &LayoutInfo,
        notes: Option<(NotesInfo, &Element, [Bu; 9])>,
    ) -> Result<(), String> {
        self.part = info.part.clone();
        let bi = self.blocks.len();
        self.blocks.push(slide_head(&layout.name));
        self.stats.slides += 1;
        let mut skel = root.clone();
        let mut ids = vec![];
        skel.walk(&mut |e| {
            if e.is("p:cNvPr") {
                ids.extend(e.get("id").and_then(|v| v.parse::<u32>().ok()));
            }
        });
        let tree =
            skel.child_mut("p:cSld").and_then(|c| c.child_mut("p:spTree")).ok_or("the slide has no p:cSld/p:spTree")?;
        let mut used: Vec<String> = vec![];
        let mut k = 0;
        // Each connector's head, and its ends' attachments (shown after the loop).
        let mut attached: Vec<(usize, geom::Attachments)> = vec![];
        for n in std::mem::take(&mut tree.children) {
            let Node::El(el) = n else {
                tree.children.push(n);
                continue;
            };
            let what = what(&el, layout, &used, &self.rels);
            match what {
                What::Other => {
                    if !el.is("p:nvGrpSpPr") && !el.is("p:grpSpPr") && !el.is("p:extLst") {
                        self.stats.kept += 1;
                    }
                    tree.children.push(Node::El(el));
                }
                What::Latent(slot) => {
                    used.push(slot.name.clone());
                    self.stats.latent += 1;
                    tree.children.push(Node::El(wrap(LATENT, &[("slot", &slot.name)], Some(el))));
                }
                What::Slot(slot) => {
                    used.push(slot.name.clone());
                    k += 1;
                    self.stats.slots += 1;
                    let fallback = emptied(&el);
                    tree.children.push(Node::El(wrap(
                        ITEM,
                        &[("k", &k.to_string()), ("slot", &slot.name)],
                        Some(fallback),
                    )));
                    let g = geom::own(&el).or(slot.geom);
                    self.blocks.push(slot_head(&slot.name, g));
                    self.show_fill(&el, slot.fill.as_ref(), &layout.fills);
                    self.show_kind(&el, slot.geo.as_ref(), false);
                    self.show_outline(&el, slot.line.as_ref(), &layout.fills, false);
                    self.show_effects(&el, slot.fx.as_ref(), &layout.fills);
                    let t = Some((&slot.text, &layout.fonts));
                    self.text_shape(&el, &slot.bullets, &k.to_string(), true, g, t)?;
                }
                What::KeepSlot(slot) => {
                    used.push(slot.name.clone());
                    k += 1;
                    self.stats.keep_slots += 1;
                    tree.children.push(Node::El(wrap(ITEM, &[("k", &k.to_string())], None)));
                    let g = object_geom(&el).or(slot.geom);
                    self.blocks.push(slot_head(&slot.name, g));
                    let bj = self.blocks.len();
                    let keep = self.keep_entry(Kind::Bkeep, &el, &[bj], None, &k.to_string(), "");
                    self.entries.last_mut().unwrap().meta.aux.push(shown(g));
                    self.blocks.push(Block::Keep(keep.id));
                }
                What::Object => {
                    k += 1;
                    self.stats.objects += 1;
                    tree.children.push(Node::El(wrap(ITEM, &[("k", &k.to_string())], None)));
                    let g = object_geom(&el);
                    self.blocks.push(object_head(g));
                    let bj = self.blocks.len();
                    let keep = self.keep_entry(Kind::Bkeep, &el, &[bj], None, &k.to_string(), OBJECT_TAG);
                    self.entries.last_mut().unwrap().meta.aux.push(shown(g));
                    self.blocks.push(Block::Keep(keep.id));
                }
                What::Shape(id, name) => {
                    k += 1;
                    self.stats.shapes += 1;
                    tree.children.push(Node::El(wrap(ITEM, &[("k", &k.to_string())], None)));
                    let g = geom::own(&el);
                    self.blocks.push(shape_head(&id, &name, g));
                    self.show_kind(&el, None, false);
                    self.show_fill(&el, None, &layout.fills);
                    self.show_outline(&el, None, &layout.fills, false);
                    self.show_effects(&el, None, &layout.fills);
                    self.name_shape(id, name);
                    let t = Some((&layout.other_text, &layout.fonts));
                    self.text_shape(&el, &layout.other, &k.to_string(), false, g, t)?;
                }
                What::Picture(pic) => {
                    k += 1;
                    self.stats.pictures += 1;
                    tree.children.push(Node::El(wrap(ITEM, &[("k", &k.to_string())], None)));
                    self.blocks.push(picture_head(&pic));
                    self.name_shape(pic.id, pic.name);
                    self.whole(&el, &k.to_string());
                }
                What::Bare(ref id, ref name) | What::Line(ref id, ref name) | What::Group(ref id, ref name) => {
                    let (id, name) = (id.clone(), name.clone());
                    k += 1;
                    tree.children.push(Node::El(wrap(ITEM, &[("k", &k.to_string())], None)));
                    let head = match &what {
                        What::Bare(..) => {
                            self.stats.bare += 1;
                            shape_head(&id, &name, geom::own(&el))
                        }
                        What::Line(..) => {
                            self.stats.lines += 1;
                            let g = geom::own(&el).unwrap_or_default();
                            line_head(&id, &name, geom::ends_of(&g))
                        }
                        _ => {
                            self.stats.groups += 1;
                            let g = group_item(&el, &Frame::SLIDE, &self.rels, Some(Styling::of(layout)))
                                .ok_or("a group that cannot be shown")?;
                            group_head(&g)
                        }
                    };
                    self.blocks.push(head);
                    if matches!(what, What::Line(..)) {
                        attached.push((self.blocks.len() - 1, geom::attachments(&el)));
                    }
                    if matches!(what, What::Bare(..)) {
                        self.show_fill(&el, None, &layout.fills);
                    }
                    if matches!(what, What::Bare(..) | What::Line(..)) {
                        self.show_kind(&el, None, matches!(what, What::Line(..)));
                        self.show_outline(&el, None, &layout.fills, matches!(what, What::Line(..)));
                        self.show_effects(&el, None, &layout.fills);
                    }
                    self.name_shape(id, name);
                    self.whole(&el, &k.to_string());
                }
            }
        }
        show_attachments(&mut self.blocks[bi..], &attached, bi);
        let fpv = canon(&without_stand_ins(&skel), &self.scope);
        let named = named_rel_ids(root).into_iter().collect();
        let aux = vec![serde_json::to_string(&SlideInfo { ids, named, ..info }).unwrap()];
        let meta = Meta { tag: "slide".into(), aux, part: Some(self.part.clone()), ..Default::default() };
        self.entry(Kind::Slide, vec![skel.to_xml()], fpv, &[bi], None, None, meta);
        if let Some((ni, nroot, bullets)) = notes {
            self.notes(bi, ni, nroot, bullets)?;
        }
        Ok(())
    }

    /// A notes page: its body placeholder's text is the `notes` slot.
    fn notes(&mut self, bi: usize, info: NotesInfo, root: &Element, bullets: [Bu; 9]) -> Result<(), String> {
        self.part = info.part.clone();
        self.stats.notes += 1;
        let mut skel = root.clone();
        let tree = skel
            .child_mut("p:cSld")
            .and_then(|c| c.child_mut("p:spTree"))
            .ok_or("the notes page has no p:cSld/p:spTree")?;
        let mut done = false;
        for n in &mut tree.children {
            let Node::El(el) = n else { continue };
            let body = el.is("p:sp") && !done && placeholder(el).is_some_and(|p| p.0 == "body");
            if !body {
                continue;
            }
            done = true;
            let shape = std::mem::take(el);
            if has_text(&shape) {
                *el = wrap(ITEM, &[("k", "notes"), ("slot", "notes")], Some(emptied(&shape)));
                self.blocks.push(slot_head("notes", None));
                self.text_shape(&shape, &bullets, "notes", true, None, None)?;
            } else {
                *el = wrap(LATENT, &[("slot", "notes")], Some(shape));
            }
        }
        let fpv = canon(&without_stand_ins(&skel), &self.scope);
        let meta = Meta {
            tag: "notes".into(),
            aux: vec![serde_json::to_string(&info).unwrap()],
            part: Some(self.part.clone()),
            ..Default::default()
        };
        self.entry(Kind::Slide, vec![skel.to_xml()], fpv, &[bi], None, None, meta);
        Ok(())
    }

    fn name_shape(&mut self, id: String, name: String) {
        if !self.shapes.iter().any(|s| s.0 == id && s.1 == name) {
            self.shapes.push((id, name));
        }
    }

    /// A shape without text, a line or a group: its element whole, a `Shape`
    /// entry on the head just pushed.
    /// The head just pushed shows the fill `el` has (§5.3).
    /// The head just pushed shows the outline `el` draws (§5.3).
    fn show_outline(&mut self, el: &Element, parent: Option<&outline::Ln>, theme: &fill::ThemeFills, line: bool) {
        if let Some(Block::Head(h)) = self.blocks.last_mut() {
            let l = outline::effective(el, parent, theme).look(line);
            (h.look.border, h.look.start, h.look.end) = (l.border, l.start, l.end);
        }
    }

    fn show_kind(&mut self, el: &Element, parent: Option<&kind::Geo>, line: bool) {
        if let Some(Block::Head(h)) = self.blocks.last_mut() {
            (h.look.kind, h.look.adj) = kind::shown(el, parent, line);
        }
    }

    fn show_effects(&mut self, el: &Element, parent: Option<&effects::EffectsXml>, theme: &fill::ThemeFills) {
        if let Some(Block::Head(h)) = self.blocks.last_mut() {
            h.look.effects = effects::shown(effects::effective(el, parent, theme).as_ref());
        }
    }

    fn show_fill(&mut self, el: &Element, parent: Option<&fill::FillXml>, theme: &fill::ThemeFills) {
        if let Some(Block::Head(h)) = self.blocks.last_mut() {
            h.look.fill = fill::shown(fill::effective(el, parent, theme).as_ref());
        }
    }

    fn whole(&mut self, el: &Element, k: &str) {
        let hi = self.blocks.len() - 1;
        // A shape's fill is the text's (§5.3), as its geometry is.
        let bare = geom::without_geometry(&match el.name.as_str() {
            "p:sp" => effects::without_effects(&kind::without_kind(&outline::without_outline(&fill::without_fill(el)))),
            // A connector's attachments are the text's too.
            "p:cxnSp" => route::without_attachments(&effects::without_effects(&kind::without_kind(
                &outline::without_outline(el),
            ))),
            // A group's shapes' text and fills are the text's too.
            "p:grpSp" => members::without_shown(el),
            _ => el.clone(),
        });
        let f = self.fp(&[Some(&if el.is("p:pic") { unmodelled_picture(&bare) } else { bare })]);
        let meta = Meta {
            tag: el.name.clone(),
            aux: vec![k.to_string(), self.part.clone(), shown(geom::own(el))],
            ..Default::default()
        };
        self.entry(Kind::Shape, vec![el.to_xml()], f, &[hi], None, None, meta);
    }

    /// A shape's element (a `Shape` entry on the head just pushed) and its
    /// paragraphs. Without `lists` (a `<shape>`, whose text has no list
    /// items) a paragraph's bullet stays in the remainder. `g` is the box
    /// the text shows for it.
    #[allow(clippy::too_many_arguments)]
    fn text_shape(
        &mut self,
        el: &Element,
        inherited: &[Bu; 9],
        k: &str,
        lists: bool,
        g: Option<Geom>,
        text: Option<(&[RunStyle; 9], &ThemeFonts)>,
    ) -> Result<(), String> {
        let hi = self.blocks.len() - 1;
        // What its runs inherit (§5.3 text formatting); notes show none.
        self.base = text.map(|(t, f)| text::base(el, t, f));
        self.fonts = text.map(|t| t.1.clone()).unwrap_or_default();
        let mut shell = el.clone();
        let tx = shell.child_mut("p:txBody").ok_or("a text shape without p:txBody")?;
        let paras: Vec<Element> = tx.elements().filter(|e| e.is("a:p")).cloned().collect();
        tx.children.retain(|n| !matches!(n, Node::El(e) if e.is("a:p")));
        let bullets = over(levels_of(tx.child("a:lstStyle")), *inherited);
        let mut fshell = shell.clone();
        if let Some(nv) = fshell.elements_mut().find(|e| e.name.starts_with("p:nv")) {
            if let Some(pr) = nv.child_mut("p:nvPr") {
                pr.children.retain(|n| !matches!(n, Node::El(e) if e.is("p:ph")));
            }
        }
        let bare =
            effects::without_effects(&kind::without_kind(&outline::without_outline(&fill::without_fill(&fshell))));
        let f = self.fp(&[Some(&geom::without_geometry(&bare))]);
        let meta = Meta {
            tag: shell.name.clone(),
            aux: vec![k.to_string(), self.part.clone(), shown(g)],
            ..Default::default()
        };
        self.entry(Kind::Shape, vec![shell.to_xml()], f, &[hi], None, None, meta);
        self.list = None;
        let bullets = if lists { bullets } else { [Bu::None; 9] };
        let first = self.blocks.len();
        for p in &paras {
            let bi = self.blocks.len();
            let para = self.para(p, bi, &bullets, lists)?;
            self.blocks.push(Block::Para(para));
        }
        self.list = None;
        if self.base.is_some() {
            self.shown_styles(first);
        }
        self.base = None;
        Ok(())
    }

    /// The canonical formatting of the shape's paragraphs from `first` on
    /// (spaces and breaks take the text's around them, §5.3).
    fn shown_styles(&mut self, first: usize) {
        let mut inls: Vec<&mut Inline> = self.blocks[first..]
            .iter_mut()
            .filter_map(|b| if let Block::Para(p) = b { Some(&mut p.content) } else { None })
            .collect();
        istyle::normalize(&mut inls);
        for i in inls {
            i.normalize();
        }
    }

    /// A placeholder entry for `el`; `tag` marks a slide object's.
    fn keep_entry(&mut self, kind: Kind, el: &Element, path: &[usize], pos: Option<usize>, k: &str, tag: &str) -> Keep {
        // Geometry is the text's (§5.3): neither the id nor the fingerprint holds it.
        let fpv = self.fp(&[Some(&geom::without_geometry(el))]);
        let id = self.keep_ids.next(&fpv);
        let keep = Keep { id, kind: keep_kind(el), summary: summary(el) };
        let aux = if kind == Kind::Bkeep { vec![k.to_string(), self.part.clone()] } else { vec![] };
        let meta = Meta { keep: Some(keep.clone()), aux, tag: tag.into(), ..Default::default() };
        self.entry(kind, vec![el.to_xml()], fpv, path, pos, pos.map(|p| p + 1), meta);
        keep
    }

    fn para(&mut self, p: &Element, bi: usize, bullets: &[Bu; 9], lists: bool) -> Result<Para, String> {
        let path = [bi];
        let ppr = p.child("a:pPr");
        let end = p.child("a:endParaRPr");
        let lvl: u32 = ppr.and_then(|x| x.get("lvl")).and_then(|v| v.parse().ok()).unwrap_or(0).min(8);
        let under = self.base.as_ref().map(|b| b[lvl as usize].clone());
        let kind = match bu_of(ppr).filter_lists(lists) {
            Bu::Unset => bullets[lvl as usize],
            b => b,
        };
        let item = match kind {
            Bu::Bullet | Bu::Number => Some(self.list_item(kind == Bu::Number, lvl)),
            _ => {
                self.list = None;
                None
            }
        };
        let shell = p.shell();
        let xml: Vec<String> =
            std::iter::once(shell.to_xml()).chain([ppr, end].into_iter().flatten().map(Element::to_xml)).collect();
        // The level and the bullet are the text's (as a run's marks are).
        let unmodelled = ppr.map(|x| {
            let mut x = x.clone();
            x.remove_attr("lvl");
            x.children.retain(|n| !matches!(n, Node::El(e) if BU.contains(&e.name.as_str())));
            x
        });
        let f = if shell.attrs.is_empty() && ppr.is_none() && end.is_none() {
            String::new()
        } else {
            self.fp(&[Some(&shell), unmodelled.as_ref(), end])
        };
        let meta = Meta { item, aux: vec![lvl.to_string()], ..Default::default() };
        self.entry(Kind::Ppr, xml, f, &path, None, None, meta);
        self.buf.clear();
        self.ustyle.clear();
        let mut texts: Vec<(usize, String)> = vec![];
        for c in &p.children {
            let c = match c {
                Node::El(c) => c,
                n if is_blank(n) => continue,
                _ => return Err("text directly inside <a:p>".into()),
            };
            match c.name.as_str() {
                "a:pPr" | "a:endParaRPr" => {}
                "a:r" if run_modellable(c) => {
                    let t = c.text_of(&["a:t"]);
                    let ix = self.run(c, &path, &t, false, under.as_ref());
                    texts.push((ix, t));
                }
                "a:br" if c.elements().all(|x| x.is("a:rPr")) => {
                    self.run(c, &path, "", true, under.as_ref());
                }
                _ => {
                    let pos = self.buf.len();
                    let keep = self.keep_entry(Kind::Keep, c, &path, Some(pos), "", "");
                    self.buf.push(Unit::new(Atom::Keep(keep), Marks::NONE));
                    self.ustyle.push(TextStyle::default());
                }
            }
        }
        let mut content = Inline { units: std::mem::take(&mut self.buf), spans: vec![] };
        if under.is_some() {
            istyle::set_unit_styles(&mut content, &std::mem::take(&mut self.ustyle));
        }
        // Whitespace alone has no line form: the paragraph is empty in the
        // text, and its runs keep their text.
        if !content.units.is_empty()
            && content.units.iter().all(|u| matches!(u.atom, Atom::Char(c) if c.is_whitespace()))
        {
            for (ix, t) in texts {
                let e = &mut self.entries[ix];
                (e.start, e.end) = (Some(0), Some(0));
                e.meta.aux = vec![t];
            }
            content.units.clear();
            content.spans.clear();
        }
        // With formatting shown, marks are canonical once the formatting is
        // (`shown_styles`): the spans it leaves decide where they can end.
        if under.is_none() {
            content.normalize();
        }
        self.stats.list_items += item.is_some() as usize;
        Ok(Para::new(String::new(), content, item))
    }

    /// The list item a paragraph at `lvl` is: levels nest by `lvl` within
    /// consecutive items, so a list starting at `lvl` 2 is at the margin.
    fn list_item(&mut self, ordered: bool, lvl: u32) -> ListItem {
        match &mut self.list {
            Some(stack) => {
                while stack.last().is_some_and(|&t| t >= lvl) {
                    stack.pop();
                }
                stack.push(lvl);
                ListItem { ordered, level: stack.len() - 1, first: false }
            }
            None => {
                self.list = Some(vec![lvl]);
                ListItem { ordered, level: 0, first: true }
            }
        }
    }

    /// `a:r` or `a:br` → a `Run` entry and its text; the entry's index.
    /// `under`: what the run inherits (its formatting is shown over it).
    fn run(&mut self, r: &Element, path: &[usize], text: &str, br: bool, under: Option<&RunStyle>) -> usize {
        let rpr = r.child("a:rPr");
        let shown = under.map(|u| rpr.map_or(RunStyle::default(), |x| RunStyle::of(x, &self.fonts)).over(u).shown());
        let marks = marks_of(rpr);
        let shell = r.shell();
        let mut rest = rpr.cloned();
        if let Some(x) = &mut rest {
            for a in MARK_ATTRS {
                x.remove_attr(a);
            }
            // Formatting the text shows is the text's (§5.3), as the marks are.
            if under.is_some() {
                text::without_shown(x);
            }
        }
        let trivial =
            shell.attrs.is_empty() && rest.as_ref().is_none_or(|x| x.attrs.is_empty() && x.children.is_empty());
        let f = if trivial { String::new() } else { self.fp(&[Some(&shell), rest.as_ref()]) };
        let xml = std::iter::once(shell.to_xml()).chain(rpr.map(Element::to_xml)).collect();
        let start = self.buf.len();
        if br {
            self.buf.push(Unit::new(Atom::Break, marks));
        } else {
            self.buf.extend(text.chars().map(|c| Unit::new(Atom::Char(c), marks)));
        }
        let end = self.buf.len();
        self.ustyle.resize(end, shown.clone().unwrap_or_default());
        let meta = Meta { marks, tag: r.name.clone(), ..Default::default() };
        self.entry(Kind::Run, xml, f, path, Some(start), Some(end), meta)
    }
}

/// What a shape-tree child is: a slot (placeholder of the layout with text,
/// or holding its object), a latent placeholder, a shape with text, or
/// something the text does not show.
fn what<'s>(el: &Element, layout: &'s LayoutInfo, used: &[String], rels: &Rels) -> What<'s> {
    let ph = placeholder(el);
    let text = el.is("p:sp") && has_text(el);
    if let Some((ty, idx, _)) = &ph {
        if let Some(slot) = match_slot(layout, ty, *idx).filter(|s| !used.contains(&s.name)) {
            return match el.name.as_str() {
                "p:sp" if text => What::Slot(slot),
                "p:sp" if el.child("p:txBody").is_some() => What::Latent(slot),
                "p:pic" | "p:graphicFrame" => What::KeepSlot(slot),
                _ => What::Other,
            };
        }
    }
    let id = c_nv_pr(el).and_then(|c| c.get("id")).filter(|v| v.parse::<u32>().is_ok());
    let name = || c_nv_pr(el).and_then(|c| c.get("name")).unwrap_or_default();
    match (text, id) {
        (true, Some(id)) => What::Shape(format!("s{id}"), name()),
        (_, Some(id)) if el.is("p:grpSp") && group_item(el, &Frame::SLIDE, rels, None).is_some() => {
            What::Group(format!("g{id}"), name())
        }
        _ if el.is("p:pic") && picture_item(el, &Frame::SLIDE, rels).is_some() => {
            What::Picture(picture_item(el, &Frame::SLIDE, rels).unwrap())
        }
        _ if is_object(el) => What::Object,
        (false, Some(id)) if el.is("p:sp") && ph.is_none() => What::Bare(format!("s{id}"), name()),
        (_, Some(id)) if el.is("p:cxnSp") && geom::own(el).is_some() => What::Line(format!("s{id}"), name()),
        _ => What::Other,
    }
}

/// The box stored with an entry: the one the text showed for it at import.
pub fn shown(g: Option<Geom>) -> String {
    serde_json::to_string(&g).unwrap()
}

/// The box an entry's `aux` says the text showed (see [`shown`]).
pub fn shown_of(aux: &[String]) -> Option<Geom> {
    aux.get(2).and_then(|a| serde_json::from_str(a).ok()).flatten()
}

/// An object's own box; an alternate-content object's is its first choice's.
pub fn object_geom(el: &Element) -> Option<Geom> {
    match chosen(el) {
        Some(inner) => geom::own(inner),
        None => geom::own(el),
    }
}

/// A group as the text shows it (§5.3): its box and its objects in slide
/// coordinates, through `parent` (the frame the group sits in); a turned or
/// flipped group's objects as they are in it before its turn and flips.
/// `None` for a group with an object the text cannot show: it is one
/// `<keep/>`.
pub fn group_item(el: &Element, parent: &Frame, rels: &Rels, sty: Option<Styling>) -> Option<GroupItem> {
    let id = geom::shape_id(el)?;
    let name = c_nv_pr(el)?.get("name").unwrap_or_default();
    let own = geom::own(el)?;
    let f = Frame::of(el)?.within(parent);
    let mut items = vec![];
    for c in el.elements() {
        match c.name.as_str() {
            "p:nvGrpSpPr" | "p:grpSpPr" | "p:extLst" => {}
            _ => items.push(member(c, &f, rels, sty)?),
        }
    }
    (!items.is_empty()).then(|| GroupItem { id: format!("g{id}"), name, geom: Some(parent.out(&own)), items })
}

/// One object of a group, in slide coordinates through `f`.
fn member(c: &Element, f: &Frame, rels: &Rels, sty: Option<Styling>) -> Option<SlideItem> {
    let id = geom::shape_id(c)?;
    let name = c_nv_pr(c).and_then(|x| x.get("name")).unwrap_or_default();
    Some(match c.name.as_str() {
        "p:sp" => SlideItem::Shape(ShapeText {
            id: format!("s{id}"),
            name,
            geom: geom::own(c).map(|g| f.out(&g)),
            look: members::look(c, sty),
            paras: members::paras(c, sty),
        }),
        "p:cxnSp" => {
            let g = f.out(&geom::own(c)?);
            SlideItem::Line(LineItem {
                id: format!("s{id}"),
                name,
                ends: geom::ends_of(&g),
                look: members::line_look(c, sty),
            })
        }
        "p:grpSp" if group_item(c, f, rels, sty).is_some() => SlideItem::Group(group_item(c, f, rels, sty)?),
        "p:pic" if picture_item(c, f, rels).is_some() => SlideItem::Picture(picture_item(c, f, rels)?),
        _ if is_object(c) => SlideItem::Object(ObjectItem {
            keep: Keep { id: format!("s{id}"), kind: keep_kind(c), summary: summary(c) },
            geom: object_geom(c).map(|g| f.out(&g)),
        }),
        _ => return None,
    })
}

/// A slide's relationships as pictures name them: relationship id → the
/// `src` of its target (see [`src_of`]).
pub type Rels = BTreeMap<String, String>;

/// The relationships of `part` in `parts`: id → `src` (internal targets only).
pub fn rels_of(parts: &[hanji_core::Part], part: &str) -> Rels {
    hanji_package::opc::rels_of(parts, part)
        .into_iter()
        .filter(|r| !r.external)
        .map(|r| (r.id, src_of(&hanji_package::opc::resolve_target(part, &r.target))))
        .collect()
}

/// A picture's `src`: its image part, from the presentation's folder
/// (`media/image1.png` for `ppt/media/image1.png`).
pub fn src_of(part: &str) -> String {
    match part.strip_prefix("ppt/") {
        Some(rest) => rest.to_string(),
        None => format!("/{part}"),
    }
}

/// The package part a `src` names.
pub fn part_of_src(src: &str) -> String {
    match src.strip_prefix('/') {
        Some(rest) => rest.to_string(),
        None => format!("ppt/{src}"),
    }
}

/// Artistic effects (`a14:imgProps`) are stored in this extension of `a:blip`.
const ARTISTIC_EFFECTS: &str = "{BEBA8EAE-BF5A-486C-A8C5-ECC9F3942E4B}";

/// A picture as the text shows it (§5.3), in slide coordinates through `f`:
/// its box, image, crop, mask and alternative text. `None` for one the text
/// keeps whole (`<keep/>`): media (a video or sound shown as a picture), a
/// linked image, a picture with artistic effects or a duotone, one cut to
/// a custom shape, or one without a box or an image in the package.
pub fn picture_item(el: &Element, f: &Frame, rels: &Rels) -> Option<PictureItem> {
    if !el.is("p:pic") {
        return None;
    }
    let nv = el.child("p:nvPicPr")?;
    let mut media = false;
    if let Some(pr) = nv.child("p:nvPr") {
        pr.walk(&mut |e| {
            media |= matches!(e.local(), "videoFile" | "audioFile" | "quickTimeFile" | "wavAudioFile" | "media")
        });
    }
    if media {
        return None;
    }
    let bf = el.child("p:blipFill")?;
    let blip = bf.child("a:blip")?;
    if blip.get("r:link").is_some() || blip.child("a:duotone").is_some() {
        return None;
    }
    let mut artistic = false;
    blip.walk(&mut |e| artistic |= e.is("a:ext") && e.get("uri").as_deref() == Some(ARTISTIC_EFFECTS));
    if artistic {
        return None;
    }
    let src = rels.get(&blip.get("r:embed")?)?.clone();
    let sppr = el.child("p:spPr")?;
    if sppr.child("a:custGeom").is_some() {
        return None;
    }
    let mask = sppr.child("a:prstGeom").and_then(|g| g.get("prst")).filter(|m| m != "rect");
    let geom = geom::own(el).map(|g| f.out(&g))?;
    let id = geom::shape_id(el)?;
    let c = c_nv_pr(el)?;
    let crop = bf.child("a:srcRect").map(crop_of).filter(|c| !c.is_zero());
    Some(PictureItem {
        id: format!("s{id}"),
        name: c.get("name").unwrap_or_default(),
        geom: Some(geom),
        src,
        crop,
        mask,
        alt: c.get("descr").filter(|d| !d.is_empty()),
    })
}

/// A picture without what the text shows of it (its image, crop, mask and
/// alternative text): what its entry's fingerprint is made from.
fn unmodelled_picture(el: &Element) -> Element {
    let mut e = el.clone();
    if let Some(c) = c_nv_pr_mut(&mut e) {
        c.remove_attr("descr");
    }
    if let Some(bf) = e.child_mut("p:blipFill") {
        if let Some(b) = bf.child_mut("a:blip") {
            b.remove_attr("r:embed");
        }
        if let Some(r) = bf.child_mut("a:srcRect") {
            for k in ["l", "t", "r", "b"] {
                r.remove_attr(k);
            }
        }
        bf.children
            .retain(|n| !matches!(n, Node::El(x) if x.is("a:srcRect") && x.attrs.is_empty() && !x.has_elements()));
    }
    if let Some(g) = e.child_mut("p:spPr").and_then(|s| s.child_mut("a:prstGeom")) {
        g.remove_attr("prst");
        g.children.clear();
    }
    e
}

/// The crop an `a:srcRect` stores.
pub fn crop_of(r: &Element) -> Crop {
    let v = |k: &str| r.get(k).and_then(|x| x.trim().parse().ok()).unwrap_or(0);
    Crop { l: v("l"), t: v("t"), r: v("r"), b: v("b") }
}

/// A picture, graphic frame (chart, table, SmartArt, OLE object), group,
/// ink or alternate-content object: what rule 8 shows as a `<keep/>` line.
fn is_object(el: &Element) -> bool {
    matches!(el.name.as_str(), "p:pic" | "p:graphicFrame" | "p:grpSp" | "p:contentPart" | "mc:AlternateContent")
}

/// The layout slot a slide placeholder of type `ty` and index `idx` fills:
/// by index when it has one, else by type (a title is a title, centred or not).
pub fn match_slot<'s>(layout: &'s LayoutInfo, ty: &str, idx: u32) -> Option<&'s SlotInfo> {
    let title = |t: &str| class_of_type(t) == TextClass::Title;
    if idx != 0 {
        if let Some(s) = layout.slots.iter().find(|s| s.idx == idx) {
            return Some(s);
        }
    }
    layout
        .slots
        .iter()
        .find(|s| s.idx == idx && (s.ty == ty || (title(ty) && title(&s.ty))))
        .or_else(|| layout.slots.iter().find(|s| s.ty == ty || (title(ty) && title(&s.ty))))
}

/// A `p:sp` whose text body holds any text (whitespace alone is none) or
/// inline object.
pub fn has_text(sp: &Element) -> bool {
    let Some(tx) = sp.child("p:txBody") else { return false };
    tx.elements().filter(|p| p.is("a:p")).any(|p| {
        p.elements().any(|c| match c.name.as_str() {
            "a:pPr" | "a:endParaRPr" => false,
            "a:r" => !c.text_of(&["a:t"]).trim().is_empty(),
            _ => true,
        })
    })
}

/// A run the text can hold: `a:rPr` and `a:t` only, no control characters.
fn run_modellable(r: &Element) -> bool {
    r.elements().all(|c| c.is("a:rPr") || (c.is("a:t") && !c.has_elements()))
        && !r.text_of(&["a:t"]).chars().any(|c| (c as u32) < 0x20 && c != '\t')
}

/// A placeholder with no text: its element with one empty paragraph (the
/// first one's properties), what a slot whose text is deleted leaves.
fn emptied(el: &Element) -> Element {
    let mut e = el.clone();
    if let Some(tx) = e.child_mut("p:txBody") {
        let first = tx.elements().find(|p| p.is("a:p")).map(|p| {
            let mut q = p.shell();
            q.children = p
                .children
                .iter()
                .filter(|n| matches!(n, Node::El(x) if x.is("a:pPr") || x.is("a:endParaRPr")))
                .cloned()
                .collect();
            q
        });
        tx.children.retain(|n| !matches!(n, Node::El(x) if x.is("a:p")));
        tx.children.push(Node::El(first.unwrap_or_else(|| Element::new("a:p"))));
    }
    e
}

/// `<name attrs…>child</name>`.
pub fn wrap(name: &str, attrs: &[(&str, &str)], child: Option<Element>) -> Element {
    let mut w = Element::new(name);
    for (k, v) in attrs {
        w = w.with_attr(k, v);
    }
    w.children.extend(child.map(Node::El));
    w
}

/// The skeleton without its stand-ins and empty placeholders, for its
/// fingerprint: what the text does not show and no slot can take.
fn without_stand_ins(skel: &Element) -> Element {
    let mut s = skel.clone();
    let empty = |x: &Element| x.is("p:sp") && placeholder(x).is_some() && !has_text(x);
    s.walk_mut(&mut |e| e.children.retain(|n| !matches!(n, Node::El(x) if x.is(ITEM) || x.is(LATENT) || empty(x))));
    s
}

/// An XML fragment with an `r:` attribute: it refers to its part's relationships.
pub(crate) fn refers(xml: &str) -> bool {
    if !xml.contains(" r:") {
        return false;
    }
    let Ok(d) = hanji_package::xml::parse(xml.as_bytes()) else { return false };
    let mut found = false;
    d.root.walk(&mut |e| found |= e.attrs.iter().any(|a| a.0.starts_with("r:")));
    found
}

fn keep_kind(el: &Element) -> String {
    // On a slide: the object its first choice holds.
    if let Some(inner) = chosen(el) {
        return keep_kind(inner);
    }
    match el.local() {
        "pic" => "picture".into(),
        "graphicFrame" => {
            let mut uri = String::new();
            el.walk(&mut |e| {
                if e.is("a:graphicData") && uri.is_empty() {
                    uri = e.get("uri").unwrap_or_default();
                }
            });
            match uri.rsplit('/').next().unwrap_or("") {
                "chart" => "chart",
                "table" => "table",
                "diagram" => "diagram",
                "ole" => "object",
                _ => "frame",
            }
            .into()
        }
        "fld" => "field".into(),
        "grpSp" => "group".into(),
        "contentPart" => "ink".into(),
        "AlternateContent" | "m" => "math".into(),
        "r" => "text".into(),
        other => other.into(),
    }
}

/// The first shape a slide's `mc:AlternateContent` holds (`None` for anything else).
fn chosen(el: &Element) -> Option<&Element> {
    if !el.is("mc:AlternateContent") {
        return None;
    }
    let mut found = None;
    el.walk(&mut |e| {
        if found.is_none() && (e.is("p:sp") || (is_object(e) && !e.is("mc:AlternateContent"))) {
            found = Some(e);
        }
    });
    found
}

/// The text of every paragraph in `el`, one space between paragraphs (a
/// table's cells, a group's shapes).
fn paragraphs_text(el: &Element) -> String {
    let mut out: Vec<String> = vec![];
    el.walk(&mut |e| {
        if e.is("a:p") {
            let t = e.text_of(&["a:t"]);
            if !t.trim().is_empty() {
                out.push(t);
            }
        }
    });
    out.join(" ")
}

fn summary(el: &Element) -> String {
    if let Some(inner) = chosen(el) {
        return summary(inner);
    }
    let s = match el.local() {
        "fld" => {
            let ty = el.get("type").unwrap_or_else(|| "field".into());
            format!("{ty}: {}", el.text_of(&["a:t"]))
        }
        "pic" | "graphicFrame" | "grpSp" | "contentPart" => {
            let c = c_nv_pr(el);
            let descr = c.and_then(|c| c.get("descr")).filter(|d| !d.trim().is_empty());
            let name = c.and_then(|c| c.get("name")).unwrap_or_default();
            let text = paragraphs_text(el);
            match (descr, text.trim().is_empty()) {
                (Some(d), _) => d,
                (None, false) => format!("{name}: {text}"),
                (None, true) => name,
            }
        }
        _ => {
            let t = paragraphs_text(el);
            if t.trim().is_empty() {
                el.local().to_string()
            } else {
                format!("{}: {t}", el.local())
            }
        }
    };
    clip(&s, 80)
}

/// A connector's end attached to an object the text shows by its id shows
/// as that object's connection site (§5.3), `s3.2`; one attached to a slot or
/// an object shown as `<keep/>` shows its point.
fn show_attachments(blocks: &mut [Block], attached: &[(usize, geom::Attachments)], base: usize) {
    fn ids(items: &[SlideItem], out: &mut Vec<(bool, u32)>) {
        for it in items {
            match it {
                SlideItem::Shape(s) => out.extend(num(&s.id).map(|n| (false, n))),
                SlideItem::Picture(p) => out.extend(num(&p.id).map(|n| (false, n))),
                SlideItem::Group(g) => {
                    out.extend(num(&g.id).map(|n| (true, n)));
                    ids(&g.items, out);
                }
                _ => {}
            }
        }
    }
    fn num(id: &str) -> Option<u32> {
        id.strip_prefix(['s', 'g'])?.parse().ok()
    }
    let mut shown: Vec<(bool, u32)> = vec![];
    for b in blocks.iter() {
        let Some(h) = b.head() else { continue };
        match (kind(h), &h.place) {
            (HeadKind::Shape { id, .. } | HeadKind::Picture { id, .. }, _) => shown.extend(num(id).map(|n| (false, n))),
            (HeadKind::Group { id, .. }, Some(hanji_core::Place::Group(g))) => {
                shown.extend(num(id).map(|n| (true, n)));
                ids(&g.items, &mut shown);
            }
            _ => {}
        }
    }
    for (b, [st, en]) in attached {
        let Some(Block::Head(h)) = blocks.get_mut(b - base) else { continue };
        let Some(hanji_core::Place::Line(e)) = &mut h.place else { continue };
        let at = |a: &Option<(u32, u32)>| {
            let (id, site) = (*a)?;
            let (group, _) = shown.iter().find(|x| x.1 == id)?;
            Some(hanji_format::Attach { group: *group, id, site })
        };
        (e.from_at, e.to_at) = (at(st), at(en));
    }
}
