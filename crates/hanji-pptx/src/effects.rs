//! Effects (DESIGN.md §5.3): what a slot, shape or line draws beyond its
//! fill and outline (a shadow, a glow, soft edges, a reflection), shown as a
//! summary, `effects="shadow"`, from its own `a:effectLst` (or
//! `a:effectDag`), else its `p:style` effect reference into the theme's
//! effect styles, else (a placeholder) its layout's and master's
//! placeholder. The text can remove them, or give an object PowerPoint's
//! preset shadow; any other change is refused.

use hanji_package::xml::{fragment, insert_ordered, Element, Node};

use crate::fill::ThemeFills;

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

/// PowerPoint's preset outer shadow (Shape Effects > Shadow > Offset:
/// Bottom Right), what `effects="shadow"` writes.
pub const SHADOW: &str = "<a:effectLst><a:outerShdw blurRad=\"50800\" dist=\"38100\" dir=\"2700000\" algn=\"tl\" rotWithShape=\"0\"><a:prstClr val=\"black\"><a:alpha val=\"40000\"/></a:prstClr></a:outerShdw></a:effectLst>";

/// An effect list or graph, as XML.
pub type EffectsXml = String;

fn is_effects(e: &Element) -> bool {
    e.is("a:effectLst") || e.is("a:effectDag")
}

/// The effects of `el`'s own `p:spPr`, if it sets them (an empty list is
/// none, set).
pub fn own(el: &Element) -> Option<EffectsXml> {
    el.child("p:spPr")?.elements().find(|e| is_effects(e)).map(Element::to_xml)
}

/// The effects `el`'s `p:style` gives: its `a:effectRef` style from the theme.
pub fn from_style(el: &Element, theme: &ThemeFills) -> Option<EffectsXml> {
    let r = el.child("p:style")?.child("a:effectRef")?;
    let idx: usize = r.get("idx")?.parse().ok()?;
    if idx == 0 {
        return None;
    }
    let style = fragment(theme.effects.get(idx - 1)?);
    let xml = style.elements().find(|e| is_effects(e)).map_or_else(|| "<a:effectLst/>".into(), Element::to_xml);
    Some(xml)
}

/// What `el` inherits: its style's effects, else `parent`'s.
pub fn inherited(el: &Element, parent: Option<&EffectsXml>, theme: &ThemeFills) -> Option<EffectsXml> {
    from_style(el, theme).or_else(|| parent.cloned())
}

/// The effects `el` draws.
pub fn effective(el: &Element, parent: Option<&EffectsXml>, theme: &ThemeFills) -> Option<EffectsXml> {
    own(el).or_else(|| inherited(el, parent, theme))
}

/// Effects as the text shows them: each effect's name once, in the file's
/// order (`shadow glow`), `custom` for an effect graph; `None` for none.
pub fn shown(xml: Option<&EffectsXml>) -> Option<String> {
    let e = fragment(xml?);
    if e.is("a:effectDag") {
        return Some("custom".into());
    }
    let mut names: Vec<&str> = vec![];
    for c in e.elements() {
        let n = match c.name.as_str() {
            "a:outerShdw" | "a:prstShdw" => "shadow",
            "a:innerShdw" => "inner-shadow",
            "a:glow" => "glow",
            "a:softEdge" => "soft-edges",
            "a:reflection" => "reflection",
            "a:blur" => "blur",
            "a:fillOverlay" => "fill-overlay",
            _ => continue,
        };
        if !names.contains(&n) {
            names.push(n);
        }
    }
    (!names.is_empty()).then(|| names.join(" "))
}

/// `el` without its own effects: what its fingerprint holds.
pub fn without_effects(el: &Element) -> Element {
    let mut e = el.clone();
    if let Some(sp) = e.child_mut("p:spPr") {
        sp.children.retain(|n| !matches!(n, Node::El(c) if is_effects(c)));
    }
    e
}

fn set_own(el: &mut Element, xml: Option<&str>) {
    if xml.is_some() && el.child("p:spPr").is_none() {
        let at = el
            .children
            .iter()
            .position(|n| matches!(n, Node::El(c) if c.name.starts_with("p:nv")))
            .map_or(0, |k| k + 1);
        el.children.insert(at, Node::El(Element::new("p:spPr")));
    }
    let Some(sp) = el.child_mut("p:spPr") else { return };
    sp.children.retain(|n| !matches!(n, Node::El(c) if is_effects(c)));
    if let Some(x) = xml {
        insert_ordered(sp, fragment(x), SPPR_ORDER);
    }
}

/// Write the effects the text gives, `want`, where they differ from what
/// `el` shows: none (an empty `a:effectLst` over a style's or layout's), the
/// preset shadow, or what the style or layout gives (the object's own goes).
pub fn write(
    el: &mut Element,
    want: Option<&str>,
    parent: Option<&EffectsXml>,
    theme: &ThemeFills,
) -> Result<(), String> {
    let had = shown(effective(el, parent, theme).as_ref());
    if had.as_deref() == want {
        return Ok(());
    }
    let under = shown(inherited(el, parent, theme).as_ref());
    match want {
        _ if under.as_deref() == want => set_own(el, None),
        None => set_own(el, Some("<a:effectLst/>")),
        Some("shadow") => set_own(el, Some(SHADOW)),
        Some(w) => {
            return Err(format!(
                "effects=\"{w}\" cannot be written: effects are shown as the file has them, and the text writes only effects=\"shadow\" (PowerPoint's preset shadow) or none (effects left out)"
            ))
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effects_read_through_the_style_and_write_none_or_a_shadow() {
        let theme = ThemeFills {
            effects: vec![
                "<a:effectStyle><a:effectLst/></a:effectStyle>".into(),
                "<a:effectStyle><a:effectLst><a:outerShdw blurRad=\"40000\"><a:srgbClr val=\"000000\"/></a:outerShdw><a:glow rad=\"1\"/></a:effectLst><a:scene3d/></a:effectStyle>".into(),
            ],
            ..Default::default()
        };
        let sp = fragment("<p:sp><p:nvSpPr/><p:spPr><a:prstGeom prst=\"rect\"/><a:ln/></p:spPr><p:style><a:effectRef idx=\"2\"/></p:style></p:sp>");
        assert_eq!(shown(effective(&sp, None, &theme).as_ref()).as_deref(), Some("shadow glow"));
        // None over the style's: an empty list after the outline.
        let mut e = sp.clone();
        write(&mut e, None, None, &theme).unwrap();
        assert!(e.to_xml().contains("<a:ln/><a:effectLst/></p:spPr>"), "{}", e.to_xml());
        // The preset shadow, then back to the style's: its own goes.
        write(&mut e, Some("shadow"), None, &theme).unwrap();
        assert!(e.to_xml().contains(&format!("<a:ln/>{SHADOW}</p:spPr>")), "{}", e.to_xml());
        write(&mut e, Some("shadow glow"), None, &theme).unwrap();
        assert_eq!(e, sp);
        assert!(write(&mut e, Some("glow"), None, &theme).is_err());
        // No style: none removes the object's own.
        let mut t = fragment(&format!("<p:sp><p:spPr>{SHADOW}</p:spPr></p:sp>"));
        write(&mut t, None, None, &theme).unwrap();
        assert_eq!(t.to_xml(), "<p:sp><p:spPr/></p:sp>");
    }
}
