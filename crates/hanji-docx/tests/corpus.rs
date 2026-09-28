//! §9 on the prototype's 13-file corpus: GetPut, the scripted edits E1–E10
//! with PutGet and remainder outcomes against the oracle, and validity.
//! A port of prototype/remainder (edits.py, compose.py, check.py, run_all.py).
//! Set HANJI_REPORT=1 to print per-file numbers, HANJI_SOFFICE=1 to also
//! convert every export to PDF with LibreOffice.
#![allow(clippy::type_complexity, clippy::too_many_arguments)]

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

use hanji_core::diff::{span_ops, Op};
use hanji_core::place::{self, Alignment, Outcome, Status};
use hanji_core::{Block, Capabilities, Engine, Entry, ImportOptions, Kind, Para, Path, Remainder};
use hanji_docx::{export_document, package, write_package, xml, DocxEngine};
use hanji_format::{Atom, Cell, Inline, Marks, Unit};

const CAPS: Capabilities = Capabilities { links: false, fields: false, footnotes: false, math: false };

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap().to_path_buf()
}

fn corpus() -> Vec<(String, Vec<u8>)> {
    let dir = root().join("prototype/remainder/corpus");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "docx"))
        .collect();
    v.sort();
    v.into_iter().map(|p| (p.file_name().unwrap().to_string_lossy().into_owned(), std::fs::read(&p).unwrap())).collect()
}

fn doc_xml(pkg: &[u8]) -> Vec<u8> {
    package::get(&package::read(pkg).unwrap(), "word/document.xml").unwrap().to_vec()
}

// ---------------------------------------------------------------- the model as the edits see it

#[derive(Clone)]
struct Doc {
    blocks: Vec<Block>,
    entries: Vec<Entry>,
}

const KEEP_CH: char = '\u{F0000}';

/// A paragraph's anchor string: text, placeholders as one private character.
fn chars(i: &Inline) -> Vec<char> {
    i.units
        .iter()
        .map(|u| match &u.atom {
            Atom::Char(c) => *c,
            Atom::Break => '\n',
            Atom::Keep(_) => KEEP_CH,
            _ => '\u{F0001}',
        })
        .collect()
}

/// Paragraphs the edits may target (text cells only).
fn paras(blocks: &[Block]) -> Vec<(Path, &Inline)> {
    let mut out = vec![];
    for (i, b) in blocks.iter().enumerate() {
        match b {
            Block::Para(p) => out.push((vec![i], &p.content)),
            Block::Table(t) => {
                for (r, row) in t.rows.iter().enumerate() {
                    for (c, cell) in row.iter().enumerate() {
                        if let Cell::Text(ps) = cell {
                            out.extend(ps.iter().enumerate().map(|(k, p)| (vec![i, r, c, k], &p.content)));
                        }
                    }
                }
            }
            Block::Keep(_) => {}
        }
    }
    out
}

fn get_para<'a>(blocks: &'a mut [Block], path: &[usize]) -> &'a mut Inline {
    match &mut blocks[path[0]] {
        Block::Para(p) => &mut p.content,
        Block::Table(t) => match &mut t.rows[path[1]][path[2]] {
            Cell::Text(ps) => &mut ps[path[3]].content,
            _ => panic!("not a text cell"),
        },
        _ => panic!("not a paragraph"),
    }
}

fn entries_at<'a>(d: &'a Doc, path: &[usize]) -> Vec<&'a Entry> {
    d.entries.iter().filter(|e| e.path == path).collect()
}

/// `<pagebreak/>` is a block line in the text, not a paragraph an edit can join or restyle.
fn is_page_break(i: &Inline) -> bool {
    i.units.len() == 1 && i.units[0].atom == Atom::PageBreak
}

fn clean(p: &[char], s: usize, e: usize) -> bool {
    p[s..e].iter().all(|&c| c != KEEP_CH && c != '\u{F0001}' && c != '\t' && c != '\n')
}

const VISIBLE_RPR: &[&str] = &["color", "sz", "highlight", "u", "shd", "strike", "caps", "vertAlign"];
const PREFERRED_STYLES: &[&str] =
    &["Quote", "Intense Quote", "List Paragraph", "Body Text", "Subtitle", "Title", "Note", "Quotations"];

fn visible_rpr(e: &Entry) -> bool {
    e.kind == Kind::Run
        && e.xml.len() >= 2
        && xml::fragment(&e.xml[1]).elements().any(|c| VISIBLE_RPR.contains(&c.local()))
}

fn has_sect(d: &Doc, i: usize) -> bool {
    entries_at(d, &[i]).iter().any(|e| e.kind == Kind::Ppr && e.xml.len() > 1 && e.xml[1].contains("sectPr"))
}

type Xmap = HashMap<Path, Vec<(usize, usize, Path, usize)>>;

struct Edit {
    name: &'static str,
    what: String,
    blocks: Vec<Block>,
    bmap: Vec<Option<usize>>,
    true_ops: HashMap<Path, Vec<Op>>,
    touched: HashSet<usize>,
    xmap: Xmap,
    lenient: HashSet<Path>,
    /// For the exact-span run: the edited units (path, start, end), if one paragraph span.
    unit_span: Option<(Path, usize, usize)>,
    /// Whether the edit is one contiguous text change (not a move).
    local: bool,
}

fn ident(n: usize) -> Vec<Option<usize>> {
    (0..n).map(Some).collect()
}

fn replace_span(d: &Doc, path: &[usize], s: usize, e: usize, new: &str, name: &'static str, what: String) -> Edit {
    let mut blocks = d.blocks.clone();
    let p = get_para(&mut blocks, path);
    let n = p.units.len();
    let src = if e > s { s } else { s.saturating_sub(1) };
    let marks = p.units.get(src).map_or(Marks::NONE, |u| u.marks);
    let ins: Vec<Unit> = new.chars().map(|c| Unit { atom: Atom::Char(c), marks }).collect();
    let k = ins.len();
    p.units.splice(s..e, ins);
    let true_ops = HashMap::from([(path.to_vec(), span_ops(n, s, e, k))]);
    Edit {
        name,
        what,
        bmap: ident(blocks.len()),
        blocks,
        true_ops,
        touched: HashSet::from([path[0]]),
        xmap: HashMap::new(),
        lenient: HashSet::new(),
        unit_span: Some((path.to_vec(), s, e)),
        local: true,
    }
}

/// `str(int(digits) * 10 + 5)` without overflow.
fn times_ten_plus_five(digits: &str) -> String {
    let t = digits.trim_start_matches('0');
    format!("{t}5")
}

fn runs_of(p: &[char], pred: impl Fn(char) -> bool, min: usize) -> Vec<(usize, usize)> {
    let mut out = vec![];
    let mut k = 0;
    while k < p.len() {
        if pred(p[k]) {
            let s = k;
            while k < p.len() && pred(p[k]) {
                k += 1;
            }
            if k - s >= min {
                out.push((s, k));
            }
        } else {
            k += 1;
        }
    }
    out
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn figure_or_word(p: &[char]) -> Option<(usize, usize, String)> {
    for (s, e) in runs_of(p, |c| c.is_ascii_digit(), 1) {
        if clean(p, s, e) {
            return Some((s, e, times_ten_plus_five(&p[s..e].iter().collect::<String>())));
        }
    }
    for (s, e) in runs_of(p, is_word, 3) {
        let w: String = p[s..e].iter().collect();
        if clean(p, s, e) && !w.chars().all(|c| c.is_ascii_digit()) {
            return Some((s, e, format!("{w}-2")));
        }
    }
    None
}

fn tag(e: &Entry) -> &str {
    e.meta.tag.strip_prefix("w:").unwrap_or(&e.meta.tag)
}

fn e1_figure(d: &Doc) -> Option<Edit> {
    let mut best: Option<((bool, bool, bool), Path, (usize, usize, String))> = None;
    for (path, p) in paras(&d.blocks) {
        let ms: Vec<&Entry> = entries_at(d, &path)
            .into_iter()
            .filter(|e| e.kind == Kind::Marker && (tag(e).starts_with("comment") || tag(e).starts_with("bookmark")))
            .collect();
        if ms.is_empty() {
            continue;
        }
        let Some(f) = figure_or_word(&chars(p)) else { continue };
        let score = (
            ms.iter().any(|e| tag(e).starts_with("comment")),
            ms.iter().any(|m| m.start.unwrap() >= f.1),
            f.2.chars().all(|c| c.is_ascii_digit()),
        );
        if best.as_ref().is_none_or(|b| score > b.0) {
            best = Some((score, path, f));
        }
    }
    let (score, path, (s, e, new)) = best?;
    let old: String =
        chars(&paras(&d.blocks).into_iter().find(|x| x.0 == path).unwrap().1.clone())[s..e].iter().collect();
    Some(replace_span(
        d,
        &path,
        s,
        e,
        &new,
        "E1 figure",
        format!("change {old:?} to {new:?} in a paragraph with a {}", if score.0 { "comment" } else { "bookmark" }),
    ))
}

fn has_drawing(d: &Doc, rem: &Remainder, b: &Block) -> bool {
    let kind_of = |id: &str| rem.keep(id).map(|k| k.kind.clone()).unwrap_or_default();
    match b {
        Block::Keep(id) => {
            let k = kind_of(id);
            (k == "drawing" || k == "table")
                && d.entries.iter().any(|e| {
                    e.kind == Kind::Bkeep
                        && e.meta.keep.as_ref().is_some_and(|x| &x.id == id)
                        && e.xml.iter().any(|x| x.contains("drawing") || x.contains("pict"))
                })
        }
        _ => {
            let ps: Vec<&Inline> = match b {
                Block::Para(p) => vec![&p.content],
                Block::Table(t) => t
                    .rows
                    .iter()
                    .flatten()
                    .filter_map(|c| if let Cell::Text(ps) = c { Some(ps.iter().map(|p| &p.content)) } else { None })
                    .flatten()
                    .collect(),
                _ => vec![],
            };
            ps.iter().any(|p| p.units.iter().any(|u| matches!(&u.atom, Atom::Keep(k) if kind_of(&k.id) == "drawing")))
        }
    }
}

fn e2_insert_before_drawing(d: &Doc, rem: &Remainder) -> Option<Edit> {
    let i = d.blocks.iter().position(|b| has_drawing(d, rem, b))?;
    let mut blocks = d.blocks.clone();
    let text = "Inserted paragraph before the drawing.";
    blocks.insert(i, Block::Para(Para { style: rem.styles.default_paragraph.clone(), content: Inline::plain(text) }));
    let bmap = (0..d.blocks.len()).map(|k| Some(if k < i { k } else { k + 1 })).collect();
    Some(Edit {
        name: "E2 insert before drawing",
        what: format!("new paragraph before block {i}"),
        blocks,
        bmap,
        true_ops: HashMap::new(),
        touched: HashSet::from([i]),
        xmap: HashMap::new(),
        lenient: HashSet::new(),
        unit_span: None,
        local: true,
    })
}

fn e3_delete_formatted(d: &Doc) -> Option<Edit> {
    for (i, b) in d.blocks.iter().enumerate() {
        let Block::Para(p) = b else { continue };
        if p.content.units.is_empty() || has_sect(d, i) {
            continue;
        }
        if entries_at(d, &[i]).iter().any(|e| visible_rpr(e)) {
            let mut blocks = d.blocks.clone();
            blocks.remove(i);
            let bmap = (0..d.blocks.len())
                .map(|k| {
                    if k < i {
                        Some(k)
                    } else if k == i {
                        None
                    } else {
                        Some(k - 1)
                    }
                })
                .collect();
            let what = format!("delete block {i} ({:?}…)", chars(&p.content).iter().take(30).collect::<String>());
            return Some(Edit {
                name: "E3 delete formatted paragraph",
                what,
                blocks,
                bmap,
                true_ops: HashMap::new(),
                touched: HashSet::from([i]),
                xmap: HashMap::new(),
                lenient: HashSet::new(),
                unit_span: None,
                local: true,
            });
        }
    }
    None
}

fn level(rem: &Remainder, b: &Block) -> Option<u8> {
    match b {
        Block::Para(p) if !p.content.units.is_empty() => rem.styles.heading_level(&p.style),
        _ => None,
    }
}

fn e4_move_section(d: &Doc, rem: &Remainder) -> Option<Edit> {
    let n = d.blocks.len();
    let heads: Vec<(usize, u8)> =
        d.blocks.iter().enumerate().filter_map(|(i, b)| level(rem, b).map(|l| (i, l))).collect();
    let mut secs = vec![];
    for (k, &(i, lv)) in heads.iter().enumerate() {
        let end = heads[k + 1..].iter().find(|x| x.1 <= lv).map_or(n, |x| x.0);
        secs.push((i, end, lv));
    }
    let mut pair = None;
    for a in &secs {
        for b in &secs {
            if a.1 == b.0 && a.2 == b.2 {
                pair = Some((*a, *b));
            }
        }
    }
    let (a0, b0, b1, what) = match pair {
        Some(((a0, _, _), (b0, b1, _))) => {
            (a0, b0, b1, format!("move the section at block {b0} ({} blocks) before the one at block {a0}", b1 - b0))
        }
        None => {
            let body: Vec<usize> = d
                .blocks
                .iter()
                .enumerate()
                .filter(|(_, b)| matches!(b, Block::Para(p) if !p.content.units.is_empty()))
                .map(|x| x.0)
                .collect();
            if body.len() < 4 {
                return None;
            }
            let (b0, b1, a0) = (body[body.len() - 3], body[body.len() - 1], body[0]);
            (a0, b0, b1, format!("no same-level headings: move blocks {b0}–{} before block {a0}", b1 - 1))
        }
    };
    let order: Vec<usize> = (0..a0).chain(b0..b1).chain(a0..b0).chain(b1..n).collect();
    let blocks = order.iter().map(|&k| d.blocks[k].clone()).collect();
    let mut bmap = vec![None; n];
    for (new, &old) in order.iter().enumerate() {
        bmap[old] = Some(new);
    }
    Some(Edit {
        name: "E4 move section",
        what,
        blocks,
        bmap,
        true_ops: HashMap::new(),
        touched: (a0..b1).collect(),
        xmap: HashMap::new(),
        lenient: HashSet::new(),
        unit_span: None,
        local: false,
    })
}

fn e5_restyle(d: &Doc, rem: &Remainder) -> Option<Edit> {
    let st = &rem.styles;
    let names: Vec<String> = st
        .paragraph
        .iter()
        .map(|s| s.name.clone())
        .filter(|n| *n != st.default_paragraph && st.heading_level(n).is_none())
        .collect();
    let used: HashSet<&str> =
        d.blocks.iter().filter_map(|b| if let Block::Para(p) = b { Some(p.style.as_str()) } else { None }).collect();
    let mut sorted = names.clone();
    sorted.sort();
    let pick = PREFERRED_STYLES
        .iter()
        .map(|s| s.to_string())
        .find(|s| names.contains(s))
        .or_else(|| sorted.iter().find(|s| used.contains(s.as_str())).cloned())
        .or_else(|| sorted.first().cloned())?;
    for (i, b) in d.blocks.iter().enumerate() {
        let Block::Para(p) = b else { continue };
        if p.content.units.is_empty() || is_page_break(&p.content) || p.style != st.default_paragraph {
            continue;
        }
        if entries_at(d, &[i]).iter().any(|e| matches!(e.kind, Kind::Run | Kind::Marker) && !e.fp.is_empty()) {
            let mut blocks = d.blocks.clone();
            if let Block::Para(p) = &mut blocks[i] {
                p.style = pick.clone();
            }
            return Some(Edit {
                name: "E5 restyle",
                what: format!("block {i} → style {pick:?}"),
                bmap: ident(blocks.len()),
                blocks,
                true_ops: HashMap::new(),
                touched: HashSet::from([i]),
                xmap: HashMap::new(),
                lenient: HashSet::new(),
                unit_span: None,
                local: true,
            });
        }
    }
    None
}

fn e6_cell_next_to_merge(d: &Doc) -> Option<Edit> {
    let mut fallback: Option<(Path, &'static str)> = None;
    for (i, b) in d.blocks.iter().enumerate() {
        let Block::Table(t) = b else { continue };
        let rows = &t.rows;
        for (r, row) in rows.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                let Cell::Text(ps) = cell else { continue };
                if ps[0].content.units.is_empty() {
                    continue;
                }
                let is_anchor =
                    row.get(c + 1) == Some(&Cell::Left) || rows.get(r + 1).and_then(|x| x.get(c)) == Some(&Cell::Up);
                let nb = [
                    (r as isize, c as isize - 1),
                    (r as isize, c as isize + 1),
                    (r as isize - 1, c as isize),
                    (r as isize + 1, c as isize),
                ];
                let adj = nb.iter().any(|&(y, x)| {
                    y >= 0
                        && x >= 0
                        && rows
                            .get(y as usize)
                            .and_then(|row| row.get(x as usize))
                            .is_some_and(|c| !matches!(c, Cell::Text(_)))
                });
                if adj && !is_anchor {
                    return Some(cell_edit(d, &[i, r, c, 0], "cell next to a merged cell"));
                }
                if fallback.is_none() {
                    fallback = Some((
                        vec![i, r, c, 0],
                        if adj || is_anchor {
                            "merged anchor cell (no free neighbour)"
                        } else {
                            "no merged cell in any modelled table"
                        },
                    ));
                }
            }
        }
    }
    fallback.map(|(p, why)| cell_edit(d, &p, why))
}

fn cell_edit(d: &Doc, path: &[usize], why: &str) -> Edit {
    let p = chars(paras(&d.blocks).into_iter().find(|x| x.0 == path).unwrap().1);
    for (s, e) in runs_of(&p, |c| c.is_ascii_digit(), 1) {
        if clean(&p, s, e) {
            let old: String = p[s..e].iter().collect();
            let new = times_ten_plus_five(&old);
            return replace_span(
                d,
                path,
                s,
                e,
                &new,
                "E6 table cell",
                format!("{why}: {old:?} → {new:?} in cell {:?}", &path[1..]),
            );
        }
    }
    let n = p.len();
    replace_span(d, path, n, n, " (rev.)", "E6 table cell", format!("{why}: append ' (rev.)' to cell {:?}", &path[1..]))
}

fn e7_overlap_run(d: &Doc) -> Option<Edit> {
    let mut cands: Vec<((bool, bool), Path, usize)> = vec![];
    for (path, p) in paras(&d.blocks) {
        let ch = chars(p);
        let mut runs: Vec<&Entry> =
            entries_at(d, &path).into_iter().filter(|e| e.kind == Kind::Run && e.end > e.start).collect();
        runs.sort_by_key(|e| e.start);
        for w in runs.windows(2) {
            let (x, y) = (w[0], w[1]);
            if x.end != y.start || x.fp == y.fp || (x.fp.is_empty() && y.fp.is_empty()) {
                continue;
            }
            let b = x.end.unwrap();
            if x.end.unwrap() - x.start.unwrap() >= 2
                && y.end.unwrap() - y.start.unwrap() >= 2
                && b + 2 <= ch.len()
                && clean(&ch, b - 2, b + 2)
            {
                cands.push(((path.len() == 1, visible_rpr(x) || visible_rpr(y)), path.clone(), b));
            }
        }
    }
    // Python's sort(reverse=True) is stable.
    cands.sort_by_key(|c| std::cmp::Reverse(c.0));
    let (_, path, b) = cands.into_iter().next()?;
    let p = chars(paras(&d.blocks).into_iter().find(|x| x.0 == path).unwrap().1);
    let old: String = p[b - 2..b + 2].iter().collect();
    Some(replace_span(
        d,
        &path,
        b - 2,
        b + 2,
        "EDITED",
        "E7 edit across a run boundary",
        format!("replace {old:?} (straddles two differently formatted runs) with 'EDITED'"),
    ))
}

fn e8_split(d: &Doc) -> Option<Edit> {
    let mut best: Option<((bool, bool, i64), usize, usize)> = None;
    for (i, b) in d.blocks.iter().enumerate() {
        let Block::Para(p) = b else { continue };
        let ch = chars(&p.content);
        let n = ch.len();
        if n < 12 {
            continue;
        }
        let ents: Vec<&Entry> = entries_at(d, &[i])
            .into_iter()
            .filter(|e| !e.fp.is_empty() && matches!(e.kind, Kind::Run | Kind::Marker | Kind::Wrap))
            .collect();
        let runs: Vec<&&Entry> =
            ents.iter().filter(|e| e.kind == Kind::Run && e.end.unwrap() - e.start.unwrap() >= 4).collect();
        for k in 2..n - 2 {
            if ch[k - 1] != ' ' || !clean(&ch, k - 1, k + 1) {
                continue;
            }
            let straddle = runs.iter().any(|r| r.start.unwrap() < k && k < r.end.unwrap());
            let both = ents.iter().any(|e| e.start.unwrap() < k) && ents.iter().any(|e| e.start.unwrap() >= k);
            let score = (straddle && both, both, -((k as i64) - (n as i64) / 2).abs());
            if best.as_ref().is_none_or(|b| score > b.0) {
                best = Some((score, i, k));
            }
        }
    }
    let (_, i, k) = best?;
    let mut blocks = d.blocks.clone();
    let Block::Para(p) = &mut blocks[i] else { unreachable!() };
    let n = p.content.units.len();
    let tail = p.content.units.split_off(k);
    let q = Block::Para(Para { style: p.style.clone(), content: Inline { units: tail, spans: vec![] } });
    blocks.insert(i + 1, q);
    let bmap = (0..d.blocks.len()).map(|x| Some(if x <= i { x } else { x + 1 })).collect();
    let xmap = HashMap::from([(vec![i], vec![(0, k, vec![i], 0), (k, n, vec![i + 1], 0)])]);
    Some(Edit {
        name: "E8 split paragraph",
        what: format!("split block {i} at offset {k} of {n}"),
        blocks,
        bmap,
        true_ops: HashMap::new(),
        touched: HashSet::from([i]),
        xmap,
        lenient: HashSet::from([vec![i]]),
        unit_span: None,
        local: true,
    })
}

fn e9_merge(d: &Doc) -> Option<Edit> {
    let mut best: Option<((bool, bool, usize), usize)> = None;
    for i in 0..d.blocks.len().saturating_sub(1) {
        let (Block::Para(a), Block::Para(b)) = (&d.blocks[i], &d.blocks[i + 1]) else { continue };
        if a.content.units.is_empty()
            || b.content.units.is_empty()
            || is_page_break(&a.content)
            || is_page_break(&b.content)
            || has_sect(d, i)
        {
            continue;
        }
        let ents: Vec<&Entry> = entries_at(d, &[i + 1])
            .into_iter()
            .filter(|e| !e.fp.is_empty() && matches!(e.kind, Kind::Run | Kind::Marker | Kind::Wrap))
            .collect();
        if ents.is_empty() {
            continue;
        }
        let score = (a.style == b.style, ents.iter().any(|e| e.kind == Kind::Marker), ents.len());
        if best.as_ref().is_none_or(|b| score > b.0) {
            best = Some((score, i));
        }
    }
    let (_, i) = best?;
    let mut blocks = d.blocks.clone();
    let Block::Para(b) = blocks.remove(i + 1) else { unreachable!() };
    let Block::Para(a) = &mut blocks[i] else { unreachable!() };
    let (n1, n2) = (a.content.units.len(), b.content.units.len());
    a.content.units.extend(b.content.units);
    let bmap = (0..d.blocks.len())
        .map(|x| {
            if x <= i {
                Some(x)
            } else if x == i + 1 {
                None
            } else {
                Some(x - 1)
            }
        })
        .collect();
    let xmap = HashMap::from([(vec![i], vec![(0, n1, vec![i], 0)]), (vec![i + 1], vec![(0, n2, vec![i], n1)])]);
    Some(Edit {
        name: "E9 merge paragraphs",
        what: format!("join blocks {i} and {}", i + 1),
        blocks,
        bmap,
        true_ops: HashMap::new(),
        touched: HashSet::from([i, i + 1]),
        xmap,
        lenient: HashSet::from([vec![i], vec![i + 1]]),
        unit_span: None,
        local: true,
    })
}

fn all_edits(d: &Doc, rem: &Remainder) -> Vec<(&'static str, Option<Edit>)> {
    vec![
        ("e1_figure", e1_figure(d)),
        ("e2_insert_before_drawing", e2_insert_before_drawing(d, rem)),
        ("e3_delete_formatted", e3_delete_formatted(d)),
        ("e4_move_section", e4_move_section(d, rem)),
        ("e5_restyle", e5_restyle(d, rem)),
        ("e6_cell_next_to_merge", e6_cell_next_to_merge(d)),
        ("e7_overlap_run", e7_overlap_run(d)),
        ("e8_split", e8_split(d)),
        ("e9_merge", e9_merge(d)),
    ]
}

// ---------------------------------------------------------------- oracle, E10

fn text_of(blocks: &[Block], rem: &Remainder) -> String {
    DocxEngine::text_of(blocks, rem, None)
}

fn written(text: &str, rem: &Remainder) -> Vec<Block> {
    hanji_core::model_of(text, rem, CAPS).unwrap_or_else(|e| panic!("{}\n{text}", hanji_format::diag::render(&e))).1
}

fn oracle(d: &Doc, new: &[Block], ed: &Edit) -> HashMap<u64, Outcome> {
    let al = Alignment {
        bmap: ed.bmap.clone(),
        para_ops: ed.true_ops.clone(),
        cross: ed.xmap.clone(),
        global: None,
        lenient: ed.lenient.clone(),
        ..Default::default()
    };
    place::place(&d.blocks, new, &d.entries, &al)
}

/// E10: every edit in one revision; the oracle composed step by step.
fn combined(d0: &Doc, rem: &Remainder) -> (Vec<Block>, HashMap<u64, Outcome>, Vec<String>) {
    let mut cur = d0.clone();
    let mut origin: HashMap<u64, u64> = d0.entries.iter().map(|e| (e.id, e.id)).collect();
    let mut next_id = 1_000_000;
    let mut gone: HashMap<u64, &'static str> = HashMap::new();
    let mut steps = vec![];
    let fns: Vec<fn(&Doc, &Remainder) -> Option<Edit>> = vec![
        |d, _| e1_figure(d),
        e2_insert_before_drawing,
        |d, _| e3_delete_formatted(d),
        e4_move_section,
        e5_restyle,
        |d, _| e6_cell_next_to_merge(d),
        |d, _| e7_overlap_run(d),
        |d, _| e8_split(d),
        |d, _| e9_merge(d),
    ];
    for f in fns {
        let Some(ed) = f(&cur, rem) else { continue };
        let new = written(&text_of(&ed.blocks, rem), rem);
        let outs = oracle(&cur, &new, &ed);
        let mut nxt = vec![];
        for e in &cur.entries {
            let o = &outs[&e.id];
            let oid = origin[&e.id];
            if o.lenient {
                gone.insert(oid, "lenient");
            }
            let Status::Placed { entry, pieces } = &o.status else {
                gone.entry(oid).or_insert(if matches!(o.status, Status::Refused(_)) { "orphan" } else { "removed" });
                continue;
            };
            if pieces.is_empty() {
                nxt.push(entry.clone());
                continue;
            }
            for (k, (path, s, en)) in pieces.iter().enumerate() {
                let mut x = entry.clone();
                (x.path, x.start, x.end) = (path.clone(), Some(*s), Some(*en));
                if k > 0 {
                    x.id = next_id;
                    next_id += 1;
                    origin.insert(x.id, oid);
                }
                nxt.push(x);
            }
        }
        cur = Doc { blocks: new, entries: nxt };
        steps.push(format!("{} ({})", ed.name, ed.what));
    }
    let mut live: HashMap<u64, Vec<&Entry>> = HashMap::new();
    for e in &cur.entries {
        live.entry(origin[&e.id]).or_default().push(e);
    }
    let mut expected = HashMap::new();
    for e in &d0.entries {
        let o = if gone.get(&e.id) == Some(&"lenient") {
            Outcome { status: Status::Removed("lenient".into()), lenient: true }
        } else if let Some(parts) = live.get(&e.id) {
            let pieces = if parts.len() > 1 {
                parts.iter().map(|x| (x.path.clone(), x.start.unwrap(), x.end.unwrap())).collect()
            } else {
                vec![]
            };
            Outcome { status: Status::Placed { entry: parts[0].clone(), pieces }, lenient: false }
        } else if gone.get(&e.id) == Some(&"orphan") {
            Outcome { status: Status::Refused("oracle".into()), lenient: false }
        } else {
            Outcome { status: Status::Removed("oracle".into()), lenient: false }
        };
        expected.insert(e.id, o);
    }
    (cur.blocks, expected, steps)
}

// ---------------------------------------------------------------- scoring (check.py)

type Key = (Kind, String, Path, Option<usize>, Option<usize>);

struct Observed {
    cov: HashMap<Path, HashMap<usize, String>>,
    multi: HashMap<Key, usize>,
}

fn obs_key(e: &Entry) -> Key {
    match e.kind {
        Kind::Marker | Kind::Rmarker | Kind::Keep => (e.kind, e.fp.clone(), e.path.clone(), e.start, None),
        Kind::Run if e.start == e.end => (e.kind, e.fp.clone(), e.path.clone(), e.start, None),
        Kind::Wrap => (e.kind, e.fp.clone(), e.path.clone(), e.start, e.end),
        _ => (e.kind, e.fp.clone(), e.path.clone(), None, None),
    }
}

impl Observed {
    fn new(entries: &[Entry]) -> Observed {
        let mut o = Observed { cov: HashMap::new(), multi: HashMap::new() };
        for e in entries {
            if e.kind == Kind::Run && e.end > e.start {
                for c in e.start.unwrap()..e.end.unwrap() {
                    o.cov.entry(e.path.clone()).or_default().insert(c, e.fp.clone());
                }
            } else {
                *o.multi.entry(obs_key(e)).or_default() += 1;
            }
        }
        o
    }

    fn take(&mut self, e: &Entry, pieces: &[(Path, usize, usize)]) -> bool {
        let pieces: Vec<(Path, usize, usize)> = if !pieces.is_empty() {
            pieces.to_vec()
        } else if e.kind == Kind::Run && e.end > e.start {
            vec![(e.path.clone(), e.start.unwrap(), e.end.unwrap())]
        } else {
            vec![]
        };
        if !pieces.is_empty() {
            return pieces
                .iter()
                .all(|(p, s, en)| (*s..*en).all(|c| self.cov.get(p).and_then(|m| m.get(&c)) == Some(&e.fp)));
        }
        match self.multi.get_mut(&obs_key(e)) {
            Some(n) if *n > 0 => {
                *n -= 1;
                true
            }
            _ => false,
        }
    }

    fn has_fp(&self, e: &Entry) -> bool {
        if e.kind == Kind::Run && e.end > e.start {
            return self.cov.values().any(|m| m.values().any(|f| *f == e.fp));
        }
        self.multi.iter().any(|(k, v)| k.0 == e.kind && k.1 == e.fp && *v > 0)
    }
}

#[derive(Default, Clone, Debug)]
struct Tally {
    landed: usize,
    orphaned: usize,
    lost: usize,
    touched: [usize; 3],
    excluded: usize,
    removed_with_text: usize,
    lost_by_kind: BTreeMap<String, usize>,
    examples: Vec<String>,
}

impl Tally {
    fn add(&mut self, o: &Tally) {
        self.landed += o.landed;
        self.orphaned += o.orphaned;
        self.lost += o.lost;
        for k in 0..3 {
            self.touched[k] += o.touched[k];
        }
        self.excluded += o.excluded;
        self.removed_with_text += o.removed_with_text;
        for (k, v) in &o.lost_by_kind {
            *self.lost_by_kind.entry(k.clone()).or_default() += v;
        }
    }
}

const COUNTED: &[Kind] = &[
    Kind::Ppr,
    Kind::Run,
    Kind::Marker,
    Kind::Rmarker,
    Kind::Wrap,
    Kind::Keep,
    Kind::Bkeep,
    Kind::Bmarker,
    Kind::Tbl,
    Kind::Tr,
    Kind::Tc,
    Kind::Tail,
];

fn score(
    d: &Doc,
    expected: &HashMap<u64, Outcome>,
    got: &HashMap<u64, Outcome>,
    observed: &[Entry],
    touched: &HashSet<usize>,
) -> Tally {
    let mut obs = Observed::new(observed);
    let mut t = Tally::default();
    let mut order: Vec<&Entry> = d.entries.iter().collect();
    order.sort_by_key(|e| (e.kind != Kind::Run, e.id));
    for e in order {
        if !COUNTED.contains(&e.kind) || e.fp.is_empty() {
            continue;
        }
        let (x, g) = (&expected[&e.id], &got[&e.id]);
        if x.lenient {
            t.excluded += 1;
            continue;
        }
        let res = match &x.status {
            Status::Placed { entry, pieces } => {
                if obs.take(entry, pieces) {
                    0
                } else if matches!(g.status, Status::Refused(_)) {
                    1
                } else {
                    2
                }
            }
            _ => match g.status {
                Status::Removed(_) => {
                    t.removed_with_text += 1;
                    0
                }
                Status::Refused(_) => 1,
                _ => 2,
            },
        };
        match res {
            0 => t.landed += 1,
            1 => t.orphaned += 1,
            _ => {
                t.lost += 1;
                *t.lost_by_kind.entry(format!("{:?}", e.kind)).or_default() += 1;
                if t.examples.len() < 4 {
                    let why = match (&x.status, &g.status) {
                        (Status::Placed { entry, .. }, Status::Placed { entry: ge, .. }) => format!(
                            "expected {:?}[{:?},{:?}] got {:?}[{:?},{:?}]{}",
                            entry.path,
                            entry.start,
                            entry.end,
                            ge.path,
                            ge.start,
                            ge.end,
                            if obs.has_fp(entry) { "" } else { " (missing)" }
                        ),
                        (a, b) => format!("expected {a:?} got {b:?}").chars().take(160).collect(),
                    };
                    t.examples.push(format!("{:?} @{:?}[{:?},{:?}]: {why}", e.kind, e.path, e.start, e.end));
                }
            }
        }
        if e.path.first().is_some_and(|p| touched.contains(p)) {
            t.touched[res.min(2)] += 1;
        }
    }
    t
}

// ---------------------------------------------------------------- the run

struct Run {
    tally: Tally,
    putget: bool,
    putget_ids: bool,
    well_formed: bool,
}

fn normalise_ids(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(k) = rest.find("<keep id=\"") {
        out.push_str(&rest[..k + 10]);
        rest = &rest[k + 10..];
        let e = rest.find('"').unwrap();
        out.push('?');
        rest = &rest[e..];
    }
    out + rest
}

fn export_and_check(
    name: &str,
    d: &Doc,
    rem: &Remainder,
    new_text: &str,
    new_blocks: &[Block],
    got: &HashMap<u64, Outcome>,
    expected: &HashMap<u64, Outcome>,
    touched: &HashSet<usize>,
    out_name: &str,
) -> Run {
    let mut next = rem.next_id;
    let entries = place::placed_entries(&d.entries, got, &mut next);
    let r2 = Remainder { entries, next_id: next, ..rem.clone() };
    let docxml = export_document(new_blocks, &r2).unwrap_or_else(|e| panic!("{name} {out_name}: {e}"));
    let well_formed = xml::parse(&docxml).is_ok();
    let pkg = write_package(rem, docxml).unwrap();
    save(name, out_name, &pkg);
    let (rb, rr, _, _) = DocxEngine::split(&pkg, &ImportOptions { neutralise: false, template: None }).unwrap();
    let re_text = text_of(&rb, &rr);
    let tally = score(d, expected, got, &rr.entries, touched);
    Run {
        tally,
        putget: re_text == new_text,
        putget_ids: normalise_ids(&re_text) == normalise_ids(new_text),
        well_formed,
    }
}

fn out_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("corpus-out")
}

fn save(name: &str, what: &str, pkg: &[u8]) {
    let dir = out_dir().join(name.trim_end_matches(".docx"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(format!("{what}.docx")), pkg).unwrap();
}

/// The exact text span of an edit: from its unit span when it has one,
/// else the minimal differing region.
fn text_span(old_text: &str, new_text: &str, rem: &Remainder, ed: &Edit) -> (usize, usize, String) {
    if let Some((path, s, e)) = &ed.unit_span {
        let parsed = hanji_format::parse_with(old_text, &hanji_core::edit::names(rem)).unwrap();
        let pm = match &parsed.map.blocks[path[0]].kind {
            hanji_format::BlockMapKind::Para(pm) => pm.clone(),
            hanji_format::BlockMapKind::Table(cells) => cells[path[1]][path[2]].clone().unwrap()[path[3]].clone(),
        };
        let at = |k: usize| if k < pm.units.len() { pm.units[k] } else { pm.mark };
        let (a, b) = (at(*s), at(*e));
        let delta = new_text.len() as isize - old_text.len() as isize;
        let nb = (b as isize + delta) as usize;
        if old_text[..a] == new_text[..a] && old_text[b..] == new_text[nb..] {
            return (a, b, new_text[a..nb].to_string());
        }
    }
    let pre = old_text.bytes().zip(new_text.bytes()).take_while(|(x, y)| x == y).count();
    let mut pre = pre;
    while !old_text.is_char_boundary(pre) || !new_text.is_char_boundary(pre) {
        pre -= 1;
    }
    let max_suf = old_text.len().min(new_text.len()) - pre;
    let mut suf = old_text.bytes().rev().zip(new_text.bytes().rev()).take_while(|(x, y)| x == y).count().min(max_suf);
    while !old_text.is_char_boundary(old_text.len() - suf) || !new_text.is_char_boundary(new_text.len() - suf) {
        suf -= 1;
    }
    (pre, old_text.len() - suf, new_text[pre..new_text.len() - suf].to_string())
}

#[derive(Default)]
struct Totals {
    by_design: BTreeMap<&'static str, Tally>,
    per_edit: BTreeMap<(String, &'static str), Tally>,
    runs: BTreeMap<&'static str, (usize, usize, usize, usize)>, // runs, putget, putget with ids normalised, well-formed
}

#[test]
fn corpus_getput_putget_remainder() {
    let report = std::env::var("HANJI_REPORT").is_ok();
    let mut totals = Totals::default();
    let mut getput_ok = 0;
    let mut failures = vec![];
    let files = corpus();
    for (name, bytes) in &files {
        let (blocks, rem, _, stats) =
            DocxEngine::split(bytes, &ImportOptions { neutralise: false, template: None }).unwrap();
        let d = Doc { blocks, entries: rem.entries.clone() };
        let text = text_of(&d.blocks, &rem);
        // The model text round-trips through the format crate.
        assert_eq!(text_of(&written(&text, &rem), &rem), text, "{name}: model text does not reparse to itself");
        // GetPut through the whole path: text → parse → place → export.
        let re = hanji_core::reanchor_rewrite(&rem, &text, &text, CAPS).unwrap();
        assert!(
            re.report.refused.is_empty() && re.report.removed.is_empty(),
            "{name}: GetPut moved entries: {:?}",
            re.report
        );
        let out = DocxEngine.export(&text, &re.remainder).unwrap();
        save(name, "getput", &out);
        save(name, "ORIGINAL", bytes);
        let same = xml::canon_part(&doc_xml(bytes)).unwrap() == xml::canon_part(&doc_xml(&out)).unwrap();
        let a = package::read(bytes).unwrap();
        let b = package::read(&out).unwrap();
        let others = a.len() == b.len()
            && a.iter().zip(&b).all(|(x, y)| x.name == y.name && (x.name == "word/document.xml" || x.data == y.data));
        if same && others {
            getput_ok += 1;
        } else {
            failures.push(format!("{name}: GetPut document.xml equal={same}, other parts byte-equal={others}"));
        }
        // Edits.
        let mut jobs: Vec<(
            String,
            String,
            String,
            HashMap<u64, Outcome>,
            HashSet<usize>,
            Option<(usize, usize, String)>,
        )> = vec![];
        for (fname, ed) in all_edits(&d, &rem) {
            let Some(ed) = ed else {
                if report {
                    println!("  {name} {fname}: n/a");
                }
                continue;
            };
            let new_text = text_of(&ed.blocks, &rem);
            let new_blocks = written(&new_text, &rem);
            let expected = oracle(&d, &new_blocks, &ed);
            let span = ed.local.then(|| text_span(&text, &new_text, &rem, &ed));
            jobs.push((ed.name.to_string(), ed.what.clone(), new_text, expected, ed.touched.clone(), span));
        }
        let (fb, fexp, steps) = combined(&d, &rem);
        jobs.push((
            "E10 all edits in one revision".into(),
            steps.join("; "),
            text_of(&fb, &rem),
            fexp,
            (0..d.blocks.len()).collect(),
            None,
        ));
        for (ename, what, new_text, expected, touched, span) in jobs {
            let short = ename.split_whitespace().next().unwrap().to_string();
            let mut designs: Vec<(&'static str, hanji_core::Reanchored)> =
                vec![("C", hanji_core::reanchor_rewrite(&rem, &text, &new_text, CAPS).unwrap())];
            if let Some((s, e, ref new)) = span {
                let r = hanji_core::reanchor_span(&rem, &text, s, e, new, CAPS)
                    .unwrap_or_else(|e| panic!("{name} {ename}: {e}"));
                assert_eq!(r.text, new_text, "{name} {ename}: the exact span does not give the edited text");
                designs.push(("exact", r));
            }
            for (design, r) in designs {
                let run = export_and_check(
                    name,
                    &d,
                    &rem,
                    &new_text,
                    &r.new,
                    &r.outcomes,
                    &expected,
                    &touched,
                    &format!("{short}-{design}"),
                );
                if !run.putget_ids {
                    failures.push(format!("{name} {ename} {design}: PutGet fails"));
                }
                if !run.well_formed {
                    failures.push(format!("{name} {ename} {design}: not well-formed"));
                }
                let rs = totals.runs.entry(design).or_default();
                rs.0 += 1;
                rs.1 += run.putget as usize;
                rs.2 += run.putget_ids as usize;
                rs.3 += run.well_formed as usize;
                if report && (run.tally.lost > 0 || run.tally.orphaned > 0) {
                    println!(
                        "  {name} {ename} [{design}] {what}: landed {} orphaned {} lost {} {:?}",
                        run.tally.landed, run.tally.orphaned, run.tally.lost, run.tally.examples
                    );
                }
                totals.by_design.entry(design).or_default().add(&run.tally);
                totals.per_edit.entry((short.clone(), design)).or_default().add(&run.tally);
            }
        }
        if report {
            println!(
                "{name}: blocks {} entries {} tables {}/{} kept {:?}",
                d.blocks.len(),
                d.entries.len(),
                stats.tables_modelled,
                stats.tables_kept,
                stats.kept_reasons
            );
        }
    }
    println!("GetPut: {getput_ok}/{} (document.xml canonically equal, other parts byte-equal)", files.len());
    for (design, (n, pg, pgi, wf)) in &totals.runs {
        println!("[{design}] runs {n}: PutGet exact {pg}/{n}, with placeholder ids normalised {pgi}/{n}, well-formed {wf}/{n}");
    }
    for (design, t) in &totals.by_design {
        let tt: usize = t.touched.iter().sum();
        let pct = |x: usize| 100.0 * x as f64 / tt.max(1) as f64;
        println!(
            "[{design}] touched blocks: landed {} ({:.1}%), orphaned {} ({:.1}%), lost {} ({:.1}%); all entries: landed {} orphaned {} lost {}; lost by kind {:?}",
            t.touched[0], pct(t.touched[0]), t.touched[1], pct(t.touched[1]), t.touched[2], pct(t.touched[2]), t.landed, t.orphaned, t.lost, t.lost_by_kind
        );
    }
    for ((ed, design), t) in &totals.per_edit {
        println!("  {ed:4} [{design:5}] touched {} / {} / {}", t.touched[0], t.touched[1], t.touched[2]);
    }
    #[cfg(not(target_os = "wasi"))]
    if std::env::var("HANJI_SOFFICE").is_ok() {
        soffice();
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(getput_ok, files.len());
}

#[cfg(not(target_os = "wasi"))]
fn soffice() {
    use std::process::Command;
    let Ok(v) = Command::new("soffice").arg("--version").output() else {
        println!("soffice: not available");
        return;
    };
    // `/Type /Page` objects (not `/Pages`).
    let pages = |p: &std::path::Path| -> Option<usize> {
        let d = std::fs::read(p).ok().filter(|d| !d.is_empty())?;
        let mut n = 0;
        for k in 0..d.len().saturating_sub(5) {
            if &d[k..k + 5] == b"/Type" {
                let mut x = k + 5;
                while d.get(x).is_some_and(|c| c.is_ascii_whitespace()) {
                    x += 1;
                }
                if d[x..].starts_with(b"/Page") && d.get(x + 5) != Some(&b's') {
                    n += 1;
                }
            }
        }
        Some(n)
    };
    let (mut ok, mut total, mut differ) = (0, 0, vec![]);
    for dir in std::fs::read_dir(out_dir()).unwrap() {
        let dir = dir.unwrap().path();
        let files: Vec<PathBuf> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "docx"))
            .collect();
        let pdf = dir.join("pdf");
        let _ = std::fs::create_dir_all(&pdf);
        let _ = Command::new("soffice")
            .args(["--headless", "--convert-to", "pdf", "--outdir"])
            .arg(&pdf)
            .args(&files)
            .output();
        let original = pages(&pdf.join("ORIGINAL.pdf"));
        for f in files.iter().filter(|f| !f.ends_with("ORIGINAL.docx")) {
            total += 1;
            let p = pdf.join(f.with_extension("pdf").file_name().unwrap());
            match pages(&p) {
                Some(n) => {
                    ok += 1;
                    if f.ends_with("getput.docx") && Some(n) != original {
                        differ.push(format!("{}: {n} pages, original {original:?}", dir.display()));
                    }
                }
                None => println!("soffice: failed {}", f.display()),
            }
        }
    }
    println!("soffice: GetPut exports whose page count differs from the original: {differ:?}");
    println!("soffice ({}): {ok}/{total} exports converted to PDF", String::from_utf8_lossy(&v.stdout).trim());
}

#[test]
fn neutralised_import_keeps_getput_for_the_rest() {
    for (name, bytes) in corpus() {
        let imp = DocxEngine.import(&bytes, &ImportOptions::default()).unwrap();
        let out = DocxEngine.export(&imp.text, &imp.remainder).unwrap();
        let (_, _, report2, _) = DocxEngine::split(&out, &ImportOptions::default()).unwrap();
        assert!(
            report2.neutralised.is_empty(),
            "{name}: a second import still finds active content: {:?}",
            report2.neutralised
        );
        let again = DocxEngine.import(&out, &ImportOptions::default()).unwrap();
        assert_eq!(again.text, imp.text, "{name}");
    }
}

#[test]
fn a_serialized_remainder_exports_the_same_package() {
    for (name, bytes) in corpus() {
        let imp = DocxEngine.import(&bytes, &ImportOptions { neutralise: false, template: None }).unwrap();
        let json = imp.remainder.to_json();
        let back = Remainder::from_json(&json).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(back, imp.remainder, "{name}: the remainder does not round-trip through JSON");
        let a = DocxEngine.export(&imp.text, &imp.remainder).unwrap();
        let b = DocxEngine.export(&imp.text, &back).unwrap();
        assert_eq!(a, b, "{name}: export from the deserialized remainder differs");
        // GetPut from the deserialized remainder.
        assert_eq!(xml::canon_part(&doc_xml(&bytes)).unwrap(), xml::canon_part(&doc_xml(&b)).unwrap(), "{name}");
    }
}
