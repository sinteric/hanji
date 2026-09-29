//! §9 on a corpus, for any engine: GetPut, the scripted edits E1–E10 with
//! PutGet and remainder outcomes against the oracle, and validity. A port
//! of prototype/remainder (edits.py, compose.py, check.py, run_all.py); an
//! engine plugs in through [`Format`]. Set HANJI_REPORT=1 to print per-file
//! numbers.
#![allow(clippy::type_complexity, clippy::too_many_arguments)]

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path as FsPath, PathBuf};

use hanji_core::diff::{span_ops, Op};
use hanji_core::place::{self, Alignment, Outcome, Status};
use hanji_core::{
    Block, Capabilities, DocumentModel, Engine, EngineError, Entry, ImportOptions, ImportReport, Kind, Para, Path,
    Remainder, TextModel,
};
use hanji_format::{Atom, Cell, Inline, Marks, Unit};
use hanji_package::{package, xml};

pub const CAPS: Capabilities = Capabilities { links: false, fields: false, footnotes: false, math: false };

/// What the harness needs from an engine.
pub trait Format {
    fn engine(&self) -> &dyn Engine;
    /// Extension of the corpus files (`docx`).
    fn ext(&self) -> &'static str;
    /// Import into blocks and remainder, with a line about the split for the report.
    fn split(
        &self,
        pkg: &[u8],
        opts: &ImportOptions,
    ) -> Result<(Vec<Block>, Remainder, ImportReport, String), EngineError>;
    fn text_of(&self, blocks: &[Block], rem: &Remainder) -> String;
    /// The package for resolved blocks placed against `rem`, and whether
    /// every part the engine wrote is well-formed XML.
    fn export_blocks(&self, blocks: &[Block], rem: &Remainder) -> Result<(Vec<u8>, bool), EngineError>;
    /// A part the engine splits: GetPut compares it canonically, the rest byte for byte.
    fn is_split_part(&self, name: &str) -> bool;
    /// A run entry whose formatting shows (colour, size, …): what E3 and E7 look for.
    fn visible_run(&self, e: &Entry) -> bool;
    /// Whether block `i` (with its entries) holds its section's settings: E3 and E9 leave it.
    fn holds_section(&self, entries: &[&Entry], i: usize) -> bool;
    /// Paragraph styles E5 prefers, in order.
    fn preferred_styles(&self) -> &'static [&'static str];
    /// Whether a block placeholder's XML draws something (E2).
    fn is_drawing(&self, xml: &str) -> bool;
    /// The text grammar (a Document's unless the engine says otherwise).
    fn model(&self) -> &dyn TextModel {
        &DocumentModel
    }
}

/// The files with extension `ext` in `dir`, by name.
pub fn corpus(dir: &FsPath, ext: &str) -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == ext))
        .collect();
    v.sort();
    v.into_iter().map(|p| (p.file_name().unwrap().to_string_lossy().into_owned(), std::fs::read(&p).unwrap())).collect()
}

/// GetPut on two packages: the split parts canonically equal, and every other part byte-equal.
pub fn getput_equal(fmt: &dyn Format, original: &[u8], out: &[u8]) -> (bool, bool) {
    let (a, b) = (package::read(original).unwrap(), package::read(out).unwrap());
    let canon = |d: &[u8]| xml::canon_part(d).ok();
    let split = a.iter().zip(&b).all(|(x, y)| !fmt.is_split_part(&x.name) || canon(&x.data) == canon(&y.data));
    let others = a.len() == b.len()
        && a.iter().zip(&b).all(|(x, y)| x.name == y.name && (fmt.is_split_part(&x.name) || x.data == y.data));
    (split, others)
}

// ---------------------------------------------------------------- the model as the edits see it

/// A revision as the edits see it: its blocks and their entries.
#[derive(Clone)]
pub struct Doc {
    pub blocks: Vec<Block>,
    pub entries: Vec<Entry>,
}

/// What the edits read besides the document: the engine and the remainder.
pub struct Cx<'a> {
    pub fmt: &'a dyn Format,
    pub rem: &'a Remainder,
}

const KEEP_CH: char = '\u{F0000}';

/// A paragraph's anchor string: text, placeholders as one private character.
pub fn chars(i: &Inline) -> Vec<char> {
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
            Block::Keep(_) | Block::Head(_) => {}
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

pub fn entries_at<'a>(d: &'a Doc, path: &[usize]) -> Vec<&'a Entry> {
    d.entries.iter().filter(|e| e.path == path).collect()
}

/// `<pagebreak/>` is a block line in the text, not a paragraph an edit can join or restyle.
fn is_page_break(i: &Inline) -> bool {
    i.units.len() == 1 && i.units[0].atom == Atom::PageBreak
}

/// Characters `s..e` hold no placeholder, atom, tab or line break.
pub fn clean(p: &[char], s: usize, e: usize) -> bool {
    p[s..e].iter().all(|&c| c != KEEP_CH && c != '\u{F0001}' && c != '\t' && c != '\n')
}

pub type Xmap = HashMap<Path, Vec<(usize, usize, Path, usize)>>;

/// A scripted edit: the new blocks, and the true alignment the oracle places by.
pub struct Edit {
    pub name: &'static str,
    pub what: String,
    pub blocks: Vec<Block>,
    pub bmap: Vec<Option<usize>>,
    pub true_ops: HashMap<Path, Vec<Op>>,
    pub touched: HashSet<usize>,
    pub xmap: Xmap,
    pub lenient: HashSet<Path>,
    /// For the exact-span run: the edited units (path, start, end), if one paragraph span.
    pub unit_span: Option<(Path, usize, usize)>,
    /// Whether the edit is one contiguous text change (not a move).
    pub local: bool,
    /// The exact text edit `(start, end, new)`, when the edit knows it
    /// (a whole slide deleted spans whole lines).
    pub span: Option<(usize, usize, String)>,
}

impl Edit {
    /// An edit that changes `blocks` as `bmap` says, with no paragraph-level ops.
    pub fn blocks(
        name: &'static str,
        what: String,
        blocks: Vec<Block>,
        bmap: Vec<Option<usize>>,
        touched: HashSet<usize>,
        local: bool,
    ) -> Edit {
        Edit {
            name,
            what,
            blocks,
            bmap,
            true_ops: HashMap::new(),
            touched,
            xmap: HashMap::new(),
            lenient: HashSet::new(),
            unit_span: None,
            local,
            span: None,
        }
    }
}

pub fn ident(n: usize) -> Vec<Option<usize>> {
    (0..n).map(Some).collect()
}

/// Units `s..e` of the paragraph at `path` become `new` (with the marks of the first replaced unit).
pub fn replace_span(d: &Doc, path: &[usize], s: usize, e: usize, new: &str, name: &'static str, what: String) -> Edit {
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
        span: None,
    }
}

/// `str(int(digits) * 10 + 5)` without overflow.
pub fn times_ten_plus_five(digits: &str) -> String {
    let t = digits.trim_start_matches('0');
    format!("{t}5")
}

pub fn runs_of(p: &[char], pred: impl Fn(char) -> bool, min: usize) -> Vec<(usize, usize)> {
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

/// A figure to change (`12` → `125`) or a word (`word` → `word-2`) in a paragraph.
pub fn figure_or_word(p: &[char]) -> Option<(usize, usize, String)> {
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

/// A marker's element name without its prefix.
fn tag(e: &Entry) -> &str {
    e.meta.tag.rsplit(':').next().unwrap_or(&e.meta.tag)
}

fn e1_figure(d: &Doc, _: &Cx) -> Option<Edit> {
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

fn has_drawing(d: &Doc, cx: &Cx, b: &Block) -> bool {
    let kind_of = |id: &str| cx.rem.keep(id).map(|k| k.kind.clone()).unwrap_or_default();
    match b {
        Block::Keep(id) => {
            let k = kind_of(id);
            (k == "drawing" || k == "table")
                && d.entries.iter().any(|e| {
                    e.kind == Kind::Bkeep
                        && e.meta.keep.as_ref().is_some_and(|x| &x.id == id)
                        && e.xml.iter().any(|x| cx.fmt.is_drawing(x))
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

fn e2_insert_before_drawing(d: &Doc, cx: &Cx) -> Option<Edit> {
    let i = d.blocks.iter().position(|b| has_drawing(d, cx, b))?;
    let mut blocks = d.blocks.clone();
    let text = "Inserted paragraph before the drawing.";
    let content = Inline::plain(text);
    blocks.insert(i, Block::Para(Para { style: cx.rem.styles.default_paragraph.clone(), content, item: None }));
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
        span: None,
    })
}

fn e3_delete_formatted(d: &Doc, cx: &Cx) -> Option<Edit> {
    for (i, b) in d.blocks.iter().enumerate() {
        let Block::Para(p) = b else { continue };
        if p.content.units.is_empty() || cx.fmt.holds_section(&entries_at(d, &[i]), i) {
            continue;
        }
        if entries_at(d, &[i]).iter().any(|e| cx.fmt.visible_run(e)) {
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
                span: None,
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

fn e4_move_section(d: &Doc, cx: &Cx) -> Option<Edit> {
    let rem = cx.rem;
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
        span: None,
    })
}

fn e5_restyle(d: &Doc, cx: &Cx) -> Option<Edit> {
    let st = &cx.rem.styles;
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
    let pick = cx
        .fmt
        .preferred_styles()
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
                span: None,
            });
        }
    }
    None
}

fn e6_cell_next_to_merge(d: &Doc, _: &Cx) -> Option<Edit> {
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

fn e7_overlap_run(d: &Doc, cx: &Cx) -> Option<Edit> {
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
                cands.push(((path.len() == 1, cx.fmt.visible_run(x) || cx.fmt.visible_run(y)), path.clone(), b));
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

fn e8_split(d: &Doc, _: &Cx) -> Option<Edit> {
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
    // A split list item gives two items of that list.
    let item = p.item.map(|it| hanji_core::ListItem { first: false, ..it });
    let q = Block::Para(Para { style: p.style.clone(), content: Inline { units: tail, spans: vec![] }, item });
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
        span: None,
    })
}

fn e9_merge(d: &Doc, cx: &Cx) -> Option<Edit> {
    let mut best: Option<((bool, bool, usize), usize)> = None;
    for i in 0..d.blocks.len().saturating_sub(1) {
        let (Block::Para(a), Block::Para(b)) = (&d.blocks[i], &d.blocks[i + 1]) else { continue };
        if a.content.units.is_empty()
            || b.content.units.is_empty()
            || is_page_break(&a.content)
            || is_page_break(&b.content)
            || cx.fmt.holds_section(&entries_at(d, &[i]), i)
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
        span: None,
    })
}

pub type EditFn = fn(&Doc, &Cx) -> Option<Edit>;

/// The Document edits E1–E9 (E10 combines them).
pub const EDITS: [(&str, EditFn); 9] = [
    ("e1_figure", e1_figure),
    ("e2_insert_before_drawing", e2_insert_before_drawing),
    ("e3_delete_formatted", e3_delete_formatted),
    ("e4_move_section", e4_move_section),
    ("e5_restyle", e5_restyle),
    ("e6_cell_next_to_merge", e6_cell_next_to_merge),
    ("e7_overlap_run", e7_overlap_run),
    ("e8_split", e8_split),
    ("e9_merge", e9_merge),
];

// ---------------------------------------------------------------- oracle, E10

/// The blocks of a text, as the engine's grammar reads it.
pub fn written(fmt: &dyn Format, text: &str, rem: &Remainder) -> Vec<Block> {
    fmt.model().resolve(text, rem, CAPS).unwrap_or_else(|e| panic!("{}\n{text}", hanji_format::diag::render(&e))).0
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
fn combined(d0: &Doc, cx: &Cx, edits: &[(&str, EditFn)]) -> (Vec<Block>, HashMap<u64, Outcome>, Vec<String>) {
    let mut cur = d0.clone();
    let mut origin: HashMap<u64, u64> = d0.entries.iter().map(|e| (e.id, e.id)).collect();
    let mut next_id = 1_000_000;
    let mut gone: HashMap<u64, &'static str> = HashMap::new();
    let mut steps = vec![];
    for (_, f) in edits {
        let Some(ed) = f(&cur, cx) else { continue };
        let new = written(cx.fmt, &cx.fmt.text_of(&ed.blocks, cx.rem), cx.rem);
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
        // A marker moved before a block lands there, or, in a format whose
        // markers live in paragraphs (hwpx), at the start of that block's paragraph.
        let mut keys = vec![obs_key(e)];
        if e.kind == Kind::Bmarker && e.path.len() == 1 {
            keys.push((Kind::Marker, e.fp.clone(), e.path.clone(), Some(0), None));
        }
        for k in keys {
            if let Some(n) = self.multi.get_mut(&k).filter(|n| **n > 0) {
                *n -= 1;
                return true;
            }
        }
        false
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
    Kind::Slide,
    Kind::Shape,
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

pub const RAW: ImportOptions = ImportOptions { neutralise: false, template: None };

fn export_and_check(
    cx: &Cx,
    name: &str,
    d: &Doc,
    new_text: &str,
    new_blocks: &[Block],
    got: &HashMap<u64, Outcome>,
    expected: &HashMap<u64, Outcome>,
    touched: &HashSet<usize>,
    out: &Out,
    out_name: &str,
) -> Run {
    let rem = cx.rem;
    let mut next = rem.next_id;
    let entries = place::placed_entries(&d.entries, got, &mut next);
    let r2 = Remainder { entries, next_id: next, ..rem.clone() };
    let (pkg, well_formed) = cx.fmt.export_blocks(new_blocks, &r2).unwrap_or_else(|e| panic!("{name} {out_name}: {e}"));
    out.save(name, out_name, &pkg);
    let (rb, rr, _, _) = cx.fmt.split(&pkg, &RAW).unwrap();
    let re_text = cx.fmt.text_of(&rb, &rr);
    let tally = score(d, expected, got, &rr.entries, touched);
    Run {
        tally,
        putget: re_text == new_text,
        putget_ids: normalise_ids(&re_text) == normalise_ids(new_text),
        well_formed,
    }
}

/// Where the exports go, one directory per file.
pub struct Out {
    pub dir: PathBuf,
    pub ext: &'static str,
}

impl Out {
    pub fn save(&self, name: &str, what: &str, pkg: &[u8]) {
        let dir = self.dir.join(name.trim_end_matches(&format!(".{}", self.ext)));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{what}.{}", self.ext)), pkg).unwrap();
    }
}

/// The exact text span of an edit: from its unit span when it has one,
/// else the minimal differing region.
fn text_span(fmt: &dyn Format, old_text: &str, new_text: &str, rem: &Remainder, ed: &Edit) -> (usize, usize, String) {
    if let Some(span) = &ed.span {
        return span.clone();
    }
    if let Some((path, s, e)) = &ed.unit_span {
        let (_, mut maps) = fmt.model().resolve(old_text, rem, CAPS).unwrap();
        let pm = match maps.swap_remove(path[0]).kind {
            hanji_core::model::SrcKind::Para(pm) => pm,
            hanji_core::model::SrcKind::Table(cells) => cells[path[1]][path[2]].clone().unwrap()[path[3]].clone(),
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

/// One scripted edit of a revision: E1–E9 and the combined one (E10).
pub struct Job {
    /// `E1 figure`, …
    pub name: String,
    pub what: String,
    pub new_text: String,
    /// The oracle's outcome for every entry.
    pub expected: HashMap<u64, Outcome>,
    pub touched: HashSet<usize>,
    /// The exact text edit, for a local edit.
    pub span: Option<(usize, usize, String)>,
}

/// The edits `edits` make to `d` (whose text is `text`), then all of them in
/// one revision, named `all`. Edits that do not apply are skipped (and
/// printed as `n/a` under `report`, the file's name).
pub fn edit_jobs(
    fmt: &dyn Format,
    d: &Doc,
    cx: &Cx,
    text: &str,
    edits: &[(&str, EditFn)],
    all: &str,
    report: Option<&str>,
) -> Vec<Job> {
    let rem = cx.rem;
    let mut jobs = vec![];
    for (fname, f) in edits {
        let Some(ed) = f(d, cx) else {
            if let Some(name) = report {
                println!("  {name} {fname}: n/a");
            }
            continue;
        };
        let new_text = fmt.text_of(&ed.blocks, rem);
        let new_blocks = written(fmt, &new_text, rem);
        let expected = oracle(d, &new_blocks, &ed);
        let span = ed.local.then(|| text_span(fmt, text, &new_text, rem, &ed));
        jobs.push(Job {
            name: ed.name.to_string(),
            what: ed.what.clone(),
            new_text,
            expected,
            touched: ed.touched.clone(),
            span,
        });
    }
    let (fb, fexp, steps) = combined(d, cx, edits);
    jobs.push(Job {
        name: format!("{all} all edits in one revision"),
        what: steps.join("; "),
        new_text: fmt.text_of(&fb, rem),
        expected: fexp,
        touched: (0..d.blocks.len()).collect(),
        span: None,
    });
    jobs
}

#[derive(Default)]
struct Totals {
    by_design: BTreeMap<&'static str, Tally>,
    per_edit: BTreeMap<(String, &'static str), Tally>,
    runs: BTreeMap<&'static str, (usize, usize, usize, usize)>, // runs, putget, putget with ids normalised, well-formed
}

/// A corpus run's numbers.
#[derive(Clone, Debug, Default)]
pub struct Summary {
    pub files: usize,
    pub getput_ok: usize,
    /// Per design (`C`, `exact`): runs, PutGet exact, PutGet with placeholder ids normalised, well-formed.
    pub runs: BTreeMap<&'static str, (usize, usize, usize, usize)>,
    /// Per design, over touched blocks: landed, orphaned, lost.
    pub touched: BTreeMap<&'static str, [usize; 3]>,
    /// Every GetPut, PutGet and well-formedness failure.
    pub failures: Vec<String>,
}

/// GetPut, E1–E10 under design C and exact spans, PutGet and well-formed
/// exports over `files`; prints the numbers (per file with HANJI_REPORT=1)
/// and saves every export under `out`.
pub fn run_corpus(fmt: &dyn Format, files: &[(String, Vec<u8>)], out: &Out) -> Summary {
    run_corpus_with(fmt, files, out, &EDITS, "E10")
}

/// [`run_corpus`] with the engine's own edit set; `all` names the run that
/// makes every edit in one revision (E10).
pub fn run_corpus_with(
    fmt: &dyn Format,
    files: &[(String, Vec<u8>)],
    out: &Out,
    edits: &[(&str, EditFn)],
    all: &str,
) -> Summary {
    let report = std::env::var("HANJI_REPORT").is_ok();
    let mut totals = Totals::default();
    let mut sum = Summary { files: files.len(), ..Default::default() };
    for (name, bytes) in files {
        let (blocks, rem, _, stats) = fmt.split(bytes, &RAW).unwrap_or_else(|e| panic!("{name}: {e}"));
        let cx = Cx { fmt, rem: &rem };
        let d = Doc { blocks, entries: rem.entries.clone() };
        let text = fmt.text_of(&d.blocks, &rem);
        // The model text round-trips through the format crate.
        assert_eq!(
            fmt.text_of(&written(fmt, &text, &rem), &rem),
            text,
            "{name}: model text does not reparse to itself"
        );
        // GetPut through the whole path: text → parse → place → export.
        let re = hanji_core::reanchor_rewrite_in(fmt.model(), &rem, &text, &text, CAPS).unwrap();
        assert!(
            re.report.refused.is_empty() && re.report.removed.is_empty(),
            "{name}: GetPut moved entries: {:?}",
            re.report
        );
        let exported = fmt.engine().export(&text, &re.remainder).unwrap_or_else(|e| panic!("{name}: {e}"));
        out.save(name, "getput", &exported);
        out.save(name, "ORIGINAL", bytes);
        let (same, others) = getput_equal(fmt, bytes, &exported);
        if same && others {
            sum.getput_ok += 1;
        } else {
            sum.failures.push(format!("{name}: GetPut split parts equal={same}, other parts byte-equal={others}"));
        }
        // Edits.
        let jobs = edit_jobs(fmt, &d, &cx, &text, edits, all, report.then_some(name.as_str()));
        for Job { name: ename, what, new_text, expected, touched, span } in jobs {
            let short = ename.split_whitespace().next().unwrap().to_string();
            let mut designs: Vec<(&'static str, hanji_core::Reanchored)> =
                vec![("C", hanji_core::reanchor_rewrite_in(fmt.model(), &rem, &text, &new_text, CAPS).unwrap())];
            if let Some((s, e, ref new)) = span {
                let r = hanji_core::reanchor_span_in(fmt.model(), &rem, &text, s, e, new, CAPS)
                    .unwrap_or_else(|e| panic!("{name} {ename}: {e}"));
                assert_eq!(r.text, new_text, "{name} {ename}: the exact span does not give the edited text");
                designs.push(("exact", r));
            }
            for (design, r) in designs {
                let run = export_and_check(
                    &cx,
                    name,
                    &d,
                    &new_text,
                    &r.new,
                    &r.outcomes,
                    &expected,
                    &touched,
                    out,
                    &format!("{short}-{design}"),
                );
                if !run.putget_ids {
                    sum.failures.push(format!("{name} {ename} {design}: PutGet fails"));
                }
                if !run.well_formed {
                    sum.failures.push(format!("{name} {ename} {design}: not well-formed"));
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
            println!("{name}: blocks {} entries {} {stats}", d.blocks.len(), d.entries.len());
        }
    }
    println!("GetPut: {}/{} (split parts canonically equal, other parts byte-equal)", sum.getput_ok, files.len());
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
        sum.touched.insert(design, t.touched);
    }
    for ((ed, design), t) in &totals.per_edit {
        println!("  {ed:4} [{design:5}] touched {} / {} / {}", t.touched[0], t.touched[1], t.touched[2]);
    }
    sum.runs = totals.runs;
    sum
}

/// A neutralised import exports a package a second import finds clean,
/// with the same text.
pub fn neutralised_import_keeps_getput_for_the_rest(fmt: &dyn Format, files: &[(String, Vec<u8>)]) {
    let e = fmt.engine();
    for (name, bytes) in files {
        let imp = e.import(bytes, &ImportOptions::default()).unwrap();
        let out = e.export(&imp.text, &imp.remainder).unwrap();
        let (_, _, report2, _) = fmt.split(&out, &ImportOptions::default()).unwrap();
        assert!(
            report2.neutralised.is_empty(),
            "{name}: a second import still finds active content: {:?}",
            report2.neutralised
        );
        let again = e.import(&out, &ImportOptions::default()).unwrap();
        assert_eq!(again.text, imp.text, "{name}");
    }
}

/// The remainder round-trips through JSON, and exports the same bytes.
pub fn a_serialized_remainder_exports_the_same_package(fmt: &dyn Format, files: &[(String, Vec<u8>)]) {
    let e = fmt.engine();
    for (name, bytes) in files {
        let imp = e.import(bytes, &RAW).unwrap();
        let json = imp.remainder.to_json();
        let back = Remainder::from_json(&json).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(back, imp.remainder, "{name}: the remainder does not round-trip through JSON");
        let a = e.export(&imp.text, &imp.remainder).unwrap();
        let b = e.export(&imp.text, &back).unwrap();
        assert_eq!(a, b, "{name}: export from the deserialized remainder differs");
        // GetPut from the deserialized remainder.
        assert_eq!(getput_equal(fmt, bytes, &b), (true, true), "{name}");
    }
}
