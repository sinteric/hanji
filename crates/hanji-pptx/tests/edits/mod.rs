//! The pptx edit set (§9), the Presentation counterpart of the Document's
//! E1–E10: P1 change a title, P2 edit a body bullet, P3 add a slide from a
//! layout, P4 delete a slide, P5 move a slide, P6 edit (or add) notes, P7
//! edit a `<shape>`'s text, P8 restyle a slide by changing its layout; P9
//! makes them all in one revision.

use std::collections::HashSet;

use hanji_core::presentation::{kind, slide_head, slot_head, HeadKind};
use hanji_core::{Block, Kind, ListItem, Para};
use hanji_format::Inline;
use hanji_pptx::import::SlideInfo;
use hanji_pptx::DeckShell;
use hanji_testkit::{chars, entries_at, figure_or_word, ident, replace_span, Cx, Doc, Edit, EditFn};

pub const EDITS: [(&str, EditFn); 8] = [
    ("p1_title", p1_title),
    ("p2_bullet", p2_bullet),
    ("p3_add_slide", p3_add_slide),
    ("p4_delete_slide", p4_delete_slide),
    ("p5_move_slide", p5_move_slide),
    ("p6_notes", p6_notes),
    ("p7_shape", p7_shape),
    ("p8_layout", p8_layout),
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
    Some(replace_span(d, &[bi], s, e, &new, name, format!("{what}: {old:?} → {new:?} in block {bi}")))
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

/// The old and new text of an edit, and the edit's span when the new text
/// is the old with `old[a..b]` replaced.
fn span_of(cx: &Cx, d: &Doc, new: &[Block], a: usize, b: usize) -> Option<(usize, usize, String)> {
    let (ot, nt) = (cx.fmt.text_of(&d.blocks, cx.rem), cx.fmt.text_of(new, cx.rem));
    let tail = ot.len() - b;
    (nt.len() >= a + tail && nt[..a] == ot[..a] && nt[nt.len() - tail..] == ot[b..])
        .then(|| (a, b, nt[a..nt.len() - tail].to_string()))
}

/// Where each block of `d` is in its text.
fn maps(cx: &Cx, d: &Doc) -> Vec<(usize, usize)> {
    let text = cx.fmt.text_of(&d.blocks, cx.rem);
    let (_, maps) = cx.fmt.model().resolve(&text, cx.rem, hanji_testkit::CAPS).unwrap();
    maps.iter().map(|m| (m.start, m.end)).collect()
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
    blocks.extend([slide_head(&layout.name), slot_head("title"), para("새 슬라이드 제목", None)]);
    if has(&layout, "body") {
        blocks.extend([slot_head("body"), para("첫째 항목", item(true, 0)), para("둘째 항목 12", item(false, 1))]);
    }
    let n = d.blocks.len();
    let mut ed = Edit::blocks(
        "P3 add a slide from a layout",
        format!("new slide from {:?} at the end", layout.name),
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
        format!("delete the slide at block {a} ({} blocks)", b - a),
        blocks,
        bmap,
        (a..b).collect(),
        true,
    );
    let m = maps(cx, d);
    ed.span = span_of(cx, d, &ed.blocks, m[a - 1].1, m[b - 1].1);
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
    let what = format!("move the slide at block {} before the one at block {}", y.head, x.head);
    Some(Edit::blocks("P5 move a slide", what, blocks, bmap, (x.head..y.end).collect(), false))
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
    blocks.splice(s.end..s.end, [slot_head("notes"), para("발표자 메모 12", None)]);
    let bmap = (0..d.blocks.len()).map(|k| Some(if k < s.end { k } else { k + 2 })).collect();
    let touched = if s.end < d.blocks.len() { HashSet::from([s.end]) } else { HashSet::new() };
    let mut ed =
        Edit::blocks("P6 add notes", format!("notes for the slide at block {}", s.head), blocks, bmap, touched, true);
    let m = maps(cx, d);
    ed.span = span_of(cx, d, &ed.blocks, m[s.end - 1].1, m[s.end - 1].1);
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
        let what = format!("slide at block {}: layout {layout:?} → {:?}", s.head, to.name);
        return Some(Edit::blocks(
            "P8 restyle a slide's layout",
            what,
            blocks,
            ident(d.blocks.len()),
            HashSet::from([s.head]),
            true,
        ));
    }
    None
}
