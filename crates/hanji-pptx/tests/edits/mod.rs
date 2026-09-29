//! The pptx edit set (§9), the Presentation counterpart of the Document's
//! E1–E10: P1 change a title, P2 edit a body bullet, P3 add a slide from a
//! layout, P4 delete a slide, P5 move a slide, P6 edit (or add) notes, P7
//! edit a `<shape>`'s text, P8 restyle a slide by changing its layout, P10
//! delete an object (a picture, chart, table or group: rule 8), P11 move an
//! object in its slide's z-order; P12 move a shape, P13 resize a picture,
//! P14 add a text box under a title, P15 align two objects' left edges
//! (geometry, §5.3); P9 makes them all in one revision.

use std::collections::HashSet;

use hanji_core::presentation::{geom_of, kind, shape_head, slide_head, slot_head, HeadKind};
use hanji_core::{Block, Kind, ListItem, Para, Place};
use hanji_format::{Geom, Inline};
use hanji_pptx::import::SlideInfo;
use hanji_pptx::DeckShell;
use hanji_testkit::{
    block_ranges, chars, deleted_span, entries_at, figure_or_word, ident, replace_span, span_of, Cx, Doc, Edit, EditFn,
};

pub const EDITS: [(&str, EditFn); 14] = [
    ("p1_title", p1_title),
    ("p2_bullet", p2_bullet),
    ("p3_add_slide", p3_add_slide),
    ("p4_delete_slide", p4_delete_slide),
    ("p5_move_slide", p5_move_slide),
    ("p6_notes", p6_notes),
    ("p7_shape", p7_shape),
    ("p8_layout", p8_layout),
    ("p10_delete_object", p10_delete_object),
    ("p11_move_object", p11_move_object),
    ("p12_move_shape", p12_move_shape),
    ("p13_resize_picture", p13_resize_picture),
    ("p14_add_text_box", p14_add_text_box),
    ("p15_align", p15_align),
];

/// A slide: its head and the end of its blocks.
struct Slide {
    head: usize,
    end: usize,
}

fn slides(blocks: &[Block]) -> Vec<Slide> {
    let heads: Vec<usize> = (0..blocks.len()).filter(|&k| blocks[k].head().is_some_and(|h| h.level == 0)).collect();
    heads
        .iter()
        .enumerate()
        .map(|(n, &h)| Slide { head: h, end: heads.get(n + 1).copied().unwrap_or(blocks.len()) })
        .collect()
}

/// The slide block `k` is on: its number from 1, the deck's slide count,
/// its head, and its title (the title slot's text, or else its first text
/// outside the notes; empty when it has none).
pub fn slide_at(blocks: &[Block], k: usize) -> (usize, usize, usize, String) {
    let ss = slides(blocks);
    let n = ss.iter().rposition(|s| s.head <= k).unwrap_or(0);
    let s = &ss[n];
    let (mut slot, mut title, mut first) = (None, None, None);
    for b in &blocks[s.head + 1..s.end] {
        match b {
            Block::Head(h) => slot = Some(kind(h)),
            Block::Para(p) => {
                let t: String = chars(&p.content).into_iter().filter(|c| (*c as u32) < 0xF0000).collect();
                let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
                if t.is_empty() {
                    continue;
                }
                if slot == Some(HeadKind::Slot { name: "title" }) {
                    title.get_or_insert(t);
                } else if slot != Some(HeadKind::Slot { name: "notes" }) {
                    first.get_or_insert(t);
                }
            }
            _ => {}
        }
    }
    let t = title.or(first).unwrap_or_default();
    let short: String = t.chars().take(30).collect();
    let more = if short.len() < t.len() { "…" } else { "" };
    (n + 1, ss.len(), s.head, if short.is_empty() { short } else { format!("{short}{more}") })
}

/// How an edit names the slide block `k` is on: `slide 3 of 7 ('지역별 현황')`.
fn slide_name(blocks: &[Block], k: usize) -> String {
    match slide_at(blocks, k) {
        (n, total, _, t) if t.is_empty() => format!("slide {n} of {total}"),
        (n, total, _, t) => format!("slide {n} of {total} ('{t}')"),
    }
}

/// The paragraphs of each slot or shape `pick` accepts: `(block, head kind)`.
fn paras_in<'b>(blocks: &'b [Block], pick: &dyn Fn(HeadKind) -> bool) -> Vec<(usize, &'b Para)> {
    let mut out = vec![];
    let mut on = false;
    for (k, b) in blocks.iter().enumerate() {
        match b {
            Block::Head(h) => on = pick(kind(h)),
            Block::Para(p) if on => out.push((k, p)),
            _ => {}
        }
    }
    out
}

/// A figure or word to change in one of `paras`, preferring one with a run
/// whose formatting shows (or several runs).
fn change_in(d: &Doc, cx: &Cx, paras: &[(usize, &Para)], name: &'static str, what: &str) -> Option<Edit> {
    let score = |bi: usize| {
        let es = entries_at(d, &[bi]);
        (es.iter().any(|e| cx.fmt.visible_run(e)), es.iter().filter(|e| e.kind == Kind::Run).count() > 1)
    };
    let mut cands: Vec<(usize, (usize, usize, String))> =
        paras.iter().filter_map(|(bi, p)| figure_or_word(&chars(&p.content)).map(|f| (*bi, f))).collect();
    cands.sort_by_key(|(bi, _)| std::cmp::Reverse(score(*bi)));
    let (bi, (s, e, new)) = cands.into_iter().next()?;
    let p = paras.iter().find(|x| x.0 == bi).unwrap().1;
    let old: String = chars(&p.content)[s..e].iter().collect();
    let mut ed =
        replace_span(d, &[bi], s, e, &new, name, format!("{what}: {old:?} → {new:?} on {}", slide_name(&d.blocks, bi)));
    ed.named = vec![slide_at(&d.blocks, bi).2];
    Some(ed)
}

fn p1_title(d: &Doc, cx: &Cx) -> Option<Edit> {
    let ps = paras_in(&d.blocks, &|k| k == HeadKind::Slot { name: "title" });
    change_in(d, cx, &ps, "P1 change a title", "title")
}

fn p2_bullet(d: &Doc, cx: &Cx) -> Option<Edit> {
    let body = |k: HeadKind| matches!(k, HeadKind::Slot { name } if name != "title" && name != "notes");
    let ps: Vec<(usize, &Para)> = paras_in(&d.blocks, &body).into_iter().filter(|(_, p)| p.item.is_some()).collect();
    change_in(d, cx, &ps, "P2 edit a body bullet", "bullet")
}

fn shell(cx: &Cx) -> DeckShell {
    DeckShell::of(cx.rem).unwrap()
}

fn item(first: bool, level: usize) -> Option<ListItem> {
    Some(ListItem { ordered: false, level, first })
}

fn para(text: &str, item: Option<ListItem>) -> Block {
    Block::Para(Para { style: String::new(), content: Inline::plain(text), item })
}

/// A new slide at the end, from a layout with a title and a body.
fn p3_add_slide(d: &Doc, cx: &Cx) -> Option<Edit> {
    let sh = shell(cx);
    let has = |l: &&hanji_pptx::deck::LayoutInfo, n: &str| l.slot(n).is_some();
    let layout = sh
        .deck
        .layouts
        .iter()
        .find(|l| has(l, "title") && has(l, "body"))
        .or_else(|| sh.deck.layouts.iter().find(|l| has(l, "title")))?;
    let mut blocks = d.blocks.clone();
    // Written with bare markers, a new slide reads back with each slot's layout box (§5.3).
    let at = |n: &str| layout.slot(n).and_then(|s| s.geom);
    blocks.extend([slide_head(&layout.name), slot_head("title", at("title")), para("새 슬라이드 제목", None)]);
    if has(&layout, "body") {
        blocks.extend([
            slot_head("body", at("body")),
            para("첫째 항목", item(true, 0)),
            para("둘째 항목 12", item(false, 1)),
        ]);
    }
    let n = d.blocks.len();
    let count = slides(&d.blocks).len() + 1;
    let mut ed = Edit::blocks(
        "P3 add a slide from a layout",
        format!("add slide {count} of {count} ('새 슬라이드 제목') from layout {:?}, at the end", layout.name),
        blocks,
        ident(n),
        HashSet::new(),
        true,
    );
    let end = cx.fmt.text_of(&d.blocks, cx.rem).len();
    ed.span = span_of(cx, d, &ed.blocks, end, end);
    Some(ed)
}

/// Whether a slide can go: nothing else links to it or shows it.
fn deletable(cx: &Cx, d: &Doc, s: &Slide) -> bool {
    let Some(e) = d.entries.iter().find(|e| e.kind == Kind::Slide && e.meta.tag == "slide" && e.path == [s.head])
    else {
        return false;
    };
    let info: SlideInfo = serde_json::from_str(&e.meta.aux[0]).unwrap();
    let parts = &cx.rem.parts;
    let linked = parts.iter().filter(|p| p.name.ends_with(".rels") && p.name.starts_with("ppt/slides/")).any(|p| {
        let src = hanji_package::opc::source_of_rels(&p.name).unwrap();
        src != info.part
            && hanji_package::opc::parse_rels(&p.data)
                .iter()
                .any(|r| !r.external && hanji_package::opc::resolve_target(&src, &r.target) == info.part)
    });
    let sh = shell(cx);
    let rid = hanji_pptx::xml::fragment(&info.sld_id).get("r:id").unwrap_or_default();
    !linked && !sh.pres.contains(&format!("r:id=\"{rid}\"/></p:custShow")) && !sh.pres.contains("custShowLst")
}

/// The slide with the most entries (after the first) goes.
fn p4_delete_slide(d: &Doc, cx: &Cx) -> Option<Edit> {
    let ss = slides(&d.blocks);
    let count =
        |s: &Slide| d.entries.iter().filter(|e| e.path.first().is_some_and(|&p| p >= s.head && p < s.end)).count();
    let s = ss.iter().skip(1).filter(|s| deletable(cx, d, s)).max_by_key(|s| (count(s), std::cmp::Reverse(s.head)))?;
    let (a, b) = (s.head, s.end);
    let mut blocks = d.blocks.clone();
    blocks.drain(a..b);
    let bmap = (0..d.blocks.len())
        .map(|k| {
            if k < a {
                Some(k)
            } else if k < b {
                None
            } else {
                Some(k - (b - a))
            }
        })
        .collect();
    let mut ed = Edit::blocks(
        "P4 delete a slide",
        format!("delete {}, {} blocks", slide_name(&d.blocks, a), b - a),
        blocks,
        bmap,
        (a..b).collect(),
        true,
    );
    ed.span = deleted_span(cx, d, &ed.blocks, a, b);
    ed.named = vec![a];
    Some(ed)
}

/// Whether two slides share a section (or the deck has none).
fn same_section(cx: &Cx, d: &Doc, a: &Slide, b: &Slide) -> bool {
    let sh = shell(cx);
    if !sh.pres.contains("sectionLst") {
        return true;
    }
    let id = |s: &Slide| {
        let e = d.entries.iter().find(|e| e.kind == Kind::Slide && e.meta.tag == "slide" && e.path == [s.head])?;
        let info: SlideInfo = serde_json::from_str(&e.meta.aux[0]).ok()?;
        hanji_pptx::xml::fragment(&info.sld_id).get("id")
    };
    let root = hanji_pptx::xml::fragment(&sh.pres);
    let mut sections: Vec<Vec<String>> = vec![];
    root.walk(&mut |e| {
        if e.local() == "section" {
            let mut ids = vec![];
            e.walk(&mut |x| {
                if x.local() == "sldId" {
                    ids.extend(x.get("id"));
                }
            });
            sections.push(ids);
        }
    });
    let (ia, ib) = (id(a), id(b));
    sections.iter().any(|s| ia.as_ref().is_some_and(|x| s.contains(x)) && ib.as_ref().is_some_and(|x| s.contains(x)))
}

/// The last slide moves before the one before it (two slides of one
/// section whose text differs: swapping identical ones changes no text).
fn p5_move_slide(d: &Doc, cx: &Cx) -> Option<Edit> {
    let ss = slides(&d.blocks);
    let differ = |a: &Slide, b: &Slide| d.blocks[a.head..a.end] != d.blocks[b.head..b.end];
    let k = (1..ss.len()).rev().find(|&k| differ(&ss[k - 1], &ss[k]) && same_section(cx, d, &ss[k - 1], &ss[k]))?;
    let (x, y) = (&ss[k - 1], &ss[k]);
    let order: Vec<usize> =
        (0..x.head).chain(y.head..y.end).chain(x.head..x.end).chain(y.end..d.blocks.len()).collect();
    let blocks = order.iter().map(|&o| d.blocks[o].clone()).collect();
    let mut bmap = vec![None; d.blocks.len()];
    for (n, &o) in order.iter().enumerate() {
        bmap[o] = Some(n);
    }
    let what = format!("move {} before {}", slide_name(&d.blocks, y.head), slide_name(&d.blocks, x.head));
    let mut ed = Edit::blocks("P5 move a slide", what, blocks, bmap, (x.head..y.end).collect(), false);
    ed.named = vec![y.head, x.head];
    Some(ed)
}

/// Edit a word of the notes, or give the first slide without notes some.
fn p6_notes(d: &Doc, cx: &Cx) -> Option<Edit> {
    let ps = paras_in(&d.blocks, &|k| k == HeadKind::Slot { name: "notes" });
    if let Some(ed) = change_in(d, cx, &ps, "P6 edit notes", "notes") {
        return Some(ed);
    }
    shell(cx).deck.notes.as_ref()?;
    let ss = slides(&d.blocks);
    let s = ss.iter().find(|s| !(s.head..s.end).any(|k| d.blocks[k].head().is_some_and(|h| h.key == "slot:notes")))?;
    let mut blocks = d.blocks.clone();
    blocks.splice(s.end..s.end, [slot_head("notes", None), para("발표자 메모 12", None)]);
    let bmap = (0..d.blocks.len()).map(|k| Some(if k < s.end { k } else { k + 2 })).collect();
    let touched = if s.end < d.blocks.len() { HashSet::from([s.end]) } else { HashSet::new() };
    let what = format!("notes for {}", slide_name(&d.blocks, s.head));
    let mut ed = Edit::blocks("P6 add notes", what, blocks, bmap, touched, true);
    let m = block_ranges(cx, d);
    ed.span = span_of(cx, d, &ed.blocks, m[s.end - 1].1, m[s.end - 1].1);
    ed.named = vec![s.head];
    Some(ed)
}

fn p7_shape(d: &Doc, cx: &Cx) -> Option<Edit> {
    let ps = paras_in(&d.blocks, &|k| matches!(k, HeadKind::Shape { .. }));
    change_in(d, cx, &ps, "P7 edit a shape's text", "shape")
}

/// A slide takes another layout that has every slot it uses.
fn p8_layout(d: &Doc, cx: &Cx) -> Option<Edit> {
    let sh = shell(cx);
    for s in slides(&d.blocks) {
        let Some(HeadKind::Slide { layout }) = d.blocks[s.head].head().map(kind) else { continue };
        let used: Vec<&str> = (s.head + 1..s.end)
            .filter_map(|k| match d.blocks[k].head().map(kind) {
                Some(HeadKind::Slot { name }) => Some(name),
                _ => None,
            })
            .filter(|n| *n != "notes")
            .collect();
        if used.is_empty() {
            continue;
        }
        let cur = sh.deck.layout(layout)?;
        let slot = |l: &hanji_pptx::deck::LayoutInfo, n: &str| l.slot(n).cloned();
        let fits = |l: &&hanji_pptx::deck::LayoutInfo| l.name != layout && used.iter().all(|u| slot(l, u).is_some());
        // Prefer a layout whose slots give the text the same bullets, then the fewest slots.
        let same_bullets = |l: &hanji_pptx::deck::LayoutInfo| {
            used.iter().all(|u| slot(l, u).map(|s| s.bullets) == slot(cur, u).map(|s| s.bullets))
        };
        let mut cands: Vec<&hanji_pptx::deck::LayoutInfo> = sh.deck.layouts.iter().filter(fits).collect();
        cands.sort_by_key(|l| (!same_bullets(l), l.slots.len()));
        let Some(to) = cands.first() else { continue };
        let mut blocks = d.blocks.clone();
        blocks[s.head] = slide_head(&to.name);
        let what = format!("{}: layout {layout:?} → {:?}", slide_name(&d.blocks, s.head), to.name);
        let mut ed = Edit::blocks(
            "P8 restyle a slide's layout",
            what,
            blocks,
            ident(d.blocks.len()),
            HashSet::from([s.head]),
            true,
        );
        ed.named = vec![s.head];
        return Some(ed);
    }
    None
}

/// Object heads (`object` head, then its placeholder) whose shape no
/// animation of their slide plays on: `(head, slide, kind)`.
fn objects(d: &Doc) -> Vec<(usize, usize, &str)> {
    let mut out = vec![];
    for sl in slides(&d.blocks) {
        let skel = d.entries.iter().find(|e| e.kind == Kind::Slide && e.meta.tag == "slide" && e.path == [sl.head]);
        let timing = skel.map_or(String::new(), |e| {
            let x = &e.xml[0];
            x.find("<p:timing").map_or(String::new(), |k| x[k..].to_string())
        });
        for k in sl.head..sl.end {
            if !d.blocks[k].head().is_some_and(|h| kind(h) == HeadKind::Object) {
                continue;
            }
            let Some(Block::Keep(id)) = d.blocks.get(k + 1) else { continue };
            let Some(e) = d.entries.iter().find(|e| e.meta.keep.as_ref().is_some_and(|x| &x.id == id)) else {
                continue;
            };
            let el = hanji_pptx::xml::fragment(&e.xml[0]);
            let mut animated = false;
            el.walk(&mut |x| {
                if x.is("p:cNvPr") {
                    animated |= x.get("id").is_some_and(|i| timing.contains(&format!("spid=\"{i}\"")));
                }
            });
            if !animated {
                out.push((k, sl.head, e.meta.keep.as_ref().map_or("object", |x| x.kind.as_str())));
            }
        }
    }
    out
}

/// The last object that no animation plays on goes, with the parts only it
/// used: the last that is not a picture when there is one (P13 resizes a picture).
fn p10_delete_object(d: &Doc, cx: &Cx) -> Option<Edit> {
    let all = objects(d);
    let &(a, slide, what) = all.iter().rev().find(|o| o.2 != "picture").or(all.last())?;
    let mut blocks = d.blocks.clone();
    blocks.drain(a..a + 2);
    let bmap = (0..d.blocks.len())
        .map(|k| {
            if k < a {
                Some(k)
            } else if k < a + 2 {
                None
            } else {
                Some(k - 2)
            }
        })
        .collect();
    let mut ed = Edit::blocks(
        "P10 delete an object",
        format!("delete the {what} on {}", slide_name(&d.blocks, a)),
        blocks,
        bmap,
        HashSet::from([a, a + 1]),
        true,
    );
    let m = block_ranges(cx, d);
    ed.span = span_of(cx, d, &ed.blocks, m[a].0, m[a + 1].1);
    ed.named = vec![slide];
    Some(ed)
}

/// The first object with an item before it on its slide moves before that item.
fn p11_move_object(d: &Doc, _cx: &Cx) -> Option<Edit> {
    let (a, slide, obj, prev) = objects(d).into_iter().find_map(|(a, slide, obj)| {
        let prev = (slide + 1..a).rev().find(|&k| d.blocks[k].head().is_some_and(|h| h.level == 1))?;
        Some((a, slide, obj, prev))
    })?;
    let order: Vec<usize> = (0..prev).chain([a, a + 1]).chain(prev..a).chain(a + 2..d.blocks.len()).collect();
    let blocks = order.iter().map(|&o| d.blocks[o].clone()).collect();
    let mut bmap = vec![None; d.blocks.len()];
    for (n, &o) in order.iter().enumerate() {
        bmap[o] = Some(n);
    }
    let behind = match d.blocks[prev].head().map(kind) {
        Some(HeadKind::Slot { name }) => format!("the {name} placeholder"),
        Some(HeadKind::Shape { name, .. }) => format!("shape '{name}'"),
        _ => "the object before it".into(),
    };
    let what = format!("move the {obj} on {} behind {behind} (before it in the z-order)", slide_name(&d.blocks, a));
    let mut ed = Edit::blocks("P11 move an object", what, blocks, bmap, (prev..a + 2).collect(), false);
    ed.named = vec![slide];
    Some(ed)
}

// ---------------------------------------------------------------- geometry (§5.3)

const PT: i64 = 12_700;

/// A box as the text shows it: `36 475 288 29`.
fn pts(g: &Geom) -> String {
    let p = hanji_format::pres::pt;
    format!("{} {} {} {}", p(g.x), p(g.y), p(g.w), p(g.h))
}

/// The slide size, else 720 × 540 pt.
fn slide_size(cx: &Cx) -> (i64, i64) {
    shell(cx).deck.size.unwrap_or((720 * PT, 540 * PT))
}

/// Whether a connector of the slide at `slide` is attached to shape `id`
/// (moving it alone is refused, §5.3).
fn attached(d: &Doc, slide: usize, id: u32) -> bool {
    let end = slides(&d.blocks).into_iter().find(|s| s.head == slide).map_or(slide + 1, |s| s.end);
    let pats = [format!("<a:stCxn id=\"{id}\""), format!("<a:endCxn id=\"{id}\"")];
    d.entries
        .iter()
        .filter(|e| e.path.first().is_some_and(|&k| (slide..end).contains(&k)))
        .any(|e| e.xml.iter().any(|x| pats.iter().any(|p| x.contains(p))))
}

/// The shape id of a shape head (`s7` → 7) or of an object head's placeholder.
fn shape_id_of(d: &Doc, k: usize) -> Option<u32> {
    match d.blocks[k].head().map(kind)? {
        HeadKind::Shape { id, .. } => id.strip_prefix('s')?.parse().ok(),
        HeadKind::Object => {
            let Some(Block::Keep(kid)) = d.blocks.get(k + 1) else { return None };
            let e = d.entries.iter().find(|e| e.meta.keep.as_ref().is_some_and(|x| &x.id == kid))?;
            let x = &e.xml[0];
            let at = x.find("<p:cNvPr id=\"")? + 13;
            x[at..].split('"').next()?.parse().ok()
        }
        _ => None,
    }
}

/// Boxed shapes and objects no connector is attached to: `(head, slide, box)`.
fn movable(d: &Doc, pick: &dyn Fn(&Doc, usize) -> bool) -> Vec<(usize, usize, Geom)> {
    let mut out = vec![];
    for sl in slides(&d.blocks) {
        for k in sl.head + 1..sl.end {
            let Some(h) = d.blocks[k].head() else { continue };
            let (Some(g), Some(id)) = (geom_of(h), shape_id_of(d, k)) else { continue };
            if g.rot == 0 && !g.flip_h && !g.flip_v && pick(d, k) && !attached(d, sl.head, id) {
                out.push((k, sl.head, g));
            }
        }
    }
    out
}

/// `blocks` with head `k` at `g`.
fn moved(d: &Doc, k: usize, g: Geom) -> Vec<Block> {
    let mut blocks = d.blocks.clone();
    if let Block::Head(h) = &mut blocks[k] {
        h.place = Some(Place::Box(g.shown()));
    }
    blocks
}

fn label(d: &Doc, k: usize) -> String {
    match d.blocks[k].head().map(kind) {
        Some(HeadKind::Shape { name, .. }) => format!("shape {name:?}"),
        Some(HeadKind::Object) => {
            let kid = match d.blocks.get(k + 1) {
                Some(Block::Keep(id)) => id.as_str(),
                _ => "",
            };
            let kind = d
                .entries
                .iter()
                .find(|e| e.meta.keep.as_ref().is_some_and(|x| x.id == kid))
                .and_then(|e| e.meta.keep.as_ref())
                .map_or("object", |x| x.kind.as_str());
            format!("the {kind}")
        }
        _ => "object".into(),
    }
}

fn geometry_edit(name: &'static str, what: String, blocks: Vec<Block>, k: usize, slide: usize) -> Edit {
    let n = blocks.len();
    let mut ed = Edit::blocks(name, what, blocks, ident(n), HashSet::from([k]), true);
    ed.named = vec![slide];
    ed
}

/// The first shape with a box moves half an inch (36 pt): right, else left,
/// down or up, the first way that keeps it on the slide.
fn p12_move_shape(d: &Doc, cx: &Cx) -> Option<Edit> {
    let is_shape = |d: &Doc, k: usize| matches!(d.blocks[k].head().map(kind), Some(HeadKind::Shape { .. }));
    let (w, h) = slide_size(cx);
    let step = 36 * PT;
    let (k, slide, g, to) = movable(d, &is_shape).into_iter().find_map(|(k, slide, g)| {
        let s = g.shown();
        let to = [
            (g.x + g.w + step <= w).then(|| Geom { x: s.x + step, ..s }),
            (g.x >= step).then(|| Geom { x: s.x - step, ..s }),
            (g.y + g.h + step <= h).then(|| Geom { y: s.y + step, ..s }),
            (g.y >= step).then(|| Geom { y: s.y - step, ..s }),
        ]
        .into_iter()
        .flatten()
        .next()?;
        Some((k, slide, g, to))
    })?;
    let what = format!("move {} from box {} to {} on {}", label(d, k), pts(&g), pts(&to), slide_name(&d.blocks, k));
    Some(geometry_edit("P12 move a shape", what, moved(d, k, to), k, slide))
}

/// The first picture grows by half about its top-left corner, or shrinks by a third when that would leave the slide.
fn p13_resize_picture(d: &Doc, cx: &Cx) -> Option<Edit> {
    let picture = |d: &Doc, k: usize| label(d, k) == "the picture";
    let (k, slide, g) = movable(d, &picture).into_iter().next()?;
    let (w, h) = slide_size(cx);
    let s = |v: i64, num: i64, den: i64| hanji_format::shown_pt(v * num / den) * PT;
    let big = Geom { w: s(g.w, 3, 2), h: s(g.h, 3, 2), ..g.shown() };
    let to = if big.x + big.w <= w && big.y + big.h <= h {
        big
    } else {
        Geom { w: s(g.w, 2, 3), h: s(g.h, 2, 3), ..g.shown() }
    };
    let what = format!("resize {} from box {} to {} on {}", label(d, k), pts(&g), pts(&to), slide_name(&d.blocks, k));
    Some(geometry_edit("P13 resize a picture", what, moved(d, k, to), k, slide))
}

/// A text box right under the first title with a box, as wide as it, 28 pt tall.
fn p14_add_text_box(d: &Doc, cx: &Cx) -> Option<Edit> {
    let (_, h) = slide_size(cx);
    let ss = slides(&d.blocks);
    for sl in &ss {
        let Some(t) = (sl.head + 1..sl.end).find(|&k| d.blocks[k].head().is_some_and(|h| h.key == "slot:title")) else {
            continue;
        };
        let Some(g) = d.blocks[t].head().and_then(geom_of) else { continue };
        let top = g.y + g.h + 6 * PT;
        if top + 28 * PT > h {
            continue;
        }
        let at = (t + 1..sl.end).find(|&k| d.blocks[k].head().is_some()).unwrap_or(sl.end);
        let bx = Geom { x: g.x, y: top, w: g.w, h: 28 * PT, ..Default::default() }.shown();
        let mut blocks = d.blocks.clone();
        blocks.splice(at..at, [shape_head("", "", Some(bx)), para("출처: 편집 12", None)]);
        let bmap = (0..d.blocks.len()).map(|k| Some(if k < at { k } else { k + 2 })).collect();
        let what = format!(
            "add a text box '출처: 편집 12' at box {} under the title of {}",
            pts(&bx),
            slide_name(&d.blocks, t)
        );
        let touched = if at < d.blocks.len() { HashSet::from([at]) } else { HashSet::new() };
        let mut ed = Edit::blocks("P14 add a text box", what, blocks, bmap, touched, true);
        let m = block_ranges(cx, d);
        ed.span = span_of(cx, d, &ed.blocks, m[at - 1].1, m[at - 1].1);
        ed.named = vec![sl.head];
        return Some(ed);
    }
    None
}

/// On the first slide with two boxed objects whose left edges differ, the second takes the first's left edge.
fn p15_align(d: &Doc, _: &Cx) -> Option<Edit> {
    let any = |_: &Doc, _: usize| true;
    let all = movable(d, &any);
    for (n, &(a, slide, ga)) in all.iter().enumerate() {
        let Some(&(b, _, gb)) = all[n + 1..].iter().find(|x| x.1 == slide && x.2.shown().x != ga.shown().x) else {
            continue;
        };
        let to = Geom { x: ga.x, ..gb.shown() }.shown();
        let what = format!(
            "align the left edge of {} (box {}) with {} (x {}) on {}",
            label(d, b),
            pts(&gb),
            label(d, a),
            hanji_format::pres::pt(ga.x),
            slide_name(&d.blocks, b)
        );
        return Some(geometry_edit("P15 align two objects", what, moved(d, b, to), b, slide));
    }
    None
}
