//! Re-anchoring (§10.3, design C): block alignment, then per-paragraph and
//! document-level character mapping. Every entry comes out placed, removed
//! with its text (reported), or refused with a reason.

use std::collections::{HashMap, HashSet};

use hanji_format::{Cell, Inline};

use crate::diff::{self, Op, Tag};
use crate::model::{keys, paras, Block, Path};
use crate::remainder::{Entry, Kind};

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)] // short-lived; one per entry
pub enum Status {
    /// Placed; a run may land in several pieces `(path, start, end)`.
    Placed { entry: Entry, pieces: Vec<(Path, usize, usize)> },
    /// Removed together with its text or block; reported.
    Removed(String),
    /// Refused: it cannot be placed without guessing.
    Refused(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub status: Status,
    /// Paragraph properties of a split or joined paragraph: no single right answer.
    pub lenient: bool,
}

impl From<Status> for Outcome {
    fn from(status: Status) -> Outcome {
        Outcome { status, lenient: false }
    }
}

impl Outcome {
    pub fn placed(&self) -> Option<&Entry> {
        match &self.status {
            Status::Placed { entry, .. } => Some(entry),
            _ => None,
        }
    }
}

// ------------------------------------------------------------------ block alignment

fn cell_keys(cell: &Cell, out: &mut Vec<u32>) {
    match cell {
        Cell::Text(ps) => {
            for (k, p) in ps.iter().enumerate() {
                if k > 0 {
                    out.push(4);
                }
                out.extend(keys(&p.content));
            }
        }
        Cell::Up => out.extend([0x5e, 0x5e]),
        Cell::Left => out.extend([0x7c, 0x7c]),
    }
}

fn row_keys(row: &[Cell], out: &mut Vec<u32>) {
    for (c, cell) in row.iter().enumerate() {
        if c > 0 {
            out.push(2);
        }
        cell_keys(cell, out);
    }
}

fn content(b: &Block) -> Vec<u32> {
    match b {
        Block::Para(p) => keys(&p.content),
        Block::Table(t) => {
            let mut out = vec![];
            for (r, row) in t.rows.iter().enumerate() {
                if r > 0 {
                    out.push(1);
                }
                row_keys(row, &mut out);
            }
            out
        }
        Block::Keep(id) => std::iter::once(3).chain(id.chars().map(|c| c as u32)).collect(),
        // What a head is, not its label: a slide whose layout changed has the same content.
        Block::Head(h) => [5, h.level as u32].into_iter().chain(h.key.chars().map(|c| c as u32)).collect(),
    }
}

pub(crate) fn same_kind(a: &Block, b: &Block) -> bool {
    match (a, b) {
        (Block::Head(x), Block::Head(y)) => x.level == y.level && x.key == y.key,
        _ => matches!((a, b), (Block::Para(_), Block::Para(_)) | (Block::Table(_), Block::Table(_))),
    }
}

/// Old block index → new block index. Diff on whole blocks, then pair moved
/// (identical) blocks, then restyled blocks (same content), then edited
/// blocks inside each replaced stretch, then similar blocks anywhere.
pub fn align(old: &[Block], new: &[Block]) -> Vec<Option<usize>> {
    align_ambiguous(old, new).0
}

/// [`align`], plus the old blocks whose pairing had to choose among
/// identical blocks: matched only through a run of repeated blocks, or
/// moved while an identical block exists.
pub fn align_ambiguous(old: &[Block], new: &[Block]) -> (Vec<Option<usize>>, HashSet<usize>) {
    // Nothing changed: every block is where it was, identical ones included.
    if old == new {
        return ((0..old.len()).map(Some).collect(), HashSet::new());
    }
    let mut bmap: Vec<Option<usize>> = vec![None; old.len()];
    let mut used: HashSet<usize> = HashSet::new();
    let mut stretches: Vec<(Vec<usize>, Vec<usize>)> = vec![];
    let mut count_old: HashMap<&Block, usize> = HashMap::new();
    let mut count_new: HashMap<&Block, usize> = HashMap::new();
    for b in old {
        *count_old.entry(b).or_default() += 1;
    }
    for b in new {
        *count_new.entry(b).or_default() += 1;
    }
    let unique = |b: &Block| count_old.get(b) == Some(&1) && count_new.get(b) == Some(&1);
    let mut ambiguous = HashSet::new();
    for op in diff::opcodes(old, new) {
        if op.tag == Tag::Equal {
            for k in 0..op.i2 - op.i1 {
                bmap[op.i1 + k] = Some(op.j1 + k);
                used.insert(op.j1 + k);
                if !unique(&old[op.i1 + k]) {
                    ambiguous.insert(op.i1 + k);
                }
            }
        } else {
            stretches.push(((op.i1..op.i2).collect(), (op.j1..op.j2).collect()));
        }
    }
    let free_new: Vec<usize> = stretches.iter().flat_map(|s| s.1.clone()).collect();
    let pair = |bmap: &mut Vec<Option<usize>>, used: &mut HashSet<usize>, i: usize, j: usize| {
        bmap[i] = Some(j);
        used.insert(j);
    };
    // Moves: unique blocks first, then repeated blocks next to an already
    // paired neighbour, then the rest (first come, ambiguous).
    let moved: Vec<usize> = stretches.iter().flat_map(|s| s.0.clone()).collect();
    for &i in &moved {
        if unique(&old[i]) {
            if let Some(&j) = free_new.iter().find(|&&j| !used.contains(&j) && new[j] == old[i]) {
                pair(&mut bmap, &mut used, i, j);
            }
        }
    }
    loop {
        let mut changed = false;
        for &i in &moved {
            if bmap[i].is_some() {
                continue;
            }
            let next_to = |j: usize| {
                (i > 0 && j > 0 && bmap[i - 1] == Some(j - 1)) || (i + 1 < old.len() && bmap[i + 1] == Some(j + 1))
            };
            if let Some(&j) = free_new.iter().find(|&&j| !used.contains(&j) && new[j] == old[i] && next_to(j)) {
                pair(&mut bmap, &mut used, i, j);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for &i in &moved {
        // A head is not paired by chance among identical heads: it follows what it holds (below).
        if bmap[i].is_none() && old[i].head().is_none() {
            if let Some(&j) = free_new.iter().find(|&&j| !used.contains(&j) && new[j] == old[i]) {
                pair(&mut bmap, &mut used, i, j);
            }
        }
        if bmap[i].is_some() && !unique(&old[i]) {
            ambiguous.insert(i);
        }
    }
    let contents_old: Vec<Vec<u32>> = old.iter().map(content).collect();
    let contents_new: Vec<Vec<u32>> = new.iter().map(content).collect();
    let pairable = |i: usize, j: usize| same_kind(&old[i], &new[j]) && old[i].head().is_none();
    let ratio = |i: usize, j: usize| diff::ratio(&contents_old[i], &contents_new[j]);
    for (olds, _) in &stretches {
        for &i in olds {
            if bmap[i].is_some() {
                continue;
            }
            if let Some(&j) =
                free_new.iter().find(|&&j| !used.contains(&j) && pairable(i, j) && contents_old[i] == contents_new[j])
            {
                pair(&mut bmap, &mut used, i, j);
            }
        }
    }
    for (olds, news) in &stretches {
        let o: Vec<usize> = olds.iter().copied().filter(|&i| bmap[i].is_none()).collect();
        let n: Vec<usize> = news.iter().copied().filter(|j| !used.contains(j)).collect();
        if o.len() == n.len() {
            for (&i, &j) in o.iter().zip(&n) {
                if pairable(i, j) && ratio(i, j) >= 0.4 {
                    pair(&mut bmap, &mut used, i, j);
                }
            }
        }
    }
    let left_o: Vec<usize> = stretches.iter().flat_map(|s| s.0.clone()).filter(|&i| bmap[i].is_none()).collect();
    let left_n: Vec<usize> = free_new.iter().copied().filter(|j| !used.contains(j)).collect();
    let mut cands: Vec<(f64, usize, usize)> = vec![];
    for &i in &left_o {
        for &j in &left_n {
            if pairable(i, j) {
                cands.push((ratio(i, j), i, j));
            }
        }
    }
    cands.sort_by(|a, b| b.partial_cmp(a).unwrap());
    for (r, i, j) in cands {
        if r < 0.5 {
            break;
        }
        if bmap[i].is_none() && !used.contains(&j) {
            pair(&mut bmap, &mut used, i, j);
        }
    }
    follow_heads(old, new, &mut bmap, &mut used);
    // Block placeholders are found by id wherever they went.
    for (i, b) in old.iter().enumerate() {
        if let Block::Keep(id) = b {
            bmap[i] = new.iter().position(|x| matches!(x, Block::Keep(y) if y == id));
            ambiguous.remove(&i);
        }
    }
    // A repeated block is pinned by a run of consecutive pairs (i → j,
    // i+1 → j+1, …) that holds a block paired without ambiguity.
    let mut k = 0;
    while k < old.len() {
        let mut e = k + 1;
        while e < old.len() && bmap[k].is_some() && bmap[e].is_some() && bmap[e] == bmap[e - 1].map(|j| j + 1) {
            e += 1;
        }
        if bmap[k].is_some() && (k..e).any(|i| !ambiguous.contains(&i)) {
            for i in k..e {
                ambiguous.remove(&i);
            }
        }
        k = e;
    }
    (bmap, ambiguous)
}

/// Heads follow what they hold: an unpaired head pairs with the head of its
/// kind that the first paired block it holds now sits under; a head that
/// holds no paired block, with the one just after where the block before it
/// went. Inner heads (a slide's slots) are paired first.
fn follow_heads(old: &[Block], new: &[Block], bmap: &mut [Option<usize>], used: &mut HashSet<usize>) {
    let end = |i: usize, lv: u8| {
        (i + 1..old.len()).find(|&k| old[k].head().is_some_and(|n| n.level <= lv)).unwrap_or(old.len())
    };
    loop {
        let mut changed = false;
        for i in (0..old.len()).rev() {
            let Some(h) = old[i].head().filter(|_| bmap[i].is_none()) else { continue };
            let under = |jc: usize| (0..jc).rev().find(|&x| new[x].head().is_some_and(|n| n.level <= h.level));
            let j = (i + 1..end(i, h.level))
                .find_map(|k| bmap[k])
                .and_then(under)
                .or_else(|| i.checked_sub(1).and_then(|p| bmap[p]).map(|j| j + 1));
            let fits = |j: usize| !used.contains(&j) && new.get(j).is_some_and(|b| same_kind(&old[i], b));
            if let Some(j) = j.filter(|&j| fits(j)) {
                bmap[i] = Some(j);
                used.insert(j);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

type CellMap = Box<dyn Fn(usize, usize) -> Option<(usize, usize)>>;

/// `(row, col)` in the old table → the new one.
fn cell_map(ot: &[Vec<Cell>], nt: &[Vec<Cell>]) -> CellMap {
    if ot.len() == nt.len() && ot.iter().zip(nt).all(|(a, b)| a.len() == b.len()) {
        return Box::new(|r, c| Some((r, c)));
    }
    let sig = |row: &Vec<Cell>| -> Vec<u32> {
        let mut out = vec![];
        row_keys(row, &mut out);
        out
    };
    let os: Vec<Vec<u32>> = ot.iter().map(sig).collect();
    let ns: Vec<Vec<u32>> = nt.iter().map(sig).collect();
    let mut rm: HashMap<usize, usize> = HashMap::new();
    for op in diff::opcodes(&os, &ns) {
        if op.tag == Tag::Equal || (op.tag == Tag::Replace && op.i2 - op.i1 == op.j2 - op.j1) {
            for k in 0..op.i2 - op.i1 {
                rm.insert(op.i1 + k, op.j1 + k);
            }
        }
    }
    let widths: HashSet<usize> = ot.iter().chain(nt).map(Vec::len).collect();
    let same_cols = widths.len() == 1;
    Box::new(move |r, c| if same_cols { rm.get(&r).map(|&nr| (nr, c)) } else { None })
}

// ------------------------------------------------------------------ character mapping

/// Position `p` in the old text → the new text; `right` for range starts.
pub fn map_pos(ops: &[Op], p: usize, right: bool) -> usize {
    let mut cands = vec![];
    for o in ops {
        if !(o.i1 <= p && p <= o.i2) {
            continue;
        }
        match o.tag {
            Tag::Equal => cands.push(o.j1 + (p - o.i1)),
            Tag::Insert => cands.extend([o.j1, o.j2]),
            _ if p == o.i1 => cands.push(o.j1),
            _ if p == o.i2 => cands.push(o.j2),
            _ => cands.push(if right { o.j2 } else { o.j1 }),
        }
    }
    let pick = if right { cands.iter().max() } else { cands.iter().min() };
    pick.copied().unwrap_or(0)
}

/// Per new character: `(old char it comes from, equal?)`. Replaced text
/// takes the run of the first replaced character; inserted text joins the
/// run on its left.
fn owners_after(ops: &[Op], new_len: usize) -> Vec<Option<(usize, bool)>> {
    let mut out = vec![None; new_len];
    for o in ops {
        for (j, slot) in out.iter_mut().enumerate().take(o.j2).skip(o.j1) {
            *slot = match o.tag {
                Tag::Equal => Some((o.i1 + (j - o.j1), true)),
                Tag::Replace => Some((o.i1, false)),
                Tag::Insert => Some((o.i1.saturating_sub(1), false)),
                Tag::Delete => None,
            };
        }
    }
    out
}

/// Owner of old char `k`; a character outside any run (a placeholder)
/// defers to the nearest run on its left, then on its right.
fn resolve_owner(owner: &[Option<u64>], k: usize) -> Option<u64> {
    if owner.is_empty() {
        return None;
    }
    resolve_in(owner, k, 0, owner.len())
}

fn resolve_in(owner: &[Option<u64>], k: usize, lo: usize, hi: usize) -> Option<u64> {
    if hi <= lo {
        return None;
    }
    let k = k.clamp(lo, hi - 1);
    (lo..=k).rev().chain(k + 1..hi).find_map(|x| owner[x])
}

/// Paragraph separator in a stream (U+2029, as in the prototype).
pub const SEP: u32 = 0x2029;

/// Paragraphs concatenated into one stream, `SEP` between them.
#[derive(Clone, Debug, Default)]
pub struct Stream {
    pub text: Vec<u32>,
    pub order: Vec<Path>,
    pub base: HashMap<Path, usize>,
    pub lens: HashMap<Path, usize>,
    starts: Vec<usize>,
}

impl Stream {
    pub fn new<'a>(items: impl IntoIterator<Item = (Path, &'a Inline)>) -> Stream {
        let mut s = Stream::default();
        for (path, i) in items {
            let k = keys(i);
            s.base.insert(path.clone(), s.text.len());
            s.starts.push(s.text.len());
            s.lens.insert(path.clone(), k.len());
            s.order.push(path);
            s.text.extend(k);
            s.text.push(SEP);
        }
        s
    }

    /// Global gap → `(path, local offset)`.
    pub fn locate(&self, g: usize) -> Option<(Path, usize)> {
        let k = self.starts.partition_point(|&s| s <= g).saturating_sub(1);
        let path = self.order.get(k)?;
        let loc = g.saturating_sub(self.base[path]).min(self.lens[path]);
        Some((path.clone(), loc))
    }

    /// Global character → `(path, local)`, `None` on a separator.
    pub fn char_at(&self, g: usize) -> Option<(Path, usize)> {
        let k = self.starts.partition_point(|&s| s <= g).checked_sub(1)?;
        let path = &self.order[k];
        let loc = g - self.base[path];
        (loc < self.lens[path]).then(|| (path.clone(), loc))
    }
}

/// A document-level character map over some paragraphs (design C, or
/// derived from an exact edit span).
#[derive(Clone, Debug, Default)]
pub struct Global {
    pub old: Stream,
    pub new: Stream,
    pub ops: Vec<Op>,
    /// Shortest equal stretch that shows a paragraph's text survived (a diff
    /// matches stray characters; an exact span does not).
    pub min_equal: usize,
}

/// How the old model maps onto the new one.
#[derive(Default)]
pub struct Alignment {
    pub bmap: Vec<Option<usize>>,
    /// True per-paragraph opcodes (old path → ops), e.g. from an exact span.
    pub para_ops: HashMap<Path, Vec<Op>>,
    /// Split/join map: old path → `[(lo, hi, new path, shift)]`.
    pub cross: HashMap<Path, Vec<(usize, usize, Path, usize)>>,
    pub global: Option<Global>,
    /// Paths whose paragraph properties have no single right answer.
    pub lenient: HashSet<Path>,
    /// Old blocks paired by choosing among identical blocks.
    pub ambiguous: HashSet<usize>,
}

impl Alignment {
    /// Design C: block alignment, then one character diff over every
    /// top-level paragraph that did not align unchanged.
    pub fn design_c(old: &[Block], new: &[Block]) -> Alignment {
        let (bmap, ambiguous) = align_ambiguous(old, new);
        let (mut same_old, mut same_new) = (HashSet::new(), HashSet::new());
        for (i, j) in bmap.iter().enumerate() {
            let Some(j) = *j else { continue };
            let same = match (&old[i], &new[j]) {
                (Block::Para(a), Block::Para(b)) => a.style == b.style && keys(&a.content) == keys(&b.content),
                (Block::Para(_), _) => false,
                _ => true,
            };
            if same {
                same_old.insert(i);
                same_new.insert(j);
            }
        }
        let o = Stream::new(top_paras(old, &same_old));
        let n = Stream::new(top_paras(new, &same_new));
        let global = (!o.order.is_empty()).then(|| {
            let ops = diff::opcodes(&o.text, &n.text);
            Global { old: o, new: n, ops, min_equal: 3 }
        });
        Alignment { bmap, global, ambiguous, ..Default::default() }
    }
}

// ------------------------------------------------------------------ placement

pub struct Placer<'a> {
    old: &'a [Block],
    new: &'a [Block],
    entries: &'a [Entry],
    al: &'a Alignment,
    out: HashMap<u64, Outcome>,
    handled: HashSet<u64>,
    by_path: HashMap<Path, Vec<&'a Entry>>,
    owners: HashMap<(Path, Path), Vec<Option<u64>>>,
}

fn top_paras<'b>(blocks: &'b [Block], same: &HashSet<usize>) -> Vec<(Path, &'b Inline)> {
    blocks
        .iter()
        .enumerate()
        .filter_map(|(j, b)| match b {
            Block::Para(p) if !same.contains(&j) => Some((vec![j], &p.content)),
            _ => None,
        })
        .collect()
}

/// Place every entry of `entries` (anchored to `old`) against `new`.
pub fn place(old: &[Block], new: &[Block], entries: &[Entry], al: &Alignment) -> HashMap<u64, Outcome> {
    let mut by_path: HashMap<Path, Vec<&Entry>> = HashMap::new();
    for e in entries {
        by_path.entry(e.path.clone()).or_default().push(e);
    }
    let mut p =
        Placer { old, new, entries, al, out: HashMap::new(), handled: HashSet::new(), by_path, owners: HashMap::new() };
    p.run();
    p.out
}

impl<'a> Placer<'a> {
    fn set(&mut self, e: &Entry, status: Status) {
        if self.handled.contains(&e.id) && self.out.contains_key(&e.id) {
            return;
        }
        self.out.insert(e.id, status.into());
    }

    fn put(&mut self, e: &Entry, path: Path, start: Option<usize>, end: Option<usize>) {
        self.set(e, Status::Placed { entry: moved(e, path, start, end), pieces: vec![] });
    }

    fn entries_at(&self, path: &Path) -> Vec<&'a Entry> {
        self.by_path.get(path).cloned().unwrap_or_default()
    }

    fn next_surviving(&self, i: usize) -> Option<usize> {
        (i + 1..self.old.len()).find_map(|k| self.al.bmap[k])
    }

    /// A durable marker whose paragraph is gone moves to the next surviving block.
    fn relocate(&mut self, e: &Entry, i: usize) {
        let mut x = moved(e, vec![], None, None);
        match self.next_surviving(i) {
            Some(j) => {
                x.kind = Kind::Bmarker;
                x.path = vec![j];
            }
            None => x.kind = Kind::Tail,
        }
        self.set(e, Status::Placed { entry: x, pieces: vec![] });
    }

    fn run(&mut self) {
        let mut where_keep: HashMap<String, (Path, Option<usize>)> = HashMap::new();
        for (j, b) in self.new.iter().enumerate() {
            if let Block::Keep(id) = b {
                where_keep.insert(id.clone(), (vec![j], None));
            }
        }
        for (path, inl) in paras(self.new) {
            for (c, u) in inl.into_iter().flat_map(|i| i.units.iter()).enumerate() {
                if let hanji_format::Atom::Keep(k) = &u.atom {
                    where_keep.insert(k.id.clone(), (path.clone(), Some(c)));
                }
            }
        }
        for e in self.entries {
            match e.kind {
                Kind::Keep | Kind::Bkeep => {
                    let id = &e.meta.keep.as_ref().expect("keep entry without placeholder").id;
                    match where_keep.get(id).cloned() {
                        Some((path, c)) => self.put(e, path, c, c.map(|c| c + 1)),
                        None => self.set(e, Status::Removed(format!("placeholder {id} deleted by the edit"))),
                    }
                }
                Kind::Tail => self.put(e, e.path.clone(), e.start, e.end),
                _ => {}
            }
        }
        self.refuse_ambiguous();
        if !self.al.cross.is_empty() {
            self.cross_map();
        }
        if let Some(g) = &self.al.global {
            self.global_map(g);
        }
        for i in 0..self.old.len() {
            let j = self.al.bmap[i];
            for e in self.entries_at(&vec![i]) {
                if e.kind == Kind::Bmarker {
                    match j {
                        None => self.relocate(e, i),
                        Some(j) => self.put(e, vec![j], None, None),
                    }
                }
            }
            let (old, new) = (self.old, self.new);
            if matches!(old[i], Block::Keep(_)) {
                continue;
            }
            let Some(j) = j else {
                self.deleted_block(i);
                continue;
            };
            match (&old[i], &new[j]) {
                (Block::Para(op), Block::Para(np)) => self.para(&op.content, vec![i], &np.content, vec![j], &[]),
                (Block::Table(ot), Block::Table(nt)) => self.table(ot, i, nt, j),
                (Block::Head(_), Block::Head(_)) => {
                    for e in self.entries_at(&vec![i]) {
                        if !matches!(e.kind, Kind::Keep | Kind::Bkeep | Kind::Bmarker) {
                            self.put(e, vec![j], None, None);
                        }
                    }
                }
                _ => self.deleted_block(i),
            }
        }
        let al = self.al;
        for p in &al.lenient {
            for e in self.entries_at(p) {
                if e.kind == Kind::Ppr {
                    if let Some(o) = self.out.get_mut(&e.id) {
                        o.lenient = true;
                    }
                }
            }
        }
    }

    /// Entries of a block paired among identical blocks are refused, unless
    /// every identical block carries the same entries (then any pairing is right).
    fn refuse_ambiguous(&mut self) {
        type Sig = Vec<(Kind, String, Vec<usize>, Option<usize>, Option<usize>)>;
        // Entries that make one of the identical blocks different from the others.
        let carried = |e: &Entry, i: usize| {
            e.path.first() == Some(&i) && !e.fp.is_empty() && !matches!(e.kind, Kind::Keep | Kind::Bkeep)
        };
        let sig = |p: &Placer, i: usize| -> Sig {
            let mut v: Vec<_> = p
                .entries
                .iter()
                .filter(|e| carried(e, i))
                .map(|e| (e.kind, e.fp.clone(), e.path[1..].to_vec(), e.start, e.end))
                .collect();
            v.sort();
            v
        };
        let mut ambiguous: Vec<usize> = self.al.ambiguous.iter().copied().collect();
        ambiguous.sort();
        for i in ambiguous {
            let mine = sig(self, i);
            let twins: Vec<usize> = (0..self.old.len()).filter(|&k| k != i && self.old[k] == self.old[i]).collect();
            if twins.iter().all(|&k| sig(self, k) == mine) {
                continue;
            }
            let n = twins.len() + 1;
            for e in self.entries.iter().filter(|e| carried(e, i)) {
                self.handled.insert(e.id);
                let why = format!("its block is one of {n} identical blocks, and the text does not say which one went where; use an exact edit");
                self.out.insert(e.id, Status::Refused(why).into());
            }
        }
    }

    fn deleted_block(&mut self, i: usize) {
        let mut paths = vec![vec![i]];
        if matches!(self.old[i], Block::Table(_)) {
            paths = self.by_path.keys().filter(|p| p.len() >= 2 && p[0] == i).cloned().collect();
            paths.sort();
            paths.push(vec![i]);
        }
        for path in paths {
            for e in self.entries_at(&path) {
                if matches!(e.kind, Kind::Keep | Kind::Bkeep)
                    || (e.kind == Kind::Bmarker && path.len() == 1)
                    || self.handled.contains(&e.id)
                {
                    continue;
                }
                if (e.kind == Kind::Marker && e.meta.durable) || e.kind == Kind::Bmarker {
                    self.relocate(e, i);
                } else {
                    self.set(e, Status::Removed("its block was deleted".into()));
                }
            }
        }
    }

    fn table(&mut self, ot: &crate::model::Table, i: usize, nt: &crate::model::Table, j: usize) {
        for e in self.entries_at(&vec![i]) {
            if e.kind == Kind::Tbl {
                self.put(e, vec![j], None, None);
            }
        }
        let cm = cell_map(&ot.rows, &nt.rows);
        for (r, row) in ot.rows.iter().enumerate() {
            let nr = cm(r, 0);
            for e in self.entries_at(&vec![i, r]) {
                match nr {
                    None => self.set(e, Status::Refused("table rows changed shape".into())),
                    Some((nr, _)) => self.put(e, vec![j, nr], None, None),
                }
            }
            // Markers after the row's last cell.
            for e in self.entries_at(&vec![i, r, row.len()]) {
                match nr {
                    None => self.set(e, Status::Refused("table rows changed shape".into())),
                    Some((nr, _)) => self.put(e, vec![j, nr, nt.rows[nr].len()], None, None),
                }
            }
            for (c, cell) in row.iter().enumerate() {
                if *cell == Cell::Left {
                    continue;
                }
                let path = vec![i, r, c];
                let tgt = cm(r, c);
                let ncell = tgt.and_then(|(nr, nc)| nt.rows.get(nr).and_then(|x| x.get(nc)));
                let Some(ncell) = ncell.filter(|c| **c != Cell::Left) else {
                    for e in self.entries_at(&path) {
                        if e.kind != Kind::Keep && !self.handled.contains(&e.id) {
                            self.set(e, Status::Refused("its table cell no longer exists".into()));
                        }
                    }
                    continue;
                };
                let (nr, nc) = tgt.unwrap();
                let npath = vec![j, nr, nc];
                for e in self.entries_at(&path) {
                    if matches!(e.kind, Kind::Tc | Kind::Bmarker) {
                        self.put(e, npath.clone(), None, None);
                    }
                }
                let olds = self.cell_paras(cell, &path);
                let news = self.cell_paras_new(ncell);
                self.cell(&path, &olds, &npath, &news, matches!((cell, ncell), (Cell::Up, Cell::Up)));
            }
        }
        // Markers after the last row.
        for e in self.entries_at(&vec![i, ot.rows.len()]) {
            self.put(e, vec![j, nt.rows.len()], None, None);
        }
    }

    /// An old cell's paragraphs; a `^^` cell has as many empty ones as the
    /// remainder holds.
    fn cell_paras(&self, cell: &Cell, path: &Path) -> Vec<Inline> {
        match cell {
            Cell::Text(ps) => ps.iter().map(|p| p.content.clone()).collect(),
            _ => {
                let n = self
                    .by_path
                    .keys()
                    .filter(|p| p.len() == 4 && p[..3] == path[..])
                    .map(|p| p[3] + 1)
                    .max()
                    .unwrap_or(1);
                vec![Inline::default(); n]
            }
        }
    }

    fn cell_paras_new(&self, cell: &Cell) -> Vec<Inline> {
        match cell {
            Cell::Text(ps) => ps.iter().map(|p| p.content.clone()).collect(),
            _ => vec![Inline::default()],
        }
    }

    /// Paragraphs of a cell: aligned like blocks; when the text changed, one
    /// character diff over the whole cell lets text carry its entries across
    /// a split or join inside it.
    fn cell(&mut self, path: &Path, olds: &[Inline], npath: &Path, news: &[Inline], both_covered: bool) {
        let pmap: Vec<Option<usize>> = if both_covered || olds.len() == news.len() {
            (0..olds.len()).map(|k| (k < news.len()).then_some(k)).collect()
        } else {
            let as_blocks = |xs: &[Inline]| -> Vec<Block> {
                xs.iter()
                    .map(|x| Block::Para(crate::model::Para { style: String::new(), content: x.clone(), item: None }))
                    .collect()
            };
            align(&as_blocks(olds), &as_blocks(news))
        };
        let sub = |base: &Path, k: usize| -> Path { base.iter().copied().chain([k]).collect() };
        let changed = olds.len() != news.len() || olds.iter().zip(news).any(|(a, b)| keys(a) != keys(b));
        if changed && olds.len() + news.len() > 2 && !both_covered {
            let o = Stream::new(olds.iter().enumerate().map(|(k, x)| (sub(path, k), x)));
            let n = Stream::new(news.iter().enumerate().map(|(k, x)| (sub(npath, k), x)));
            let ops = diff::opcodes(&o.text, &n.text);
            self.global_map(&Global { old: o, new: n, ops, min_equal: 3 });
        }
        // Markers before a cell paragraph (or after the last) go with it.
        for k in 0..=olds.len() {
            let to = if k == olds.len() { Some(news.len()) } else { (k..olds.len()).find_map(|x| pmap[x]) };
            for e in self.entries_at(&sub(path, k)) {
                if e.kind == Kind::Bmarker {
                    self.put(e, sub(npath, to.unwrap_or(news.len())), None, None);
                }
            }
        }
        for (k, op) in olds.iter().enumerate() {
            let p = sub(path, k);
            match pmap[k] {
                Some(k2) => self.para(op, p, &news[k2], sub(npath, k2), &[]),
                None => {
                    let target = (k + 1..olds.len())
                        .find_map(|x| pmap[x])
                        .map(|k2| (k2, 0))
                        .or_else(|| (0..k).rev().find_map(|x| pmap[x]).map(|k2| (k2, news[k2].units.len())));
                    for e in self.entries_at(&p) {
                        if matches!(e.kind, Kind::Keep | Kind::Bkeep | Kind::Bmarker) || self.handled.contains(&e.id) {
                            continue;
                        }
                        match target {
                            Some((k2, at)) if e.kind == Kind::Marker && e.meta.durable => {
                                self.put(e, sub(npath, k2), Some(at), Some(at))
                            }
                            _ => self.set(e, Status::Removed("its paragraph was deleted".into())),
                        }
                    }
                }
            }
        }
    }

    fn para(&mut self, op: &Inline, path: Path, np: &Inline, npath: Path, skip: &[Kind]) {
        let ents: Vec<&Entry> = self
            .entries_at(&path)
            .into_iter()
            .filter(|e| {
                !matches!(e.kind, Kind::Keep | Kind::Bkeep | Kind::Bmarker)
                    && !skip.contains(&e.kind)
                    && !self.handled.contains(&e.id)
            })
            .collect();
        let (ok, nk) = (keys(op), keys(np));
        let ops = match self.al.para_ops.get(&path) {
            Some(o) => o.clone(),
            None if ok == nk => diff::equal_ops(ok.len()),
            None => diff::opcodes(&ok, &nk),
        };
        for e in ents {
            if e.kind == Kind::Ppr {
                self.put(e, npath.clone(), None, None);
            } else {
                self.by_ops(e, &ops, &path, np.units.len(), &npath);
            }
        }
    }

    fn old_owner(&self, path: &Path, len: usize) -> Vec<Option<u64>> {
        let mut owner = vec![None; len];
        for x in self.entries_at(path) {
            if x.kind == Kind::Run {
                let (s, e) = (x.start.unwrap(), x.end.unwrap());
                for slot in owner.iter_mut().take(e.min(len)).skip(s) {
                    *slot = Some(x.id);
                }
            }
        }
        owner
    }

    fn by_ops(&mut self, e: &Entry, ops: &[Op], path: &Path, new_len: usize, npath: &Path) {
        let (s, en) = (e.start.unwrap_or(0), e.end.unwrap_or(0));
        match e.kind {
            Kind::Marker | Kind::Rmarker => {
                let p = map_pos(ops, s, e.meta.opens);
                self.put(e, npath.clone(), Some(p), Some(p));
            }
            Kind::Wrap => {
                let (a, b) = (map_pos(ops, s, true), map_pos(ops, en, false));
                if en > s && b <= a {
                    self.set(e, Status::Removed("all of its text was deleted".into()));
                } else {
                    self.put(e, npath.clone(), Some(a), Some(b.max(a)));
                }
            }
            Kind::Run if s == en => {
                let p = map_pos(ops, s, false);
                self.put(e, npath.clone(), Some(p), Some(p));
            }
            Kind::Run => {
                let key = (path.clone(), npath.clone());
                if !self.owners.contains_key(&key) {
                    let old_len = ops.iter().map(|o| o.i2).max().unwrap_or(0);
                    let owner = self.old_owner(path, old_len);
                    let own = owners_after(ops, new_len)
                        .into_iter()
                        .map(|src| match src {
                            None => None,
                            Some((k, true)) => owner.get(k).copied().flatten(),
                            Some((k, false)) => resolve_owner(&owner, k),
                        })
                        .collect();
                    self.owners.insert(key.clone(), own);
                }
                let chars: Vec<(Path, usize)> = self.owners[&key]
                    .iter()
                    .enumerate()
                    .filter(|(_, o)| **o == Some(e.id))
                    .map(|(c, _)| (npath.clone(), c))
                    .collect();
                self.place_run(e, chars);
            }
            _ => self.put(e, npath.clone(), e.start, e.end),
        }
    }

    /// `chars`: new characters owned by run `e` → one entry, maybe in pieces.
    fn place_run(&mut self, e: &Entry, chars: Vec<(Path, usize)>) {
        if chars.is_empty() {
            self.set(e, Status::Removed("all of its text was deleted".into()));
            return;
        }
        let mut pieces: Vec<(Path, usize, usize)> = vec![];
        for (path, c) in chars {
            match pieces.last_mut() {
                Some(last) if last.0 == path && last.2 == c => last.2 = c + 1,
                _ => pieces.push((path, c, c + 1)),
            }
        }
        let mut x = e.clone();
        (x.path, x.start, x.end) = (pieces[0].0.clone(), Some(pieces[0].1), Some(pieces[0].2));
        if pieces.len() == 1 {
            pieces.clear();
        }
        self.set(e, Status::Placed { entry: x, pieces });
    }

    fn offset_entries(&self, path: &Path) -> Vec<&'a Entry> {
        self.entries_at(path).into_iter().filter(|e| e.kind.is_offset()).collect()
    }

    /// Oracle: a true split/join map.
    fn cross_map(&mut self) {
        let cross = &self.al.cross;
        for (path, segs) in cross {
            let mp = |pos: usize, right: bool| {
                let hits: Vec<(Path, usize)> =
                    segs.iter().filter(|s| s.0 <= pos && pos <= s.1).map(|s| (s.2.clone(), pos - s.0 + s.3)).collect();
                if right { hits.last().cloned() } else { hits.first().cloned() }.unwrap()
            };
            for e in self.offset_entries(path) {
                self.handled.insert(e.id);
                let (s, en) = (e.start.unwrap(), e.end.unwrap());
                match e.kind {
                    Kind::Marker | Kind::Rmarker => {
                        let (np, p) = mp(s, e.meta.opens);
                        self.out.insert(e.id, placed(e, np, p, p));
                    }
                    Kind::Run if s == en => {
                        let (np, p) = mp(s, false);
                        self.out.insert(e.id, placed(e, np, p, p));
                    }
                    Kind::Wrap => {
                        let ((a, s2), (b, e2)) = (mp(s, true), mp(en, false));
                        let o = if a != b {
                            Status::Refused("a wrapper cannot span two paragraphs".into()).into()
                        } else {
                            placed(e, a, s2, e2)
                        };
                        self.out.insert(e.id, o);
                    }
                    _ => {
                        let chars: Vec<(Path, usize)> = (s..en)
                            .filter_map(|c| {
                                segs.iter().find(|g| g.0 <= c && c < g.1).map(|g| (g.2.clone(), c - g.0 + g.3))
                            })
                            .collect();
                        self.place_run(e, chars);
                    }
                }
            }
        }
    }

    /// The document-level map: text can carry its entries into another
    /// paragraph (split, join, move-and-edit).
    fn global_map(&mut self, g: &Global) {
        let (old, new, ops) = (&g.old, &g.new, &g.ops);
        let mut survived = vec![false; old.text.len()];
        for o in ops {
            if o.tag == Tag::Equal && (o.i2 - o.i1 >= g.min_equal || o.i2 - o.i1 == old.text.len()) {
                survived[o.i1..o.i2].fill(true);
            }
        }
        let mut old_owner: Vec<Option<u64>> = vec![None; old.text.len()];
        for e in self.entries {
            if e.kind == Kind::Run {
                if let Some(&b) = old.base.get(&e.path) {
                    let n = old.lens[&e.path];
                    for c in e.start.unwrap()..e.end.unwrap().min(n) {
                        old_owner[b + c] = Some(e.id);
                    }
                }
            }
        }
        let mut new_owner: Vec<Option<u64>> = vec![None; new.text.len()];
        for o in ops {
            for j in o.j1..o.j2 {
                if o.tag == Tag::Equal {
                    new_owner[j] = old_owner[o.i1 + (j - o.j1)];
                    continue;
                }
                let mut src = if o.tag == Tag::Replace { o.i1 as isize } else { o.i1 as isize - 1 };
                if src < 0 || old.text[src as usize] == SEP {
                    src = o.i1 as isize; // inserted at a paragraph start: join the run on the right
                }
                // Replaced text that starts with a paragraph mark takes the run of
                // the first character it replaces.
                if o.tag == Tag::Replace && old.text[src as usize] == SEP {
                    src = (o.i1..o.i2).find(|&k| old.text[k] != SEP).unwrap_or(o.i1) as isize;
                }
                let src = (src as usize).min(old.text.len().saturating_sub(1));
                let Some((p, _)) = old.char_at(src) else { continue };
                let lo = old.base[&p];
                new_owner[j] = resolve_in(&old_owner, src, lo, lo + old.lens[&p]);
            }
        }
        let mut owned: HashMap<u64, Vec<(Path, usize)>> = HashMap::new();
        for (j, o) in new_owner.iter().enumerate() {
            if let (Some(o), true) = (o, new.text[j] != SEP) {
                if let Some(loc) = new.char_at(j) {
                    owned.entry(*o).or_default().push(loc);
                }
            }
        }
        for path in &old.order {
            let (b, n) = (old.base[path], old.lens[path]);
            if !survived[b..b + n].iter().any(|&x| x) {
                continue; // wholly deleted (or moved unchanged): the per-block rules apply
            }
            for e in self.offset_entries(path) {
                if !self.handled.insert(e.id) {
                    continue; // already placed by a wider map
                }
                let (s, en) = (e.start.unwrap(), e.end.unwrap());
                let o = match e.kind {
                    Kind::Run if en > s => {
                        self.place_run(e, owned.remove(&e.id).unwrap_or_default());
                        continue;
                    }
                    Kind::Wrap => {
                        let a = new.locate(map_pos(ops, b + s, true));
                        let z = new.locate(map_pos(ops, b + en, false));
                        match (a, z) {
                            (Some(a), Some(z)) if a.0 == z.0 => {
                                if en > s && z.1 <= a.1 {
                                    Status::Removed("all of its text was deleted".into()).into()
                                } else {
                                    placed(e, a.0, a.1, z.1)
                                }
                            }
                            _ => Status::Refused("its two ends now fall in different paragraphs".into()).into(),
                        }
                    }
                    _ => match new.locate(map_pos(ops, b + s, e.meta.opens)) {
                        Some((p, loc)) => placed(e, p, loc, loc),
                        None => {
                            let x = Entry { kind: Kind::Tail, ..moved(e, vec![], None, None) };
                            Status::Placed { entry: x, pieces: vec![] }.into()
                        }
                    },
                };
                self.out.insert(e.id, o);
            }
        }
    }
}

/// `e` re-anchored at `path`.
fn moved(e: &Entry, path: Path, start: Option<usize>, end: Option<usize>) -> Entry {
    Entry { path, start, end, ..e.clone() }
}

fn placed(e: &Entry, path: Path, s: usize, en: usize) -> Outcome {
    Status::Placed { entry: moved(e, path, Some(s), Some(en)), pieces: vec![] }.into()
}

/// The new revision's entries: placed entries, runs split into pieces with
/// fresh ids.
pub fn placed_entries(entries: &[Entry], outcomes: &HashMap<u64, Outcome>, next_id: &mut u64) -> Vec<Entry> {
    let mut out = vec![];
    for e in entries {
        let Some(Outcome { status: Status::Placed { entry, pieces }, .. }) = outcomes.get(&e.id) else { continue };
        if pieces.is_empty() {
            out.push(entry.clone());
            continue;
        }
        for (k, (path, s, en)) in pieces.iter().enumerate() {
            let mut x = moved(entry, path.clone(), Some(*s), Some(*en));
            if k > 0 {
                x.id = *next_id;
                *next_id += 1;
                x.seq = entry.seq + k as u64;
            }
            out.push(x);
        }
    }
    out
}
