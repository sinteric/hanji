//! Resolved blocks + placed entries → the package: every slide and notes
//! page the text has, `presentation.xml` with its slide list, and the
//! relationships and content types that follow from new, deleted and moved
//! slides.
//!
//! A slide's shape tree is its stored skeleton with each stand-in replaced:
//! modelled shapes are written in the text's order, and everything the text
//! does not show stays before the modelled shape it preceded. A new slot is
//! made from the layout's placeholder (never geometry).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use hanji_core::presentation::{geom_of, kind, HeadKind};
use hanji_core::{Block, Entry, Head, Kind, Para, Part, Place, Remainder};
use hanji_format::{Atom, Crop, Geom, GroupItem, Inline, Marks, PictureItem, SlideItem};
use hanji_package::opc::{self, Rel};
use hanji_package::package;
use hanji_package::xml::{self, fragment, insert_ordered, Element, Node};

use crate::deck::{LayoutInfo, SlotInfo};
use crate::fill;
use crate::geom::{self, Frame};
use crate::import::{
    crop_of, group_item, object_geom, part_of_src, picture_item, shown_of, NotesInfo, Rels, SlideInfo, OBJECT_TAG,
};
use crate::members::{self, Styling};
use crate::pml::*;
use crate::text::{self, RunStyle, ThemeFonts};
use crate::DeckShell;
use hanji_format::inline_style::{self as istyle, TextStyle};

fn el(name: &str) -> Element {
    Element::new(name)
}

/// One slide of the text: its head, and each slot's or shape's head and blocks.
struct SlideG<'b> {
    head: usize,
    layout: &'b str,
    items: Vec<ItemG<'b>>,
}

struct ItemG<'b> {
    head: usize,
    kind: HeadKind<'b>,
    body: std::ops::Range<usize>,
}

fn group(blocks: &[Block]) -> Result<Vec<SlideG<'_>>, String> {
    let mut out: Vec<SlideG> = vec![];
    let mut k = 0;
    while k < blocks.len() {
        let Block::Head(h) = &blocks[k] else { return Err("a block outside any slide or slot".into()) };
        let end = (k + 1..blocks.len()).find(|&x| blocks[x].head().is_some()).unwrap_or(blocks.len());
        match kind(h) {
            HeadKind::Slide { layout } => out.push(SlideG { head: k, layout, items: vec![] }),
            other => out.last_mut().ok_or("a slot before the first slide")?.items.push(ItemG {
                head: k,
                kind: other,
                body: k + 1..end,
            }),
        }
        k = end;
    }
    Ok(out)
}

/// Files the host hands the write, by the name a picture's `src` gives
/// them (§5.3: a picture from a file).
pub type Files = BTreeMap<String, Vec<u8>>;

static NO_FILES: Files = BTreeMap::new();

pub struct Exporter<'a> {
    blocks: &'a [Block],
    shell: &'a DeckShell,
    parts: &'a [Part],
    files: &'a Files,
    by: HashMap<(usize, Kind), Vec<&'a Entry>>,
    keep: HashMap<&'a str, &'a Entry>,
    slide: HashMap<usize, &'a Entry>,
    notes: HashMap<usize, &'a Entry>,
}

/// What one slide writes.
struct SlideOut {
    info: SlideInfo,
    xml: Vec<u8>,
    /// Its relationships part, when it changed or is new.
    rels: Option<Vec<u8>>,
    new: bool,
    /// Image parts made from the host's files: part name, bytes, content type.
    media: Vec<(String, Vec<u8>, &'static str)>,
    notes: Option<NotesOut>,
}

struct NotesOut {
    part: String,
    xml: Vec<u8>,
    /// Relationships of a new notes page.
    rels: Option<Vec<u8>>,
}

impl<'a> Exporter<'a> {
    pub fn new(blocks: &'a [Block], rem: &'a Remainder, shell: &'a DeckShell) -> Self {
        let mut ex = Exporter {
            blocks,
            shell,
            parts: &rem.parts,
            files: &NO_FILES,
            by: HashMap::new(),
            keep: HashMap::new(),
            slide: HashMap::new(),
            notes: HashMap::new(),
        };
        for e in &rem.entries {
            let at = e.path.first().copied().unwrap_or(usize::MAX);
            match e.kind {
                Kind::Keep | Kind::Bkeep => {
                    if let Some(k) = &e.meta.keep {
                        ex.keep.insert(k.id.as_str(), e);
                    }
                }
                Kind::Slide if e.meta.tag == "notes" => {
                    ex.notes.insert(at, e);
                }
                Kind::Slide => {
                    ex.slide.insert(at, e);
                }
                _ => {}
            }
            ex.by.entry((at, e.kind)).or_default().push(e);
        }
        ex
    }

    /// The host's files a new picture or a changed `src` may name.
    pub fn with_files(mut self, files: &'a Files) -> Self {
        self.files = files;
        self
    }

    fn head(&self, it: &ItemG) -> &'a Head {
        match &self.blocks[it.head] {
            Block::Head(h) => h,
            _ => unreachable!("an item starts at its head"),
        }
    }

    fn at(&self, bi: usize, kind: Kind) -> Vec<&'a Entry> {
        self.by.get(&(bi, kind)).cloned().unwrap_or_default()
    }

    /// The package's parts after the edit.
    pub fn package(&self) -> Result<Vec<Part>, String> {
        let slides = group(self.blocks)?;
        let mut parts: Vec<Part> = self.parts.to_vec();
        let names: BTreeSet<String> = parts.iter().map(|p| p.name.clone()).collect();
        let mut numbers = Numbers::new(&names);
        let mut outs = vec![];
        let mut seen: HashSet<String> = HashSet::new();
        for s in &slides {
            let out = self.slide(s, &mut numbers)?;
            if !seen.insert(out.info.part.clone()) {
                return Err(format!("{} is written twice", out.info.part));
            }
            outs.push(out);
        }
        self.presentation(&mut parts, &outs)?;
        let template = parts.iter().find(|p| p.name == self.shell.pres_part).cloned().ok_or("no presentation part")?;
        let mut added: Vec<(String, String)> = vec![];
        for o in &outs {
            if put(&mut parts, &template, &o.info.part, o.xml.clone()) {
                added.push((o.info.part.clone(), CT_SLIDE.into()));
            }
            if let Some(r) = &o.rels {
                put(&mut parts, &template, &opc::rels_part(&o.info.part), r.clone());
            }
            for (name, data, ct) in &o.media {
                if put(&mut parts, &template, name, data.clone()) {
                    added.push((name.clone(), ct.to_string()));
                }
            }
            if let Some(n) = &o.notes {
                if put(&mut parts, &template, &n.part, n.xml.clone()) {
                    added.push((n.part.clone(), CT_NOTES.into()));
                }
                if let Some(r) = &n.rels {
                    put(&mut parts, &template, &opc::rels_part(&n.part), r.clone());
                }
            }
        }
        // Slides the text no longer has go, with what only they used.
        let kept: BTreeSet<&str> = outs.iter().map(|o| o.info.part.as_str()).collect();
        let deleted: Vec<&String> = self.shell.slides.iter().filter(|p| !kept.contains(p.as_str())).collect();
        // Only a deleted slide or a slide's changed relationships can leave parts unreached.
        let gone = if deleted.is_empty() && outs.iter().all(|o| o.rels.is_none()) {
            BTreeSet::new()
        } else {
            self.delete(&mut parts, &deleted)?
        };
        parts.retain(|p| !gone.contains(&p.name));
        if let Some(ct) = opc::content_types(&parts, &gone, &added) {
            put(&mut parts, &template, opc::CT_PART, ct);
        }
        Ok(parts)
    }

    // ------------------------------------------------------------ slides

    fn slide(&self, s: &SlideG, numbers: &mut Numbers) -> Result<SlideOut, String> {
        let layout =
            self.shell.deck.layout(s.layout).ok_or_else(|| format!("layout {:?} is not in the file", s.layout))?;
        let (mut info, skel, new) = match self.slide.get(&s.head) {
            Some(e) => {
                let info: SlideInfo = serde_json::from_str(&e.meta.aux[0]).map_err(|e| e.to_string())?;
                (info, fragment(&e.xml[0]), false)
            }
            None => {
                let part = numbers.next("ppt/slides/slide");
                let info = SlideInfo {
                    part,
                    prolog: "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n".into(),
                    layout: layout.part.clone(),
                    ..Default::default()
                };
                (info, new_slide(), true)
            }
        };
        let relayout = info.layout != layout.part;
        let part = info.part.clone();
        let items: Vec<&ItemG> = s.items.iter().filter(|i| i.kind != HeadKind::Slot { name: "notes" }).collect();
        let bullets_of = |it: &ItemG, el: &Element| -> [Bu; 9] {
            let own = levels_of(el.child("p:txBody").and_then(|t| t.child("a:lstStyle")));
            match it.kind {
                HeadKind::Slot { name } => over(own, layout.slot(name).map_or(layout.other, |x| x.bullets)),
                _ => over(own, layout.other),
            }
        };
        // The slide's relationships: a picture's image is one of them.
        let mut rels = if new {
            vec![Rel {
                id: "rId1".into(),
                ty: REL_LAYOUT.into(),
                target: opc::relative_target(&part, &layout.part),
                external: false,
            }]
        } else {
            opc::rels_of(self.parts, &part)
        };
        let mut rels_changed = new;
        let mut media: Vec<(String, Vec<u8>, &'static str)> = vec![];
        let mut xml_root = skel;
        let tree = xml_root
            .child_mut("p:cSld")
            .and_then(|c| c.child_mut("p:spTree"))
            .ok_or_else(|| format!("{part}: no shape tree"))?;
        let mut stands = stand_ins(std::mem::take(&mut tree.children));
        // Each item's element: from its entry (claiming its stand-in), then a
        // latent or emptied placeholder of its slot, else a new placeholder.
        let mut els: Vec<Option<Element>> = vec![None; items.len()];
        let mut claims: Vec<Option<usize>> = vec![None; items.len()];
        // Items new to this slide need a shape id of their own.
        let mut fresh = vec![false; items.len()];
        // The box the text showed for each item's element (§5.3), and which elements are new objects.
        let mut shown: Vec<Option<Geom>> = vec![None; items.len()];
        let mut created = vec![false; items.len()];
        for (n, it) in items.iter().enumerate() {
            let (e, k) = match self.keep_slot(it)? {
                Some(bk) => (Some(bk), bk.meta.aux.first()),
                None => match self.at(it.head, Kind::Shape).first() {
                    Some(sh) => (Some(*sh), sh.meta.aux.first()),
                    None => (None, None),
                },
            };
            let Some(e) = e else { continue };
            self.check_part(e, &part)?;
            if e.meta.aux.get(1) == Some(&part) {
                claims[n] =
                    k.and_then(|k| stands.iter().position(|x| matches!(x, Stand::Item { k: kk, .. } if kk == k)));
            } else {
                fresh[n] = true;
            }
            els[n] = Some(fragment(&e.xml[0]));
            shown[n] = shown_of(&e.meta.aux);
        }
        for (n, it) in items.iter().enumerate() {
            if els[n].is_some() {
                continue;
            }
            let head = self.head(it);
            match it.kind {
                HeadKind::Shape { id: "", .. } => {
                    let g = geom_of(head).ok_or("a new text box needs its box")?;
                    els[n] = Some(new_text_box(&g));
                    (fresh[n], created[n]) = (true, true);
                }
                HeadKind::Line { id: "", .. } => {
                    let Some(Place::Line(ends)) = &head.place else { return Err("a line without its ends".into()) };
                    els[n] = Some(new_line(ends));
                    (fresh[n], created[n]) = (true, true);
                }
                HeadKind::Picture { id: "", .. } => {
                    let Some(Place::Picture(pic)) = &head.place else {
                        return Err("a picture without its image".into());
                    };
                    let g = pic.geom.ok_or("a new picture needs its box")?;
                    els[n] = Some(new_picture(&g));
                    (fresh[n], created[n]) = (true, true);
                }
                HeadKind::Picture { id, .. } => {
                    return Err(format!("<picture id=\"{id}\"> is not a picture of this slide's file: keep a picture's id and name as they are, and write a new picture without them"));
                }
                HeadKind::Shape { id, .. } => {
                    return Err(format!("<shape id=\"{id}\"> is not a shape of this slide's file: keep a shape's id and name as they are, and write a new text box without them"));
                }
                HeadKind::Line { id, .. } => {
                    return Err(format!(
                        "<line id=\"{id}\"> is not a line of this file: write a new line without an id"
                    ));
                }
                HeadKind::Group { id, .. } => {
                    return Err(format!(
                        "<group id=\"{id}\"> is not a group of this file: groups are never created here"
                    ));
                }
                HeadKind::Slot { name } => {
                    let taken: Vec<usize> = claims.iter().flatten().copied().collect();
                    let found = stands.iter().enumerate().find(|(x, st)| !taken.contains(x) && st.slot() == Some(name));
                    match found {
                        Some((x, st)) => {
                            let el = st.element().ok_or("a stand-in without its placeholder")?;
                            shown[n] = geom::own(&el).or(layout.slot(name).and_then(|s| s.geom));
                            els[n] = Some(el);
                            claims[n] = Some(x);
                        }
                        None => {
                            let objects: Vec<&Element> = items
                                .iter()
                                .zip(&els)
                                .filter(|(i, _)| i.kind == HeadKind::Object)
                                .filter_map(|(_, e)| e.as_ref())
                                .collect();
                            if occupied(&stands, &objects, layout, name) {
                                return Err(format!("::{name}:: is filled by a placeholder the text does not show (in a group, or holding an object): the slot cannot be written twice"));
                            }
                            let slot = layout.need_slot(name)?;
                            els[n] = Some(new_placeholder(slot));
                            shown[n] = slot.geom;
                            fresh[n] = true;
                        }
                    }
                }
                // A slide head is no item; an object always has its entry (keep_slot).
                HeadKind::Slide { .. } | HeadKind::Object => unreachable!(),
            }
        }
        // Text, placeholder binding and names.
        let mut out_items: Vec<Element> = vec![];
        for (n, it) in items.iter().enumerate() {
            let mut e = els[n].take().unwrap();
            if let HeadKind::Slot { name } = it.kind {
                if relayout && placeholder(&e).is_some() {
                    let slot = layout.need_slot(name)?;
                    set_ph(&mut e, slot);
                }
            }
            if let HeadKind::Shape { name, .. } = it.kind {
                if let Some(c) = c_nv_pr_mut(&mut e) {
                    if c.get("name").as_deref() != Some(name) {
                        c.set("name", name);
                    }
                }
            }
            let text = matches!(it.kind, HeadKind::Slot { .. } | HeadKind::Shape { .. });
            // Its fill (§5.3): written where the text changes what it shows.
            if text && e.is("p:sp") && self.keep_slot(it)?.is_none() {
                let old = self.shell.deck.layouts.iter().find(|l| l.part == info.layout);
                let (now, then) = match it.kind {
                    HeadKind::Slot { name } => (
                        layout.slot(name).and_then(|s| s.fill.as_ref()),
                        old.and_then(|o| o.slot(name)).and_then(|s| s.fill.as_ref()),
                    ),
                    _ => (None, None),
                };
                let then = if old.is_some() { then } else { now };
                let p = fill::Parents { now, then, theme: &layout.fills };
                fill::write(&mut e, self.head(it).look.fill.as_deref(), p)?;
            }
            if text && self.keep_slot(it)?.is_none() {
                let bullets = bullets_of(it, &e);
                let lists = matches!(it.kind, HeadKind::Slot { .. });
                // What its runs inherit (§5.3 text formatting).
                let inherited = match it.kind {
                    HeadKind::Slot { name } => layout.slot(name).map(|x| &x.text),
                    _ => Some(&layout.other_text),
                };
                let base = inherited.map(|t| text::base(&e, t, &layout.fonts));
                // What they inherited from the layout the slide had, when it changes.
                let before = self.shell.deck.layouts.iter().find(|l| l.part == info.layout).map(|old| {
                    let t = match it.kind {
                        HeadKind::Slot { name } => old.slot(name).map_or(&old.other_text, |x| &x.text),
                        _ => &old.other_text,
                    };
                    text::base(&e, t, &old.fonts)
                });
                let inherit = base.as_ref().map(|now| Inherit {
                    now,
                    then: before.as_ref().unwrap_or(now),
                    fonts: &layout.fonts,
                });
                let paras = self.paras(it, &bullets, &part, lists, inherit)?;
                if e.child("p:txBody").is_none() && !paras.is_empty() {
                    if !e.is("p:sp") {
                        return Err("only a shape can hold text".into());
                    }
                    e.children.push(Node::El(fragment("<p:txBody><a:bodyPr/><a:lstStyle/></p:txBody>")));
                }
                if let Some(tx) = e.child_mut("p:txBody") {
                    let had = tx.elements().any(|p| p.is("a:p"));
                    // A shape without text keeps its empty paragraphs; a text body has at least one.
                    if !paras.is_empty() || !had || !it.body.is_empty() {
                        tx.children.retain(|x| !matches!(x, Node::El(p) if p.is("a:p")));
                        if paras.is_empty() {
                            tx.children.push(Node::El(Element::new("a:p")));
                        }
                        tx.children.extend(paras.into_iter().map(Node::El));
                    }
                } else if matches!(it.kind, HeadKind::Slot { .. }) {
                    return Err("a placeholder without p:txBody cannot hold text".into());
                }
            }
            out_items.push(e);
        }
        // Geometry (§5.3): each written box against the one the text showed,
        // a changed number snapped to another object's that shows the same.
        let boxes: Vec<geom::Written> = items
            .iter()
            .enumerate()
            .map(|(n, it)| {
                let e = &out_items[n];
                let slot = match it.kind {
                    HeadKind::Slot { name } => layout.slot(name).and_then(|s| s.geom),
                    _ => None,
                };
                let stored = (!created[n]).then(|| shown[n].or(object_geom(e)).or(slot)).flatten();
                match &self.head(it).place {
                    Some(Place::Box(g)) => geom::Written { stored, written: Some(*g) },
                    Some(Place::Picture(p)) => geom::Written { stored, written: p.geom },
                    Some(Place::Group(g)) => geom::Written { stored: geom::own(e), written: g.geom },
                    _ => geom::Written::default(),
                }
            })
            .collect();
        let mut moved: Vec<(u32, String)> = vec![];
        let mut lines_written: HashSet<u32> = HashSet::new();
        for (n, it) in items.iter().enumerate() {
            let e = &mut out_items[n];
            let head = self.head(it);
            let snapped = geom::snapped(&boxes, n);
            let what = match (&it.kind, self.blocks.get(it.body.start)) {
                (HeadKind::Object, Some(Block::Keep(id))) => format!("<keep id=\"{id}\">"),
                _ => item_label(it),
            };
            let mut changed = vec![];
            match (&it.kind, &head.place) {
                (HeadKind::Line { .. }, Some(Place::Line(w))) => {
                    let st = geom::own(e).unwrap_or_default();
                    let ends = geom::ends_of(&st);
                    if !created[n] && !w.shows_as(&ends) {
                        geom::write(e, &geom::box_of(&ends.merged(w), st.rot)).map_err(|m| format!("{what}: {m}"))?;
                        lines_written.extend(geom::shape_id(e));
                    }
                }
                (HeadKind::Group { .. }, Some(Place::Group(w))) => {
                    let w = GroupItem { geom: snapped, ..w.clone() };
                    let map = rels_map(&rels, &part);
                    changed = apply_group(e, &w, &geom::Frame::SLIDE, &map, Some(Styling::of(layout)))
                        .map_err(|m| format!("{what}: {m}"))?;
                    lines_written.extend(changed.iter().filter(|x| x.1).map(|x| x.0));
                }
                (kind, place) => {
                    let written = match place {
                        Some(Place::Box(_)) | Some(Place::Picture(_)) => snapped,
                        _ => None,
                    };
                    if let Some(Place::Picture(pic)) = place {
                        self.picture(e, pic, &part, &mut rels, &mut rels_changed, &mut media, numbers)
                            .map_err(|m| format!("{what}: {m}"))?;
                    }
                    if created[n] {
                        // A new text box, made at its box as written.
                        if let Some(g) = written.filter(|g| Some(*g) != geom::own(e)) {
                            geom::write(e, &g).map_err(|m| format!("{what}: {m}"))?;
                        }
                        continue;
                    }
                    let slot = match kind {
                        HeadKind::Slot { name } => Some(layout.slot(name).and_then(|s| s.geom)),
                        _ => None,
                    };
                    if place_box(e, shown[n], written, slot).map_err(|m| format!("{what}: {m}"))? {
                        changed.push((geom::shape_id(e).unwrap_or(0), false));
                    }
                }
            }
            for (id, _) in changed {
                moved.push((id, what.clone()));
            }
        }
        assign_ids(&stands, &mut out_items, &fresh);
        name_new(&mut out_items, &created);
        check_connectors(&stands, &out_items, &moved, &lines_written)?;
        let tree_children =
            assemble(std::mem::take(&mut stands), &claims, out_items, &|slot: &str| layout.slot(slot).is_some())?;
        let tree = xml_root.child_mut("p:cSld").and_then(|c| c.child_mut("p:spTree")).unwrap();
        tree.children = tree_children;
        check_timing(&xml_root, &info)?;
        if relayout && !new {
            let r = rels
                .iter_mut()
                .find(|r| r.ty == REL_LAYOUT)
                .ok_or_else(|| format!("{part} has no layout relationship"))?;
            r.target = opc::relative_target(&part, &layout.part);
            rels_changed = true;
            info.layout = layout.part.clone();
        }
        // A relationship the slide named and no longer names (a deleted
        // object's picture, chart or media, a deleted link) goes; its part
        // goes with it when nothing else reaches it.
        if !new {
            let now = named_rel_ids(&xml_root);
            let n = rels.len();
            rels.retain(|r| !info.named.contains(&r.id) || now.contains(&r.id));
            rels_changed |= rels.len() != n;
        }
        let notes = self.notes_page(s, &part, &mut rels, &mut rels_changed, numbers)?;
        let xml = format!("{}{}{}", info.prolog, xml_root.to_xml(), info.epilog).into_bytes();
        let rels_data = if rels_changed {
            Some(if new { opc::write_rels(&rels) } else { rewrite_rels(self.parts, &part, &rels)? })
        } else {
            None
        };
        Ok(SlideOut { info, xml, rels: rels_data, new, notes, media })
    }

    /// A picture's image, crop, mask and alternative text as the text
    /// writes them (§5.3): each written into its own XML child only where
    /// it differs from what the element stores.
    #[allow(clippy::too_many_arguments)]
    fn picture(
        &self,
        e: &mut Element,
        w: &PictureItem,
        part: &str,
        rels: &mut Vec<Rel>,
        rels_changed: &mut bool,
        media: &mut Vec<(String, Vec<u8>, &'static str)>,
        numbers: &mut Numbers,
    ) -> Result<(), String> {
        let map = rels_map(rels, part);
        let st = picture_item(e, &geom::Frame::SLIDE, &map);
        if st.is_none() && !w.id.is_empty() {
            return Err("this picture is kept as it is (it is not shown as a <picture/>)".into());
        }
        // A new picture starts with no image, crop, mask or alternative text.
        let st =
            st.unwrap_or_else(|| PictureItem { src: String::new(), crop: None, mask: None, alt: None, ..w.clone() });
        if st.src != w.src {
            let target = self.image_part(&w.src, media, numbers)?;
            let rel = target_rel(rels, part, &target, rels_changed);
            let blip =
                e.child_mut("p:blipFill").and_then(|b| b.child_mut("a:blip")).ok_or("the picture has no a:blip")?;
            blip.set("r:embed", &rel);
        }
        if !w.crop.unwrap_or_default().shows_as(&st.crop.unwrap_or_default()) {
            let bf = e.child_mut("p:blipFill").ok_or("the picture has no p:blipFill")?;
            let stored = bf.child("a:srcRect").map(crop_of).unwrap_or_default();
            let c = stored.merged(&w.crop.unwrap_or_default());
            set_crop(bf, &c);
        }
        if st.mask != w.mask {
            let sp = e.child_mut("p:spPr").ok_or("the picture has no p:spPr")?;
            set_preset(sp, w.mask.as_deref().unwrap_or("rect"));
        }
        if st.alt != w.alt {
            let c = c_nv_pr_mut(e).ok_or("the picture has no p:cNvPr")?;
            match &w.alt {
                Some(a) => c.set("descr", a),
                None => {
                    c.remove_attr("descr");
                }
            }
        }
        Ok(())
    }

    /// The part a picture's `src` names: one of the package's, else a new
    /// image part from the host's file of that name.
    fn image_part(
        &self,
        src: &str,
        media: &mut Vec<(String, Vec<u8>, &'static str)>,
        numbers: &mut Numbers,
    ) -> Result<String, String> {
        let p = part_of_src(src);
        if self.parts.iter().any(|x| x.name == p) || media.iter().any(|m| m.0 == p) {
            return Ok(p);
        }
        if let Some(done) = numbers.host.get(src) {
            return Ok(done.clone());
        }
        let Some(data) = self.files.get(src) else {
            return Err(format!("src=\"{src}\" is neither an image of this file (such as media/image1.png, as another picture shows it) nor a file handed to this write: a picture from a file needs the host to hand over the image"));
        };
        let (ext, ct) = image_type(data).ok_or_else(|| format!("{src} is not a PNG, JPEG, GIF or BMP image"))?;
        let name = numbers.next_media(ext);
        numbers.host.insert(src.to_string(), name.clone());
        media.push((name.clone(), data.clone(), ct));
        Ok(name)
    }

    /// The `Bkeep` entry of a slot that holds a placeholder's object, or of
    /// a slide object (rule 8). Neither goes where the other belongs.
    fn keep_slot(&self, it: &ItemG) -> Result<Option<&'a Entry>, String> {
        let body = &self.blocks[it.body.clone()];
        let keeps: Vec<&Entry> = body
            .iter()
            .filter_map(|b| if let Block::Keep(id) = b { self.keep.get(id.as_str()).copied() } else { None })
            .filter(|e| e.kind == Kind::Bkeep)
            .collect();
        let object = it.kind == HeadKind::Object;
        match (keeps.as_slice(), body.len()) {
            ([], _) if object => Err("an object's <keep/> line stands for an object of the file".into()),
            ([], _) => Ok(None),
            ([e], 1) if object && e.meta.tag != OBJECT_TAG => Err(format!(
                "placeholder {} is the object of a slot: keep its <keep/> line directly after its slot's marker",
                keep_id(e)
            )),
            ([e], 1) if !object && e.meta.tag == OBJECT_TAG => Err(format!(
                "placeholder {} is an object of the slide, not a placeholder's: it cannot fill a slot; keep its <keep/> line outside the slot's text",
                keep_id(e)
            )),
            ([e], 1) => Ok(Some(*e)),
            _ => Err("a placeholder's object is the whole text of its slot: keep its <keep/> alone in the slot".into()),
        }
    }

    /// An entry whose XML refers to its part's relationships stays in its part.
    fn check_part(&self, e: &Entry, part: &str) -> Result<(), String> {
        match &e.meta.part {
            Some(p) if p != part => Err(format!(
                "content with a picture, link or other reference to {p} cannot move to {part}: move it in PowerPoint, or leave it where it is"
            )),
            _ => Ok(()),
        }
    }

    // ------------------------------------------------------------ paragraphs

    /// The `a:p` elements of an item's paragraphs. Without `lists` (a
    /// `<shape>`, whose text has no list items) each paragraph keeps its
    /// bullet and level as they are.
    fn paras(
        &self,
        it: &ItemG,
        bullets: &[Bu; 9],
        part: &str,
        lists: bool,
        base: Option<Inherit>,
    ) -> Result<Vec<Element>, String> {
        let mut out = vec![];
        // Text level → lvl of the open items (as import reads them back).
        let mut open: Vec<u32> = vec![];
        let lang = self.lang_of(it);
        for bi in it.body.clone() {
            let p = match &self.blocks[bi] {
                Block::Para(p) => p,
                Block::Keep(id) => {
                    return Err(format!("placeholder {id} holds a slot's object and cannot sit among paragraphs"))
                }
                _ => return Err("a table cannot be written in a slide yet".into()),
            };
            let ppr_e = self.at(bi, Kind::Ppr).first().copied();
            if let Some(e) = ppr_e {
                self.check_part(e, part)?;
            }
            let stored_lvl: u32 = ppr_e.and_then(|e| e.meta.aux.first()).and_then(|v| v.parse().ok()).unwrap_or(0);
            if !lists {
                out.push(self.para(p, bi, ppr_e, None, bullets, &lang, part, base)?);
                continue;
            }
            let lvl = match p.item {
                None => {
                    open.clear();
                    stored_lvl
                }
                Some(it) => {
                    let t = it.level;
                    let cand = match ppr_e.and_then(|e| e.meta.item) {
                        Some(si) => (stored_lvl as i64 + t as i64 - si.level as i64).clamp(0, 8) as u32,
                        None if ppr_e.is_some() => stored_lvl,
                        None => t as u32,
                    };
                    if it.first {
                        open.clear();
                    }
                    let sib = open.get(t).copied();
                    let parent = t.checked_sub(1).and_then(|k| open.get(k)).copied();
                    let fits = |l: u32| parent.is_none_or(|lp| l > lp) && sib.is_none_or(|ls| l <= ls);
                    let l = if fits(cand) { cand } else { sib.unwrap_or_else(|| parent.map_or(0, |lp| lp + 1)) };
                    if l > 8 {
                        return Err("a slide's lists nest at most nine levels".into());
                    }
                    open.truncate(t);
                    open.push(l);
                    l
                }
            };
            out.push(self.para(p, bi, ppr_e, Some(lvl), bullets, &lang, part, base)?);
        }
        Ok(out)
    }

    /// The language new runs of an item take: its first run's.
    fn lang_of(&self, it: &ItemG) -> String {
        for bi in it.body.clone() {
            for r in self.at(bi, Kind::Run) {
                if let Some(l) = r.xml.get(1).map(|x| fragment(x)).and_then(|x| x.get("lang")) {
                    return l;
                }
            }
        }
        "en-US".into()
    }

    #[allow(clippy::too_many_arguments)]
    fn para(
        &self,
        p: &Para,
        bi: usize,
        ppr_e: Option<&Entry>,
        lvl: Option<u32>,
        bullets: &[Bu; 9],
        lang: &str,
        part: &str,
        base: Option<Inherit>,
    ) -> Result<Element, String> {
        let (mut pel, mut ppr, end) = match ppr_e {
            Some(e) => {
                let frags: Vec<Element> = e.xml[1..].iter().map(|x| fragment(x)).collect();
                let ppr = frags.iter().find(|x| x.is("a:pPr")).cloned();
                let end = frags.iter().find(|x| x.is("a:endParaRPr")).cloned();
                (fragment(&e.xml[0]), ppr, end)
            }
            None => (el("a:p"), None, None),
        };
        // Level and bullet as the text says (a shape's paragraphs keep theirs).
        if let Some(lvl) = lvl {
            let stored_lvl: u32 = ppr_e.and_then(|e| e.meta.aux.first()).and_then(|v| v.parse().ok()).unwrap_or(0);
            if lvl != stored_lvl || ppr_e.is_none() && lvl > 0 {
                let x = ppr.get_or_insert_with(|| el("a:pPr"));
                if lvl == 0 {
                    x.remove_attr("lvl");
                } else {
                    x.set("lvl", &lvl.to_string());
                }
            }
            let want = match p.item {
                Some(it) if it.ordered => Bu::Number,
                Some(_) => Bu::Bullet,
                None => Bu::None,
            };
            set_bullet(&mut ppr, lvl, want, bullets);
        }
        if let Some(x) = ppr {
            pel.children.push(Node::El(x));
        }
        let stored_lvl: u32 = ppr_e.and_then(|e| e.meta.aux.first()).and_then(|v| v.parse().ok()).unwrap_or(0);
        let under = base.map(|b| b.at(lvl.unwrap_or(stored_lvl).min(8) as usize));
        self.runs(&mut pel, &p.content, bi, lang, part, under)?;
        if let Some(x) = end {
            pel.children.push(Node::El(x));
        }
        Ok(pel)
    }

    fn runs(
        &self,
        pel: &mut Element,
        c: &Inline,
        bi: usize,
        lang: &str,
        part: &str,
        under: Option<Under>,
    ) -> Result<(), String> {
        let n = c.units.len();
        let runs = self.at(bi, Kind::Run);
        let mut owner: Vec<Option<&Entry>> = vec![None; n];
        let mut zruns = vec![];
        for r in &runs {
            self.check_part(r, part)?;
            let (s, e) = (r.start.unwrap_or(0), r.end.unwrap_or(0));
            if s == e {
                zruns.push(*r);
            }
            for slot in owner.iter_mut().take(e.min(n)).skip(s) {
                *slot = Some(r);
            }
        }
        // Marks as the text states them: formatting spans split them where
        // the text writes their brackets (§5.3), each on shown text.
        let as_written = istyle::lift(&[c]).2.remove(0);
        let eff = as_written.written_marks(&|k| owner[k].map_or(Marks::NONE, |o| o.meta.marks));
        let mut keeps: BTreeMap<usize, &Entry> = BTreeMap::new();
        for (k, u) in c.units.iter().enumerate() {
            if let Atom::Keep(kp) = &u.atom {
                let e = self
                    .keep
                    .get(kp.id.as_str())
                    .copied()
                    .ok_or_else(|| format!("placeholder {} has no remainder entry", kp.id))?;
                if e.kind == Kind::Bkeep {
                    return Err(format!("placeholder {} holds a slot's object and cannot sit in a paragraph", kp.id));
                }
                self.check_part(e, part)?;
                keeps.insert(k, e);
            }
        }
        let zat: HashSet<usize> = zruns.iter().map(|z| z.start.unwrap_or(0)).collect();
        // The formatting each unit's run is written with (§5.3): what the
        // text states for shown text; a space or break in a run with shown
        // text goes with that text, one of a new run with what it shows, and
        // one of a run with no shown text keeps the run's (`None`).
        let want = istyle::unit_styles(c);
        let wanted: Vec<Option<TextStyle>> = (0..n)
            .map(|k| {
                if under.is_none() || istyle::visible(&c.units[k].atom) {
                    return Some(want[k].clone());
                }
                let Some(o) = owner[k] else { return Some(want[k].clone()) };
                let mine = |j: &usize| owner[*j].is_some_and(|x| x.id == o.id);
                let shows = |j: &usize| istyle::visible(&c.units[*j].atom);
                let prev = (0..k).rev().take_while(mine).find(shows);
                let next = (k + 1..n).take_while(mine).find(shows);
                prev.or(next).map(|j| want[j].clone())
            })
            .collect();
        let key = |k: usize| (owner[k].map(|o| o.id), eff[k], &wanted[k]);
        let mut segs: Vec<(usize, usize)> = vec![];
        let mut k = 0;
        while k < n {
            if keeps.contains_key(&k) {
                k += 1;
                continue;
            }
            let a = k;
            k += 1;
            while k < n && !keeps.contains_key(&k) && !zat.contains(&k) && key(k) == key(a) {
                k += 1;
            }
            segs.push((a, k));
        }
        enum It<'e> {
            Zrun(&'e Entry),
            Keep(&'e Entry),
            Seg(usize, usize),
        }
        let mut items: Vec<(usize, u8, u64, It)> = vec![];
        for z in &zruns {
            items.push((z.start.unwrap_or(0), 0, z.seq, It::Zrun(z)));
        }
        for (k, e) in &keeps {
            items.push((*k, 1, e.seq, It::Keep(e)));
        }
        for &(a, b) in &segs {
            items.push((a, 1, owner[a].map_or(u64::MAX, |o| o.seq), It::Seg(a, b)));
        }
        items.sort_by_key(|t| (t.0, t.1, t.2));
        for (_, _, _, it) in items {
            match it {
                It::Zrun(z) => {
                    let mut r = fragment(&z.xml[0]);
                    if let Some(x) = z.xml.get(1) {
                        r.children.push(Node::El(fragment(x)));
                    }
                    if r.is("a:r") {
                        // Its own text: whitespace the text shows as an empty paragraph.
                        let t = z.meta.aux.first().filter(|_| n == 0).cloned().unwrap_or_default();
                        let mut te = el("a:t");
                        if !t.is_empty() {
                            te.children.push(Node::Text(xml::escape_text(&t)));
                        }
                        r.children.push(Node::El(te));
                    }
                    pel.children.push(Node::El(r));
                }
                It::Keep(e) => pel.children.push(Node::El(fragment(&e.xml[0]))),
                It::Seg(a, b) => self.seg(pel, c, &eff, a, b, owner[a], lang, under.zip(wanted[a].as_ref()))?,
            }
        }
        Ok(())
    }

    /// Units `a..b` of one run: `a:r` elements, and `a:br` for line breaks.
    #[allow(clippy::too_many_arguments)]
    /// `style`: what the run inherits and the formatting the text writes for it.
    #[allow(clippy::too_many_arguments)]
    fn seg(
        &self,
        pel: &mut Element,
        c: &Inline,
        eff: &[Marks],
        a: usize,
        b: usize,
        own: Option<&Entry>,
        lang: &str,
        style: Option<(Under, &TextStyle)>,
    ) -> Result<(), String> {
        let text: String =
            c.units[a..b].iter().filter_map(|u| if let Atom::Char(ch) = u.atom { Some(ch) } else { None }).collect();
        let (shell, rpr) = match own {
            Some(o) => {
                let s = fragment(&o.xml[0]);
                let shell = if s.is("a:r") { s } else { el("a:r") };
                (shell, o.xml.get(1).map(|x| fragment(x)))
            }
            None => {
                let mut r = el("a:rPr");
                if text.chars().any(is_hangul) {
                    r = r.with_attr("lang", "ko-KR").with_attr("altLang", "en-US");
                } else {
                    r = r.with_attr("lang", lang);
                }
                (el("a:r"), Some(r))
            }
        };
        let mut rpr = rpr.map(|mut r| {
            set_marks(&mut r, eff[a]);
            r
        });
        // Text formatting (§5.3): what the text states and the run does not
        // show where it now stands is written; the rest stays as it is.
        if let Some((u, want)) = style {
            // What the text leaves unsaid, the run inherits.
            let mut want = want.over(&u.now.shown());
            let own = rpr.as_ref().map_or_else(RunStyle::default, |r| RunStyle::of(r, u.fonts));
            let had = own.over(u.now).shown();
            // A colour kept as the file stores it (`*`, a gradient…) is not
            // written from the text: one the run showed under the slide's old
            // layout is copied from there.
            let then = own.over(u.then);
            let mut pin = None;
            if want.color.as_ref().is_some_and(|c| text::is_kept(c)) && want.color == then.color {
                if want.color != had.color {
                    pin = then.fill.clone();
                }
                want.color = had.color.clone();
            }
            if had != want || pin.is_some() {
                let r = rpr.get_or_insert_with(|| el("a:rPr").with_attr("lang", lang));
                text::write(r, &want, &had, u.now)?;
                if let Some(f) = pin {
                    text::set_fill(r, &f);
                }
            }
        }
        let mut buf = String::new();
        let flush = |buf: &mut String, pel: &mut Element| {
            if buf.is_empty() {
                return;
            }
            let mut r = shell.clone();
            if let Some(x) = &rpr {
                r.children.push(Node::El(x.clone()));
            }
            let mut t = el("a:t");
            t.children.push(Node::Text(xml::escape_text(buf)));
            r.children.push(Node::El(t));
            pel.children.push(Node::El(r));
            buf.clear();
        };
        for u in &c.units[a..b] {
            match &u.atom {
                Atom::Char(ch) => buf.push(*ch),
                Atom::Break => {
                    flush(&mut buf, pel);
                    let mut br = el("a:br");
                    if let Some(x) = &rpr {
                        br.children.push(Node::El(x.clone()));
                    }
                    pel.children.push(Node::El(br));
                }
                _ => {}
            }
        }
        flush(&mut buf, pel);
        Ok(())
    }

    // ------------------------------------------------------------ notes

    fn notes_page(
        &self,
        s: &SlideG,
        slide_part: &str,
        slide_rels: &mut Vec<Rel>,
        rels_changed: &mut bool,
        numbers: &mut Numbers,
    ) -> Result<Option<NotesOut>, String> {
        let item = s.items.iter().find(|i| i.kind == HeadKind::Slot { name: "notes" });
        let entry = self.notes.get(&s.head).copied();
        let (info, skel, new) = match entry {
            Some(e) => {
                let info: NotesInfo = serde_json::from_str(&e.meta.aux[0]).map_err(|e| e.to_string())?;
                (info, fragment(&e.xml[0]), false)
            }
            None if item.is_none() => return Ok(None),
            None => {
                let master = self.shell.deck.notes.as_ref().ok_or(
                    "this file has no notes master, so a slide that had no notes cannot get them: remove the ::notes:: slot",
                )?;
                let _ = master;
                let part = numbers.next("ppt/notesSlides/notesSlide");
                let info = NotesInfo {
                    part,
                    prolog: "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n".into(),
                    epilog: String::new(),
                };
                (info, new_notes(), true)
            }
        };
        let mut root = skel;
        let tree = root
            .child_mut("p:cSld")
            .and_then(|c| c.child_mut("p:spTree"))
            .ok_or("a notes page without a shape tree")?;
        let mut stands = stand_ins(std::mem::take(&mut tree.children));
        let mut items = vec![];
        let mut claims = vec![];
        let mut fresh = vec![];
        if let Some(it) = item {
            let mut claim = None;
            let mut body = match self.at(it.head, Kind::Shape).first() {
                Some(sh) => {
                    self.check_part(sh, &info.part)?;
                    if sh.meta.aux.get(1) == Some(&info.part) {
                        claim = stands.iter().position(|x| matches!(x, Stand::Item { k, .. } if k == "notes"));
                    }
                    fragment(&sh.xml[0])
                }
                None => match stands.iter().position(|x| x.slot() == Some("notes")) {
                    Some(x) => {
                        claim = Some(x);
                        stands[x].element().unwrap()
                    }
                    None => new_notes_body(),
                },
            };
            fresh.push(claim.is_none());
            let bullets = over(
                levels_of(body.child("p:txBody").and_then(|t| t.child("a:lstStyle"))),
                self.shell.deck.notes.as_ref().map_or([Bu::Unset; 9], |n| n.bullets),
            );
            let paras = self.paras(it, &bullets, &info.part, true, None)?;
            let tx = body.child_mut("p:txBody").ok_or("the notes placeholder has no p:txBody")?;
            tx.children.retain(|x| !matches!(x, Node::El(p) if p.is("a:p")));
            tx.children.extend(paras.into_iter().map(Node::El));
            items.push(body);
            claims.push(claim);
        }
        assign_ids(&stands, &mut items, &fresh);
        let children = assemble(std::mem::take(&mut stands), &claims, items, &|slot| slot == "notes")?;
        let tree = root.child_mut("p:cSld").and_then(|c| c.child_mut("p:spTree")).unwrap();
        tree.children = children;
        let xml = format!("{}{}{}", info.prolog, root.to_xml(), info.epilog).into_bytes();
        let rels = if new {
            let master = &self.shell.deck.notes.as_ref().unwrap().part;
            slide_rels.push(Rel {
                id: opc::free_rel_id(slide_rels),
                ty: REL_NOTES.into(),
                target: opc::relative_target(slide_part, &info.part),
                external: false,
            });
            *rels_changed = true;
            Some(opc::write_rels(&[
                Rel {
                    id: "rId1".into(),
                    ty: REL_NOTES_MASTER.into(),
                    target: opc::relative_target(&info.part, master),
                    external: false,
                },
                Rel {
                    id: "rId2".into(),
                    ty: REL_SLIDE.into(),
                    target: opc::relative_target(&info.part, slide_part),
                    external: false,
                },
            ]))
        } else {
            None
        };
        Ok(Some(NotesOut { part: info.part, xml, rels }))
    }

    // ------------------------------------------------------------ presentation.xml

    fn presentation(&self, parts: &mut [Part], outs: &[SlideOut]) -> Result<(), String> {
        let sh = self.shell;
        let mut root = fragment(&sh.pres);
        let mut rels = opc::rels_of(parts, &sh.pres_part);
        let before = rels.clone();
        // Relationships of slides that stay, by part; new slides get new ones.
        let rid_of: HashMap<String, String> = rels
            .iter()
            .filter(|r| r.ty == REL_SLIDE)
            .map(|r| (opc::resolve_target(&sh.pres_part, &r.target), r.id.clone()))
            .collect();
        let mut max_id: u32 = sh.slide_ids.iter().copied().max().unwrap_or(255).max(255);
        let mut ids = vec![];
        let mut list = vec![];
        let kept: BTreeSet<&str> = outs.iter().map(|o| o.info.part.as_str()).collect();
        rels.retain(|r| r.ty != REL_SLIDE || kept.contains(opc::resolve_target(&sh.pres_part, &r.target).as_str()));
        for o in outs {
            let e = if o.new {
                let rid = opc::free_rel_id(&rels);
                rels.push(Rel {
                    id: rid.clone(),
                    ty: REL_SLIDE.into(),
                    target: opc::relative_target(&sh.pres_part, &o.info.part),
                    external: false,
                });
                max_id += 1;
                el("p:sldId").with_attr("id", &max_id.to_string()).with_attr("r:id", &rid)
            } else {
                let e = fragment(&o.info.sld_id);
                if e.get("r:id").as_deref() != rid_of.get(&o.info.part).map(String::as_str) {
                    return Err(format!("{} has no slide relationship", o.info.part));
                }
                e
            };
            ids.push(e.get("id").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0));
            list.push(Node::El(e));
        }
        // Custom shows name slides by relationship: a slide one names cannot go.
        let gone_rids: BTreeSet<String> = before.iter().filter(|r| !rels.contains(r)).map(|r| r.id.clone()).collect();
        if let Some(cs) = root.child("p:custShowLst") {
            let mut bad = None;
            cs.walk(&mut |e| {
                if e.is("p:sld") && e.get("r:id").is_some_and(|r| gone_rids.contains(&r)) && bad.is_none() {
                    bad = Some(e.get("r:id").unwrap());
                }
            });
            if bad.is_some() {
                return Err("a deleted slide is part of a custom show: remove it from the show in PowerPoint first, or keep the slide".into());
            }
        }
        let order_changed = sh.order != ids;
        match root.child_mut("p:sldIdLst") {
            Some(l) => l.children = list,
            None if !list.is_empty() => {
                let mut l = el("p:sldIdLst");
                l.children = list;
                let at = root.children.iter().position(|n| matches!(n, Node::El(e) if e.is("p:sldSz") || e.is("p:notesSz") || e.is("p:smartTags") || e.is("p:embeddedFontLst") || e.is("p:custShowLst") || e.is("p:photoAlbum") || e.is("p:custDataLst") || e.is("p:kinsoku") || e.is("p:defaultTextStyle") || e.is("p:modifyVerifier") || e.is("p:extLst"))).unwrap_or(root.children.len());
                root.children.insert(at, Node::El(l));
            }
            None => {}
        }
        if order_changed {
            sections(&mut root, &ids)?;
        }
        let data = format!("{}{}{}", sh.prolog, root.to_xml(), sh.epilog).into_bytes();
        for p in parts.iter_mut() {
            if p.name == sh.pres_part {
                p.data = data.clone();
            }
        }
        if rels != before {
            let rp = opc::rels_part(&sh.pres_part);
            let d = rewrite_rels(parts, &sh.pres_part, &rels)?;
            for p in parts.iter_mut() {
                if p.name == rp {
                    p.data = d.clone();
                }
            }
        }
        Ok(())
    }

    /// Parts that go with deleted slides and objects: the slides, their
    /// notes, and what only they reached. Refused when a part that stays points at one,
    /// except the view settings' outline state, which loses the slide.
    fn delete(&self, parts: &mut [Part], deleted: &[&String]) -> Result<BTreeSet<String>, String> {
        let mut gone: BTreeSet<String> = BTreeSet::new();
        let dset: BTreeSet<&str> = deleted.iter().map(|s| s.as_str()).collect();
        for d in deleted {
            gone.insert((*d).clone());
            gone.insert(opc::rels_part(d));
            for r in opc::rels_of(parts, d) {
                let t = opc::resolve_target(d, &r.target);
                if r.ty == REL_NOTES {
                    gone.insert(opc::rels_part(&t));
                    gone.insert(t);
                }
            }
        }
        // Nothing that stays may point at a deleted slide.
        let mut view = None;
        for p in parts.iter().filter(|p| p.name.ends_with(".rels")) {
            let Some(src) = opc::source_of_rels(&p.name) else { continue };
            if gone.contains(&src) || src == self.shell.pres_part {
                continue;
            }
            let rels = opc::parse_rels(&p.data);
            let to_deleted = |r: &Rel| !r.external && dset.contains(opc::resolve_target(&src, &r.target).as_str());
            if src.ends_with("/viewProps.xml") && rels.iter().any(|r| to_deleted(r) && r.ty == REL_SLIDE) {
                let ids: BTreeSet<String> = rels.iter().filter(|r| to_deleted(r)).map(|r| r.id.clone()).collect();
                let kept: Vec<Rel> = rels.iter().filter(|r| !ids.contains(&r.id)).cloned().collect();
                view = Some((src.clone(), ids, kept));
                continue;
            }
            if let Some(r) = rels.iter().find(|r| to_deleted(r)) {
                return Err(format!(
                    "{src} links to {}, a slide the edit deletes: remove the link in PowerPoint first, or keep the slide",
                    opc::resolve_target(&src, &r.target)
                ));
            }
        }
        // The outline view's `p:sld` entries name slides by relationship.
        if let Some((src, ids, kept)) = view {
            let rels = rewrite_rels(parts, &src, &kept)?;
            let v = package::get(parts, &src).ok_or_else(|| format!("no {src}"))?;
            let mut d = xml::parse(v).map_err(|e| format!("{src}: {e}"))?;
            d.root.walk_mut(&mut |e| {
                e.children.retain(
                    |n| !matches!(n, Node::El(x) if x.is("p:sld") && x.get("r:id").is_some_and(|r| ids.contains(&r))),
                )
            });
            let data = xml::write_doc(&d);
            let rp = opc::rels_part(&src);
            for p in parts.iter_mut() {
                if p.name == src {
                    p.data = data.clone();
                } else if p.name == rp {
                    p.data = rels.clone();
                }
            }
        }
        // What only the deleted slides and objects reached.
        let before = opc::reachable(self.parts);
        let rest: Vec<Part> = parts.iter().filter(|p| !gone.contains(&p.name)).cloned().collect();
        let after = opc::reachable(&rest);
        for p in before.difference(&after) {
            gone.insert(opc::rels_part(p));
            gone.insert(p.clone());
        }
        Ok(gone)
    }
}

fn keep_id(e: &Entry) -> &str {
    e.meta.keep.as_ref().map_or("", |k| k.id.as_str())
}

/// Set a part's data, adding the part (with `template`'s zip metadata) when
/// the package has none: whether it was added.
fn put(parts: &mut Vec<Part>, template: &Part, name: &str, data: Vec<u8>) -> bool {
    match parts.iter_mut().find(|p| p.name == name) {
        Some(p) => {
            p.data = data;
            false
        }
        None => {
            parts.push(Part { name: name.into(), data, ..template.clone() });
            true
        }
    }
}

/// Part numbers in use; new ones count on from the highest.
struct Numbers {
    names: BTreeSet<String>,
    /// The part each host file became, by its name.
    host: BTreeMap<String, String>,
}

impl Numbers {
    fn new(names: &BTreeSet<String>) -> Self {
        Numbers { names: names.clone(), host: BTreeMap::new() }
    }

    /// `ppt/media/image7.png`: the next image number of the package.
    fn next_media(&mut self, ext: &str) -> String {
        let base = "ppt/media/image";
        let n = self
            .names
            .iter()
            .filter_map(|p| p.strip_prefix(base)?.split('.').next()?.parse::<u32>().ok())
            .max()
            .unwrap_or(0);
        let name = format!("{base}{}.{ext}", n + 1);
        self.names.insert(name.clone());
        name
    }

    /// `ppt/slides/slide7.xml` for base `ppt/slides/slide`.
    fn next(&mut self, base: &str) -> String {
        let n = self
            .names
            .iter()
            .filter_map(|p| p.strip_prefix(base)?.strip_suffix(".xml")?.parse::<u32>().ok())
            .max()
            .unwrap_or(0);
        let name = format!("{base}{}.xml", n + 1);
        self.names.insert(name.clone());
        name
    }
}

// ---------------------------------------------------------------- shape trees

enum Stand {
    /// A modelled shape: its stand-in number, the slot it filled, and the
    /// emptied placeholder it leaves when its text goes.
    Item {
        k: String,
        slot: Option<String>,
        fallback: Option<Element>,
    },
    Latent {
        slot: String,
        el: Element,
    },
    Raw(Node),
}

impl Stand {
    fn slot(&self) -> Option<&str> {
        match self {
            Stand::Item { slot: Some(s), fallback: Some(_), .. } => Some(s),
            Stand::Latent { slot, .. } => Some(slot),
            _ => None,
        }
    }

    /// The placeholder a slot takes: with no paragraphs.
    fn element(&self) -> Option<Element> {
        let mut e = match self {
            Stand::Item { fallback: Some(f), .. } => f.clone(),
            Stand::Latent { el, .. } => el.clone(),
            _ => return None,
        };
        if let Some(tx) = e.child_mut("p:txBody") {
            tx.children.retain(|x| !matches!(x, Node::El(p) if p.is("a:p")));
        }
        Some(e)
    }
}

fn stand_ins(children: Vec<Node>) -> Vec<Stand> {
    children
        .into_iter()
        .map(|n| match n {
            Node::El(e) if e.is(ITEM) => Stand::Item {
                k: e.get("k").unwrap_or_default(),
                slot: e.get("slot"),
                fallback: e.elements().next().cloned(),
            },
            Node::El(e) if e.is(LATENT) => Stand::Latent {
                slot: e.get("slot").unwrap_or_default(),
                el: e.elements().next().cloned().unwrap_or_default(),
            },
            n => Stand::Raw(n),
        })
        .collect()
}

/// Whether a placeholder the text does not show as a slot (one in a group
/// or another object, a second one of the slot) already fills slot `name`.
fn occupied(stands: &[Stand], objects: &[&Element], layout: &LayoutInfo, name: &str) -> bool {
    let raw = stands.iter().filter_map(|st| if let Stand::Raw(Node::El(e)) = st { Some(e) } else { None });
    raw.chain(objects.iter().copied()).any(|e| {
        let mut hit = false;
        e.walk(&mut |x| {
            if let Some((ty, idx, _)) = placeholder(x) {
                hit |= crate::import::match_slot(layout, &ty, idx).is_some_and(|s| s.name == name);
            }
        });
        hit
    })
}

/// The shape tree: `p:nvGrpSpPr` and `p:grpSpPr` first, then the items in
/// text order, each preceded by what stood before its stand-in (up to the
/// previous claimed one); what follows the last claimed stand-in comes last.
/// An unclaimed stand-in leaves its emptied placeholder when its slot is
/// still one of the layout's, and nothing otherwise (a deleted shape).
fn assemble(
    stands: Vec<Stand>,
    claims: &[Option<usize>],
    items: Vec<Element>,
    in_layout: &dyn Fn(&str) -> bool,
) -> Result<Vec<Node>, String> {
    let claimed: HashMap<usize, usize> = claims.iter().enumerate().filter_map(|(n, c)| c.map(|x| (x, n))).collect();
    let mut head = vec![];
    let mut before: Vec<Vec<Node>> = vec![vec![]; items.len()];
    let mut pending: Vec<Node> = vec![];
    for (x, st) in stands.into_iter().enumerate() {
        if let Some(&n) = claimed.get(&x) {
            before[n].append(&mut pending);
            continue;
        }
        match st {
            Stand::Raw(Node::El(e)) if e.is("p:nvGrpSpPr") || e.is("p:grpSpPr") => head.push(Node::El(e)),
            Stand::Raw(n) => pending.push(n),
            Stand::Latent { el, .. } => pending.push(Node::El(el)),
            Stand::Item { slot: Some(s), fallback: Some(f), .. } if in_layout(&s) => pending.push(Node::El(f)),
            Stand::Item { .. } => {}
        }
    }
    let mut out = head;
    for (n, it) in items.into_iter().enumerate() {
        out.append(&mut before[n]);
        out.push(Node::El(it));
    }
    // An extension list stays last.
    let ext: Vec<Node> = pending.iter().filter(|n| matches!(n, Node::El(e) if e.is("p:extLst"))).cloned().collect();
    pending.retain(|n| !matches!(n, Node::El(e) if e.is("p:extLst")));
    out.extend(pending);
    out.extend(ext);
    Ok(out)
}

/// A shape id of its own for each fresh item (a new placeholder, id 0, or a
/// shape or object moved in from another slide whose ids are taken): the next free one.
fn assign_ids(stands: &[Stand], items: &mut [Element], fresh: &[bool]) {
    let mut taken: HashSet<u32> = HashSet::new();
    let mut note = |e: &Element| {
        e.walk(&mut |x| {
            if x.is("p:cNvPr") {
                taken.extend(x.get("id").and_then(|v| v.parse::<u32>().ok()));
            }
        })
    };
    for st in stands {
        match st {
            Stand::Raw(Node::El(e)) | Stand::Latent { el: e, .. } | Stand::Item { fallback: Some(e), .. } => note(e),
            _ => {}
        }
    }
    for (e, f) in items.iter().zip(fresh) {
        if !f {
            note(e);
        }
    }
    let mut next = taken.iter().max().copied().unwrap_or(1);
    // A group's shapes too: every id in a fresh item.
    for (e, _) in items.iter_mut().zip(fresh).filter(|(_, f)| **f) {
        e.walk_mut(&mut |c| {
            if !c.is("p:cNvPr") {
                return;
            }
            let id = c.get("id").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
            if id == 0 || !taken.insert(id) {
                next += 1;
                c.set("id", &next.to_string());
                taken.insert(next);
            }
        });
    }
}

/// How an error names an item: `<shape id="s4" name="출처">`, `::title::`.
fn item_label(it: &ItemG) -> String {
    match it.kind {
        HeadKind::Slot { name } => format!("::{name}::"),
        HeadKind::Shape { id: "", .. } => "the new text box".into(),
        HeadKind::Shape { id, name } => format!("<shape id=\"{id}\" name=\"{name}\">"),
        HeadKind::Line { id: "", .. } => "the new line".into(),
        HeadKind::Line { id, name } => format!("<line id=\"{id}\" name=\"{name}\">"),
        HeadKind::Group { id, name } => format!("<group id=\"{id}\" name=\"{name}\">"),
        HeadKind::Picture { id: "", .. } => "the new picture".into(),
        HeadKind::Picture { id, name } => format!("<picture id=\"{id}\" name=\"{name}\">"),
        HeadKind::Object => "an object's <keep/>".into(),
        HeadKind::Slide { .. } => "the slide".into(),
    }
}

/// Write a box (§5.3). `shown` is the box the text showed for the element,
/// `layout` (for a slot) the box its layout gives it now. Nothing is written
/// when the element already sits where `written` says (its own box, or the
/// one it inherits); a slot written at its layout's box goes back to it;
/// else the box is written, the numbers left as shown keeping their exact
/// value. Without a box, a slot goes back to its layout's place and anything
/// else stays. Whether the element moved or was resized.
fn place_box(
    e: &mut Element,
    shown: Option<Geom>,
    written: Option<Geom>,
    layout: Option<Option<Geom>>,
) -> Result<bool, String> {
    let slot = layout.is_some();
    // An alternate-content object has no box of its own: its first choice's is the one shown.
    let own = object_geom(e);
    let now = own.or(layout.flatten());
    let g = match written {
        Some(w) if now.is_some_and(|n| w.shows_as(&n)) => return Ok(false),
        Some(w) if slot && layout.flatten().is_some_and(|l| w.shows_as(&l)) => {
            geom::remove(e);
            return Ok(true);
        }
        Some(w) => shown.or(now).map_or(w, |s| s.merged(&w)),
        None if slot && geom::xfrm(e).is_some() => {
            geom::remove(e);
            return Ok(true);
        }
        None => return Ok(false),
    };
    if e.is("mc:AlternateContent") {
        return Err("this object is stored in more than one form, for different applications, and cannot be moved or resized here; move it in PowerPoint".into());
    }
    if g.w < 0 || g.h < 0 {
        return Err("a box's width and height are 0 or more".into());
    }
    geom::write(e, &g)?;
    if let Some(b) = now.filter(|b| (b.w, b.h) != (g.w, g.h)) {
        if e.is("p:graphicFrame") {
            geom::scale_table(e, (b.w, b.h), (g.w, g.h));
        }
    }
    Ok(true)
}

/// A group's objects as a signature without their geometry: what must not
/// change when a group is moved or its objects are.
pub fn sig(items: &[SlideItem]) -> Vec<String> {
    items
        .iter()
        .map(|it| match it {
            // A shape's text and fill are written (§5.3); its id and name are what it is.
            SlideItem::Shape(sh) => format!("<shape id=\"{}\" name=\"{}\"/>", sh.id, sh.name),
            SlideItem::Object(o) => {
                hanji_format::pres::object_line(&hanji_format::ObjectItem { geom: None, ..o.clone() })
            }
            SlideItem::Line(l) => format!("<line id=\"{}\" name=\"{}\"/>", l.id, l.name),
            SlideItem::Picture(p) => hanji_format::pres::picture_line(&PictureItem { geom: None, ..p.clone() }),
            SlideItem::Group(g) => {
                format!("<group id=\"{}\" name=\"{}\">{}</group>", g.id, g.name, sig(&g.items).join(""))
            }
            SlideItem::Slot(_) => "::slot::".into(),
        })
        .collect()
}

/// The box of a group's object as the text shows it.
fn member_box(it: &SlideItem) -> Option<Geom> {
    match it {
        SlideItem::Shape(sh) => sh.geom,
        SlideItem::Object(o) => o.geom,
        SlideItem::Line(l) => Some(geom::box_of(&l.ends, 0)),
        SlideItem::Group(g) => g.geom,
        SlideItem::Picture(p) => p.geom,
        SlideItem::Slot(_) => None,
    }
}

/// Whether a member was written other than the text showed it.
fn member_changed(st: &SlideItem, w: &SlideItem) -> bool {
    match (st, w) {
        (SlideItem::Line(a), SlideItem::Line(b)) => !b.ends.shows_as(&a.ends),
        (SlideItem::Group(a), SlideItem::Group(b)) => {
            b.geom.zip(a.geom).is_some_and(|(x, y)| !x.shows_as(&y))
                || a.items.iter().zip(&b.items).any(|(x, y)| member_changed(x, y))
        }
        _ => match (member_box(st), member_box(w)) {
            (Some(a), Some(b)) => !b.shows_as(&a),
            _ => false,
        },
    }
}

/// Every shape id in a group (itself included), each marked when it is a connector.
fn ids_in(el: &Element) -> Vec<(u32, bool)> {
    let mut out = vec![];
    el.walk(&mut |x| {
        if x.is("p:cNvPr") {
            out.extend(x.get("id").and_then(|v| v.parse().ok()).map(|id| (id, false)));
        }
    });
    let mut cxn = vec![];
    el.walk(&mut |x| {
        if x.is("p:cxnSp") {
            cxn.extend(geom::shape_id(x));
        }
    });
    for o in &mut out {
        o.1 = cxn.contains(&o.0);
    }
    out
}

/// The group's written geometry onto its element `el`, which sits in
/// `parent` (§5.3). Its box alone moves or scales it; its objects' boxes
/// move them, and the group's box follows them; both at once must agree.
/// Anything else about its objects is refused. The shape ids that moved
/// (marked when a connector).
pub fn apply_group(
    el: &mut Element,
    w: &GroupItem,
    parent: &Frame,
    rels: &Rels,
    sty: Option<Styling>,
) -> Result<Vec<(u32, bool)>, String> {
    let st = group_item(el, parent, rels, sty).ok_or("the group's objects are not as they were read")?;
    let ids = group_geometry(el, w, parent, rels, sty)?;
    group_text(el, &st, w, sty)?;
    Ok(ids)
}

/// The text and fill of a group's shapes, where the text changes them.
fn group_text(el: &mut Element, st: &GroupItem, w: &GroupItem, sty: Option<Styling>) -> Result<(), String> {
    let kids = el.elements_mut().filter(|e| !matches!(e.name.as_str(), "p:nvGrpSpPr" | "p:grpSpPr" | "p:extLst"));
    for ((c, a), b) in kids.zip(&st.items).zip(&w.items) {
        match (a, b) {
            (SlideItem::Shape(a), SlideItem::Shape(b)) => {
                let what = format!("<shape id=\"{}\" name=\"{}\">", b.id, b.name);
                members::write(c, &a.paras, &b.paras, sty, &what).map_err(|m| format!("{what}: {m}"))?;
                if a.look != b.look {
                    let theme = sty.map(|s| s.fills).ok_or("a group's fills are written only on a slide")?;
                    fill::write(c, b.look.fill.as_deref(), fill::Parents { now: None, then: None, theme })
                        .map_err(|m| format!("{what}: {m}"))?;
                }
            }
            (SlideItem::Group(a), SlideItem::Group(b)) => group_text(c, a, b, sty)?,
            _ => {}
        }
    }
    Ok(())
}

/// The group's geometry (see [`apply_group`]).
fn group_geometry(
    el: &mut Element,
    w: &GroupItem,
    parent: &Frame,
    rels: &Rels,
    sty: Option<Styling>,
) -> Result<Vec<(u32, bool)>, String> {
    let st = group_item(el, parent, rels, sty).ok_or("the group's objects are not as they were read")?;
    if sig(&st.items) != sig(&w.items) {
        return Err("a group's objects can be moved and resized here, and their shapes' text and fill changed, but never added, deleted, reordered, renamed or otherwise edited: keep each object's id and name as they are, or change the group in PowerPoint".into());
    }
    let (Some(sg), wg) = (st.geom, w.geom) else { return Err("the group has no box".into()) };
    let box_changed = wg.is_some_and(|g| !g.shows_as(&sg));
    let members: Vec<usize> = (0..st.items.len()).filter(|&k| member_changed(&st.items[k], &w.items[k])).collect();
    if !box_changed && members.is_empty() {
        return Ok(vec![]);
    }
    let local = Frame::of(el).ok_or("the group has no child offset and extent")?;
    // The group's box alone, its objects where that puts them (as the
    // canonical text shows them after a box edit): its objects follow.
    if box_changed && !members.is_empty() {
        let mut alone = el.clone();
        let only = GroupItem { items: st.items.clone(), ..w.clone() };
        if let Ok(ids) = group_geometry(&mut alone, &only, parent, rels, sty) {
            let after = group_item(&alone, parent, rels, sty);
            if after.is_some_and(|a| a.items.iter().zip(&w.items).all(|(x, y)| !member_changed(x, y))) {
                *el = alone;
                return Ok(ids);
            }
        }
    }
    if members.is_empty() {
        // The group's box alone: its objects follow in its child coordinates.
        let g = sg.merged(&wg.unwrap());
        if g.rot != 0 || g.flip_h || g.flip_v {
            return Err("a group is not rotated or flipped here".into());
        }
        let own = geom::own(el).unwrap_or_default();
        let at = parent.back(&g);
        let keep = |a: i64, b: i64, s: i64, x: i64| {
            if hanji_format::shown_pt(s) * hanji_format::EMU_PER_PT == x {
                a
            } else {
                b
            }
        };
        let wv = wg.unwrap();
        let next = Geom {
            x: keep(own.x, at.x, sg.x, wv.x),
            y: keep(own.y, at.y, sg.y, wv.y),
            w: keep(own.w, at.w, sg.w, wv.w),
            h: keep(own.h, at.h, sg.h, wv.h),
            ..own
        };
        geom::write(el, &next)?;
        return Ok(ids_in(el));
    }
    if let Some(g) = wg.filter(|_| box_changed) {
        let boxes: Vec<Geom> = w.items.iter().filter_map(member_box).collect();
        let u = geom::union(&boxes).unwrap_or_default();
        let near = |a: i64, b: i64| (a - b).abs() <= 2 * hanji_format::EMU_PER_PT;
        if !(near(u.x, g.x) && near(u.y, g.y) && near(u.w, g.w) && near(u.h, g.h)) {
            return Err(format!(
                "the group's box and its objects' boxes both changed and disagree (the objects' box is {} {} {} {}): change the group's box to move the whole group, or its objects, not both",
                hanji_format::pres::pt(u.x), hanji_format::pres::pt(u.y), hanji_format::pres::pt(u.w), hanji_format::pres::pt(u.h)
            ));
        }
    }
    let f = local.within(parent);
    let mut moved = vec![];
    let kids: Vec<usize> = el
        .children
        .iter()
        .enumerate()
        .filter(
            |(_, n)| matches!(n, Node::El(e) if !matches!(e.name.as_str(), "p:nvGrpSpPr" | "p:grpSpPr" | "p:extLst")),
        )
        .map(|(k, _)| k)
        .collect();
    for &m in &members {
        let Node::El(c) = &mut el.children[kids[m]] else { unreachable!() };
        match (&st.items[m], &w.items[m]) {
            (SlideItem::Group(_), SlideItem::Group(wg)) => moved.extend(group_geometry(c, wg, &f, rels, sty)?),
            (SlideItem::Line(a), SlideItem::Line(b)) => {
                let own = geom::own(c).unwrap_or_default();
                let ends = a.ends.merged(&b.ends);
                let g = f.back(&geom::box_of(&ends, own.rot));
                geom::write(c, &Geom { rot: own.rot, ..g })?;
                moved.extend(geom::shape_id(c).map(|id| (id, true)));
            }
            (a, b) => {
                let (Some(sa), Some(sb)) = (member_box(a), member_box(b)) else { continue };
                let own = geom::own(c).ok_or("an object in the group has no box")?;
                let merged = sa.merged(&sb);
                let at = f.back(&merged);
                let keep = |o: i64, n: i64, s: i64, x: i64| {
                    if hanji_format::shown_pt(s) * hanji_format::EMU_PER_PT == x {
                        o
                    } else {
                        n
                    }
                };
                let next = Geom {
                    x: keep(own.x, at.x, sa.x, sb.x),
                    y: keep(own.y, at.y, sa.y, sb.y),
                    w: keep(own.w, at.w, sa.w, sb.w),
                    h: keep(own.h, at.h, sa.h, sb.h),
                    rot: merged.rot,
                    flip_h: merged.flip_h,
                    flip_v: merged.flip_v,
                };
                if c.is("mc:AlternateContent") {
                    return Err(
                        "an object in the group is stored in more than one form and cannot be moved here".into()
                    );
                }
                geom::write(c, &next)?;
                moved.extend(geom::shape_id(c).map(|id| (id, false)));
            }
        }
    }
    // The group's child box is the box around its objects; its box follows.
    let boxes: Vec<Geom> = kids
        .iter()
        .filter_map(|&k| match &el.children[k] {
            Node::El(c) => object_geom(c).map(|g| Geom { rot: 0, flip_h: false, flip_v: false, ..g }),
            _ => None,
        })
        .collect();
    let u = geom::union(&boxes).ok_or("a group without objects")?;
    if (u.x, u.y, u.w, u.h) != (local.ch_off.0, local.ch_off.1, local.ch_ext.0, local.ch_ext.1) && u.w > 0 && u.h > 0 {
        let out = local.out(&u);
        let own = geom::own(el).unwrap_or_default();
        geom::write(el, &Geom { x: out.x, y: out.y, w: out.w, h: out.h, ..own })?;
        let x = el.child_mut("p:grpSpPr").and_then(|p| p.child_mut("a:xfrm")).ok_or("the group has no a:xfrm")?;
        for (name, a, b, va, vb) in [("a:chOff", "x", "y", u.x, u.y), ("a:chExt", "cx", "cy", u.w, u.h)] {
            let c = x.child_mut(name).ok_or("the group has no child offset and extent")?;
            c.set(a, &va.to_string());
            c.set(b, &vb.to_string());
        }
    }
    Ok(moved)
}

/// Names for new objects, once they have ids: `TextBox 7`, `Straight Connector 8`.
fn name_new(items: &mut [Element], created: &[bool]) {
    for (e, _) in items.iter_mut().zip(created).filter(|(_, c)| **c) {
        let base = match e.name.as_str() {
            "p:cxnSp" => "Straight Connector",
            "p:pic" => "Picture",
            _ => "TextBox",
        };
        let id = geom::shape_id(e).unwrap_or(1);
        if let Some(c) = c_nv_pr_mut(e) {
            c.set("name", &format!("{base} {}", id.saturating_sub(1)));
        }
    }
}

/// Connectors are not rerouted (§5.3): moving or resizing an object a
/// connector is attached to is refused, unless the same edit moves that
/// connector too.
fn check_connectors(
    stands: &[Stand],
    items: &[Element],
    moved: &[(u32, String)],
    written: &HashSet<u32>,
) -> Result<(), String> {
    if moved.is_empty() {
        return Ok(());
    }
    let mut all: Vec<&Element> = items.iter().collect();
    for st in stands {
        if let Stand::Raw(Node::El(e)) = st {
            all.push(e);
        }
    }
    for e in all {
        let mut found: Vec<&Element> = vec![];
        e.walk(&mut |x| {
            if x.is("p:cxnSp") {
                found.push(x);
            }
        });
        for c in found {
            let cid = geom::shape_id(c).unwrap_or(0);
            if written.contains(&cid) {
                continue;
            }
            for (end, target) in geom::connections(c) {
                if let Some((_, what)) = moved.iter().find(|m| m.0 == target && m.0 != cid) {
                    let name = c_nv_pr(c).and_then(|x| x.get("name")).unwrap_or_default();
                    return Err(format!(
                        "{what} is moved or resized, and connector <line id=\"s{cid}\" name=\"{name}\"> has its {end} attached to it. Connectors are not rerouted here, so the connector would come loose in PowerPoint: move that end of the <line> in the same edit, or leave {what} where it is"
                    ));
                }
            }
        }
    }
    Ok(())
}

/// A new text box (§5.3): `txBox`, no fill, the master's `otherStyle`; its
/// id, name and paragraphs are written after.
/// What a text item's runs inherit, by level (§5.3 text formatting): where
/// the slide now stands, and under the layout it had.
#[derive(Clone, Copy)]
struct Inherit<'a> {
    now: &'a [RunStyle; 9],
    then: &'a [RunStyle; 9],
    fonts: &'a ThemeFonts,
}

impl<'a> Inherit<'a> {
    fn at(self, lvl: usize) -> Under<'a> {
        Under { now: &self.now[lvl], then: &self.then[lvl], fonts: self.fonts }
    }
}

/// [`Inherit`] at one paragraph level.
#[derive(Clone, Copy)]
struct Under<'a> {
    now: &'a RunStyle,
    then: &'a RunStyle,
    fonts: &'a ThemeFonts,
}

fn new_text_box(g: &Geom) -> Element {
    let mut e = fragment(concat!(
        "<p:sp><p:nvSpPr><p:cNvPr id=\"0\" name=\"TextBox\"/><p:cNvSpPr txBox=\"1\"/><p:nvPr/></p:nvSpPr>",
        "<p:spPr><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom><a:noFill/></p:spPr>",
        "<p:txBody><a:bodyPr wrap=\"square\" rtlCol=\"0\"/><a:lstStyle/></p:txBody></p:sp>"
    ));
    geom::write(&mut e, g).expect("a new text box has p:spPr");
    e
}

/// A new picture at `g`, stretched to its box; its image, id and name are written after.
fn new_picture(g: &Geom) -> Element {
    let mut e = fragment(concat!(
        "<p:pic><p:nvPicPr><p:cNvPr id=\"0\" name=\"Picture\"/><p:cNvPicPr><a:picLocks noChangeAspect=\"1\"/></p:cNvPicPr><p:nvPr/></p:nvPicPr>",
        "<p:blipFill><a:blip r:embed=\"\"/><a:stretch><a:fillRect/></a:stretch></p:blipFill>",
        "<p:spPr><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr></p:pic>"
    ));
    geom::write(&mut e, g).expect("a new picture has p:spPr");
    e
}

/// The slide's relationships as pictures name them.
fn rels_map(rels: &[Rel], part: &str) -> Rels {
    rels.iter()
        .filter(|r| !r.external)
        .map(|r| (r.id.clone(), crate::import::src_of(&opc::resolve_target(part, &r.target))))
        .collect()
}

/// The id of the slide's image relationship to `target`, added when it has none.
fn target_rel(rels: &mut Vec<Rel>, part: &str, target: &str, changed: &mut bool) -> String {
    if let Some(r) =
        rels.iter().find(|r| !r.external && r.ty == REL_IMAGE && opc::resolve_target(part, &r.target) == target)
    {
        return r.id.clone();
    }
    let id = opc::free_rel_id(rels);
    rels.push(Rel {
        id: id.clone(),
        ty: REL_IMAGE.into(),
        target: opc::relative_target(part, target),
        external: false,
    });
    *changed = true;
    id
}

/// An image's extension and content type from its first bytes.
fn image_type(d: &[u8]) -> Option<(&'static str, &'static str)> {
    if d.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(("png", "image/png"))
    } else if d.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(("jpeg", "image/jpeg"))
    } else if d.starts_with(b"GIF87a") || d.starts_with(b"GIF89a") {
        Some(("gif", "image/gif"))
    } else if d.starts_with(b"BM") && d.len() > 26 {
        Some(("bmp", "image/bmp"))
    } else {
        None
    }
}

/// `a:srcRect` for a crop (after `a:blip`, as the schema orders
/// `p:blipFill`); only the values that differ are set, and an empty crop
/// leaves no `a:srcRect` unless it holds something else.
fn set_crop(bf: &mut Element, c: &Crop) {
    if bf.child("a:srcRect").is_none() {
        if c.is_zero() {
            return;
        }
        let at = bf.children.iter().position(|n| matches!(n, Node::El(e) if e.is("a:blip"))).map_or(0, |k| k + 1);
        bf.children.insert(at, Node::El(el("a:srcRect")));
    }
    let r = bf.child_mut("a:srcRect").unwrap();
    for (k, v) in [("l", c.l), ("t", c.t), ("r", c.r), ("b", c.b)] {
        let had = r.get(k).and_then(|x| x.trim().parse::<i64>().ok()).unwrap_or(0);
        if had != v || (v == 0 && r.get(k).is_some()) {
            if v == 0 {
                r.remove_attr(k);
            } else {
                r.set(k, &v.to_string());
            }
        }
    }
    if r.attrs.is_empty() && r.children.is_empty() {
        bf.children.retain(|n| !matches!(n, Node::El(e) if e.is("a:srcRect")));
    }
}

/// The `p:spPr` order (CT_ShapeProperties) around the geometry.
const SPPR_ORDER: &[&str] = &[
    "xfrm",
    "custGeom",
    "prstGeom",
    "noFill",
    "solidFill",
    "gradFill",
    "blipFill",
    "pattFill",
    "grpFill",
    "ln",
    "effectLst",
    "effectDag",
    "scene3d",
    "sp3d",
    "extLst",
];

/// The preset shape of `p:spPr`: its `a:prstGeom` with the preset's own
/// adjustments (an old preset's do not fit a new one).
fn set_preset(sp: &mut Element, prst: &str) {
    match sp.child_mut("a:prstGeom") {
        Some(g) => {
            g.set("prst", prst);
            if let Some(av) = g.child_mut("a:avLst") {
                av.children.clear();
            }
        }
        None => {
            insert_ordered(sp, fragment(&format!("<a:prstGeom prst=\"{prst}\"><a:avLst/></a:prstGeom>")), SPPR_ORDER)
        }
    }
}

/// A new straight line from `ends`, drawn in the text colour.
fn new_line(ends: &hanji_format::Ends) -> Element {
    let mut e = fragment(concat!(
        "<p:cxnSp><p:nvCxnSpPr><p:cNvPr id=\"0\" name=\"Straight Connector\"/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr>",
        "<p:spPr><a:prstGeom prst=\"line\"><a:avLst/></a:prstGeom>",
        "<a:ln w=\"12700\"><a:solidFill><a:schemeClr val=\"tx1\"/></a:solidFill></a:ln></p:spPr></p:cxnSp>"
    ));
    geom::write(&mut e, &geom::box_of(ends, 0)).expect("a new line has p:spPr");
    e
}

/// Animations may not lose their shape: an animated shape that was on the
/// slide and is gone, or a paragraph range past its shape's paragraphs, is refused.
fn check_timing(root: &Element, info: &SlideInfo) -> Result<(), String> {
    let Some(timing) = root.child("p:timing") else { return Ok(()) };
    let mut paras: HashMap<u32, usize> = HashMap::new();
    if let Some(tree) = root.child("p:cSld").and_then(|c| c.child("p:spTree")) {
        tree.walk(&mut |e| {
            if let Some(id) = c_nv_pr(e).and_then(|c| c.get("id")).and_then(|v| v.parse().ok()) {
                let n = e.child("p:txBody").map_or(0, |t| t.elements().filter(|p| p.is("a:p")).count());
                paras.insert(id, n);
            }
        });
    }
    let mut err = None;
    timing.walk(&mut |e| {
        let Some(spid) = e.get("spid").and_then(|v| v.parse::<u32>().ok()) else { return };
        if err.is_some() {
            return;
        }
        match paras.get(&spid) {
            None if info.ids.contains(&spid) => {
                err = Some(format!("the slide's animation plays on shape {spid}, which the edit deletes: remove the animation in PowerPoint first"))
            }
            Some(&n) => {
                let mut end = None;
                e.walk(&mut |x| {
                    if x.is("p:pRg") {
                        end = x.get("end").and_then(|v| v.parse::<usize>().ok()).max(end);
                    }
                });
                if end.is_some_and(|k| k >= n) {
                    err = Some(format!("the slide's animation plays on paragraph {} of shape {spid}, which the edit deletes", end.unwrap() + 1));
                }
            }
            None => {}
        }
    });
    err.map_or(Ok(()), Err)
}

/// Sections (`p14:sectionLst`) after the slide order changed: each slide keeps
/// its section, a new one joins the section of the slide before it, and the
/// sections must stay contiguous and in order.
fn sections(root: &mut Element, new: &[u32]) -> Result<(), String> {
    let mut lst = None;
    root.walk_mut(&mut |e| {
        if e.local() == "sectionLst" && lst.is_none() {
            lst = Some(e.clone());
        }
    });
    let Some(lst) = lst else { return Ok(()) };
    let secs: Vec<&Element> = lst.elements().filter(|e| e.local() == "section").collect();
    let mut of: HashMap<u32, usize> = HashMap::new();
    for (k, s) in secs.iter().enumerate() {
        s.walk(&mut |e| {
            if e.local() == "sldId" {
                if let Some(id) = e.get("id").and_then(|v| v.parse().ok()) {
                    of.insert(id, k);
                }
            }
        });
    }
    let mut assigned: Vec<(u32, usize)> = vec![];
    for &id in new {
        let s = of.get(&id).copied().or_else(|| assigned.last().map(|a| a.1)).unwrap_or(0);
        assigned.push((id, s));
    }
    if assigned.windows(2).any(|w| w[1].1 < w[0].1) {
        return Err("the new slide order takes slides out of their sections: move slides within their section, or change sections in PowerPoint".into());
    }
    let mut new_lst = lst.clone();
    for (k, s) in new_lst.elements_mut().filter(|e| e.local() == "section").enumerate() {
        let ids: Vec<u32> = assigned.iter().filter(|a| a.1 == k).map(|a| a.0).collect();
        let Some(list) = s.elements_mut().find(|e| e.local() == "sldIdLst") else { continue };
        let prefix = list.name.split_once(':').map(|x| x.0.to_string());
        let name = match &prefix {
            Some(p) => format!("{p}:sldId"),
            None => "sldId".into(),
        };
        list.children = ids.iter().map(|id| Node::El(el(&name).with_attr("id", &id.to_string()))).collect();
    }
    let mut done = false;
    root.walk_mut(&mut |e| {
        if !done {
            for c in e.children.iter_mut() {
                if matches!(c, Node::El(x) if x.local() == "sectionLst") && !done {
                    *c = Node::El(new_lst.clone());
                    done = true;
                }
            }
        }
    });
    Ok(())
}

// ---------------------------------------------------------------- new elements

fn new_slide() -> Element {
    fragment(&format!(
        r#"<p:sld xmlns:a="{A_NS}" xmlns:r="{R_NS}" xmlns:p="{P_NS}"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr></p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>"#
    ))
}

fn new_notes() -> Element {
    fragment(&format!(
        r#"<p:notes xmlns:a="{A_NS}" xmlns:r="{R_NS}" xmlns:p="{P_NS}"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr><p:sp><p:nvSpPr><p:cNvPr id="2" name="Slide Image Placeholder 1"/><p:cNvSpPr><a:spLocks noGrp="1" noRot="1" noChangeAspect="1"/></p:cNvSpPr><p:nvPr><p:ph type="sldImg"/></p:nvPr></p:nvSpPr><p:spPr/></p:sp></p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:notes>"#
    ))
}

fn new_notes_body() -> Element {
    fragment(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="0" name="Notes Placeholder 2"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/></p:txBody></p:sp>"#,
    )
}

/// A slide placeholder for a layout slot: its `p:ph` and name, no geometry.
fn new_placeholder(slot: &SlotInfo) -> Element {
    let mut sp = fragment(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="0" name=""/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/></p:txBody></p:sp>"#,
    );
    if let Some(c) = c_nv_pr_mut(&mut sp) {
        c.set("name", &slot.shape_name);
    }
    set_ph(&mut sp, slot);
    sp
}

/// The shape's `p:ph` becomes the layout slot's (a slide whose layout changed).
fn set_ph(e: &mut Element, slot: &SlotInfo) {
    let mut ph = fragment(&slot.ph);
    ph.remove_attr("hasCustomPrompt");
    if let Some(nv) = e.elements_mut().find(|x| x.name.starts_with("p:nv")) {
        if let Some(pr) = nv.child_mut("p:nvPr") {
            match pr.children.iter().position(|n| matches!(n, Node::El(x) if x.is("p:ph"))) {
                Some(k) => pr.children[k] = Node::El(ph),
                None => pr.children.insert(0, Node::El(ph)),
            }
        }
    }
}

/// The bullet a paragraph at `lvl` reads as: its own, else inherited; none when neither.
fn reads(ppr: Option<&Element>, lvl: u32, bullets: &[Bu; 9]) -> Bu {
    match bu_of(ppr) {
        Bu::Unset => match bullets[lvl as usize] {
            Bu::Unset => Bu::None,
            b => b,
        },
        b => b,
    }
}

/// Make the paragraph read as `want` (plain `None`, bullet or number),
/// changing its `a:pPr` only when it does not already.
fn set_bullet(ppr: &mut Option<Element>, lvl: u32, want: Bu, bullets: &[Bu; 9]) {
    if reads(ppr.as_ref(), lvl, bullets) == want {
        return;
    }
    let p = ppr.get_or_insert_with(|| el("a:pPr"));
    p.children.retain(|n| !matches!(n, Node::El(e) if BU.contains(&e.name.as_str())));
    if reads(Some(p), lvl, bullets) == want {
        return;
    }
    let indent = |p: &mut Element| {
        if p.attr("marL").is_none() && p.attr("indent").is_none() {
            p.set("marL", &(342900 * (lvl + 1)).to_string());
            p.set("indent", "-342900");
        }
    };
    match want {
        Bu::None => {
            if p.attr("marL").is_none() && p.attr("indent").is_none() {
                p.set("marL", "0");
                p.set("indent", "0");
            }
            insert_ordered(p, el("a:buNone"), PPR_ORDER);
        }
        Bu::Bullet => {
            indent(p);
            if p.child("a:buFont").is_none() {
                insert_ordered(p, el("a:buFont").with_attr("typeface", "Arial"), PPR_ORDER);
            }
            insert_ordered(p, el("a:buChar").with_attr("char", "•"), PPR_ORDER);
        }
        Bu::Number => {
            indent(p);
            insert_ordered(p, el("a:buAutoNum").with_attr("type", "arabicPeriod"), PPR_ORDER);
        }
        Bu::Unset => {}
    }
}

/// Bold, italic, underline and strike on a run's `a:rPr`, changed only where they differ.
pub(crate) fn set_marks(rpr: &mut Element, want: Marks) {
    let have = marks_of(Some(rpr));
    for (m, attr, on) in [
        (Marks::BOLD, "b", "1"),
        (Marks::ITALIC, "i", "1"),
        (Marks::UNDERLINE, "u", "sng"),
        (Marks::STRIKE, "strike", "sngStrike"),
    ] {
        if have.has(m) != want.has(m) {
            if want.has(m) {
                rpr.set(attr, on);
            } else {
                rpr.remove_attr(attr);
            }
        }
    }
}

fn is_hangul(c: char) -> bool {
    matches!(c as u32, 0xAC00..=0xD7A3 | 0x1100..=0x11FF | 0x3130..=0x318F)
}

/// A part's relationships rewritten: its `.rels` with `rels` in place of what it had.
fn rewrite_rels(parts: &[Part], part: &str, rels: &[Rel]) -> Result<Vec<u8>, String> {
    let rp = opc::rels_part(part);
    let Some(data) = package::get(parts, &rp) else { return Ok(opc::write_rels(rels)) };
    let mut d = xml::parse(data).map_err(|e| format!("{rp}: {e}"))?;
    let old = opc::parse_rels(data);
    // Keep each unchanged relationship element as it was.
    let mut kids = vec![];
    for r in rels {
        let same = old.iter().position(|o| o == r);
        let existing = same.and_then(|k| d.root.elements().filter(|e| e.local() == "Relationship").nth(k)).cloned();
        kids.push(Node::El(existing.unwrap_or_else(|| {
            let mut e =
                el("Relationship").with_attr("Id", &r.id).with_attr("Type", &r.ty).with_attr("Target", &r.target);
            if r.external {
                e = e.with_attr("TargetMode", "External");
            }
            e
        })));
    }
    d.root.children = kids;
    Ok(xml::write_doc(&d))
}
