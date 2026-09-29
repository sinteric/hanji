//! List numbering at export (§5.2): which list definition and level each
//! list item of the text takes. Engine-neutral: an engine names its list
//! definitions by number and answers [`ListDefs`] about them.
//!
//! The numbering an item had at import is stored on its paragraph's `Ppr`
//! entry: `meta.item` and, in `meta.aux`, the list number then the level.

use std::collections::HashMap;

use crate::model::{Block, ListItem};
use crate::remainder::{Entry, Kind};

/// What a top-level paragraph's numbering becomes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListPlan {
    /// A list item as imported: its numbering stays as it is.
    Keep,
    /// A list item that takes this numbering (a new item, or a changed level
    /// or kind).
    Set { num: u32, ilvl: u32 },
    /// A former list item, now a paragraph: its numbering goes.
    Strip,
}

/// An engine's list definitions.
pub trait ListDefs {
    /// `Some(true)` for a numbered level of list `num`, `Some(false)` for a
    /// bullet, `None` when the level shows no marker or is not defined.
    fn ordered(&self, num: u32, ilvl: u32) -> Option<bool>;
    /// The file's default bullet (`ordered: false`) or decimal list: its
    /// number, and the definition a new list is made from.
    fn default_list(&self, ordered: bool) -> Option<(u32, u32)>;
    /// A new list over definition `base` that starts again at 1.
    fn new_list(&mut self, base: u32) -> u32;
}

/// The numbering stored for the paragraph at `[bi]`: its item at import, list and level.
pub fn stored_item(entries: &HashMap<usize, &Entry>, bi: usize) -> Option<(ListItem, u32, u32)> {
    let e = entries.get(&bi)?;
    let n = |k: usize| e.meta.aux.get(k).and_then(|v| v.parse().ok());
    Some((e.meta.item?, n(0)?, n(1)?))
}

/// Numbering for every list item and former list item. An item keeps its
/// own numbering (its level moved by the text's change of level); a new one
/// takes its nearest sibling's, at its level or, one level off, the same
/// list's; a new list takes the file's default bullet or decimal list, a
/// decimal one restarting at 1.
pub fn plan_lists(
    blocks: &[Block],
    entries: &[Entry],
    defs: &mut dyn ListDefs,
) -> Result<HashMap<usize, ListPlan>, String> {
    let mut ppr: HashMap<usize, &Entry> = HashMap::new();
    for e in entries.iter().filter(|e| e.kind == Kind::Ppr && e.path.len() == 1) {
        ppr.entry(e.path[0]).or_insert(e);
    }
    let item = |bi: usize| match &blocks[bi] {
        Block::Para(p) => p.item,
        _ => None,
    };
    let mut plan = HashMap::new();
    let mut assigned: HashMap<usize, (u32, u32)> = HashMap::new();
    let mut pending = vec![];
    for bi in 0..blocks.len() {
        let stored = stored_item(&ppr, bi);
        let Some(it) = item(bi) else {
            if stored.is_some() {
                plan.insert(bi, ListPlan::Strip);
            }
            continue;
        };
        let kept = stored.filter(|s| s.0.ordered == it.ordered).and_then(|(si, num, ilvl)| {
            let nl = ilvl as i64 + it.level as i64 - si.level as i64;
            let nl = u32::try_from(nl).ok().filter(|&l| l <= 8)?;
            (defs.ordered(num, nl) == Some(it.ordered)).then_some((num, nl, nl == ilvl))
        });
        match kept {
            Some((num, ilvl, same)) => {
                assigned.insert(bi, (num, ilvl));
                plan.insert(bi, if same { ListPlan::Keep } else { ListPlan::Set { num, ilvl } });
            }
            None => pending.push(bi),
        }
    }
    // The text list an item belongs to: from its first item to the next first.
    let list_of = |bi: usize| {
        let lo = (0..=bi).rev().find(|&k| item(k).is_none_or(|i| i.first)).map_or(0, |k| {
            if item(k).is_some() {
                k
            } else {
                k + 1
            }
        });
        let hi = (bi + 1..blocks.len()).find(|&k| item(k).is_none_or(|i| i.first)).unwrap_or(blocks.len());
        (lo, hi)
    };
    let mut new_lists: HashMap<usize, u32> = HashMap::new();
    while !pending.is_empty() {
        let mut progress = false;
        pending.retain(|&bi| {
            let it = item(bi).unwrap();
            let (lo, hi) = list_of(bi);
            let mut near: Vec<usize> = (lo..hi).filter(|k| assigned.contains_key(k)).collect();
            near.sort_by_key(|&k| (k.abs_diff(bi), k));
            let pick = near.iter().find_map(|&k| {
                let (num, ilvl) = assigned[&k];
                let sib = item(k).unwrap();
                let nl = u32::try_from(ilvl as i64 + it.level as i64 - sib.level as i64).ok()?;
                (sib.ordered == it.ordered && (sib.level == it.level || defs.ordered(num, nl) == Some(it.ordered)))
                    .then_some((num, nl))
            });
            match pick {
                Some((num, ilvl)) => {
                    assigned.insert(bi, (num, ilvl));
                    plan.insert(bi, ListPlan::Set { num, ilvl });
                    progress = true;
                    false
                }
                None => true,
            }
        });
        if progress || pending.is_empty() {
            continue;
        }
        // No sibling anywhere: the first pending item starts from the file's default list.
        let bi = pending.remove(0);
        let it = item(bi).unwrap();
        let what = if it.ordered { "numbered" } else { "bulleted" };
        let (num, base) = defs.default_list(it.ordered).ok_or_else(|| {
            format!("this file has no {what} list to take numbering from, so the new list item cannot be written; write it as a paragraph, or add it next to an existing {what} item")
        })?;
        let num = if it.ordered { *new_lists.entry(list_of(bi).0).or_insert_with(|| defs.new_list(base)) } else { num };
        let ilvl = it.level as u32;
        assigned.insert(bi, (num, ilvl));
        plan.insert(bi, ListPlan::Set { num, ilvl });
    }
    Ok(plan)
}
