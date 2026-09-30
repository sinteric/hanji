//! Edits against a revision: a whole-file rewrite (aligned by diff, design
//! C) or an exact span `old → new` (re-anchored from the span itself).

use std::collections::{HashMap, HashSet};

use hanji_format::{self as fmt, Diagnostic, Names, ParaMap, Parsed};

use crate::diff::{Op, Tag};
use crate::model::{self, Block, BlockSrc, Capabilities, Path, SrcKind, EMPTY};
use crate::place::{self, same_kind, Alignment, Global, Outcome, Status, Stream};
use crate::remainder::{Kind, Remainder};

/// What happened to the remainder: counts, and every entry that did not land.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub placed: usize,
    /// Removed together with its text or block.
    pub removed: Vec<(u64, Kind, String)>,
    /// Could not be placed without guessing.
    pub refused: Vec<(u64, Kind, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The new text is not valid for this file.
    Invalid(Vec<Diagnostic>),
    /// The exact edit does not apply (old text missing or not unique).
    Edit(String),
    /// Some entries cannot be placed; nothing was changed.
    Unplaceable(Report),
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::Invalid(d) => write!(f, "{}", fmt::diag::render(d)),
            Refusal::Edit(m) => write!(f, "{}", fmt::chars::name_in(m)),
            Refusal::Unplaceable(r) => {
                let mut m = "the edit would lose content the text does not show:".to_string();
                for (_, k, why) in &r.refused {
                    m.push_str(&format!("\n- {k:?}: {why}"));
                }
                write!(f, "{}", fmt::chars::name_in(&m))
            }
        }
    }
}

/// Names the text may use, from the remainder.
pub fn names(rem: &Remainder) -> Names {
    Names {
        paragraph_styles: Some(rem.styles.paragraph_names()),
        table_styles: Some(rem.styles.table_names()),
        fields: None,
        keeps: Some(rem.keep_list()),
        formats: Some(vec![rem.format.clone()]),
        ..Default::default()
    }
}

/// Parse, validate and resolve model text against a remainder.
pub fn model_of(text: &str, rem: &Remainder, caps: Capabilities) -> Result<(Parsed, Vec<Block>), Vec<Diagnostic>> {
    let parsed = fmt::parse_with(text, &names(rem))?;
    let blocks = model::resolve(&parsed, text, &rem.styles, caps, &|id| rem.is_block_keep(id))?;
    Ok((parsed, blocks))
}

/// A text grammar resolved against a remainder: the Document grammar
/// (§5.2, [`DocumentModel`]) or a Presentation's (§5.3). Re-anchoring works
/// on what it gives, whatever the grammar.
pub trait TextModel {
    /// Parse and check `text` against `rem`: its model blocks, and where
    /// each is in the text.
    fn resolve(
        &self,
        text: &str,
        rem: &Remainder,
        caps: Capabilities,
    ) -> Result<(Vec<Block>, Vec<BlockSrc>), Vec<Diagnostic>>;
}

/// The Document grammar (§5.2).
pub struct DocumentModel;

impl TextModel for DocumentModel {
    fn resolve(
        &self,
        text: &str,
        rem: &Remainder,
        caps: Capabilities,
    ) -> Result<(Vec<Block>, Vec<BlockSrc>), Vec<Diagnostic>> {
        let (parsed, blocks) = model_of(text, rem, caps)?;
        Ok((blocks, model::block_maps(&parsed)))
    }
}

/// Place the remainder of `old` against `new` under `al`: the new
/// remainder (placed entries only), the report and every outcome.
pub fn reanchor(
    rem: &Remainder,
    old: &[Block],
    new: &[Block],
    al: &Alignment,
) -> (Remainder, Report, HashMap<u64, Outcome>) {
    let outcomes = place::place(old, new, &rem.entries, al);
    let mut next = rem.next_id;
    let entries = place::placed_entries(&rem.entries, &outcomes, &mut next);
    let mut report = Report::default();
    for e in &rem.entries {
        match outcomes.get(&e.id).map(|o| &o.status) {
            Some(Status::Placed { .. }) => report.placed += 1,
            Some(Status::Removed(why)) => report.removed.push((e.id, e.kind, why.clone())),
            Some(Status::Refused(why)) => report.refused.push((e.id, e.kind, why.clone())),
            None => report.refused.push((e.id, e.kind, "no rule placed it".into())),
        }
    }
    let new_rem = Remainder { entries, next_id: next, ..rem.clone() };
    (new_rem, report, outcomes)
}

/// A re-anchored revision.
#[derive(Clone, Debug)]
pub struct Reanchored {
    pub text: String,
    pub old: Vec<Block>,
    pub new: Vec<Block>,
    pub remainder: Remainder,
    pub report: Report,
    pub outcomes: HashMap<u64, Outcome>,
    /// How `old` maps onto `new`: what a tracked-change export reads.
    pub alignment: Alignment,
}

fn refuse_unplaceable(r: Reanchored) -> Result<Reanchored, Refusal> {
    if r.report.refused.is_empty() {
        Ok(r)
    } else {
        Err(Refusal::Unplaceable(r.report))
    }
}

/// Re-anchor a whole-file rewrite (design C); refusals are in the report.
pub fn reanchor_rewrite(
    rem: &Remainder,
    old_text: &str,
    new_text: &str,
    caps: Capabilities,
) -> Result<Reanchored, Refusal> {
    reanchor_rewrite_in(&DocumentModel, rem, old_text, new_text, caps)
}

/// [`reanchor_rewrite`] for a text of grammar `m`.
pub fn reanchor_rewrite_in(
    m: &dyn TextModel,
    rem: &Remainder,
    old_text: &str,
    new_text: &str,
    caps: Capabilities,
) -> Result<Reanchored, Refusal> {
    let (old, _) = m.resolve(old_text, rem, caps).map_err(Refusal::Invalid)?;
    let (new, _) = m.resolve(new_text, rem, caps).map_err(Refusal::Invalid)?;
    let alignment = Alignment::design_c(&old, &new);
    let (remainder, report, outcomes) = reanchor(rem, &old, &new, &alignment);
    Ok(Reanchored { text: new_text.to_string(), old, new, remainder, report, outcomes, alignment })
}

/// Re-anchor the edit `text[start..end] → new`; refusals are in the report.
pub fn reanchor_span(
    rem: &Remainder,
    text: &str,
    start: usize,
    end: usize,
    new: &str,
    caps: Capabilities,
) -> Result<Reanchored, Refusal> {
    reanchor_span_in(&DocumentModel, rem, text, start, end, new, caps)
}

/// [`reanchor_span`] for a text of grammar `m`.
pub fn reanchor_span_in(
    m: &dyn TextModel,
    rem: &Remainder,
    text: &str,
    start: usize,
    end: usize,
    new: &str,
    caps: Capabilities,
) -> Result<Reanchored, Refusal> {
    if start > end || end > text.len() || !text.is_char_boundary(start) || !text.is_char_boundary(end) {
        return Err(Refusal::Edit(format!(
            "bytes {start}..{end} are not a span of characters of the text ({} bytes).",
            text.len()
        )));
    }
    let (s, e, repl) = trim_span(text, start, end, new);
    let new_text = format!("{}{}{}", &text[..s], repl, &text[e..]);
    let (old, om) = m.resolve(text, rem, caps).map_err(Refusal::Invalid)?;
    let (nb, nm) = m.resolve(&new_text, rem, caps).map_err(Refusal::Invalid)?;
    let (mut s, mut e) = (s, e);
    let mut alignment = exact_alignment(&om, &old, &nm, &nb, s, e, repl.len());
    // A span that cuts a head's line and leaves it unpaired is read by
    // whole lines: the same text change, with the lines it repeats left out
    // (deleting a slide whose neighbour begins alike cuts both slide lines).
    let cut = cut_heads(&old, &om, &alignment, s, e);
    if !cut.is_empty() {
        let (s2, e2, r2) = whole_lines(text, s, e, repl);
        debug_assert_eq!(format!("{}{r2}{}", &text[..s2], &text[e2..]), new_text);
        let al2 = exact_alignment(&om, &old, &nm, &nb, s2, e2, r2.len());
        let gone_whole = |i: usize| {
            let level = old[i].head().map_or(0, |h| h.level);
            let end =
                (i + 1..old.len()).find(|&k| old[k].head().is_some_and(|h| h.level <= level)).unwrap_or(old.len());
            s2 <= om[i].start && om[end - 1].end <= e2
        };
        if cut.iter().all(|&i| al2.bmap[i].is_some() || gone_whole(i)) && cut_heads(&old, &om, &al2, s2, e2).is_empty()
        {
            (s, e, alignment) = (s2, e2, al2);
        }
    }
    let (remainder, mut report, mut outcomes) = reanchor(rem, &old, &nb, &alignment);
    refuse_cut_heads(rem, text, &om, &cut_heads(&old, &om, &alignment, s, e), &mut report, &mut outcomes);
    Ok(Reanchored { text: new_text, old, new: nb, remainder, report, outcomes, alignment })
}

/// Heads (a slide's `layout:` line, a slot's or shape's) the span `s..e`
/// cuts through, part in and part out, that found no new head.
fn cut_heads(old: &[Block], om: &[BlockSrc], al: &Alignment, s: usize, e: usize) -> Vec<usize> {
    (0..old.len())
        .filter(|&i| {
            let m = &om[i];
            matches!(old[i], Block::Head(_))
                && al.bmap[i].is_none()
                && m.start < e
                && s < m.end
                && !(s <= m.start && m.end <= e)
        })
        .collect()
}

/// The edit `text[s..e] → repl` as a span of whole lines, less the lines
/// it starts or ends with unchanged.
fn whole_lines(text: &str, s: usize, e: usize, repl: &str) -> (usize, usize, String) {
    let s0 = text[..s].rfind('\n').map_or(0, |k| k + 1);
    let e0 =
        if e == 0 || text[..e].ends_with('\n') { e } else { text[e..].find('\n').map_or(text.len(), |k| e + k + 1) };
    let old = &text[s0..e0];
    let new = format!("{}{repl}{}", &text[s0..s], &text[e..e0]);
    let (ol, nl): (Vec<&str>, Vec<&str>) = (old.split_inclusive('\n').collect(), new.split_inclusive('\n').collect());
    let pre = ol.iter().zip(&nl).take_while(|(a, b)| a == b).count();
    let suf = ol[pre..].iter().rev().zip(nl[pre..].iter().rev()).take_while(|(a, b)| a == b).count();
    let bytes = |ls: &[&str]| ls.iter().map(|l| l.len()).sum::<usize>();
    let (a, b) = (bytes(&ol[..pre]), bytes(&ol[ol.len() - suf..]));
    let nb = bytes(&nl[nl.len() - suf..]);
    (s0 + a, e0 - b, new[a..new.len() - nb].to_string())
}

/// What the entries of a head in `cut` hold (a slide's shapes without
/// text, its geometry) would go with it, though the text keeps the part of
/// its line the edit left: that is not the edit the text states, so they
/// are refused, not removed.
fn refuse_cut_heads(
    rem: &Remainder,
    text: &str,
    om: &[BlockSrc],
    cut: &[usize],
    report: &mut Report,
    outcomes: &mut HashMap<u64, Outcome>,
) {
    for en in &rem.entries {
        let Some(&i) = en.path.first().filter(|i| cut.contains(i)) else { continue };
        let Some(o) = outcomes.get_mut(&en.id) else { continue };
        if matches!(o.status, Status::Removed(_)) {
            let line = text[om[i].start..om[i].end].trim_end();
            let why = format!(
                "the edit cuts through the line {line:?}: include the whole line in the edit (from the line \
                 before), or leave it out"
            );
            o.status = Status::Refused(why.clone());
            report.removed.retain(|x| x.0 != en.id);
            report.refused.push((en.id, en.kind, why));
        }
    }
}

/// A whole-file rewrite: `old_text` (the revision `rem` belongs to) becomes
/// `new_text`. Blocks are aligned by diff (design C). Refused when an entry
/// cannot be placed.
pub fn rewrite(rem: &Remainder, old_text: &str, new_text: &str, caps: Capabilities) -> Result<Reanchored, Refusal> {
    rewrite_in(&DocumentModel, rem, old_text, new_text, caps)
}

/// [`rewrite`] for a text of grammar `m`.
pub fn rewrite_in(
    m: &dyn TextModel,
    rem: &Remainder,
    old_text: &str,
    new_text: &str,
    caps: Capabilities,
) -> Result<Reanchored, Refusal> {
    refuse_unplaceable(reanchor_rewrite_in(m, rem, old_text, new_text, caps)?)
}

/// An exact edit: the one occurrence of `old` in `text` becomes `new`, and
/// the remainder is re-anchored from that span. Refused when an entry
/// cannot be placed.
pub fn edit(rem: &Remainder, text: &str, old: &str, new: &str, caps: Capabilities) -> Result<Reanchored, Refusal> {
    edit_in(&DocumentModel, rem, text, old, new, caps)
}

/// [`edit`] for a text of grammar `m`.
pub fn edit_in(
    m: &dyn TextModel,
    rem: &Remainder,
    text: &str,
    old: &str,
    new: &str,
    caps: Capabilities,
) -> Result<Reanchored, Refusal> {
    if old.is_empty() {
        return Err(Refusal::Edit("the old text is empty; include the text around the place to edit.".into()));
    }
    // Every occurrence, overlapping ones included ("aa" occurs twice in "aaa").
    let mut hits: Vec<usize> = vec![];
    let mut from = 0;
    while let Some(k) = text[from..].find(old) {
        hits.push(from + k);
        from += k + text[from + k..].chars().next().map_or(1, char::len_utf8);
    }
    let s = match hits.as_slice() {
        [s] => *s,
        [] => {
            let mut m = "the old text was not found; copy it exactly from the current revision.".to_string();
            if let Some(near) = fmt::chars::near_miss(text, old) {
                m.push(' ');
                m.push_str(&near.message);
            }
            return Err(Refusal::Edit(m));
        }
        _ => {
            return Err(Refusal::Edit(format!(
                "the old text occurs {} times; include more of the text around it so it is unique.",
                hits.len()
            )))
        }
    };
    refuse_unplaceable(reanchor_span_in(m, rem, text, s, s + old.len(), new, caps)?)
}

/// Context the edit repeats unchanged is not part of the span: trim a
/// common prefix and suffix, but only at word boundaries (whitespace), so a
/// replaced word stays one replacement.
fn trim_span<'a>(text: &str, s: usize, e: usize, new: &'a str) -> (usize, usize, &'a str) {
    let ws_or_end = |t: &str| t.chars().next().is_none_or(char::is_whitespace);
    let ws_or_start = |t: &str| t.chars().next_back().is_none_or(char::is_whitespace);
    let old = &text[s..e];
    let pre = common_prefix(old, new);
    let pre = if ws_or_end(&old[pre..]) && ws_or_end(&new[pre..]) {
        pre
    } else {
        old[..pre].rfind(char::is_whitespace).map_or(0, |k| k + old[k..].chars().next().unwrap().len_utf8())
    };
    let (o2, n2) = (&old[pre..], &new[pre..]);
    let suf = common_suffix(o2, n2);
    let suf = if ws_or_start(&o2[..o2.len() - suf]) && ws_or_start(&n2[..n2.len() - suf]) {
        suf
    } else {
        o2[o2.len() - suf..].find(char::is_whitespace).map_or(0, |k| suf - k)
    };
    (s + pre, e - suf, &n2[..n2.len() - suf])
}

fn common_prefix(a: &str, b: &str) -> usize {
    a.char_indices()
        .zip(b.chars())
        .find(|((_, x), y)| x != y)
        .map_or(a.len().min(b.len()), |((k, _), _)| k)
        .min(a.len().min(b.len()))
}

fn common_suffix(a: &str, b: &str) -> usize {
    let mut n = 0;
    for (x, y) in a.chars().rev().zip(b.chars().rev()) {
        if x != y {
            break;
        }
        n += x.len_utf8();
    }
    n.min(a.len()).min(b.len())
}

/// Every paragraph position of a resolved text: path, content, source map.
fn para_maps<'a>(
    blocks: &'a [Block],
    maps: &'a [BlockSrc],
) -> Vec<(Path, Option<&'a fmt::Inline>, &'a ParaMap, usize)> {
    let mut out = vec![];
    for (j, (b, m)) in blocks.iter().zip(maps).enumerate() {
        match (b, &m.kind) {
            (Block::Table(_), SrcKind::Table(cells)) => {
                for (r, row) in cells.iter().enumerate() {
                    for (c, pms) in row.iter().enumerate() {
                        for (k, pm) in pms.iter().flatten().enumerate() {
                            out.push((vec![j, r, c, k], model::content_at(blocks, &[j, r, c, k]), pm, j));
                        }
                    }
                }
            }
            (_, SrcKind::Para(pm)) => out.push((vec![j], model::content_at(blocks, &[j]), pm, j)),
            _ => {}
        }
    }
    out
}

/// The alignment an exact span implies: blocks outside it map one to one;
/// units inside the touched blocks map through their source offsets; a
/// paragraph follows its paragraph mark.
fn exact_alignment(
    old_maps: &[BlockSrc],
    ob: &[Block],
    new_maps: &[BlockSrc],
    nb: &[Block],
    s: usize,
    e: usize,
    new_len: usize,
) -> Alignment {
    let shift = |o: usize| -> Option<usize> {
        if o < s {
            Some(o)
        } else if o >= e {
            Some(o + new_len - (e - s))
        } else {
            None
        }
    };
    let new_start: HashMap<usize, usize> = new_maps.iter().enumerate().map(|(j, m)| (m.start, j)).collect();
    let mut bmap = vec![None; ob.len()];
    let mut touched_old = vec![];
    for (i, m) in old_maps.iter().enumerate() {
        let untouched = m.end <= s || m.start >= e;
        match untouched.then(|| shift(m.start).and_then(|x| new_start.get(&x).copied())).flatten() {
            Some(j) => bmap[i] = Some(j),
            None => touched_old.push(i),
        }
    }
    let touched_old: HashSet<usize> = touched_old.into_iter().collect();
    let claimed: HashSet<usize> = bmap.iter().flatten().copied().collect();
    let old_items: Vec<_> = para_maps(ob, old_maps).into_iter().filter(|x| touched_old.contains(&x.3)).collect();
    let new_items: Vec<_> = para_maps(nb, new_maps).into_iter().filter(|x| !claimed.contains(&x.3)).collect();
    let old_stream = Stream::new(old_items.iter().map(|x| (x.0.clone(), x.1.unwrap_or(&EMPTY))));
    let new_stream = Stream::new(new_items.iter().map(|x| (x.0.clone(), x.1.unwrap_or(&EMPTY))));
    // Source offset of every stream position (units, then the mark). A
    // block placeholder's line is a unit in its source map but no content
    // in the stream: only its mark counts, so the offsets stay in step.
    let offsets = |items: &[(Path, Option<&fmt::Inline>, &ParaMap, usize)]| -> Vec<usize> {
        items
            .iter()
            .flat_map(|x| {
                let n = x.1.map_or(0, |c| c.units.len()).min(x.2.units.len());
                x.2.units[..n].iter().copied().chain([x.2.mark])
            })
            .collect()
    };
    let (oo, no) = (offsets(&old_items), offsets(&new_items));
    let at_new: HashMap<usize, usize> = no.iter().enumerate().map(|(k, &o)| (o, k)).collect();
    let mut pairs: Vec<(usize, usize)> = vec![];
    for (k, &o) in oo.iter().enumerate() {
        if let Some(&n) = shift(o).and_then(|x| at_new.get(&x)) {
            // Both positions exist: two past the end are no match.
            let same = matches!((old_stream.text.get(k), new_stream.text.get(n)), (Some(a), Some(b)) if a == b);
            if same && pairs.last().is_none_or(|p| p.1 < n) {
                pairs.push((k, n));
            }
        }
    }
    let ops = pairs_to_ops(&pairs, &old_stream.text, &new_stream.text);
    // A paragraph follows its mark; one whose mark went follows its first surviving unit.
    let loc = |st: &Stream, g: usize| st.locate(g).map(|p| p.0[0]);
    let mut owner_of_new: HashMap<usize, usize> = HashMap::new();
    for &(k, n) in &pairs {
        if old_stream.text[k] == place::SEP && old_stream.char_at(k).is_none() {
            if let (Some(i), Some(j)) = (loc(&old_stream, k), loc(&new_stream, n)) {
                if same_kind(&ob[i], &nb[j]) && bmap[i].is_none() {
                    bmap[i] = Some(j);
                    owner_of_new.insert(j, i);
                }
            }
        }
    }
    for &(k, n) in &pairs {
        if let (Some(i), Some(j)) = (old_stream.char_at(k).map(|p| p.0[0]), new_stream.char_at(n).map(|p| p.0[0])) {
            if bmap[i].is_none()
                && !owner_of_new.contains_key(&j)
                && same_kind(&ob[i], &nb[j])
                && touched_old.contains(&i)
            {
                bmap[i] = Some(j);
                owner_of_new.insert(j, i);
            }
        }
    }
    for (i, b) in ob.iter().enumerate() {
        if let Block::Keep(id) = b {
            bmap[i] = nb.iter().position(|x| matches!(x, Block::Keep(y) if y == id));
        }
    }
    let global =
        (!old_stream.order.is_empty()).then_some(Global { old: old_stream, new: new_stream, ops, min_equal: 1 });
    Alignment { bmap, global, ..Default::default() }
}

/// Matched pairs → opcodes; an unmatched stretch whose units are identical
/// on both sides (a restyle, an emphasis change) counts as equal, and so do
/// its units when the two sides differ only in paragraph marks (a join or a
/// split: the span then also covers syntax the text rewrites around the mark,
/// such as an escape at the start of a line or a `<div>` wrapper).
fn pairs_to_ops(pairs: &[(usize, usize)], a: &[u32], b: &[u32]) -> Vec<Op> {
    let mut ops: Vec<Op> = vec![];
    let push = |ops: &mut Vec<Op>, tag: Tag, i1, i2, j1, j2| {
        if let Some(last) = ops.last_mut() {
            if last.tag == tag && last.i2 == i1 && last.j2 == j1 {
                last.i2 = i2;
                last.j2 = j2;
                return;
            }
        }
        ops.push(Op { tag, i1, i2, j1, j2 });
    };
    let (mut i, mut j) = (0, 0);
    let ends = pairs.iter().copied().chain([(a.len(), b.len())]);
    for (k, n) in ends {
        if k > i || n > j {
            let tag = if a[i..k] == b[j..n] {
                Tag::Equal
            } else if k > i && n > j {
                Tag::Replace
            } else if k > i {
                Tag::Delete
            } else {
                Tag::Insert
            };
            let same_text = || {
                let text = |x: &[u32]| x.iter().filter(|&&u| u != place::SEP).copied().collect::<Vec<_>>();
                text(&a[i..k]) == text(&b[j..n])
            };
            if tag == Tag::Equal && k > i {
                push(&mut ops, tag, i, k, j, n);
            } else if tag == Tag::Replace && same_text() {
                let (mut x, mut y) = (i, j);
                while x < k || y < n {
                    if x < k && a[x] == place::SEP && (y == n || b[y] != place::SEP) {
                        push(&mut ops, Tag::Delete, x, x + 1, y, y);
                        x += 1;
                    } else if y < n && b[y] == place::SEP && (x == k || a[x] != place::SEP) {
                        push(&mut ops, Tag::Insert, x, x, y, y + 1);
                        y += 1;
                    } else {
                        push(&mut ops, Tag::Equal, x, x + 1, y, y + 1);
                        (x, y) = (x + 1, y + 1);
                    }
                }
            } else if tag != Tag::Equal {
                ops.push(Op { tag, i1: i, i2: k, j1: j, j2: n });
            }
        }
        if k < a.len() {
            push(&mut ops, Tag::Equal, k, k + 1, n, n + 1);
        }
        (i, j) = (k + 1, n + 1);
    }
    ops
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Opcodes as (tag, old range, new range).
    fn ops_of(pairs: &[(usize, usize)], a: &str, b: &str) -> Vec<(Tag, usize, usize, usize, usize)> {
        let units = |s: &str| s.chars().map(|c| if c == '|' { place::SEP } else { c as u32 }).collect::<Vec<_>>();
        pairs_to_ops(pairs, &units(a), &units(b)).iter().map(|o| (o.tag, o.i1, o.i2, o.j1, o.j2)).collect()
    }

    #[test]
    fn a_stretch_that_differs_only_in_paragraph_marks_keeps_its_units() {
        // A join ("ab|cd" → "abcd"), nothing paired inside the span.
        assert_eq!(
            ops_of(&[(0, 0), (4, 3)], "ab|cd", "abcd"),
            [(Tag::Equal, 0, 2, 0, 2), (Tag::Delete, 2, 3, 2, 2), (Tag::Equal, 3, 5, 2, 4)]
        );
        // A split, and a paragraph mark that moved.
        assert_eq!(
            ops_of(&[], "abcd", "ab|cd"),
            [(Tag::Equal, 0, 2, 0, 2), (Tag::Insert, 2, 2, 2, 3), (Tag::Equal, 2, 4, 3, 5)]
        );
        assert_eq!(
            ops_of(&[], "a|bc", "ab|c"),
            [
                (Tag::Equal, 0, 1, 0, 1),
                (Tag::Delete, 1, 2, 1, 1),
                (Tag::Equal, 2, 3, 1, 2),
                (Tag::Insert, 3, 3, 2, 3),
                (Tag::Equal, 3, 4, 3, 4)
            ]
        );
        // Other text changes stay one replacement.
        assert_eq!(ops_of(&[], "ab|c", "ax|c"), [(Tag::Replace, 0, 4, 0, 4)]);
    }
    use crate::model::{StyleDef, StyleSet};
    use crate::remainder::{Entry, Meta};

    fn entry(id: u64, kind: Kind, path: Path, start: Option<usize>, end: Option<usize>) -> Entry {
        Entry { id, kind, xml: vec![], fp: format!("fp{id}"), path, start, end, seq: id * 1000, meta: Meta::default() }
    }

    #[test]
    fn a_missing_old_names_the_characters_it_missed() {
        let rem = Remainder { format: "hwpx".into(), ..Default::default() };
        let text = "---\ntype: document\nformat: hwpx\nschema: 1\n---\n□\u{2007}추진 배경\n";
        let e = edit(&rem, text, "□ 추진 배경", "□ 추진 경과", Capabilities::default()).unwrap_err();
        let m = e.to_string();
        assert!(m.contains("matches line 6") && m.contains("\"□⟨U+2007 FIGURE SPACE⟩추진 배경\""), "{m}");
        // A refusal quoting text with a debug escape names the character too.
        let r = Refusal::Edit("the edit cuts through the line \"a\\u{f076}b\"".into());
        assert!(r.to_string().contains("\"a⟨U+F076 private use⟩b\""), "{r}");
    }

    #[test]
    fn footnote_definitions_do_not_shift_later_blocks() {
        let styles = StyleSet {
            paragraph: vec![StyleDef { id: "Normal".into(), name: "Normal".into() }],
            default_paragraph: "Normal".into(),
            ..Default::default()
        };
        // Resolved blocks: [a[^1], middle, last para]; the definition is not a block.
        let entries = vec![
            entry(1, Kind::Ppr, vec![0], None, None),
            entry(2, Kind::Ppr, vec![2], None, None),
            entry(3, Kind::Run, vec![2], Some(0), Some(9)),
            entry(4, Kind::Ppr, vec![1], None, None),
        ];
        let rem = Remainder { format: "docx".into(), styles, entries, next_id: 5, ..Default::default() };
        let text = "---\ntype: document\nformat: docx\nschema: 1\n---\na[^1]\n\n[^1]: note\n\nmiddle\n\nlast para\n";
        let caps = Capabilities { footnotes: true, ..Default::default() };
        let at = |r: &Reanchored, id| {
            assert!(r.report.refused.is_empty() && r.report.removed.is_empty(), "{:?}", r.report);
            r.remainder.entries.iter().find(|e| e.id == id).map(|e| (e.path.clone(), e.start, e.end))
        };
        // Edits after the definition, as an exact span and as a rewrite.
        for r in [edit(&rem, text, "last", "final", caps), rewrite(&rem, text, &text.replace("last", "final"), caps)] {
            let r = r.unwrap();
            assert_eq!(at(&r, 2), Some((vec![2], None, None)));
            assert_eq!(at(&r, 3), Some((vec![2], Some(0), Some(10))));
            assert_eq!(at(&r, 4), Some((vec![1], None, None)));
        }
        // An edit before it: every block after the definition is untouched.
        let r = edit(&rem, text, "a[^1]", "b[^1]", caps).unwrap();
        assert_eq!(at(&r, 3), Some((vec![2], Some(0), Some(9))));
        assert_eq!(at(&r, 4), Some((vec![1], None, None)));
    }
}
