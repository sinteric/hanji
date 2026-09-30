//! Shape fill (DESIGN.md §5.3): the fill a slot or shape shows, read from
//! its own `p:spPr`, else its `p:style` fill reference into the theme's fill
//! styles, else (a placeholder) its layout's and master's placeholder; and
//! written back into its `p:spPr` only where the text changes it.

use serde::{Deserialize, Serialize};

use hanji_format::look;
use hanji_package::xml::{fragment, insert_ordered, Element, Node};

use crate::text;

/// A fill element as the file stores it (`a:solidFill`, `a:noFill`, …).
pub type FillXml = String;

const FILLS: &[&str] = &["a:noFill", "a:solidFill", "a:gradFill", "a:blipFill", "a:pattFill", "a:grpFill"];

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

/// A theme's fill styles (`a:fmtScheme`): `a:fillStyleLst` and
/// `a:bgFillStyleLst`, each style's XML.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeFills {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fills: Vec<FillXml>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bg: Vec<FillXml>,
}

impl ThemeFills {
    pub fn of(theme: &Element) -> ThemeFills {
        let scheme = theme.child("a:themeElements").and_then(|t| t.child("a:fmtScheme"));
        let list = |n: &str| -> Vec<FillXml> {
            scheme.and_then(|s| s.child(n)).map(|l| l.elements().map(Element::to_xml).collect()).unwrap_or_default()
        };
        ThemeFills { fills: list("a:fillStyleLst"), bg: list("a:bgFillStyleLst") }
    }
}

/// The fill element of `el`'s own `p:spPr`, if it sets one.
pub fn own(el: &Element) -> Option<FillXml> {
    el.child("p:spPr")?.elements().find(|e| FILLS.contains(&e.name.as_str())).map(Element::to_xml)
}

/// The fill `el`'s `p:style` gives: its `a:fillRef` style from the theme,
/// the style's placeholder colour (`phClr`) the reference's colour.
pub fn from_style(el: &Element, theme: &ThemeFills) -> Option<FillXml> {
    let r = el.child("p:style")?.child("a:fillRef")?;
    let idx: usize = r.get("idx")?.trim().parse().ok()?;
    let style = match idx {
        0 => return Some("<a:noFill/>".into()),
        1..=999 => theme.fills.get(idx - 1)?,
        _ => theme.bg.get(idx.checked_sub(1001)?)?,
    };
    let mut f = fragment(style);
    if let Some(c) = r.elements().next() {
        with_color(&mut f, c);
    }
    Some(f.to_xml())
}

/// Every `phClr` in `el` becomes `c`, the placeholder's own adjustments after `c`'s.
fn with_color(el: &mut Element, c: &Element) {
    for n in &mut el.children {
        let Node::El(e) = n else { continue };
        if e.is("a:schemeClr") && e.get("val").as_deref() == Some("phClr") {
            let mut x = c.clone();
            x.children.extend(e.children.iter().cloned());
            *e = x;
        } else {
            with_color(e, c);
        }
    }
}

/// What `el` shows when its own `p:spPr` sets no fill: its style's, else
/// `parent` (a placeholder's layout placeholder's).
pub fn inherited(el: &Element, parent: Option<&FillXml>, theme: &ThemeFills) -> Option<FillXml> {
    from_style(el, theme).or_else(|| parent.cloned())
}

/// The fill `el` has.
pub fn effective(el: &Element, parent: Option<&FillXml>, theme: &ThemeFills) -> Option<FillXml> {
    own(el).or_else(|| inherited(el, parent, theme))
}

/// A fill as the text shows it (canonical, §5.1); `None` is no fill (and a
/// group fill, which the text does not show).
pub fn shown(fill: Option<&FillXml>) -> Option<String> {
    let f = fragment(fill?);
    match f.name.as_str() {
        "a:solidFill" => text::color_of(&f),
        "a:gradFill" => Some("gradient".into()),
        "a:pattFill" => Some("pattern".into()),
        "a:blipFill" => Some("picture".into()),
        _ => None,
    }
}

/// `el` (a shape) without its own fill: what its fingerprint holds.
pub fn without_fill(el: &Element) -> Element {
    let mut e = el.clone();
    if let Some(sp) = e.child_mut("p:spPr") {
        sp.children.retain(|n| !matches!(n, Node::El(c) if FILLS.contains(&c.name.as_str())));
    }
    e
}

/// Where a fill comes from when the text leaves it as it was: the layout
/// the slide has now (`now`) and the one it had (`then`).
#[derive(Clone, Copy)]
pub struct Parents<'a> {
    pub now: Option<&'a FillXml>,
    pub then: Option<&'a FillXml>,
    pub theme: &'a ThemeFills,
}

/// Write the fill the text shows for `el` (`want`, canonical; `None` no
/// fill) where it differs from what `el` shows now: a fill equal to the one
/// it inherits removes its own; any other is set in its `p:spPr`, in schema
/// order. A fill the text cannot write (a gradient, a colour with `*`) is
/// refused, unless it is the one `el` showed under its old layout (copied).
pub fn write(el: &mut Element, want: Option<&str>, p: Parents) -> Result<(), String> {
    let had = shown(effective(el, p.now, p.theme).as_ref());
    if had.as_deref() == want {
        return Ok(());
    }
    let inh = inherited(el, p.now, p.theme);
    let set = if shown(inh.as_ref()).as_deref() == want {
        None
    } else {
        Some(match want {
            None => "<a:noFill/>".to_string(),
            Some(w) if look::is_kept_fill(w) => {
                let then = effective(el, p.then, p.theme);
                match then.filter(|t| shown(Some(t)).as_deref() == Some(w)) {
                    Some(t) => t,
                    None => {
                        return Err(format!(
                            "fill={w} is shown for a fill the file stores as it is (a gradient, pattern, picture or adjusted colour), and cannot be written: write none or a colour, {}",
                            hanji_format::vocab::COLOR_FORM
                        ))
                    }
                }
            }
            Some(w) => text::fill_xml(w),
        })
    };
    if el.child("p:spPr").is_none() {
        let at = el
            .children
            .iter()
            .position(|n| matches!(n, Node::El(c) if c.name.starts_with("p:nv")))
            .map_or(0, |k| k + 1);
        el.children.insert(at, Node::El(Element::new("p:spPr")));
    }
    let sp = el.child_mut("p:spPr").expect("p:spPr");
    sp.children.retain(|n| !matches!(n, Node::El(c) if FILLS.contains(&c.name.as_str())));
    if let Some(f) = set {
        insert_ordered(sp, fragment(&f), SPPR_ORDER);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_style_reference_fills_its_placeholder_colour() {
        let theme = ThemeFills {
            fills: vec![
                "<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>".into(),
                "<a:solidFill><a:schemeClr val=\"phClr\"><a:tint val=\"50000\"/></a:schemeClr></a:solidFill>".into(),
            ],
            bg: vec![],
        };
        let sp = |idx: u32| {
            fragment(&format!(
                "<p:sp><p:nvSpPr/><p:spPr/><p:style><a:fillRef idx=\"{idx}\"><a:schemeClr val=\"accent2\"/></a:fillRef></p:style></p:sp>"
            ))
        };
        let p = Parents { now: None, then: None, theme: &theme };
        assert_eq!(shown(effective(&sp(1), None, &theme).as_ref()).as_deref(), Some("accent2"));
        assert_eq!(shown(effective(&sp(2), None, &theme).as_ref()).as_deref(), Some("accent2*"));
        assert_eq!(shown(effective(&sp(0), None, &theme).as_ref()), None);
        // Removing the style's fill writes none; writing it back removes the shape's own.
        let mut e = sp(1);
        write(&mut e, None, p).unwrap();
        assert!(e.to_xml().contains("<p:spPr><a:noFill/></p:spPr>"), "{}", e.to_xml());
        write(&mut e, Some("accent2"), p).unwrap();
        assert!(e.to_xml().contains("<p:spPr/>") || e.to_xml().contains("<p:spPr></p:spPr>"), "{}", e.to_xml());
        write(&mut e, Some("#FF7F50/50%"), p).unwrap();
        assert!(e
            .to_xml()
            .contains("<a:solidFill><a:srgbClr val=\"FF7F50\"><a:alpha val=\"50000\"/></a:srgbClr></a:solidFill>"));
        assert!(write(&mut sp(1), Some("gradient"), p).is_err());
        // Order: after the geometry, before the outline.
        let mut e = fragment("<p:sp><p:nvSpPr/><p:spPr><a:xfrm/><a:prstGeom prst=\"rect\"/><a:ln/></p:spPr></p:sp>");
        write(&mut e, Some("accent1"), p).unwrap();
        assert!(e
            .to_xml()
            .contains("<a:prstGeom prst=\"rect\"/><a:solidFill><a:schemeClr val=\"accent1\"/></a:solidFill><a:ln/>"));
    }
}
