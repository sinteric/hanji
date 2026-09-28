//! A port of Python's `difflib.SequenceMatcher` (no junk, `autojunk=False`).
//! The remainder prototype's numbers were measured with it; the same
//! matcher keeps alignment and re-anchoring comparable.

use std::collections::HashMap;
use std::hash::Hash;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tag {
    Equal,
    Replace,
    Delete,
    Insert,
}

/// `(tag, i1, i2, j1, j2)`: `a[i1..i2]` becomes `b[j1..j2]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Op {
    pub tag: Tag,
    pub i1: usize,
    pub i2: usize,
    pub j1: usize,
    pub j2: usize,
}

pub struct Matcher<'a, T> {
    a: &'a [T],
    b: &'a [T],
    b2j: HashMap<&'a T, Vec<usize>>,
}

impl<'a, T: Hash + Eq> Matcher<'a, T> {
    pub fn new(a: &'a [T], b: &'a [T]) -> Self {
        let mut b2j: HashMap<&T, Vec<usize>> = HashMap::new();
        for (j, x) in b.iter().enumerate() {
            b2j.entry(x).or_default().push(j);
        }
        Matcher { a, b, b2j }
    }

    fn longest(
        &self,
        alo: usize,
        ahi: usize,
        blo: usize,
        bhi: usize,
        j2len: &mut [usize],
        new: &mut [usize],
    ) -> (usize, usize, usize) {
        let (a, b) = (self.a, self.b);
        let (mut bi, mut bj, mut bs) = (alo, blo, 0);
        let mut touched: Vec<usize> = vec![];
        for (i, ai) in a.iter().enumerate().take(ahi).skip(alo) {
            let mut new_touched = vec![];
            if let Some(js) = self.b2j.get(ai) {
                for &j in js {
                    if j < blo {
                        continue;
                    }
                    if j >= bhi {
                        break;
                    }
                    let k = if j > 0 { j2len[j - 1] } else { 0 } + 1;
                    new[j] = k;
                    new_touched.push(j);
                    if k > bs {
                        bi = i + 1 - k;
                        bj = j + 1 - k;
                        bs = k;
                    }
                }
            }
            for &j in &touched {
                j2len[j] = 0;
            }
            for &j in &new_touched {
                j2len[j] = new[j];
                new[j] = 0;
            }
            touched = new_touched;
        }
        for &j in &touched {
            j2len[j] = 0;
        }
        while bi > alo && bj > blo && a[bi - 1] == b[bj - 1] {
            bi -= 1;
            bj -= 1;
            bs += 1;
        }
        while bi + bs < ahi && bj + bs < bhi && a[bi + bs] == b[bj + bs] {
            bs += 1;
        }
        (bi, bj, bs)
    }

    pub fn matching_blocks(&self) -> Vec<(usize, usize, usize)> {
        let (la, lb) = (self.a.len(), self.b.len());
        let mut j2len = vec![0; lb + 1];
        let mut new = vec![0; lb + 1];
        let mut queue = vec![(0, la, 0, lb)];
        let mut blocks = vec![];
        while let Some((alo, ahi, blo, bhi)) = queue.pop() {
            let (i, j, k) = self.longest(alo, ahi, blo, bhi, &mut j2len, &mut new);
            if k > 0 {
                blocks.push((i, j, k));
                if alo < i && blo < j {
                    queue.push((alo, i, blo, j));
                }
                if i + k < ahi && j + k < bhi {
                    queue.push((i + k, ahi, j + k, bhi));
                }
            }
        }
        blocks.sort();
        let mut out = vec![];
        let (mut i1, mut j1, mut k1) = (0, 0, 0);
        for (i2, j2, k2) in blocks {
            if i1 + k1 == i2 && j1 + k1 == j2 {
                k1 += k2;
            } else {
                if k1 > 0 {
                    out.push((i1, j1, k1));
                }
                (i1, j1, k1) = (i2, j2, k2);
            }
        }
        if k1 > 0 {
            out.push((i1, j1, k1));
        }
        out.push((la, lb, 0));
        out
    }

    pub fn opcodes(&self) -> Vec<Op> {
        let (mut i, mut j) = (0, 0);
        let mut out = vec![];
        for (ai, bj, size) in self.matching_blocks() {
            let tag = match (i < ai, j < bj) {
                (true, true) => Some(Tag::Replace),
                (true, false) => Some(Tag::Delete),
                (false, true) => Some(Tag::Insert),
                _ => None,
            };
            if let Some(tag) = tag {
                out.push(Op { tag, i1: i, i2: ai, j1: j, j2: bj });
            }
            (i, j) = (ai + size, bj + size);
            if size > 0 {
                out.push(Op { tag: Tag::Equal, i1: ai, i2: i, j1: bj, j2: j });
            }
        }
        out
    }

    pub fn ratio(&self) -> f64 {
        let m: usize = self.matching_blocks().iter().map(|b| b.2).sum();
        let t = self.a.len() + self.b.len();
        if t == 0 {
            1.0
        } else {
            2.0 * m as f64 / t as f64
        }
    }
}

pub fn opcodes<T: Hash + Eq>(a: &[T], b: &[T]) -> Vec<Op> {
    Matcher::new(a, b).opcodes()
}

pub fn ratio<T: Hash + Eq>(a: &[T], b: &[T]) -> f64 {
    Matcher::new(a, b).ratio()
}

/// Opcodes for replacing `old[s..e]` with `new_len` units.
pub fn span_ops(n_old: usize, s: usize, e: usize, new_len: usize) -> Vec<Op> {
    let mut ops = vec![];
    if s > 0 {
        ops.push(Op { tag: Tag::Equal, i1: 0, i2: s, j1: 0, j2: s });
    }
    if e > s || new_len > 0 {
        let tag = match (e > s, new_len > 0) {
            (true, true) => Tag::Replace,
            (true, false) => Tag::Delete,
            _ => Tag::Insert,
        };
        ops.push(Op { tag, i1: s, i2: e, j1: s, j2: s + new_len });
    }
    if e < n_old {
        ops.push(Op { tag: Tag::Equal, i1: e, i2: n_old, j1: s + new_len, j2: s + new_len + n_old - e });
    }
    ops
}

pub fn equal_ops(n: usize) -> Vec<Op> {
    vec![Op { tag: Tag::Equal, i1: 0, i2: n, j1: 0, j2: n }]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codes(a: &str, b: &str) -> Vec<(Tag, usize, usize, usize, usize)> {
        let a: Vec<char> = a.chars().collect();
        let b: Vec<char> = b.chars().collect();
        opcodes(&a, &b).iter().map(|o| (o.tag, o.i1, o.i2, o.j1, o.j2)).collect()
    }

    #[test]
    fn matches_python_difflib() {
        // Expected values from CPython 3.11 difflib.SequenceMatcher(None, a, b, autojunk=False).
        use Tag::*;
        assert_eq!(
            codes("qabxcd", "abycdf"),
            vec![
                (Delete, 0, 1, 0, 0),
                (Equal, 1, 3, 0, 2),
                (Replace, 3, 4, 2, 3),
                (Equal, 4, 6, 3, 5),
                (Insert, 6, 6, 5, 6)
            ]
        );
        assert_eq!(codes("2023", "20235"), vec![(Equal, 0, 4, 0, 4), (Insert, 4, 4, 4, 5)]);
        assert_eq!(codes("abcabc", "abc"), vec![(Equal, 0, 3, 0, 3), (Delete, 3, 6, 3, 3)]);
        assert_eq!(codes("", ""), vec![]);
        let a: Vec<char> = "abcd".chars().collect();
        let b: Vec<char> = "bcde".chars().collect();
        assert!((ratio(&a, &b) - 0.75).abs() < 1e-9);
    }
}
