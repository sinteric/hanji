//! `word/numbering.xml` (and the numbering carried by paragraph styles):
//! which list paragraphs are bullets and which are numbered, and the list
//! definitions a new list can take its numbering from.

use std::collections::HashMap;

use hanji_core::Part;

use crate::package;
use crate::xml::{self, Element, Node};

pub const NUMBERING_PART: &str = "word/numbering.xml";

type StyleNum = (Option<u32>, Option<u32>, Option<String>);

#[derive(Default)]
pub struct Numbering {
    doc: Option<xml::Doc>,
    /// numId → abstractNumId.
    nums: HashMap<u32, u32>,
    /// (numId, ilvl) → numFmt from a `w:lvlOverride`.
    overrides: HashMap<(u32, u32), String>,
    /// (abstractNumId, ilvl) → numFmt.
    formats: HashMap<(u32, u32), String>,
    /// abstractNumId → the numbering style it links to (`w:numStyleLink`).
    style_links: HashMap<u32, String>,
    /// Paragraph or numbering style id → (numId, ilvl) from its `w:numPr`,
    /// through `w:basedOn`.
    style_num: HashMap<String, (u32, u32)>,
    /// Paragraph style id of "List Paragraph", if the file has it.
    pub list_paragraph: Option<String>,
    added: Vec<(u32, u32)>,
}

fn val(e: &Element, child: &str) -> Option<u32> {
    e.child(child).and_then(|c| c.get("w:val")).and_then(|v| v.parse().ok())
}

/// `w:numPr` → (numId, ilvl); either may be missing.
pub fn num_pr(ppr: Option<&Element>) -> (Option<u32>, Option<u32>) {
    let np = ppr.and_then(|p| p.child("w:numPr"));
    (np.and_then(|n| val(n, "w:numId")), np.and_then(|n| val(n, "w:ilvl")))
}

impl Numbering {
    pub fn read(parts: &[Part]) -> Numbering {
        let mut n = Numbering::default();
        if let Some(d) = package::get(parts, NUMBERING_PART).and_then(|d| xml::parse(d).ok()) {
            for e in d.root.elements() {
                let id = |k: &str| e.get(k).and_then(|v| v.parse::<u32>().ok());
                if e.is("w:abstractNum") {
                    let Some(a) = id("w:abstractNumId") else { continue };
                    if let Some(s) = e.child("w:numStyleLink").and_then(|s| s.get("w:val")) {
                        n.style_links.insert(a, s);
                    }
                    for l in e.elements().filter(|l| l.is("w:lvl")) {
                        let (Some(k), Some(f)) = (l.get("w:ilvl").and_then(|v| v.parse().ok()), l.child("w:numFmt"))
                        else {
                            continue;
                        };
                        n.formats.insert((a, k), f.get("w:val").unwrap_or_default());
                    }
                } else if e.is("w:num") {
                    let (Some(num), Some(a)) = (id("w:numId"), val(e, "w:abstractNumId")) else { continue };
                    n.nums.insert(num, a);
                    for o in e.elements().filter(|o| o.is("w:lvlOverride")) {
                        let k = o.get("w:ilvl").and_then(|v| v.parse().ok()).unwrap_or(0);
                        if let Some(f) = o.child("w:lvl").and_then(|l| l.child("w:numFmt")) {
                            n.overrides.insert((num, k), f.get("w:val").unwrap_or_default());
                        }
                    }
                }
            }
            n.doc = Some(d);
        }
        if let Some(d) = package::get(parts, "word/styles.xml").and_then(|d| xml::parse(d).ok()) {
            // Style id → its own (numId, ilvl) and the style it is based on.
            let mut own: HashMap<String, StyleNum> = HashMap::new();
            for st in d.root.elements().filter(|e| e.is("w:style")) {
                let Some(id) = st.get("w:styleId") else { continue };
                if st.child("w:name").and_then(|x| x.get("w:val")).as_deref() == Some("List Paragraph") {
                    n.list_paragraph = Some(id.clone());
                }
                let (num, lvl) = num_pr(st.child("w:pPr"));
                own.insert(id, (num, lvl, st.child("w:basedOn").and_then(|b| b.get("w:val"))));
            }
            for id in own.keys() {
                // Follow basedOn (bounded) for the first style with a numId.
                let (mut cur, mut lvl) = (Some(id.clone()), None);
                for _ in 0..16 {
                    let Some((num, l, base)) = cur.as_ref().and_then(|c| own.get(c)) else { break };
                    lvl = lvl.or(*l);
                    if let Some(num) = num {
                        n.style_num.insert(id.clone(), (*num, lvl.unwrap_or(0)));
                        break;
                    }
                    cur = base.clone();
                }
            }
        }
        n
    }

    /// The numbering a paragraph style gives its paragraphs.
    pub fn of_style(&self, style_id: &str) -> Option<(u32, u32)> {
        self.style_num.get(style_id).copied()
    }

    fn format(&self, num: u32, ilvl: u32) -> Option<&str> {
        if let Some(f) = self.overrides.get(&(num, ilvl)) {
            return Some(f);
        }
        let mut a = *self.nums.get(&num)?;
        for _ in 0..4 {
            if let Some(f) = self.formats.get(&(a, ilvl)) {
                return Some(f);
            }
            // A list that links to a numbering style takes that style's definition.
            let (linked, _) = self.of_style(self.style_links.get(&a)?)?;
            a = *self.nums.get(&linked)?;
        }
        None
    }

    /// `Some(true)` for a numbered level, `Some(false)` for a bullet, `None`
    /// when the level shows no marker or is not defined.
    pub fn ordered(&self, num: u32, ilvl: u32) -> Option<bool> {
        match self.format(num, ilvl)? {
            "none" | "" => None,
            "bullet" => Some(false),
            _ => Some(true),
        }
    }

    /// The file's first list whose top level is a bullet (`ordered: false`)
    /// or decimal: (numId, abstractNumId).
    pub fn default_list(&self, ordered: bool) -> Option<(u32, u32)> {
        let mut nums: Vec<(&u32, &u32)> = self.nums.iter().collect();
        nums.sort();
        let want = if ordered { "decimal" } else { "bullet" };
        nums.into_iter().find(|(n, _)| self.format(**n, 0) == Some(want)).map(|(n, a)| (*n, *a))
    }

    /// A new list over `abstract_id` that starts again at 1.
    pub fn new_list(&mut self, abstract_id: u32) -> u32 {
        let id = self.nums.keys().chain(self.added.iter().map(|a| &a.0)).max().map_or(1, |m| m + 1);
        self.added.push((id, abstract_id));
        self.nums.insert(id, abstract_id);
        id
    }

    /// `numbering.xml` with the new lists, if any were added.
    pub fn part(&self) -> Option<Vec<u8>> {
        if self.added.is_empty() {
            return None;
        }
        let mut d = self.doc.clone()?;
        let at = d
            .root
            .children
            .iter()
            .position(|c| matches!(c, Node::El(e) if e.is("w:numIdMacAtCleanup")))
            .unwrap_or(d.root.children.len());
        for (k, (id, a)) in self.added.iter().enumerate() {
            let mut num = Element::new("w:num").with_attr("w:numId", &id.to_string());
            num.children.push(Node::El(Element::new("w:abstractNumId").with_attr("w:val", &a.to_string())));
            let mut o = Element::new("w:lvlOverride").with_attr("w:ilvl", "0");
            o.children.push(Node::El(Element::new("w:startOverride").with_attr("w:val", "1")));
            num.children.push(Node::El(o));
            d.root.children.insert(at + k, Node::El(num));
        }
        Some(xml::write_doc(&d))
    }
}
