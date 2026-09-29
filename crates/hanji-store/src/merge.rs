//! Text over lines: the diff between two revisions, and the three-way merge
//! that rebases a model edit over a person's re-imported edits (DESIGN.md §2
//! rule 7: merged on the text, or refused, never guessed). Both use the
//! core's difflib port, so they align lines as re-anchoring aligns blocks.

use hanji_core::diff::{opcodes, Op, Tag};

/// Lines with their line ends.
pub fn lines(text: &str) -> Vec<&str> {
    text.split_inclusive('\n').collect()
}

/// The 1-based line of byte `at`.
pub fn line_of(text: &str, at: usize) -> usize {
    text[..at.min(text.len())].matches('\n').count() + 1
}

/// Byte offset of the start of each line, and one past the end.
fn starts(ls: &[&str]) -> Vec<usize> {
    let mut v = Vec::with_capacity(ls.len() + 1);
    let mut at = 0;
    v.push(0);
    for l in ls {
        at += l.len();
        v.push(at);
    }
    v
}

/// A unified diff of `a` → `b` with three lines of context; empty when
/// they are the same.
pub fn unified(a: &str, b: &str, a_name: &str, b_name: &str) -> String {
    const CONTEXT: usize = 3;
    let (al, bl) = (lines(a), lines(b));
    let ops = opcodes(&al, &bl);
    if ops.iter().all(|o| o.tag == Tag::Equal) {
        return String::new();
    }
    // Group the changes with their context, as difflib's grouped_opcodes does.
    let mut groups: Vec<Vec<Op>> = vec![];
    let mut cur: Vec<Op> = vec![];
    for (k, o) in ops.iter().enumerate() {
        let mut o = *o;
        if o.tag == Tag::Equal {
            let first = k == 0;
            let last = k + 1 == ops.len();
            if first {
                o.i1 = o.i1.max(o.i2.saturating_sub(CONTEXT));
                o.j1 = o.j1.max(o.j2.saturating_sub(CONTEXT));
            } else if last {
                o.i2 = o.i2.min(o.i1 + CONTEXT);
                o.j2 = o.j2.min(o.j1 + CONTEXT);
            } else if o.i2 - o.i1 > 2 * CONTEXT {
                cur.push(Op { i2: o.i1 + CONTEXT, j2: o.j1 + CONTEXT, ..o });
                groups.push(std::mem::take(&mut cur));
                o.i1 = o.i2 - CONTEXT;
                o.j1 = o.j2 - CONTEXT;
            }
        }
        cur.push(o);
    }
    if cur.iter().any(|o| o.tag != Tag::Equal) {
        groups.push(cur);
    }
    let mut out = format!("--- {a_name}\n+++ {b_name}\n");
    let range = |s: usize, e: usize| match e - s {
        0 => format!("{s},0"),
        1 => format!("{}", s + 1),
        n => format!("{},{n}", s + 1),
    };
    let push = |out: &mut String, mark: char, l: &str| {
        out.push(mark);
        out.push_str(l);
        if !l.ends_with('\n') {
            out.push_str("\n\\ No newline at end of file\n");
        }
    };
    for g in groups {
        let (f, l) = (g[0], g[g.len() - 1]);
        out.push_str(&format!("@@ -{} +{} @@\n", range(f.i1, l.i2), range(f.j1, l.j2)));
        for o in g {
            match o.tag {
                Tag::Equal => al[o.i1..o.i2].iter().for_each(|l| push(&mut out, ' ', l)),
                _ => {
                    al[o.i1..o.i2].iter().for_each(|l| push(&mut out, '-', l));
                    bl[o.j1..o.j2].iter().for_each(|l| push(&mut out, '+', l));
                }
            }
        }
    }
    out
}

/// A replacement of `theirs[start..end]` (whole lines).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hunk {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Merged {
    /// Our changes as replacements in `theirs`, in text order; empty when
    /// `theirs` already holds them.
    Clean(Vec<Hunk>),
    /// Lines of `base` (1-based, inclusive) both sides change differently.
    Conflict(Vec<(usize, usize)>),
}

/// Three-way merge of `base` → `ours` into `theirs`, by lines. A change of
/// ours that overlaps or touches a change of theirs conflicts, unless the
/// two are the same change.
pub fn merge(base: &str, ours: &str, theirs: &str) -> Merged {
    let (bl, ol, tl) = (lines(base), lines(ours), lines(theirs));
    let changes = |ops: Vec<Op>| ops.into_iter().filter(|o| o.tag != Tag::Equal).collect::<Vec<_>>();
    let theirs_ops = opcodes(&bl, &tl);
    let (ours_ch, theirs_ch) = (changes(opcodes(&bl, &ol)), changes(theirs_ops.clone()));
    let t_starts = starts(&tl);
    let mut hunks = vec![];
    let mut conflicts = vec![];
    for c in &ours_ch {
        let hit: Vec<&Op> = theirs_ch.iter().filter(|t| c.i1 <= t.i2 && t.i1 <= c.i2).collect();
        if !hit.is_empty() {
            let same = hit.len() == 1 && {
                let t = hit[0];
                t.i1 == c.i1 && t.i2 == c.i2 && tl[t.j1..t.j2] == ol[c.j1..c.j2]
            };
            if !same {
                let lo = hit.iter().map(|t| t.i1).chain([c.i1]).min().unwrap();
                let hi = hit.iter().map(|t| t.i2).chain([c.i2]).max().unwrap();
                conflicts.push((lo + 1, hi.max(lo + 1)));
            }
            continue;
        }
        // No change of theirs touches this one: it sits inside one equal stretch.
        let j = theirs_ops
            .iter()
            .find(|o| o.tag == Tag::Equal && o.i1 <= c.i1 && c.i2 <= o.i2)
            .map_or(c.i1, |o| o.j1 + (c.i1 - o.i1));
        hunks.push(Hunk { start: t_starts[j], end: t_starts[j + (c.i2 - c.i1)], text: ol[c.j1..c.j2].concat() });
    }
    if conflicts.is_empty() {
        Merged::Clean(hunks)
    } else {
        Merged::Conflict(conflicts)
    }
}

/// `theirs` with the hunks applied.
pub fn apply(theirs: &str, hunks: &[Hunk]) -> String {
    let mut out = String::with_capacity(theirs.len());
    let mut at = 0;
    for h in hunks {
        out.push_str(&theirs[at..h.start]);
        out.push_str(&h.text);
        at = h.end;
    }
    out.push_str(&theirs[at..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_changes_apart_and_refuses_changes_together() {
        let base = "a\n\nb\n\nc\n\nd\n";
        let theirs = "a\n\nB\n\nc\n\nd\n";
        let ours = "a\n\nb\n\nc\n\nD!\n";
        let Merged::Clean(h) = merge(base, ours, theirs) else { panic!() };
        assert_eq!(apply(theirs, &h), "a\n\nB\n\nc\n\nD!\n");
        // Both change b.
        assert_eq!(merge(base, "a\n\nx\n\nc\n\nd\n", theirs), Merged::Conflict(vec![(3, 3)]));
        // The same change on both sides is taken once.
        assert_eq!(merge(base, theirs, theirs), Merged::Clean(vec![]));
        // Lines added on both sides at the same place conflict.
        assert!(matches!(merge("a\n", "a\nx\n", "a\ny\n"), Merged::Conflict(_)));
        // A change to the last line without a newline.
        let Merged::Clean(h) = merge("a\nb", "a\nc", "z\na\nb") else { panic!() };
        assert_eq!(apply("z\na\nb", &h), "z\na\nc");
    }

    #[test]
    fn unified_diff() {
        assert_eq!(unified("a\nb\n", "a\nb\n", "r1", "r2"), "");
        assert_eq!(unified("a\nb\nc\n", "a\nx\nc\n", "r1", "r2"), "--- r1\n+++ r2\n@@ -1,3 +1,3 @@\n a\n-b\n+x\n c\n");
        let mut v: Vec<String> = (1..=20).map(|k| format!("{k}\n")).collect();
        let long = v.concat();
        (v[1], v[18]) = ("two\n".into(), "nineteen\n".into());
        let d = unified(&long, &v.concat(), "a", "b");
        assert_eq!(d.matches("@@ -").count(), 2, "{d}");
    }
}
