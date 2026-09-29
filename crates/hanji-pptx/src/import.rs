//! Slides and notes pages → resolved model blocks + remainder entries.
//!
//! A slide is a `slide` head (its layout the label), then, in the slide's
//! z-order, a head and its paragraphs for each slot and shape with text, then
//! the notes slot. What becomes what:
//!
//! - The slide part and its notes page: a [`Kind::Slide`] entry each on the
//!   slide head, holding the part as it is but for the modelled shapes, which
//!   are stand-ins (`hanji-item`). Pictures, charts, tables, groups,
//!   connectors, shapes without text, transitions and animations stay there.
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

use hanji_core::presentation::{shape_head, slide_head, slot_head};
use hanji_core::{Block, Entry, KeepIds, Kind, ListItem, Meta, Para};
use hanji_format::{Atom, Inline, Keep, Marks, Unit};
use hanji_package::clip;
use hanji_package::xml::{canon, fp, is_blank, Element, Node, Scope};

use crate::deck::{LayoutInfo, SlotInfo};
use crate::pml::*;

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
    /// Shape-tree children kept as they are (pictures, charts, groups, …).
    pub kept: usize,
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
    /// Shapes shown as `<shape>`: `(id, name)`.
    pub shapes: Vec<(String, String)>,
    /// The part being read.
    part: String,
    /// The open list of the slot being read: the `lvl` of each open level.
    list: Option<Vec<u32>>,
    _deck: std::marker::PhantomData<&'a ()>,
}

/// What one shape-tree child is to the text.
enum What<'s> {
    Slot(&'s SlotInfo),
    KeepSlot(&'s SlotInfo),
    Latent(&'s SlotInfo),
    Shape(String, String),
    Other,
}

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
        for n in std::mem::take(&mut tree.children) {
            let Node::El(el) = n else {
                tree.children.push(n);
                continue;
            };
            let what = what(&el, layout, &used);
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
                    self.blocks.push(slot_head(&slot.name));
                    self.text_shape(&el, &slot.bullets, &k.to_string(), true)?;
                }
                What::KeepSlot(slot) => {
                    used.push(slot.name.clone());
                    k += 1;
                    self.stats.keep_slots += 1;
                    tree.children.push(Node::El(wrap(ITEM, &[("k", &k.to_string())], None)));
                    self.blocks.push(slot_head(&slot.name));
                    let bj = self.blocks.len();
                    let keep = self.keep_entry(Kind::Bkeep, &el, &[bj], None, &k.to_string());
                    self.blocks.push(Block::Keep(keep.id));
                }
                What::Shape(id, name) => {
                    k += 1;
                    self.stats.shapes += 1;
                    tree.children.push(Node::El(wrap(ITEM, &[("k", &k.to_string())], None)));
                    self.blocks.push(shape_head(&id, &name));
                    if !self.shapes.iter().any(|s| s.0 == id && s.1 == name) {
                        self.shapes.push((id, name));
                    }
                    self.text_shape(&el, &layout.other, &k.to_string(), false)?;
                }
            }
        }
        let fpv = canon(&without_stand_ins(&skel), &self.scope);
        let aux = vec![serde_json::to_string(&SlideInfo { ids, ..info }).unwrap()];
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
                self.blocks.push(slot_head("notes"));
                self.text_shape(&shape, &bullets, "notes", true)?;
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

    /// A shape's element (a `Shape` entry on the head just pushed) and its
    /// paragraphs. Without `lists` (a `<shape>`, whose text has no list
    /// items) a paragraph's bullet stays in the remainder.
    fn text_shape(&mut self, el: &Element, inherited: &[Bu; 9], k: &str, lists: bool) -> Result<(), String> {
        let hi = self.blocks.len() - 1;
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
        let f = self.fp(&[Some(&fshell)]);
        let meta = Meta { tag: shell.name.clone(), aux: vec![k.to_string(), self.part.clone()], ..Default::default() };
        self.entry(Kind::Shape, vec![shell.to_xml()], f, &[hi], None, None, meta);
        self.list = None;
        let bullets = if lists { bullets } else { [Bu::None; 9] };
        for p in &paras {
            let bi = self.blocks.len();
            let para = self.para(p, bi, &bullets, lists)?;
            self.blocks.push(Block::Para(para));
        }
        self.list = None;
        Ok(())
    }

    fn keep_entry(&mut self, kind: Kind, el: &Element, path: &[usize], pos: Option<usize>, k: &str) -> Keep {
        let fpv = self.fp(&[Some(el)]);
        let id = self.keep_ids.next(&fpv);
        let keep = Keep { id, kind: keep_kind(el), summary: summary(el) };
        let aux = if kind == Kind::Bkeep { vec![k.to_string(), self.part.clone()] } else { vec![] };
        let meta = Meta { keep: Some(keep.clone()), aux, ..Default::default() };
        self.entry(kind, vec![el.to_xml()], fpv, path, pos, pos.map(|p| p + 1), meta);
        keep
    }

    fn para(&mut self, p: &Element, bi: usize, bullets: &[Bu; 9], lists: bool) -> Result<Para, String> {
        let path = [bi];
        let ppr = p.child("a:pPr");
        let end = p.child("a:endParaRPr");
        let lvl: u32 = ppr.and_then(|x| x.get("lvl")).and_then(|v| v.parse().ok()).unwrap_or(0).min(8);
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
                    let ix = self.run(c, &path, &t, false);
                    texts.push((ix, t));
                }
                "a:br" if c.elements().all(|x| x.is("a:rPr")) => {
                    self.run(c, &path, "", true);
                }
                _ => {
                    let pos = self.buf.len();
                    let keep = self.keep_entry(Kind::Keep, c, &path, Some(pos), "");
                    self.buf.push(Unit { atom: Atom::Keep(keep), marks: Marks::NONE });
                }
            }
        }
        let mut content = Inline { units: std::mem::take(&mut self.buf), spans: vec![] };
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
        }
        content.normalize();
        self.stats.list_items += item.is_some() as usize;
        Ok(Para { style: String::new(), content, item })
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
    fn run(&mut self, r: &Element, path: &[usize], text: &str, br: bool) -> usize {
        let rpr = r.child("a:rPr");
        let marks = marks_of(rpr);
        let shell = r.shell();
        let mut rest = rpr.cloned();
        if let Some(x) = &mut rest {
            for a in MARK_ATTRS {
                x.remove_attr(a);
            }
        }
        let trivial =
            shell.attrs.is_empty() && rest.as_ref().is_none_or(|x| x.attrs.is_empty() && x.children.is_empty());
        let f = if trivial { String::new() } else { self.fp(&[Some(&shell), rest.as_ref()]) };
        let xml = std::iter::once(shell.to_xml()).chain(rpr.map(Element::to_xml)).collect();
        let start = self.buf.len();
        if br {
            self.buf.push(Unit { atom: Atom::Break, marks });
        } else {
            self.buf.extend(text.chars().map(|c| Unit { atom: Atom::Char(c), marks }));
        }
        let end = self.buf.len();
        let meta = Meta { marks, tag: r.name.clone(), ..Default::default() };
        self.entry(Kind::Run, xml, f, path, Some(start), Some(end), meta)
    }
}

/// What a shape-tree child is: a slot (placeholder of the layout with text,
/// or holding its object), a latent placeholder, a shape with text, or
/// something the text does not show.
fn what<'s>(el: &Element, layout: &'s LayoutInfo, used: &[String]) -> What<'s> {
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
    match (text, id) {
        (true, Some(id)) => {
            let name = c_nv_pr(el).and_then(|c| c.get("name")).unwrap_or_default();
            What::Shape(format!("s{id}"), name)
        }
        _ => What::Other,
    }
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
        "AlternateContent" | "m" => "math".into(),
        "r" => "text".into(),
        other => other.into(),
    }
}

fn summary(el: &Element) -> String {
    let s = match el.local() {
        "fld" => {
            let ty = el.get("type").unwrap_or_else(|| "field".into());
            format!("{ty}: {}", el.text_of(&["a:t"]))
        }
        "pic" | "graphicFrame" => {
            let c = c_nv_pr(el);
            let descr = c.and_then(|c| c.get("descr")).filter(|d| !d.trim().is_empty());
            let name = c.and_then(|c| c.get("name")).unwrap_or_default();
            let text = el.text_of(&["a:t"]);
            match (descr, text.trim().is_empty()) {
                (Some(d), _) => d,
                (None, false) => format!("{name}: {text}"),
                (None, true) => name,
            }
        }
        _ => {
            let t = el.text_of(&["a:t"]);
            if t.trim().is_empty() {
                el.local().to_string()
            } else {
                format!("{}: {t}", el.local())
            }
        }
    };
    clip(&s, 80)
}
