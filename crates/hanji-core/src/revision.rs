//! What stayed the same between two revisions of a Document (§10.2): the
//! units, paragraph marks, tables and block placeholders an alignment pairs
//! unchanged. A tracked-change export writes the rest as inserted or
//! deleted. Maps compose, so a chain of edits gives the map from the
//! imported revision to the last one.

use std::collections::{HashMap, HashSet};

use hanji_format::{Cell, Inline};

use crate::diff::{self, Op, Tag};
use crate::model::{keys, Block, Path};
use crate::place::{Alignment, Stream};

/// A position in a Document's model.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Pos {
    /// Unit `.1` of the paragraph at `.0` (`[block]`, or `[block, row, col, paragraph]` in a table).
    Unit(Path, usize),
    /// The mark that ends the paragraph at the path.
    Mark(Path),
    /// A table or a block placeholder.
    Block(usize),
}

/// Old positions → the new positions that are the same thing, unchanged:
/// equal units, paragraph marks, paired tables and block placeholders.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Kept(pub HashMap<Pos, Pos>);

/// Every paragraph of `blocks` with its path: top-level ones and the
/// paragraphs of text cells.
pub fn paragraphs(blocks: &[Block]) -> Vec<(Path, &Inline)> {
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

/// The same grid of rows, cells and merge markers.
pub fn same_shape(a: &[Vec<Cell>], b: &[Vec<Cell>]) -> bool {
    let kind = |c: &Cell| match c {
        Cell::Text(_) => 0,
        Cell::Up => 1,
        Cell::Left => 2,
    };
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| x.len() == y.len() && x.iter().zip(y).all(|(p, q)| kind(p) == kind(q)))
}

impl Kept {
    /// Every position of `blocks` to itself (a revision against itself).
    pub fn identity(blocks: &[Block]) -> Kept {
        let mut m = HashMap::new();
        for (i, b) in blocks.iter().enumerate() {
            if matches!(b, Block::Table(_) | Block::Keep(_)) {
                m.insert(Pos::Block(i), Pos::Block(i));
            }
        }
        for (path, p) in paragraphs(blocks) {
            for u in 0..p.units.len() {
                m.insert(Pos::Unit(path.clone(), u), Pos::Unit(path.clone(), u));
            }
            m.insert(Pos::Mark(path.clone()), Pos::Mark(path));
        }
        Kept(m)
    }

    /// What `al` (an exact span's or design C's alignment of `old` onto
    /// `new`) keeps unchanged.
    pub fn of(old: &[Block], new: &[Block], al: &Alignment) -> Kept {
        let mut m = HashMap::new();
        let (mut in_old, mut in_new) = (HashSet::new(), HashSet::new());
        if let Some(g) = &al.global {
            in_old.extend(g.old.order.iter().cloned());
            in_new.extend(g.new.order.iter().cloned());
            stream_pairs(&g.old, &g.new, &g.ops, g.min_equal, &mut m);
        }
        for (i, j) in al.bmap.iter().enumerate().filter_map(|(i, j)| j.map(|j| (i, j))) {
            match (&old[i], &new[j]) {
                (Block::Para(a), Block::Para(b)) if !in_old.contains(&vec![i]) && !in_new.contains(&vec![j]) => {
                    let ops = al.para_ops.get(&vec![i]).cloned();
                    para_pairs(&a.content, &b.content, &[i], &[j], ops, &mut m);
                }
                (Block::Table(a), Block::Table(b)) => {
                    m.insert(Pos::Block(i), Pos::Block(j));
                    if !same_shape(&a.rows, &b.rows) {
                        continue;
                    }
                    for (r, (ra, rb)) in a.rows.iter().zip(&b.rows).enumerate() {
                        for (c, (ca, cb)) in ra.iter().zip(rb).enumerate() {
                            let (Cell::Text(pa), Cell::Text(pb)) = (ca, cb) else { continue };
                            if in_old.contains(&vec![i, r, c, 0]) || in_new.contains(&vec![j, r, c, 0]) {
                                continue; // the global map has them
                            }
                            let olds: Vec<&Inline> = pa.iter().map(|p| &p.content).collect();
                            let news: Vec<&Inline> = pb.iter().map(|p| &p.content).collect();
                            cell_pairs(&olds, &news, &[i, r, c], &[j, r, c], &mut m);
                        }
                    }
                }
                (Block::Keep(a), Block::Keep(b)) if a == b => {
                    m.insert(Pos::Block(i), Pos::Block(j));
                }
                _ => {}
            }
        }
        Kept(m)
    }

    /// `self` (old → mid), then `next` (mid → new): old → new.
    pub fn then(&self, next: &Kept) -> Kept {
        Kept(self.0.iter().filter_map(|(a, b)| next.0.get(b).map(|c| (a.clone(), c.clone()))).collect())
    }

    pub fn get(&self, p: &Pos) -> Option<&Pos> {
        self.0.get(p)
    }
}

/// A stream position: a unit, or the mark of the paragraph a separator ends.
fn stream_pos(s: &Stream, g: usize) -> Option<Pos> {
    match s.char_at(g) {
        Some((p, u)) => Some(Pos::Unit(p, u)),
        None => s.locate(g).map(|(p, _)| Pos::Mark(p)),
    }
}

/// Shortest equal stretch a diff (rather than an exact span) keeps as the
/// same text: shorter matches are stray characters of a replaced stretch.
const MIN_EQUAL: usize = 3;

/// A placeholder, note reference or other atom: never a stray match.
fn is_atom(key: u32) -> bool {
    key > 0x10_FFFF
}

/// Pairs of equal stream positions; units only from equal stretches of at
/// least `min` (paragraph marks from any).
fn stream_pairs(a: &Stream, b: &Stream, ops: &[Op], min: usize, m: &mut HashMap<Pos, Pos>) {
    for o in ops.iter().filter(|o| o.tag == Tag::Equal) {
        let short = o.i2 - o.i1 < min;
        for k in 0..o.i2 - o.i1 {
            let (x, y) = (o.i1 + k, o.j1 + k);
            if a.text.get(x) != b.text.get(y) {
                continue;
            }
            match (stream_pos(a, x), stream_pos(b, y)) {
                (Some(p @ Pos::Unit(..)), Some(q @ Pos::Unit(..))) if !short || is_atom(a.text[x]) => {
                    m.insert(p, q);
                }
                (Some(p @ Pos::Mark(_)), Some(q @ Pos::Mark(_))) => {
                    m.insert(p, q);
                }
                _ => {}
            }
        }
    }
}

fn para_pairs(a: &Inline, b: &Inline, pa: &[usize], pb: &[usize], ops: Option<Vec<Op>>, m: &mut HashMap<Pos, Pos>) {
    let (ka, kb) = (keys(a), keys(b));
    let min = if ops.is_some() { 1 } else { MIN_EQUAL };
    let ops = ops.unwrap_or_else(|| if ka == kb { diff::equal_ops(ka.len()) } else { diff::opcodes(&ka, &kb) });
    for o in ops.iter().filter(|o| o.tag == Tag::Equal) {
        let long = o.i2 - o.i1 >= min || o.i2 - o.i1 == ka.len();
        for k in 0..o.i2 - o.i1 {
            let x = o.i1 + k;
            if x < ka.len() && ka.get(x) == kb.get(o.j1 + k) && (long || is_atom(ka[x])) {
                m.insert(Pos::Unit(pa.to_vec(), x), Pos::Unit(pb.to_vec(), o.j1 + k));
            }
        }
    }
    m.insert(Pos::Mark(pa.to_vec()), Pos::Mark(pb.to_vec()));
}

/// A cell's paragraphs: one by one when their number is the same, else one
/// diff over the cell (text may cross a split or join inside it).
fn cell_pairs(olds: &[&Inline], news: &[&Inline], ca: &[usize], cb: &[usize], m: &mut HashMap<Pos, Pos>) {
    let sub = |base: &[usize], k: usize| -> Path { base.iter().copied().chain([k]).collect() };
    if olds.len() == news.len() {
        for (k, (a, b)) in olds.iter().zip(news).enumerate() {
            para_pairs(a, b, &sub(ca, k), &sub(cb, k), None, m);
        }
        return;
    }
    let a = Stream::new(olds.iter().enumerate().map(|(k, x)| (sub(ca, k), *x)));
    let b = Stream::new(news.iter().enumerate().map(|(k, x)| (sub(cb, k), *x)));
    let ops = diff::opcodes(&a.text, &b.text);
    stream_pairs(&a, &b, &ops, MIN_EQUAL, m);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Para;

    fn para(s: &str) -> Block {
        Block::Para(Para { style: "Normal".into(), content: Inline::plain(s), item: None })
    }

    #[test]
    fn a_split_keeps_every_unit_and_one_mark() {
        let old = vec![para("ab cd")];
        let new = vec![para("ab "), para("cd")];
        let al = Alignment::design_c(&old, &new);
        let k = Kept::of(&old, &new, &al);
        for u in 0..5 {
            assert!(k.get(&Pos::Unit(vec![0], u)).is_some(), "{u}: {k:?}");
        }
        // The old mark ends the second paragraph now; the first one's is new.
        assert_eq!(k.get(&Pos::Mark(vec![0])), Some(&Pos::Mark(vec![1])));
        let id = Kept::identity(&new);
        assert_eq!(k.then(&id), k);
    }
}
