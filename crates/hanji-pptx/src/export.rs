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

use hanji_core::presentation::{kind, HeadKind};
use hanji_core::{Block, Entry, Kind, Para, Part, Remainder};
use hanji_format::{Atom, Inline, Marks};
use hanji_package::opc::{self, Rel};
use hanji_package::package;
use hanji_package::xml::{self, fragment, insert_ordered, Element, Node};

use crate::deck::{LayoutInfo, SlotInfo};
use crate::import::{NotesInfo, SlideInfo, OBJECT_TAG};
use crate::pml::*;
use crate::DeckShell;

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

pub struct Exporter<'a> {
    blocks: &'a [Block],
    shell: &'a DeckShell,
    parts: &'a [Part],
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
        let gone = self.delete(&mut parts, &deleted)?;
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
        }
        for (n, it) in items.iter().enumerate() {
            if els[n].is_some() {
                continue;
            }
            match it.kind {
                HeadKind::Shape { id, .. } => {
                    return Err(format!("<shape id=\"{id}\"> has no shape in the file to take its text: shapes come from the file and are never created. Write new text in a slot"));
                }
                HeadKind::Slot { name } => {
                    let taken: Vec<usize> = claims.iter().flatten().copied().collect();
                    let found = stands.iter().enumerate().find(|(x, st)| !taken.contains(x) && st.slot() == Some(name));
                    match found {
                        Some((x, st)) => {
                            els[n] = Some(st.element().ok_or("a stand-in without its placeholder")?);
                            claims[n] = Some(x);
                        }
                        None => {
                            if occupied(&stands, layout, name) {
                                return Err(format!("::{name}:: is filled by a placeholder the text does not show (in a group, or holding an object): the slot cannot be written twice"));
                            }
                            let slot = layout.need_slot(name)?;
                            els[n] = Some(new_placeholder(slot));
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
            if self.keep_slot(it)?.is_none() {
                let bullets = bullets_of(it, &e);
                let lists = matches!(it.kind, HeadKind::Slot { .. });
                let paras = self.paras(it, &bullets, &part, lists)?;
                let tx = e.child_mut("p:txBody").ok_or("a placeholder without p:txBody cannot hold text")?;
                tx.children.retain(|x| !matches!(x, Node::El(p) if p.is("a:p")));
                tx.children.extend(paras.into_iter().map(Node::El));
            }
            out_items.push(e);
        }
        assign_ids(&stands, &mut out_items, &fresh);
        let tree_children =
            assemble(std::mem::take(&mut stands), &claims, out_items, &|slot: &str| layout.slot(slot).is_some())?;
        let tree = xml_root.child_mut("p:cSld").and_then(|c| c.child_mut("p:spTree")).unwrap();
        tree.children = tree_children;
        check_timing(&xml_root, &info)?;
        let mut rels = opc::rels_of(self.parts, &part);
        let mut rels_changed = new;
        if new {
            rels = vec![Rel {
                id: "rId1".into(),
                ty: REL_LAYOUT.into(),
                target: opc::relative_target(&part, &layout.part),
                external: false,
            }];
        } else if relayout {
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
        Ok(SlideOut { info, xml, rels: rels_data, new, notes })
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
    fn paras(&self, it: &ItemG, bullets: &[Bu; 9], part: &str, lists: bool) -> Result<Vec<Element>, String> {
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
                out.push(self.para(p, bi, ppr_e, None, bullets, &lang, part)?);
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
            out.push(self.para(p, bi, ppr_e, Some(lvl), bullets, &lang, part)?);
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
        self.runs(&mut pel, &p.content, bi, lang, part)?;
        if let Some(x) = end {
            pel.children.push(Node::El(x));
        }
        Ok(pel)
    }

    fn runs(&self, pel: &mut Element, c: &Inline, bi: usize, lang: &str, part: &str) -> Result<(), String> {
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
        let eff = c.written_marks(&|k| owner[k].map_or(Marks::NONE, |o| o.meta.marks));
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
        let key = |k: usize| (owner[k].map(|o| o.id), eff[k]);
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
                It::Seg(a, b) => self.seg(pel, c, &eff, a, b, owner[a], lang),
            }
        }
        Ok(())
    }

    /// Units `a..b` of one run: `a:r` elements, and `a:br` for line breaks.
    #[allow(clippy::too_many_arguments)]
    fn seg(&self, pel: &mut Element, c: &Inline, eff: &[Marks], a: usize, b: usize, own: Option<&Entry>, lang: &str) {
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
        let rpr = rpr.map(|mut r| {
            set_marks(&mut r, eff[a]);
            r
        });
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
            let paras = self.paras(it, &bullets, &info.part, true)?;
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

/// Every relationship id an attribute of `root` names (`r:id`, `r:embed`, …).
pub(crate) fn named_rel_ids(root: &Element) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    root.walk(&mut |e| ids.extend(e.attrs.iter().filter(|a| a.0.starts_with("r:")).map(|a| a.1.clone())));
    ids
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
}

impl Numbers {
    fn new(names: &BTreeSet<String>) -> Self {
        Numbers { names: names.clone() }
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

/// Whether a placeholder the text does not show (one in a group, a second
/// one of the slot) already fills slot `name`.
fn occupied(stands: &[Stand], layout: &LayoutInfo, name: &str) -> bool {
    stands.iter().any(|st| {
        let Stand::Raw(Node::El(e)) = st else { return false };
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
fn set_marks(rpr: &mut Element, want: Marks) {
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
