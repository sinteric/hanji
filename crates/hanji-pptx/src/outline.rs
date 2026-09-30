//! Outlines (DESIGN.md §5.3): the line a slot, shape or connector draws, as
//! `border="<width>pt <style> <colour>"`, and a line's arrowheads as
//! `start=` and `end=`. Read property by property from its own `a:ln`, else
//! its `p:style` line reference into the theme's line styles, else (a
//! placeholder) its layout's and master's placeholder; written back into
//! its own `a:ln` only where the text changes them.

use hanji_format::look::{self, Look};
use hanji_format::vocab::{Border, Color};
use hanji_package::xml::{fragment, insert_ordered, Element, Node};

use crate::fill::ThemeFills;
use crate::text;

const FILLS: &[&str] = &["a:noFill", "a:solidFill", "a:gradFill", "a:pattFill"];

/// `a:ln`'s children in schema order (local names).
const LN_ORDER: &[&str] = &[
    "noFill",
    "solidFill",
    "gradFill",
    "pattFill",
    "prstDash",
    "custDash",
    "round",
    "bevel",
    "miter",
    "headEnd",
    "tailEnd",
    "extLst",
];

/// `p:spPr`'s children in schema order (local names).
const SPPR_ORDER: &[&str] = &[
    "xfrm",
    "custGeom",
    "prstGeom",
    "noFill",
    "solidFill",
    "gradFill",
    "blipFill",
    "pattFill",
    "grpFill",
    "ln",
    "effectLst",
    "effectDag",
    "scene3d",
    "sp3d",
    "extLst",
];

/// EMU per hundredth of a point.
const EMU_PER_CENTI_PT: i64 = 127;

/// What an `a:ln` sets of what the text shows, each property on its own.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Ln {
    /// Width in EMU.
    pub w: Option<i64>,
    /// The fill element's XML.
    pub fill: Option<String>,
    /// `a:prstDash`'s value, or `custom` for `a:custDash`.
    pub dash: Option<String>,
    /// `cmpd`: `sng`, `dbl`, `thickThin`, `thinThick`, `tri`.
    pub cmpd: Option<String>,
    pub head: Option<String>,
    pub tail: Option<String>,
}

impl Ln {
    pub fn of(ln: &Element) -> Ln {
        Ln {
            w: ln.get("w").and_then(|v| v.trim().parse().ok()),
            fill: ln.elements().find(|e| FILLS.contains(&e.name.as_str())).map(Element::to_xml),
            dash: ln
                .child("a:prstDash")
                .and_then(|d| d.get("val"))
                .or_else(|| ln.child("a:custDash").map(|_| "custom".to_string())),
            cmpd: ln.get("cmpd"),
            head: ln.child("a:headEnd").and_then(|e| e.get("type")),
            tail: ln.child("a:tailEnd").and_then(|e| e.get("type")),
        }
    }

    /// `self`'s values, and `under`'s where `self` has none.
    pub fn over(&self, under: &Ln) -> Ln {
        Ln {
            w: self.w.or(under.w),
            fill: self.fill.clone().or_else(|| under.fill.clone()),
            dash: self.dash.clone().or_else(|| under.dash.clone()),
            cmpd: self.cmpd.clone().or_else(|| under.cmpd.clone()),
            head: self.head.clone().or_else(|| under.head.clone()),
            tail: self.tail.clone().or_else(|| under.tail.clone()),
        }
    }

    /// The outline as the text shows it: `None` for no outline, and for one
    /// the text cannot show (a gradient or pattern line).
    pub fn border(&self) -> Option<String> {
        let f = fragment(self.fill.as_ref()?);
        let color = match f.name.as_str() {
            "a:solidFill" => text::color_of(&f)?,
            _ => return None,
        };
        let style = match self.cmpd.as_deref() {
            Some("dbl") => "double",
            Some("tri") => "triple",
            Some("thickThin") => "thick-thin",
            Some("thinThick") => "thin-thick",
            _ => match self.dash.as_deref() {
                None | Some("solid") => "solid",
                Some("dot" | "sysDot") => "dotted",
                Some("dashDot" | "lgDashDot" | "sysDashDot") => "dash-dot",
                Some("lgDashDotDot" | "sysDashDotDot") => "dash-dot-dot",
                _ => "dashed",
            },
        };
        // 0.75 pt when nothing sets it, as PowerPoint draws it.
        let w = self.w.unwrap_or(9525);
        let width = ((w as f64) / EMU_PER_CENTI_PT as f64).round().max(1.0) as i64;
        Some(Border::Line { width, style: style.into(), color: Color::parse(&color).ok()? }.to_string())
    }

    fn arrow(v: &Option<String>) -> Option<String> {
        v.clone().filter(|t| t != "none" && look::ARROWS.contains(&t.as_str()))
    }

    /// The look this outline gives: its border, and for a line its arrowheads.
    pub fn look(&self, line: bool) -> Look {
        Look {
            border: self.border(),
            start: if line { Ln::arrow(&self.head) } else { None },
            end: if line { Ln::arrow(&self.tail) } else { None },
            ..Default::default()
        }
    }
}

/// The line `el`'s `p:style` gives: its `a:lnRef` style from the theme, the
/// style's `phClr` the reference's colour.
pub fn from_style(el: &Element, theme: &ThemeFills) -> Option<Ln> {
    let r = el.child("p:style")?.child("a:lnRef")?;
    let idx: usize = r.get("idx")?.trim().parse().ok()?;
    if idx == 0 {
        return Some(Ln { fill: Some("<a:noFill/>".into()), ..Default::default() });
    }
    let mut ln = fragment(theme.lines.get(idx - 1)?);
    if let Some(c) = r.elements().next() {
        crate::fill::with_color(&mut ln, c);
    }
    Some(Ln::of(&ln))
}

/// `el`'s own `a:ln`.
pub fn own(el: &Element) -> Option<Ln> {
    el.child("p:spPr")?.child("a:ln").map(Ln::of)
}

/// What `el` draws without its own `a:ln`: its style's, over `parent`.
pub fn inherited(el: &Element, parent: Option<&Ln>, theme: &ThemeFills) -> Ln {
    let p = parent.cloned().unwrap_or_default();
    from_style(el, theme).map_or(p.clone(), |s| s.over(&p))
}

/// The outline `el` draws.
pub fn effective(el: &Element, parent: Option<&Ln>, theme: &ThemeFills) -> Ln {
    let i = inherited(el, parent, theme);
    own(el).map_or(i.clone(), |o| o.over(&i))
}

/// `el` without its own outline: what its fingerprint holds.
pub fn without_outline(el: &Element) -> Element {
    let mut e = el.clone();
    if let Some(sp) = e.child_mut("p:spPr") {
        sp.children.retain(|n| !matches!(n, Node::El(c) if c.is("a:ln")));
    }
    e
}

/// The parts of a canonical border: width (hundredths of a point), style, colour.
fn parts(b: &str) -> Option<(i64, String, String)> {
    match Border::parse(b).ok()? {
        Border::Line { width, style, color } => Some((width, style, color.to_string())),
        Border::None => None,
    }
}

fn dash_of(style: &str) -> &'static str {
    match style {
        "dashed" => "dash",
        "dotted" => "sysDot",
        "dash-dot" => "dashDot",
        "dash-dot-dot" => "lgDashDotDot",
        _ => "solid",
    }
}

/// Write the outline and arrowheads the text shows for `el` (`want`) where
/// they differ from what `el` draws now: each changed property (width,
/// style, colour, an arrowhead) is set in `el`'s own `a:ln`, the others
/// kept; no border is `a:noFill`. A style or colour the text cannot write
/// (`triple`, `accent1*`) is refused.
pub fn write(el: &mut Element, want: &Look, parent: Option<&Ln>, theme: &ThemeFills, line: bool) -> Result<(), String> {
    let now = effective(el, parent, theme);
    let had = now.look(line);
    if had.border == want.border && had.start == want.start && had.end == want.end {
        return Ok(());
    }
    // Taken back to what the style or layout gives: the element's own outline goes.
    let base = inherited(el, parent, theme).look(line);
    if own(el).is_some() && base.border == want.border && base.start == want.start && base.end == want.end {
        if let Some(sp) = el.child_mut("p:spPr") {
            sp.children.retain(|n| !matches!(n, Node::El(c) if c.is("a:ln")));
        }
        return Ok(());
    }
    if el.child("p:spPr").is_none() {
        let at = el
            .children
            .iter()
            .position(|n| matches!(n, Node::El(c) if c.name.starts_with("p:nv")))
            .map_or(0, |k| k + 1);
        el.children.insert(at, Node::El(Element::new("p:spPr")));
    }
    let sp = el.child_mut("p:spPr").expect("p:spPr");
    if sp.child("a:ln").is_none() {
        insert_ordered(sp, Element::new("a:ln"), SPPR_ORDER);
    }
    let ln = sp.child_mut("a:ln").expect("a:ln");
    if had.border != want.border {
        match (&want.border, had.border.as_deref().and_then(parts)) {
            (None, _) => set_fill(ln, "<a:noFill/>"),
            (Some(w), old) => {
                let (width, style, color) = parts(w).ok_or("a border that cannot be read")?;
                let (ow, os, oc) = old.map_or((None, None, None), |(a, b, c)| (Some(a), Some(b), Some(c)));
                if ow != Some(width) {
                    ln.set("w", &(width * EMU_PER_CENTI_PT).to_string());
                }
                if oc.as_deref() != Some(color.as_str()) {
                    if text::is_kept(&color) {
                        return Err(format!("border={w}: the colour {color} is shown for one the file stores with an adjustment, and cannot be written; write #RRGGBB or a theme colour"));
                    }
                    set_fill(ln, &text::fill_xml(&color));
                }
                if os.as_deref() != Some(style.as_str()) && !(os.is_none() && style == "solid") {
                    if look::is_kept_border(w) {
                        return Err(format!("border={w}: the style {style} is shown for a line the file draws so, and cannot be written; write solid, dashed, dotted, double, dash-dot or dash-dot-dot"));
                    }
                    ln.children.retain(|n| !matches!(n, Node::El(c) if c.is("a:prstDash") || c.is("a:custDash")));
                    if style == "double" {
                        ln.set("cmpd", "dbl");
                    } else {
                        if now.cmpd.as_deref().is_some_and(|c| c != "sng") {
                            ln.set("cmpd", "sng");
                        }
                        insert_ordered(ln, Element::new("a:prstDash").with_attr("val", dash_of(&style)), LN_ORDER);
                    }
                }
            }
        }
    }
    for (name, want, had) in [("a:headEnd", &want.start, &had.start), ("a:tailEnd", &want.end, &had.end)] {
        if want == had {
            continue;
        }
        let t = want.as_deref().unwrap_or("none");
        match ln.child_mut(name) {
            Some(e) => e.set("type", t),
            None => insert_ordered(ln, Element::new(name).with_attr("type", t), LN_ORDER),
        }
    }
    Ok(())
}

fn set_fill(ln: &mut Element, xml: &str) {
    ln.children.retain(|n| !matches!(n, Node::El(c) if FILLS.contains(&c.name.as_str())));
    insert_ordered(ln, fragment(xml), LN_ORDER);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outlines_read_through_the_style_and_write_what_changes() {
        let theme = ThemeFills {
            lines: vec!["<a:ln w=\"9525\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill><a:prstDash val=\"solid\"/></a:ln>".into(), "<a:ln w=\"25400\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:ln>".into()],
            ..Default::default()
        };
        let sp = fragment("<p:sp><p:nvSpPr/><p:spPr><a:prstGeom prst=\"rect\"/></p:spPr><p:style><a:lnRef idx=\"2\"><a:schemeClr val=\"accent1\"><a:shade val=\"50000\"/></a:schemeClr></a:lnRef></p:style></p:sp>");
        let l = effective(&sp, None, &theme);
        assert_eq!(l.border().as_deref(), Some("2pt solid accent1*"));
        // A width alone: the colour the style gives is kept.
        let mut e = sp.clone();
        let want = Look { border: Some("3pt solid accent1*".into()), ..Default::default() };
        write(&mut e, &want, None, &theme, false).unwrap();
        assert!(e.to_xml().contains("<a:prstGeom prst=\"rect\"/><a:ln w=\"38100\"/>"), "{}", e.to_xml());
        // A kept colour cannot be written anew; a dashed red one can.
        let want = Look { border: Some("3pt solid accent2*".into()), ..Default::default() };
        assert!(write(&mut e.clone(), &want, None, &theme, false).is_err());
        let want = Look { border: Some("3pt dashed #FF0000".into()), ..Default::default() };
        write(&mut e, &want, None, &theme, false).unwrap();
        assert!(e.to_xml().contains("<a:ln w=\"38100\"><a:solidFill><a:srgbClr val=\"FF0000\"/></a:solidFill><a:prstDash val=\"dash\"/></a:ln>"), "{}", e.to_xml());
        // None, and a line's arrowheads.
        write(&mut e, &Look::default(), None, &theme, false).unwrap();
        assert_eq!(effective(&e, None, &theme).border(), None);
        let mut c = fragment("<p:cxnSp><p:nvCxnSpPr/><p:spPr><a:ln w=\"12700\"><a:solidFill><a:schemeClr val=\"tx1\"/></a:solidFill><a:tailEnd type=\"triangle\" w=\"lg\"/></a:ln></p:spPr></p:cxnSp>");
        assert_eq!(effective(&c, None, &theme).look(true).end.as_deref(), Some("triangle"));
        let want = Look {
            border: Some("1pt solid tx1".into()),
            start: Some("oval".into()),
            end: Some("arrow".into()),
            ..Default::default()
        };
        write(&mut c, &want, None, &theme, true).unwrap();
        assert!(
            c.to_xml().contains("<a:headEnd type=\"oval\"/><a:tailEnd type=\"arrow\" w=\"lg\"/>"),
            "{}",
            c.to_xml()
        );
    }
}
