//! The text and fill of a group's shapes (DESIGN.md §5.3). A group is one
//! remainder entry, its objects inside it: their text is read from the
//! group's XML as a slide shape's is (marks, formatting, fill), and a
//! changed paragraph is rewritten in place, each new character taking the
//! run of the old character it aligns with.

use hanji_format::inline_style::{self as istyle, TextStyle};
use hanji_format::look::Look;
use hanji_format::{Atom, Inline, Marks, Unit};
use hanji_package::xml::{self, Element, Node};

use crate::deck::LayoutInfo;
use crate::effects;
use crate::fill::{self, ThemeFills};
use crate::kind;
use crate::outline;
use crate::pml::*;
use crate::text::{self, RunStyle, ThemeFonts};

/// What a group's shapes inherit: the text formatting of shapes that are
/// not placeholders, the theme's fonts and fill styles.
#[derive(Clone, Copy)]
pub struct Styling<'a> {
    pub text: &'a [RunStyle; 9],
    pub fonts: &'a ThemeFonts,
    pub fills: &'a ThemeFills,
}

impl<'a> Styling<'a> {
    pub fn of(l: &'a LayoutInfo) -> Styling<'a> {
        Styling { text: &l.other_text, fonts: &l.fonts, fills: &l.fills }
    }
}

/// The level (0–8) of paragraph `p`.
fn level(p: &Element) -> usize {
    p.child("a:pPr").and_then(|x| x.get("lvl")).and_then(|v| v.parse().ok()).unwrap_or(0usize).min(8)
}

/// The preset shape, fill and outline a group's shape shows.
pub fn look(sp: &Element, sty: Option<Styling>) -> Look {
    let (kind, adj) = kind::shown(sp, None, false);
    let Some(s) = sty else { return Look { kind, adj, ..Default::default() } };
    Look {
        kind,
        adj,
        fill: fill::shown(fill::effective(sp, None, s.fills).as_ref()),
        effects: effects::shown(effects::effective(sp, None, s.fills).as_ref()),
        ..outline::effective(sp, None, s.fills).look(false)
    }
}

/// The kind, outline and arrowheads a group's line shows.
pub fn line_look(c: &Element, sty: Option<Styling>) -> Look {
    let (kind, adj) = kind::shown(c, None, true);
    let l = sty.map_or_else(Look::default, |s| Look {
        effects: effects::shown(effects::effective(c, None, s.fills).as_ref()),
        ..outline::effective(c, None, s.fills).look(true)
    });
    Look { kind, adj, ..l }
}

/// One source unit of a paragraph: its atom, marks and the run (`a:rPr`) it
/// comes from.
struct Src {
    unit: Unit,
    rpr: Option<Element>,
}

/// A paragraph's units as the text reads them, each with its run's `a:rPr`.
fn sources(p: &Element) -> Vec<Src> {
    let mut out = vec![];
    for c in p.elements() {
        match c.name.as_str() {
            "a:r" | "a:fld" => {
                let rpr = c.child("a:rPr").cloned();
                let m = marks_of(rpr.as_ref());
                for ch in c.text_of(&["a:t"]).chars().filter(|ch| *ch as u32 >= 0x20) {
                    out.push(Src { unit: Unit::new(Atom::Char(ch), m), rpr: rpr.clone() });
                }
            }
            "a:br" => out.push(Src { unit: Unit::new(Atom::Break, Marks::NONE), rpr: c.child("a:rPr").cloned() }),
            _ => {}
        }
    }
    out
}

/// A group shape's paragraphs as the text shows them: runs and fields as
/// their text with their marks, line breaks, and (with `sty`) each run's
/// font, size and colour; none when it has no text.
pub fn paras(sp: &Element, sty: Option<Styling>) -> Vec<Inline> {
    let Some(tx) = sp.child("p:txBody").filter(|_| crate::import::has_text(sp)) else { return vec![] };
    let base = sty.map(|s| text::base(sp, s.text, s.fonts));
    let mut out: Vec<Inline> = tx
        .elements()
        .filter(|p| p.is("a:p"))
        .map(|p| {
            let src = sources(p);
            let mut i = Inline { units: src.iter().map(|s| s.unit.clone()).collect(), spans: vec![] };
            if i.units.iter().all(|u| matches!(u.atom, Atom::Char(c) if c.is_whitespace())) {
                return Inline::default();
            }
            if let (Some(b), Some(s)) = (&base, sty) {
                let under = &b[level(p)];
                let st: Vec<TextStyle> = src
                    .iter()
                    .map(|x| {
                        x.rpr.as_ref().map_or_else(RunStyle::default, |r| RunStyle::of(r, s.fonts)).over(under).shown()
                    })
                    .collect();
                istyle::set_unit_styles(&mut i, &st);
            }
            i
        })
        .collect();
    if sty.is_some() {
        let mut refs: Vec<&mut Inline> = out.iter_mut().collect();
        istyle::normalize(&mut refs);
    }
    for i in &mut out {
        i.normalize();
    }
    out
}

/// Write `new` (a group shape's paragraphs as the text writes them) into
/// `sp`, whose text read as `old`: an unchanged paragraph keeps its XML;
/// a changed one keeps its `a:pPr` and `a:endParaRPr`, and each character
/// takes the run of the old character it aligns with (marks and formatting
/// set where the text changes them). New paragraphs follow the last one.
pub fn write(sp: &mut Element, old: &[Inline], new: &[Inline], sty: Option<Styling>, what: &str) -> Result<(), String> {
    if old == new {
        return Ok(());
    }
    let base = sty.map(|s| text::base(sp, s.text, s.fonts));
    let Some(tx) = sp.child_mut("p:txBody") else {
        return Err(format!("{what} has no text body: its text cannot be written here"));
    };
    let ps: Vec<Element> = tx.elements().filter(|p| p.is("a:p")).cloned().collect();
    let empty = Inline::default();
    let mut out: Vec<Element> = vec![];
    for (k, n) in new.iter().enumerate() {
        let o = old.get(k).unwrap_or(&empty);
        let p = ps.get(k).or(ps.last()).cloned().unwrap_or_else(|| Element::new("a:p"));
        if k < ps.len() && o == n {
            out.push(p);
            continue;
        }
        if let Some(c) = p.elements().find(|c| !matches!(c.name.as_str(), "a:pPr" | "a:r" | "a:br" | "a:endParaRPr")) {
            return Err(format!(
                "a paragraph of {what} holds {} the text does not show: its text cannot be written here; edit it in PowerPoint",
                c.name
            ));
        }
        let under = base.as_ref().map(|b| &b[level(&p)]);
        out.push(paragraph(&p, if k < ps.len() { o } else { &empty }, n, under, sty.map(|s| s.fonts))?);
    }
    // Paragraphs of the old text that the new one no longer has go; a shape keeps one.
    if out.is_empty() {
        let mut p = ps.first().cloned().unwrap_or_else(|| Element::new("a:p"));
        p.children.retain(|c| matches!(c, Node::El(e) if e.is("a:pPr") || e.is("a:endParaRPr")));
        out.push(p);
    }
    let first = tx.children.iter().position(|c| matches!(c, Node::El(e) if e.is("a:p"))).unwrap_or(tx.children.len());
    tx.children.retain(|c| !matches!(c, Node::El(e) if e.is("a:p")));
    let at = first.min(tx.children.len());
    for (j, p) in out.into_iter().enumerate() {
        tx.children.insert(at + j, Node::El(p));
    }
    Ok(())
}

/// Paragraph `p` (which read as `o`) rewritten to read as `n`.
fn paragraph(
    p: &Element,
    o: &Inline,
    n: &Inline,
    under: Option<&RunStyle>,
    fonts: Option<&ThemeFonts>,
) -> Result<Element, String> {
    let src = sources(p);
    // The old text's units map onto the paragraph's runs only when they are the same units.
    let mapped = src.len() == o.units.len() && src.iter().zip(&o.units).all(|(s, u)| s.unit.atom == u.atom);
    let first = src.first().and_then(|s| s.rpr.clone()).or_else(|| {
        p.child("a:endParaRPr").map(|e| {
            let mut r = e.clone();
            r.name = "a:rPr".into();
            r
        })
    });
    // Each new unit's template run: the old unit it aligns with.
    let pairs = if mapped { align(&o.units, &n.units) } else { vec![] };
    let mut tpl: Vec<Option<Element>> = vec![None; n.units.len()];
    for &(a, b) in &pairs {
        tpl[b] = src[a].rpr.clone();
    }
    let aligned: Vec<bool> = {
        let mut v = vec![false; n.units.len()];
        for &(_, b) in &pairs {
            v[b] = true;
        }
        v
    };
    for k in 0..tpl.len() {
        if !aligned[k] {
            tpl[k] = (0..k)
                .rev()
                .find(|&j| aligned[j])
                .or_else(|| (k + 1..tpl.len()).find(|&j| aligned[j]))
                .map_or(first.clone(), |j| tpl[j].clone());
        }
    }
    let lifted = istyle::lift(&[n]).2.remove(0);
    let marks = lifted.written_marks(&|k| marks_of(tpl[k].as_ref()));
    let want = istyle::unit_styles(n);
    let mut out = Element::new("a:p");
    out.attrs = p.attrs.clone();
    if let Some(ppr) = p.child("a:pPr") {
        out.children.push(Node::El(ppr.clone()));
    }
    let mut k = 0;
    while k < n.units.len() {
        let mut rpr = tpl[k].clone().unwrap_or_else(|| Element::new("a:rPr"));
        crate::export::set_marks(&mut rpr, marks[k]);
        if let (Some(u), Some(f)) = (under, fonts) {
            let w = want[k].over(&u.shown());
            let had = RunStyle::of(&rpr, f).over(u).shown();
            if istyle::visible(&n.units[k].atom) && had != w {
                text::write(&mut rpr, &w, &had, u)?;
            }
        }
        if n.units[k].atom == Atom::Break {
            let mut br = Element::new("a:br");
            br.children.push(Node::El(rpr));
            out.children.push(Node::El(br));
            k += 1;
            continue;
        }
        let a = k;
        let mut t = String::new();
        while k < n.units.len() {
            let Atom::Char(c) = n.units[k].atom else { break };
            let same = k == a
                || (tpl[k] == tpl[a]
                    && marks[k] == marks[a]
                    && (want[k] == want[a] || !istyle::visible(&n.units[k].atom)));
            if !same {
                break;
            }
            t.push(c);
            k += 1;
        }
        if t.is_empty() {
            // A placeholder or other unit a group's text cannot hold.
            return Err("a group's shape holds only text and line breaks here".into());
        }
        let mut r = Element::new("a:r");
        r.children.push(Node::El(rpr));
        let mut te = Element::new("a:t");
        te.children.push(Node::Text(xml::escape_text(&t)));
        r.children.push(Node::El(te));
        out.children.push(Node::El(r));
    }
    if let Some(end) = p.child("a:endParaRPr") {
        out.children.push(Node::El(end.clone()));
    }
    Ok(out)
}

/// Matched positions of two unit sequences (longest common subsequence of
/// their atoms), in order.
fn align(a: &[Unit], b: &[Unit]) -> Vec<(usize, usize)> {
    let (n, m) = (a.len(), b.len());
    let mut t = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            t[i][j] = if a[i].atom == b[j].atom { t[i + 1][j + 1] + 1 } else { t[i + 1][j].max(t[i][j + 1]) };
        }
    }
    let (mut i, mut j, mut out) = (0, 0, vec![]);
    while i < n && j < m {
        if a[i].atom == b[j].atom {
            out.push((i, j));
            i += 1;
            j += 1;
        } else if t[i + 1][j] >= t[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

/// A group without what the text shows of its shapes (their paragraphs and
/// fills), nested groups too: what the group's fingerprint holds.
pub fn without_shown(group: &Element) -> Element {
    let mut g = group.clone();
    for c in g.elements_mut() {
        match c.name.as_str() {
            "p:cxnSp" => *c = effects::without_effects(&kind::without_kind(&outline::without_outline(c))),
            "p:sp" => {
                *c = effects::without_effects(&kind::without_kind(&outline::without_outline(&fill::without_fill(c))));
                if let Some(tx) = c.child_mut("p:txBody") {
                    tx.children.retain(|n| !matches!(n, Node::El(e) if e.is("a:p")));
                }
            }
            "p:grpSp" => *c = without_shown(c),
            _ => {}
        }
    }
    g
}
