//! Tracked-change export (DESIGN.md §10.2): an export option, off by
//! default. The model edits made since import come out as `w:ins` / `w:del`
//! with an author and a date, so a person reviews them in Word; with the
//! option off, export is unchanged.
//!
//! How: the imported revision and the current one are merged into one
//! model. Each unit, paragraph mark, table and placeholder is kept (the
//! edits' alignments say it stayed the same), inserted or deleted. Live
//! content carries the current remainder's entries; deleted content its
//! imported runs, in `w:del` with `w:delText`. The shapes are Word's:
//!
//! - an inserted paragraph marks the mark of the paragraph before it as
//!   inserted ("Enter, then type"), so rejecting it leaves no empty paragraph;
//! - a deleted paragraph has its runs and its own mark deleted (the mark
//!   before it when a table or the end of its cell or document follows);
//! - a changed paragraph or run property is `w:pPrChange` / `w:rPrChange`
//!   holding the imported properties;
//! - an inserted or deleted table has each row marked in `w:trPr`.
//!
//! What cannot be tracked faithfully is refused with the reason: an edit
//! that deletes or moves a placeholder (a field, another author's tracked
//! change, a note reference, a moved picture, which would be written twice),
//! text inside a field's result, a section break inserted or deleted,
//! changed table shape or style, and a whole-file rewrite whose alignment
//! chose among identical blocks (make it as exact edits instead).

use std::cell::Cell as Counter;
use std::collections::{HashMap, HashSet};

use hanji_core::revision::{paragraphs, same_shape};
use hanji_core::{Block, EngineError, Entry, Kept, Kind, Para, Path, Pos, Reanchored, Remainder, Table};
use hanji_format::{Atom, Cell, CellPara, Inline, Marks, Unit};

use crate::export::{with_marks, Exporter};
use crate::numbering::Numbering;
use crate::ooxml::{insert_ordered, remove_child, PPR_ORDER};
use crate::xml::{self, fragment, Element, Node, Scope};
use crate::{write_package, DocShell, DocxEngine, Exported};

/// Who the revisions are by: `w:author` and `w:date` (ISO 8601, e.g.
/// `2026-09-29T00:00:00Z`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reviewer {
    pub author: String,
    pub date: String,
}

/// Export options for docx.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExportOptions {
    /// Write the edits since import as tracked changes by this reviewer.
    /// Off (`None`) by default: the edits are direct changes.
    pub tracked_changes: Option<Reviewer>,
}

/// The imported revision and the edits made since: what a tracked export
/// writes as revisions. Each edit is pushed as the [`Reanchored`] revision
/// `hanji_core::edit` (an exact span) or `rewrite` (design C) returned.
#[derive(Clone, Debug)]
pub struct History {
    base_blocks: Vec<Block>,
    base: Remainder,
    blocks: Vec<Block>,
    text: String,
    kept: Kept,
    /// The current revision's remainder.
    rem: Remainder,
}

fn refused(m: impl Into<String>) -> EngineError {
    EngineError::Refused(m.into())
}

impl History {
    /// The imported revision: its text and remainder.
    pub fn new(text: &str, rem: &Remainder) -> Result<History, EngineError> {
        let caps = hanji_core::Engine::capabilities(&DocxEngine);
        let (_, blocks) = hanji_core::model_of(text, rem, caps).map_err(EngineError::Invalid)?;
        Ok(History {
            kept: Kept::identity(&blocks),
            base_blocks: blocks.clone(),
            base: rem.clone(),
            blocks,
            text: text.to_string(),
            rem: rem.clone(),
        })
    }

    /// The current revision's text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The next edit. Refused when it does not start from the current
    /// revision, when it could not place the remainder, and for a rewrite
    /// whose alignment had to choose among identical blocks: its changed
    /// spans cannot be derived without guessing.
    pub fn push(&mut self, r: &Reanchored) -> Result<(), EngineError> {
        if r.old != self.blocks {
            return Err(refused("the edit does not start from the current revision of this history"));
        }
        if !r.report.refused.is_empty() {
            return Err(refused(format!(
                "the edit would lose content the text does not show ({} entries could not be placed)",
                r.report.refused.len()
            )));
        }
        if !r.alignment.ambiguous.is_empty() || !r.alignment.unsure.is_empty() {
            return Err(refused(
                "the rewrite pairs blocks among identical ones, so its changes cannot be told apart without \
                 guessing; make the change as exact edits (old text → new text) to export it as tracked changes",
            ));
        }
        self.kept = self.kept.then(&Kept::of(&r.old, &r.new, &r.alignment));
        self.blocks = r.new.clone();
        self.text = r.text.clone();
        self.rem = r.remainder.clone();
        Ok(())
    }

    /// The current revision's remainder.
    pub fn remainder(&self) -> &Remainder {
        &self.rem
    }

    /// The next revision given as its whole text (a stored revision, with
    /// no record of the edits that made it): each run of changed lines
    /// becomes an exact edit, the last first so earlier offsets hold.
    pub fn push_text(&mut self, new_text: &str) -> Result<(), EngineError> {
        let lines = |t: &str| -> Vec<String> { t.split_inclusive('\n').map(String::from).collect() };
        let (old, new) = (lines(&self.text), lines(new_text));
        let at: Vec<usize> = std::iter::once(0)
            .chain(old.iter().scan(0, |n, l| {
                *n += l.len();
                Some(*n)
            }))
            .collect();
        let caps = hanji_core::Engine::capabilities(&DocxEngine);
        for op in hanji_core::diff::opcodes(&old, &new).into_iter().rev() {
            if op.tag == hanji_core::diff::Tag::Equal {
                continue;
            }
            let repl = new[op.j1..op.j2].concat();
            let r = hanji_core::reanchor_span(&self.rem, &self.text, at[op.i1], at[op.i2], &repl, caps)
                .map_err(|e| refused(e.to_string()))?;
            self.push(&r)?;
        }
        if self.text != new_text {
            return Err(refused("the revision's text could not be rebuilt from exact edits"));
        }
        Ok(())
    }
}

impl DocxEngine {
    /// [`Engine::export`](hanji_core::Engine::export) with options. With
    /// `tracked_changes` off this is `export`, byte for byte; with it on,
    /// `history` holds the edits since import (`text` is its current
    /// revision, `rem` that revision's remainder).
    pub fn export_with(
        &self,
        text: &str,
        rem: &Remainder,
        opts: &ExportOptions,
        history: Option<&History>,
    ) -> Result<Vec<u8>, EngineError> {
        let Some(reviewer) = &opts.tracked_changes else {
            return hanji_core::Engine::export(self, text, rem);
        };
        let h = history.ok_or_else(|| {
            refused("tracked changes are written from the edits made since import: pass their History")
        })?;
        if text != h.text || rem.entries != h.rem.entries || rem.next_id != h.rem.next_id {
            return Err(refused("the text and remainder are not the current revision of the history"));
        }
        export_tracked(h, rem, reviewer)
    }
}

fn export_tracked(h: &History, rem: &Remainder, reviewer: &Reviewer) -> Result<Vec<u8>, EngineError> {
    let shell = DocShell::of(rem)?;
    let mut numbering = Numbering::read(&rem.parts);
    let lists = hanji_core::plan_lists(&h.blocks, &rem.entries, &mut numbering).map_err(EngineError::Refused)?;
    let mut m = Merge::new(h, rem);
    m.build().map_err(EngineError::Refused)?;
    let lists = lists.into_iter().filter_map(|(bi, plan)| m.new_mark.get(&vec![bi]).map(|p| (p[0], plan))).collect();
    let entries = m.entries().map_err(EngineError::Refused)?;
    let scope: Scope = rem.namespaces.iter().cloned().collect();
    let track = Track {
        author: reviewer.author.clone(),
        date: reviewer.date.clone(),
        next_id: Counter::new(max_id(&h.base, rem) + 1),
        units: m.units,
        marks: m.marks,
        rows: m.rows,
        scope,
        default_style: rem.styles.default_paragraph_id().to_string(),
    };
    let ex = Exporter::new(&rem.styles, &entries, &numbering, lists).tracked(&track);
    let body = ex.body(&m.blocks).map_err(EngineError::Refused)?;
    let document = shell.document(&body).into_bytes();
    write_package(rem, Exported { document, numbering: numbering.part() })
}

/// The largest numeric `w:id` anywhere in the package, so revision ids are new.
fn max_id(base: &Remainder, rem: &Remainder) -> u64 {
    let mut max = 0;
    let mut scan = |s: &str| {
        let mut rest = s;
        while let Some(k) = rest.find("w:id=") {
            rest = &rest[k + 5..];
            let v = rest.trim_start_matches(['"', '\'', '\\']);
            let n: String = v.chars().take_while(char::is_ascii_digit).collect();
            max = max.max(n.parse::<u64>().unwrap_or(0));
        }
    };
    for p in rem.parts.iter().filter(|p| p.name.ends_with(".xml") || p.name.ends_with(".rels")) {
        scan(&String::from_utf8_lossy(&p.data));
    }
    for s in &rem.shell {
        scan(s);
    }
    for e in base.entries.iter().chain(&rem.entries) {
        e.xml.iter().for_each(|x| scan(x));
    }
    max
}

// ------------------------------------------------------------------ what the exporter reads

/// Kept, inserted or deleted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum St {
    Eq,
    Ins,
    Del,
}

/// A unit of a merged paragraph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UnitRev {
    pub st: St,
    /// The marks it is written with (its own revision's).
    pub eff: Marks,
    /// A kept unit whose run properties changed: the imported `w:rPr` (empty for none).
    pub old_rpr: Option<String>,
}

/// A merged paragraph's mark.
#[derive(Clone, Debug)]
pub(crate) struct MarkRev {
    pub st: St,
    /// A kept mark: the imported paragraph's `w:pPr`, if it had one.
    pub old_ppr: Option<Option<String>>,
}

/// Revisions of the merged model, by merged path.
pub(crate) struct Track {
    author: String,
    date: String,
    next_id: Counter<u64>,
    pub units: HashMap<Path, Vec<UnitRev>>,
    marks: HashMap<Path, MarkRev>,
    rows: HashMap<Path, St>,
    scope: Scope,
    /// The default paragraph style's id.
    default_style: String,
}

impl Track {
    /// A revision element with a fresh id, the author and the date.
    fn rev(&self, name: &str) -> Element {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        Element::new(name)
            .with_attr("w:id", &id.to_string())
            .with_attr("w:author", &self.author)
            .with_attr("w:date", &self.date)
    }

    fn canon(&self, e: &Element) -> String {
        xml::canon(e, &self.scope)
    }

    /// A written run as its revision says: in `w:ins`, in `w:del` (its text
    /// as `w:delText`), or with a `w:rPrChange` holding the imported properties.
    pub fn run(&self, mut r: Element, u: &UnitRev) -> Element {
        match u.st {
            St::Ins => {
                let mut w = self.rev("w:ins");
                w.children.push(Node::El(r));
                w
            }
            St::Del => {
                r.walk_mut(&mut |e| match e.name.as_str() {
                    "w:t" => e.name = "w:delText".into(),
                    "w:instrText" => e.name = "w:delInstrText".into(),
                    _ => {}
                });
                let mut w = self.rev("w:del");
                w.children.push(Node::El(r));
                w
            }
            St::Eq => {
                if let Some(old) = &u.old_rpr {
                    if r.child("w:rPr").is_none() {
                        r.children.insert(0, Node::El(Element::new("w:rPr")));
                    }
                    let mut change = self.rev("w:rPrChange");
                    let before = if old.is_empty() { Element::new("w:rPr") } else { fragment(old) };
                    change.children.push(Node::El(before));
                    r.child_mut("w:rPr").unwrap().children.push(Node::El(change));
                }
                r
            }
        }
    }

    /// A merged paragraph's mark: inserted or deleted in `w:pPr/w:rPr`, or
    /// a `w:pPrChange` / `w:rPrChange` when a kept mark's properties changed.
    pub fn mark(&self, p: &mut Element, path: &[usize]) -> Result<(), String> {
        let Some(m) = self.marks.get(path) else { return Ok(()) };
        if m.st == St::Eq {
            let old = m.old_ppr.clone().flatten().map(|x| fragment(&x)).unwrap_or_else(|| Element::new("w:pPr"));
            return self.ppr_change(p, &old);
        }
        let ppr = ensure_ppr(p);
        if ppr.child("w:sectPr").is_some() {
            return Err(
                "the edit inserts or deletes a section break, which is not written as a tracked change yet".into()
            );
        }
        if ppr.child("w:rPr").is_none() {
            insert_ordered(ppr, Element::new("w:rPr"), PPR_ORDER);
        }
        let rpr = ppr.child_mut("w:rPr").unwrap();
        if rpr.elements().any(|e| matches!(e.local(), "ins" | "del" | "moveFrom" | "moveTo")) {
            return Err("the edit inserts or deletes a paragraph mark that is already a tracked change of another \
                        author; accept or reject that change first"
                .into());
        }
        let mark = self.rev(if m.st == St::Ins { "w:ins" } else { "w:del" });
        rpr.children.insert(0, Node::El(mark));
        Ok(())
    }

    fn ppr_change(&self, p: &mut Element, old: &Element) -> Result<(), String> {
        let new = p.child("w:pPr").cloned().unwrap_or_else(|| Element::new("w:pPr"));
        let part = |e: &Element, n: &str| e.child(n).map(|x| self.canon(x));
        if part(&new, "w:sectPr") != part(old, "w:sectPr") {
            return Err("the edit changes a section break, which is not written as a tracked change yet".into());
        }
        let base = |e: &Element| {
            let mut x = e.clone();
            for n in ["w:rPr", "w:sectPr", "w:pPrChange"] {
                while remove_child(&mut x, n).is_some() {}
            }
            x
        };
        let (bn, mut bo) = (base(&new), base(old));
        let para_changed = self.canon(&bn) != self.canon(&bo);
        // The default style by name, as Word writes it: an empty stored pPr
        // reads as "no change" to some applications.
        if para_changed && bo.child("w:pStyle").is_none() && bn.child("w:pStyle").is_some() {
            bo.children.insert(0, Node::El(Element::new("w:pStyle").with_attr("w:val", &self.default_style)));
        }
        let rpr_changed = part(&new, "w:rPr") != part(old, "w:rPr");
        if !para_changed && !rpr_changed {
            return Ok(());
        }
        let ppr = ensure_ppr(p);
        if para_changed {
            if ppr.child("w:pPrChange").is_some() {
                return Err("the edit changes a paragraph's properties that are already a tracked change of \
                            another author; accept or reject that change first"
                    .into());
            }
            let mut change = self.rev("w:pPrChange");
            change.children.push(Node::El(bo));
            ppr.children.push(Node::El(change));
        }
        if rpr_changed {
            let mut before = old.child("w:rPr").cloned().unwrap_or_else(|| Element::new("w:rPr"));
            before.children.retain(|n| {
                !matches!(n, Node::El(e) if matches!(e.local(), "ins" | "del" | "moveFrom" | "moveTo" | "rPrChange"))
            });
            if ppr.child("w:rPr").is_none() {
                insert_ordered(ppr, Element::new("w:rPr"), PPR_ORDER);
            }
            let rpr = ppr.child_mut("w:rPr").unwrap();
            if rpr.child("w:rPrChange").is_some() {
                return Err("the edit changes a paragraph mark's formatting that is already a tracked change of \
                            another author"
                    .into());
            }
            let mut change = self.rev("w:rPrChange");
            change.children.push(Node::El(before));
            rpr.children.push(Node::El(change));
        }
        Ok(())
    }

    /// An inserted or deleted table row: `w:trPr/w:ins` or `w:trPr/w:del`.
    pub fn row(&self, tr: &mut Element, path: &[usize]) {
        let name = match self.rows.get(path) {
            Some(St::Ins) => "w:ins",
            Some(St::Del) => "w:del",
            _ => return,
        };
        if tr.child("w:trPr").is_none() {
            let at =
                tr.children.iter().position(|n| matches!(n, Node::El(e) if e.is("w:tblPrEx"))).map_or(0, |k| k + 1);
            tr.children.insert(at, Node::El(Element::new("w:trPr")));
        }
        let trpr = tr.child_mut("w:trPr").unwrap();
        let at = trpr.children.iter().position(|n| matches!(n, Node::El(e) if e.is("w:trPrChange")));
        let mark = Node::El(self.rev(name));
        match at {
            Some(k) => trpr.children.insert(k, mark),
            None => trpr.children.push(mark),
        }
    }
}

fn ensure_ppr(p: &mut Element) -> &mut Element {
    if p.child("w:pPr").is_none() {
        p.children.insert(0, Node::El(Element::new("w:pPr")));
    }
    p.child_mut("w:pPr").unwrap()
}

// ------------------------------------------------------------------ one revision's side of the merge

struct Side<'a> {
    blocks: &'a [Block],
    rem: &'a Remainder,
    ppr: HashMap<Path, &'a Entry>,
    /// Per paragraph: the run entry of each unit, and the marks it is written with.
    owner: HashMap<Path, Vec<Option<&'a Entry>>>,
    eff: HashMap<Path, Vec<Marks>>,
    keeps: HashMap<&'a str, &'a Entry>,
}

impl<'a> Side<'a> {
    fn new(blocks: &'a [Block], rem: &'a Remainder) -> Side<'a> {
        let mut runs: HashMap<&Path, Vec<&Entry>> = HashMap::new();
        let mut s = Side {
            blocks,
            rem,
            ppr: HashMap::new(),
            owner: HashMap::new(),
            eff: HashMap::new(),
            keeps: HashMap::new(),
        };
        for e in &rem.entries {
            match e.kind {
                Kind::Ppr => {
                    s.ppr.entry(e.path.clone()).or_insert(e);
                }
                Kind::Run => runs.entry(&e.path).or_default().push(e),
                Kind::Keep | Kind::Bkeep => {
                    s.keeps.insert(e.meta.keep.as_ref().unwrap().id.as_str(), e);
                }
                _ => {}
            }
        }
        for (path, p) in paragraphs(blocks) {
            let n = p.units.len();
            let mut owner: Vec<Option<&Entry>> = vec![None; n];
            for r in runs.get(&path).into_iter().flatten() {
                let (a, b) = (r.start.unwrap(), r.end.unwrap());
                for slot in owner.iter_mut().take(b.min(n)).skip(a) {
                    *slot = Some(r);
                }
            }
            let eff = p.written_marks(&|c| owner[c].map_or(Marks::NONE, |o| o.meta.marks));
            s.owner.insert(path.clone(), owner);
            s.eff.insert(path, eff);
        }
        s
    }

    fn unit(&self, path: &Path, u: usize) -> Unit {
        hanji_core::model::content_at(self.blocks, path).expect("a paragraph").units[u].clone()
    }

    fn len(&self, path: &Path) -> usize {
        hanji_core::model::content_at(self.blocks, path).map_or(0, |i| i.units.len())
    }

    /// The `w:rPr` unit `u` of `path` is written with.
    fn rpr(&self, path: &Path, u: usize) -> String {
        let own = self.owner[path][u];
        with_marks(own.and_then(|o| o.xml.get(1)).map(|x| fragment(x)), self.eff[path][u])
            .map(|x| x.to_xml())
            .unwrap_or_default()
    }

    fn has_sect(&self, mark: &Path) -> bool {
        self.ppr.get(mark).and_then(|e| e.xml.get(1)).is_some_and(|x| fragment(x).child("w:sectPr").is_some())
    }

    fn para(&self, path: &Path) -> (String, Option<hanji_core::ListItem>, Option<String>) {
        match (&self.blocks[path[0]], path.len()) {
            (Block::Para(p), 1) => (p.style.clone(), p.item, None),
            (Block::Table(t), 4) => match &t.rows[path[1]][path[2]] {
                Cell::Text(ps) => (String::new(), None, ps[path[3]].style.clone()),
                _ => (String::new(), None, None),
            },
            _ => (String::new(), None, None),
        }
    }
}

// ------------------------------------------------------------------ the merge

/// Merged positions → runs of consecutive ones in one paragraph: `(path, start, end)`.
fn pieces(units: impl IntoIterator<Item = (Path, usize)>) -> Vec<(Path, usize, usize)> {
    let mut out: Vec<(Path, usize, usize)> = vec![];
    for (p, s) in units {
        match out.last_mut() {
            Some(l) if l.0 == p && l.2 == s => l.2 = s + 1,
            _ => out.push((p, s, s + 1)),
        }
    }
    out
}

/// One position of the merged model and what it is.
#[derive(Clone, Debug)]
struct It {
    st: St,
    old: Option<Pos>,
    new: Option<Pos>,
}

impl It {
    fn is_mark(&self) -> bool {
        matches!(self.old.as_ref().or(self.new.as_ref()), Some(Pos::Mark(_)))
    }
    fn is_para(&self) -> bool {
        !matches!(self.old.as_ref().or(self.new.as_ref()), Some(Pos::Block(_)))
    }
}

fn mark_path(p: &Option<Pos>) -> Option<&Path> {
    match p {
        Some(Pos::Mark(x)) => Some(x),
        _ => None,
    }
}

/// The heaviest chain of pairs increasing on both sides (pairs are sorted
/// by the old side, new sides distinct): a Fenwick tree of best chains by
/// new position.
fn chain(pairs: &[(usize, usize)], weight: &dyn Fn(usize) -> u64, n_new: usize) -> Vec<(usize, usize)> {
    // tree[i]: the best (weight, pair index) over a range of new positions ending at i.
    let mut tree: Vec<(u64, usize)> = vec![(0, usize::MAX); n_new + 1];
    let mut prev = vec![usize::MAX; pairs.len()];
    let mut best = (0, usize::MAX);
    for (k, &(a, b)) in pairs.iter().enumerate() {
        // Best chain over new positions < b.
        let (mut q, mut i) = ((0, usize::MAX), b);
        while i > 0 {
            q = q.max(tree[i]);
            i &= i - 1;
        }
        prev[k] = q.1;
        let here = (q.0 + weight(a), k);
        best = best.max(here);
        let mut i = b + 1;
        while i <= n_new {
            tree[i] = tree[i].max(here);
            i += i & i.wrapping_neg();
        }
    }
    let mut out = vec![];
    let mut k = best.1;
    while k != usize::MAX {
        out.push(pairs[k]);
        k = prev[k];
    }
    out.reverse();
    out
}

fn para_tokens(path: &Path, n: usize, out: &mut Vec<Pos>) {
    out.extend((0..n).map(|u| Pos::Unit(path.clone(), u)));
    out.push(Pos::Mark(path.clone()));
}

fn top_tokens(blocks: &[Block]) -> Vec<Pos> {
    let mut out = vec![];
    for (i, b) in blocks.iter().enumerate() {
        match b {
            Block::Para(p) => para_tokens(&vec![i], p.content.units.len(), &mut out),
            _ => out.push(Pos::Block(i)),
        }
    }
    out
}

fn cell_tokens(ps: &[CellPara], base: &[usize]) -> Vec<Pos> {
    let mut out = vec![];
    for (k, p) in ps.iter().enumerate() {
        para_tokens(&base.iter().copied().chain([k]).collect(), p.content.units.len(), &mut out);
    }
    out
}

/// A merged paragraph (its units and its mark) or a block.
enum Item {
    Para(Vec<It>, It),
    Block(It),
}

struct Merge<'a> {
    old: Side<'a>,
    new: Side<'a>,
    kept: &'a Kept,
    blocks: Vec<Block>,
    units: HashMap<Path, Vec<UnitRev>>,
    marks: HashMap<Path, MarkRev>,
    rows: HashMap<Path, St>,
    /// Where each old and new unit and mark went in the merged model.
    new_unit: HashMap<Pos, (Path, usize)>,
    old_unit: HashMap<Pos, (Path, usize)>,
    new_mark: HashMap<Path, Path>,
    old_mark: HashMap<Path, Path>,
    /// Merged paragraph lengths.
    lens: HashMap<Path, usize>,
    /// New (old) table or placeholder, or the first merged paragraph of a new paragraph → merged block.
    new_block: HashMap<usize, usize>,
    /// The last merged block holding each new block's content.
    new_last: HashMap<usize, usize>,
    old_block: HashMap<usize, usize>,
    /// First merged cell paragraph of a new cell paragraph; merged paragraph count per cell.
    new_cell_para: HashMap<Path, usize>,
    cell_len: HashMap<Path, usize>,
    /// Covered cells, whose entries move with their table: new / old `[block, row, col]` → merged block.
    new_covered: HashMap<Path, usize>,
    old_covered: HashMap<Path, usize>,
    /// Complex fields open at this point of the merged model (they may span paragraphs).
    depth: i32,
}

impl<'a> Merge<'a> {
    fn new(h: &'a History, rem: &'a Remainder) -> Merge<'a> {
        Merge {
            old: Side::new(&h.base_blocks, &h.base),
            new: Side::new(&h.blocks, rem),
            kept: &h.kept,
            blocks: vec![],
            units: HashMap::new(),
            marks: HashMap::new(),
            rows: HashMap::new(),
            new_unit: HashMap::new(),
            old_unit: HashMap::new(),
            new_mark: HashMap::new(),
            old_mark: HashMap::new(),
            lens: HashMap::new(),
            new_block: HashMap::new(),
            new_last: HashMap::new(),
            old_block: HashMap::new(),
            new_cell_para: HashMap::new(),
            cell_len: HashMap::new(),
            new_covered: HashMap::new(),
            old_covered: HashMap::new(),
            depth: 0,
        }
    }

    /// Old and new positions of one container (the body, or a cell) in one
    /// order: kept ones paired, the rest deleted before inserted.
    fn sequence(&self, old: &[Pos], new: &[Pos]) -> Result<Vec<It>, String> {
        let at_new: HashMap<&Pos, usize> = new.iter().enumerate().map(|(k, p)| (p, k)).collect();
        let pairs: Vec<(usize, usize)> = old
            .iter()
            .enumerate()
            .filter_map(|(a, p)| self.kept.get(p).and_then(|q| at_new.get(q)).map(|&b| (a, b)))
            .collect();
        let mut seq = vec![];
        let (mut i, mut j) = (0, 0);
        // What stays in place: as much as possible, placeholders and tables
        // first (moving one would write it twice).
        let heavy = |a: usize| match &old[a] {
            Pos::Block(_) => 1 << 32,
            Pos::Unit(p, u) if matches!(self.old.unit(p, *u).atom, Atom::Keep(_)) => 1 << 32,
            _ => 1,
        };
        for (a, b) in chain(&pairs, &heavy, new.len()).into_iter().chain([(old.len(), new.len())]) {
            seq.extend(old[i..a].iter().map(|p| It { st: St::Del, old: Some(p.clone()), new: None }));
            seq.extend(new[j..b].iter().map(|p| It { st: St::Ins, old: None, new: Some(p.clone()) }));
            if a < old.len() {
                seq.push(It { st: St::Eq, old: Some(old[a].clone()), new: Some(new[b].clone()) });
            }
            (i, j) = (a + 1, b + 1);
        }
        // Deleted paragraphs replaced by inserted ones: the last deleted mark
        // and the last inserted mark are one kept mark whose properties
        // changed, so the replacement ends in a mark kept on both sides.
        let run_end =
            |seq: &[It], from: usize, st: St| (from..seq.len()).find(|&x| seq[x].st != st).unwrap_or(seq.len());
        let mut out = Vec::with_capacity(seq.len());
        let mut k = 0;
        while k < seq.len() {
            let st = seq[k].st;
            if st == St::Eq {
                out.push(seq[k].clone());
                k += 1;
                continue;
            }
            let a = run_end(&seq, k, st);
            let b = run_end(&seq, a, if st == St::Del { St::Ins } else { St::Del });
            if b > a && seq[a - 1].is_mark() && seq[b - 1].is_mark() {
                let (d, n) = if st == St::Del { (&seq[a - 1], &seq[b - 1]) } else { (&seq[b - 1], &seq[a - 1]) };
                let kept = It { st: St::Eq, old: d.old.clone(), new: n.new.clone() };
                out.extend_from_slice(&seq[k..a - 1]);
                out.extend_from_slice(&seq[a..b - 1]);
                out.push(kept);
                k = b;
            } else {
                out.extend_from_slice(&seq[k..a]);
                k = a;
            }
        }
        let mut seq = out;
        let blocked = self.slide(&mut seq);
        // Each inserted or deleted mark joins its paragraph to the next when
        // rejected or accepted: a paragraph must follow it (after the tables
        // inserted or deleted with it).
        for (k, it) in seq.iter().enumerate() {
            // Tables inserted or deleted with it go with it.
            let next = seq[k + 1..].iter().find(|x| x.is_para() || x.st != it.st);
            if it.is_mark() && it.st != St::Eq && !next.is_some_and(It::is_para) {
                if blocked.contains(&k) {
                    return Err("the edit inserts or deletes a paragraph next to a section break at the end of \
                                the document or before a table, which is not written as a tracked change yet"
                        .into());
                }
                return Err(format!(
                    "a paragraph {} just before a table, a block placeholder or the end of its cell or of the \
                     document cannot be tracked: nothing follows it to join when the change is {}",
                    if it.st == St::Ins { "inserted" } else { "deleted" },
                    if it.st == St::Ins { "rejected" } else { "accepted" }
                ));
            }
        }
        Ok(seq)
    }

    /// Word's shapes. Inserted paragraphs after a kept mark: that mark is
    /// the inserted one and the last new paragraph takes it. Deleted
    /// paragraphs just before a table or the end: the mark before them is
    /// the deleted one, and the last deleted mark is kept for the paragraph
    /// before. Not across a section break.
    /// Returns the marks a section break kept from sliding.
    fn slide(&self, seq: &mut [It]) -> HashSet<usize> {
        let mut blocked = HashSet::new();
        let mut k = 1;
        while k < seq.len() {
            let st = seq[k].st;
            if st == St::Eq || seq[k - 1].st != St::Eq || !seq[k - 1].is_mark() {
                k += 1;
                continue;
            }
            let mut e = k;
            while e < seq.len() && seq[e].st == st {
                e += 1;
            }
            let last = e - 1;
            let before_block = !seq.get(e).is_some_and(It::is_para);
            let (o, n) = (mark_path(&seq[k - 1].old), mark_path(&seq[k - 1].new));
            let sect = o.is_some_and(|p| self.old.has_sect(p)) || n.is_some_and(|p| self.new.has_sect(p));
            let sect_last = match st {
                St::Del => mark_path(&seq[last].old).is_some_and(|p| self.old.has_sect(p)),
                _ => mark_path(&seq[last].new).is_some_and(|p| self.new.has_sect(p)),
            };
            let wanted = seq[last].is_mark() && (st == St::Ins || before_block);
            if wanted && (sect || sect_last) {
                blocked.insert(last);
            }
            if wanted && !sect && !sect_last {
                let prev = seq[k - 1].clone();
                match st {
                    St::Ins => {
                        seq[k - 1] = It { st: St::Ins, old: None, new: prev.new };
                        seq[last] = It { st: St::Eq, old: prev.old, new: seq[last].new.clone() };
                    }
                    _ => {
                        seq[k - 1] = It { st: St::Del, old: prev.old, new: None };
                        seq[last] = It { st: St::Eq, old: seq[last].old.clone(), new: prev.new };
                    }
                }
            }
            k = e;
        }
        blocked
    }

    fn items(seq: Vec<It>) -> Result<Vec<Item>, String> {
        let mut out = vec![];
        let mut units = vec![];
        for it in seq {
            if it.is_mark() {
                out.push(Item::Para(std::mem::take(&mut units), it));
            } else if it.is_para() {
                units.push(it);
            } else {
                if !units.is_empty() {
                    return Err("the edit puts a table or block placeholder inside a paragraph".into());
                }
                out.push(Item::Block(it));
            }
        }
        if !units.is_empty() {
            return Err("text after the last paragraph mark".into());
        }
        Ok(out)
    }

    fn build(&mut self) -> Result<(), String> {
        let seq = self.sequence(&top_tokens(self.old.blocks), &top_tokens(self.new.blocks))?;
        for item in Self::items(seq)? {
            let mi = self.blocks.len();
            match item {
                Item::Para(units, mark) => {
                    let (inline, style, item) = self.para(&units, &mark, &[mi])?;
                    self.blocks.push(Block::Para(Para { style, content: inline, item }));
                }
                Item::Block(it) => {
                    let b = self.block(&it, mi)?;
                    self.blocks.push(b);
                }
            }
        }
        Ok(())
    }

    /// Register a merged paragraph at `mp`: its units and mark, and what the exporter reads.
    fn para(
        &mut self,
        units: &[It],
        mark: &It,
        mp: &[usize],
    ) -> Result<(Inline, String, Option<hanji_core::ListItem>), String> {
        let mp = mp.to_vec();
        let mut out = vec![];
        let mut revs = vec![];
        for (k, it) in units.iter().enumerate() {
            let (unit, eff, old_rpr) = match (&it.old, &it.new) {
                (_, Some(Pos::Unit(np, nu))) => {
                    let unit = self.new.unit(np, *nu);
                    let old_rpr = match &it.old {
                        Some(Pos::Unit(op, ou)) => {
                            let (a, b) = (self.old.rpr(op, *ou), self.new.rpr(np, *nu));
                            if a != b && a.contains("rPrChange") {
                                return Err("the edit changes formatting that is already a tracked change of \
                                            another author; accept or reject that change first"
                                    .into());
                            }
                            (a != b).then_some(a)
                        }
                        _ => None,
                    };
                    (unit, self.new.eff[np][*nu], old_rpr)
                }
                (Some(Pos::Unit(op, ou)), None) => (self.old.unit(op, *ou), self.old.eff[op][*ou], None),
                _ => unreachable!("a paragraph unit"),
            };
            if let Atom::Keep(kp) = &unit.atom {
                let entry = if it.st == St::Del {
                    self.old.keeps.get(kp.id.as_str())
                } else {
                    self.new.keeps.get(kp.id.as_str())
                };
                let xml = entry.map(|e| e.xml.concat()).unwrap_or_default();
                self.depth += xml.matches("w:fldCharType=\"begin\"").count() as i32;
                self.depth -= xml.matches("w:fldCharType=\"end\"").count() as i32;
                if it.st != St::Eq {
                    self.check_keep(kp, it.st)?;
                }
            } else if it.st != St::Eq && self.depth > 0 {
                return Err("the edit changes the result of a field, which cannot be tracked; update the field in \
                            Office instead"
                    .into());
            }
            if let Some(p) = &it.new {
                self.new_unit.insert(p.clone(), (mp.clone(), k));
            }
            if let Some(p) = &it.old {
                self.old_unit.insert(p.clone(), (mp.clone(), k));
            }
            out.push(unit);
            revs.push(UnitRev { st: it.st, eff, old_rpr });
        }
        self.lens.insert(mp.clone(), out.len());
        let (np, op) = (mark_path(&mark.new).cloned(), mark_path(&mark.old).cloned());
        if let Some(np) = &np {
            self.new_mark.insert(np.clone(), mp.clone());
        }
        if let Some(op) = &op {
            self.old_mark.insert(op.clone(), mp.clone());
        }
        // The first merged paragraph a new paragraph's content is in.
        for p in units.iter().filter_map(|it| it.new.as_ref()).chain(mark.new.as_ref()) {
            if let Pos::Unit(np, _) | Pos::Mark(np) = p {
                if np.len() == 1 {
                    self.new_block.entry(np[0]).or_insert(mp[0]);
                    self.new_last.insert(np[0], mp[0]);
                } else {
                    self.new_cell_para.entry(np.clone()).or_insert(mp[3]);
                }
            }
        }
        let old_ppr = match (&op, mark.st) {
            (Some(op), St::Eq) => Some(self.old.ppr.get(op).and_then(|e| e.xml.get(1).cloned())),
            _ => None,
        };
        self.units.insert(mp.clone(), revs);
        self.marks.insert(mp.clone(), MarkRev { st: mark.st, old_ppr });
        let (style, item, _) = match (&np, &op) {
            (Some(np), _) => self.new.para(np),
            (None, Some(op)) => self.old.para(op),
            _ => unreachable!("a mark"),
        };
        Ok((Inline { units: out, spans: vec![] }, style, item))
    }

    /// Why a placeholder cannot be inserted or deleted as a tracked change, if it cannot.
    fn check_keep(&self, k: &hanji_format::Keep, st: St) -> Result<(), String> {
        let what = format!("placeholder {} ({}: {})", k.id, k.kind, k.summary);
        if st == St::Ins || self.new.keeps.contains_key(k.id.as_str()) {
            return Err(format!(
                "the edit moves {what}; a tracked move would write it twice. Leave it in place or export \
                 without tracked changes"
            ));
        }
        let in_run = self.old.keeps.get(k.id.as_str()).is_some_and(|e| e.meta.run.is_some());
        if k.kind.starts_with("tracked") {
            return Err(format!(
                "the edit deletes {what}, a tracked change of another author: a deletion would nest inside it. \
                 Accept or reject that change first"
            ));
        }
        if k.kind.starts_with("field") {
            return Err(format!("the edit deletes {what}: edits to fields cannot be tracked"));
        }
        let run_content = ["drawing", "object", "symbol", "tab", "hyphen", "break", "text"];
        if !in_run || !run_content.contains(&k.kind.as_str()) {
            return Err(format!("the edit deletes {what}, which is not written as a tracked deletion yet"));
        }
        Ok(())
    }

    fn block(&mut self, it: &It, mi: usize) -> Result<Block, String> {
        let (o, n) = match (&it.old, &it.new) {
            (Some(Pos::Block(o)), Some(Pos::Block(n))) => (Some(*o), Some(*n)),
            (Some(Pos::Block(o)), None) => (Some(*o), None),
            (None, Some(Pos::Block(n))) => (None, Some(*n)),
            _ => unreachable!("a block"),
        };
        if let Some(o) = o {
            self.old_block.insert(o, mi);
        }
        if let Some(n) = n {
            self.new_block.insert(n, mi);
            self.new_last.insert(n, mi);
        }
        let (ob, nb) = (o.map(|o| &self.old.blocks[o]), n.map(|n| &self.new.blocks[n]));
        match (ob, nb) {
            (_, Some(Block::Keep(id))) if it.st == St::Eq => Ok(Block::Keep(id.clone())),
            (Some(Block::Keep(id)), _) | (_, Some(Block::Keep(id))) => {
                let k = self.old.keeps.get(id.as_str()).or(self.new.keeps.get(id.as_str()));
                let k = k.and_then(|e| e.meta.keep.as_ref());
                let what = k.map_or(id.clone(), |k| format!("{} ({}: {})", k.id, k.kind, k.summary));
                Err(format!(
                    "the edit {} the block placeholder {what}, which is not written as a tracked change yet",
                    if it.st == St::Del { "deletes" } else { "moves" }
                ))
            }
            (Some(Block::Table(a)), Some(Block::Table(b))) => self.table_eq(o.unwrap(), a, n.unwrap(), b, mi),
            (Some(Block::Table(a)), None) => self.table_whole(a, &[o.unwrap()], mi, St::Del),
            (None, Some(Block::Table(b))) => self.table_whole(b, &[n.unwrap()], mi, St::Ins),
            _ => Err("a block that is neither a table nor a placeholder".into()),
        }
    }

    fn table_eq(&mut self, i: usize, a: &Table, j: usize, b: &Table, mi: usize) -> Result<Block, String> {
        if !same_shape(&a.rows, &b.rows) {
            return Err("the edit adds, removes or merges table rows or cells, which is not written as a tracked \
                        change yet; make the text edits in one export and the table change in another"
                .into());
        }
        if a.style != b.style {
            return Err("the edit changes a table's style, which is not written as a tracked change yet".into());
        }
        let mut rows = vec![];
        for (r, (ra, rb)) in a.rows.iter().zip(&b.rows).enumerate() {
            let mut row = vec![];
            for (c, (ca, cb)) in ra.iter().zip(rb).enumerate() {
                row.push(match (ca, cb) {
                    (Cell::Text(pa), Cell::Text(pb)) => {
                        let seq = self.sequence(&cell_tokens(pa, &[i, r, c]), &cell_tokens(pb, &[j, r, c]))?;
                        Cell::Text(self.cell(seq, &[mi, r, c])?)
                    }
                    (Cell::Up, _) => {
                        self.new_covered.insert(vec![j, r, c], mi);
                        self.old_covered.insert(vec![i, r, c], mi);
                        Cell::Up
                    }
                    _ => Cell::Left,
                });
            }
            rows.push(row);
        }
        Ok(Block::Table(Table { style: b.style.clone(), rows }))
    }

    /// A table inserted or deleted whole.
    fn table_whole(&mut self, t: &Table, at: &[usize], mi: usize, st: St) -> Result<Block, String> {
        let mut rows = vec![];
        for (r, row) in t.rows.iter().enumerate() {
            self.rows.insert(vec![mi, r], st);
            let mut out = vec![];
            for (c, cell) in row.iter().enumerate() {
                let base = [at[0], r, c];
                out.push(match cell {
                    Cell::Text(ps) => {
                        let seq = cell_tokens(ps, &base)
                            .into_iter()
                            .map(|p| match st {
                                St::Del => It { st, old: Some(p), new: None },
                                _ => It { st, old: None, new: Some(p) },
                            })
                            .collect();
                        Cell::Text(self.cell(seq, &[mi, r, c])?)
                    }
                    Cell::Up => {
                        let m = if st == St::Del { &mut self.old_covered } else { &mut self.new_covered };
                        m.insert(base.to_vec(), mi);
                        Cell::Up
                    }
                    Cell::Left => Cell::Left,
                });
            }
            rows.push(out);
        }
        let style = t.style.clone();
        Ok(Block::Table(Table { style, rows }))
    }

    fn cell(&mut self, seq: Vec<It>, at: &[usize]) -> Result<Vec<CellPara>, String> {
        let mut out = vec![];
        for item in Self::items(seq)? {
            let Item::Para(units, mark) = item else { unreachable!("a cell holds paragraphs") };
            let mp: Path = at.iter().copied().chain([out.len()]).collect();
            let side = if mark.new.is_some() { &self.new } else { &self.old };
            let style = side.para(mark_path(&mark.new).or(mark_path(&mark.old)).unwrap()).2;
            let (content, _, _) = self.para(&units, &mark, &mp)?;
            out.push(CellPara { style, content });
        }
        self.cell_len.insert(at.to_vec(), out.len());
        Ok(out)
    }

    // -------------------------------------------------------------- entries in merged coordinates

    /// A position in new paragraph `path` (before its unit `p`, or at its end) → merged.
    fn new_pos(&self, path: &Path, p: usize) -> Option<(Path, usize)> {
        if p < self.new.len(path) {
            return self.new_unit.get(&Pos::Unit(path.clone(), p)).cloned();
        }
        let mp = self.new_mark.get(path)?;
        Some((mp.clone(), self.lens[mp]))
    }

    fn old_pos(&self, path: &Path, p: usize) -> Option<(Path, usize)> {
        if p < self.old.len(path) {
            return self.old_unit.get(&Pos::Unit(path.clone(), p)).cloned();
        }
        let mp = self.old_mark.get(path)?;
        Some((mp.clone(), self.lens[mp]))
    }

    /// A block-level path (a block, row, cell or cell paragraph, or one past
    /// the last of them) of a new (`new`) or old table → merged.
    fn block_path(&self, path: &Path, new: bool) -> Option<Path> {
        let blocks = if new { &self.new_block } else { &self.old_block };
        let mi = *blocks.get(&path[0])?;
        let mut out = vec![mi];
        out.extend(&path[1..]);
        if path.len() == 4 {
            let cell = vec![mi, path[1], path[2]];
            let covered = if new { &self.new_covered } else { &self.old_covered };
            if new && !covered.contains_key(&path[..3]) {
                out[3] = match self.new_cell_para.get(path) {
                    Some(k) => *k,
                    None => *self.cell_len.get(&cell)?,
                };
            }
        }
        Some(out)
    }

    /// The entries of the merged model: the current remainder's, moved to
    /// merged positions, and the imported runs and paragraph properties of
    /// what was deleted.
    fn entries(&self) -> Result<Vec<Entry>, String> {
        let mut out = vec![];
        let mut next = self.new.rem.next_id.max(self.old.rem.next_id) + 1_000_000;
        let mut fresh = || {
            next += 1;
            next
        };
        let live: HashSet<u64> = self.new.rem.entries.iter().map(|e| e.id).collect();
        let lost = || "an entry of the current revision has no place in the merged document".to_string();
        for e in &self.new.rem.entries {
            let at = |x: Option<(Path, usize)>| x.ok_or_else(lost);
            let covered = e.path.len() >= 3 && self.new_covered.contains_key(&e.path[..3]);
            let mut x = e.clone();
            match e.kind {
                Kind::Tail => {}
                _ if covered => x.path = self.block_path(&e.path, true).ok_or_else(lost)?,
                // Between blocks: right after the block before it, so a
                // deleted paragraph stays next to the paragraph it joins.
                Kind::Bmarker if e.path.len() == 1 => {
                    let j = e.path[0];
                    x.path = vec![if j == 0 { 0 } else { self.new_last.get(&(j - 1)).ok_or_else(lost)? + 1 }];
                }
                Kind::Bkeep | Kind::Bmarker | Kind::Tbl | Kind::Tr | Kind::Tc => {
                    x.path = self.block_path(&e.path, true).ok_or_else(lost)?
                }
                Kind::Ppr => x.path = self.new_mark.get(&e.path).cloned().ok_or_else(lost)?,
                Kind::Keep => {
                    let (p, s) = at(self.new_pos(&e.path, e.start.unwrap()))?;
                    (x.path, x.start, x.end) = (p, Some(s), Some(s + 1));
                }
                Kind::Run if e.end > e.start => {
                    let units = (e.start.unwrap()..e.end.unwrap().min(self.new.len(&e.path)))
                        .map(|u| at(self.new_unit.get(&Pos::Unit(e.path.clone(), u)).cloned()))
                        .collect::<Result<Vec<_>, _>>()?;
                    for (k, (p, s, en)) in pieces(units).into_iter().enumerate() {
                        let mut y = e.clone();
                        (y.path, y.start, y.end) = (p, Some(s), Some(en));
                        if k > 0 {
                            y.id = fresh();
                        }
                        out.push(y);
                    }
                    continue;
                }
                Kind::Wrap => {
                    let (s, en) = (e.start.unwrap(), e.end.unwrap());
                    let (p, a) = at(self.new_pos(&e.path, s))?;
                    let (q, b) = if en > s {
                        let (q, b) = at(self.new_pos(&e.path, en - 1))?;
                        (q, b + 1)
                    } else {
                        (p.clone(), a)
                    };
                    if p != q {
                        return Err("a hyperlink or other wrapper would span a tracked paragraph mark".into());
                    }
                    (x.path, x.start, x.end) = (p, Some(a), Some(b));
                }
                _ => {
                    let (p, s) = at(self.new_pos(&e.path, e.start.unwrap_or(0)))?;
                    (x.path, x.start, x.end) = (p, Some(s), Some(s));
                }
            }
            out.push(x);
        }
        // What was deleted: its runs, placeholders and paragraph properties as imported.
        for e in &self.old.rem.entries {
            let covered = e.path.len() >= 3 && self.old_covered.contains_key(&e.path[..3]);
            let deleted_table = self.old_block.get(&e.path.first().copied().unwrap_or(usize::MAX)).is_some_and(|&mi| {
                matches!(self.old.blocks[e.path[0]], Block::Table(_)) && self.rows.contains_key(&vec![mi, 0])
            });
            let mut x = e.clone();
            x.id = fresh();
            match e.kind {
                Kind::Tbl | Kind::Tr | Kind::Tc if deleted_table => {
                    x.path = self.block_path(&e.path, false).ok_or_else(lost)?
                }
                _ if covered && deleted_table => x.path = self.block_path(&e.path, false).ok_or_else(lost)?,
                Kind::Ppr => {
                    let Some(mp) = self.old_mark.get(&e.path) else { continue };
                    if self.marks[mp].st != St::Del {
                        continue;
                    }
                    x.path = mp.clone();
                    if live.contains(&e.id) {
                        // Its paragraph is also live elsewhere: one paragraph id per paragraph.
                        let mut shell = fragment(&x.xml[0]);
                        shell.remove_attr("w14:paraId");
                        shell.remove_attr("w14:textId");
                        x.xml[0] = shell.to_xml();
                    }
                }
                Kind::Run if e.end > e.start => {
                    let deleted = (e.start.unwrap()..e.end.unwrap().min(self.old.len(&e.path)))
                        .filter_map(|u| self.old_unit.get(&Pos::Unit(e.path.clone(), u)).cloned())
                        .filter(|(p, s)| self.units[p][*s].st == St::Del);
                    for (p, s, en) in pieces(deleted) {
                        let mut y = x.clone();
                        (y.path, y.start, y.end) = (p, Some(s), Some(en));
                        y.id = fresh();
                        out.push(y);
                    }
                    continue;
                }
                Kind::Keep => {
                    let Some((p, s)) = self.old_pos(&e.path, e.start.unwrap()) else { continue };
                    if self.units[&p].get(s).is_none_or(|u| u.st != St::Del) {
                        continue;
                    }
                    (x.path, x.start, x.end) = (p, Some(s), Some(s + 1));
                }
                Kind::Wrap if !live.contains(&e.id) => {
                    let (s, en) = (e.start.unwrap(), e.end.unwrap());
                    if en <= s {
                        continue;
                    }
                    let (Some((p, a)), Some((q, b))) = (self.old_pos(&e.path, s), self.old_pos(&e.path, en - 1)) else {
                        continue;
                    };
                    if p != q || !(a..=b).all(|k| self.units[&p][k].st == St::Del) {
                        return Err("the edit deletes text inside a hyperlink or other wrapper that would span a \
                                    tracked paragraph mark"
                            .into());
                    }
                    (x.path, x.start, x.end) = (p, Some(a), Some(b + 1));
                }
                _ => continue,
            }
            out.push(x);
        }
        Ok(out)
    }
}
