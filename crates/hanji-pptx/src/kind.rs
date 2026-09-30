//! Preset shapes (DESIGN.md §5.3): the shape a slot, shape or connector
//! draws, as `kind="roundRect"`, and its preset's adjustments as `adj=`.
//! Read from its own `a:prstGeom`, else (a placeholder) its layout's and
//! master's placeholder; written back only where the text changes them.
//! Custom geometry (`a:custGeom`) shows no kind and is kept.

use hanji_format::look::{self, Look};
use hanji_package::xml::{insert_ordered, Element, Node};
use serde::{Deserialize, Serialize};

/// `p:spPr`'s children in schema order (local names), up to the fills.
const SPPR_ORDER: &[&str] =
    &["xfrm", "custGeom", "prstGeom", "noFill", "solidFill", "gradFill", "blipFill", "pattFill"];

/// An object's geometry: a preset with its adjustment guides (name,
/// formula), or custom geometry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Geo {
    Preset { prst: String, av: Vec<(String, String)> },
    Custom,
}

impl Geo {
    fn of(sp_pr: &Element) -> Option<Geo> {
        if sp_pr.child("a:custGeom").is_some() {
            return Some(Geo::Custom);
        }
        let g = sp_pr.child("a:prstGeom")?;
        let av = g
            .child("a:avLst")
            .map(|l| {
                l.elements()
                    .filter(|e| e.is("a:gd"))
                    .map(|e| (e.get("name").unwrap_or_default(), e.get("fmla").unwrap_or_default()))
                    .collect()
            })
            .unwrap_or_default();
        Some(Geo::Preset { prst: g.get("prst").unwrap_or_else(|| "rect".into()), av })
    }

    /// Its kind and adjustments as the text shows them: no kind for a
    /// rectangle (a straight line) or custom geometry, no adjustments for a
    /// preset's own or for guides that are not plain values (kept).
    pub fn shown(&self, line: bool) -> (Option<String>, Option<String>) {
        let Geo::Preset { prst, av } = self else { return (None, None) };
        let plain = matches!(prst.as_str(), "line" | "straightConnector1") && line || prst == "rect" && !line;
        let kind = (!plain).then(|| prst.clone());
        let vals: Option<Vec<(String, i64)>> =
            av.iter().map(|(n, f)| Some((n.clone(), f.strip_prefix("val ")?.trim().parse().ok()?))).collect();
        let adj = vals.filter(|v| !v.is_empty()).map(|v| look::adj_text(&v));
        (kind, adj)
    }
}

/// `el`'s own geometry.
pub fn own(el: &Element) -> Option<Geo> {
    el.child("p:spPr").and_then(Geo::of)
}

/// The geometry `el` draws: its own, else `parent`'s.
pub fn effective(el: &Element, parent: Option<&Geo>) -> Option<Geo> {
    own(el).or_else(|| parent.cloned())
}

/// The kind and adjustments `el` shows.
pub fn shown(el: &Element, parent: Option<&Geo>, line: bool) -> (Option<String>, Option<String>) {
    effective(el, parent).map_or((None, None), |g| g.shown(line))
}

/// `el` without its own preset geometry: what its fingerprint holds.
pub fn without_kind(el: &Element) -> Element {
    let mut e = el.clone();
    if let Some(sp) = e.child_mut("p:spPr") {
        sp.children.retain(|n| !matches!(n, Node::El(c) if c.is("a:prstGeom")));
    }
    e
}

/// Write `want`'s kind and adjustments into `el` where they differ from
/// what it shows: `a:prstGeom`'s `prst` and `a:avLst`, other children kept.
/// A preset equal to what the layout gives removes the object's own.
pub fn write(el: &mut Element, want: &Look, parent: Option<&Geo>, line: bool) -> Result<(), String> {
    let now = effective(el, parent);
    let (hk, ha) = now.as_ref().map_or((None, None), |g| g.shown(line));
    if hk == want.kind && ha == want.adj {
        return Ok(());
    }
    if now == Some(Geo::Custom) {
        return Err(
            "it is drawn with custom geometry, which the text keeps as the file has it; leave kind= and adj= out"
                .into(),
        );
    }
    if let (Some(p), true) = (parent, own(el).is_some()) {
        if p.shown(line) == (want.kind.clone(), want.adj.clone()) {
            if let Some(sp) = el.child_mut("p:spPr") {
                sp.children.retain(|n| !matches!(n, Node::El(c) if c.is("a:prstGeom")));
            }
            return Ok(());
        }
    }
    let old = match &now {
        Some(Geo::Preset { prst, .. }) => prst.clone(),
        _ => String::new(),
    };
    let prst = match &want.kind {
        Some(k) => k.clone(),
        None if line && old == "line" => "line".into(),
        None if line => "straightConnector1".into(),
        None => "rect".into(),
    };
    let av: Vec<Element> = match &want.adj {
        // The preset's own adjustments where the kind stays and they were not shown.
        None if prst == old && ha.is_none() => match &now {
            Some(Geo::Preset { av, .. }) => av.iter().map(|(n, f)| gd(n, f)).collect(),
            _ => vec![],
        },
        None => vec![],
        Some(a) => look::adj_values(a)?.iter().map(|(n, v)| gd(n, &format!("val {v}"))).collect(),
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
    if sp.child("a:prstGeom").is_none() {
        let mut g = Element::new("a:prstGeom");
        g.children.push(Node::El(Element::new("a:avLst")));
        insert_ordered(sp, g, SPPR_ORDER);
    }
    let g = sp.child_mut("a:prstGeom").expect("a:prstGeom");
    g.set("prst", &prst);
    let same_av = matches!(&now, Some(Geo::Preset { prst: p, av: a }) if *p == prst
        && a.iter().map(|(n, f)| gd(n, f).to_xml()).eq(av.iter().map(Element::to_xml)));
    if !same_av {
        g.children.retain(|n| !matches!(n, Node::El(c) if c.is("a:avLst")));
        let mut l = Element::new("a:avLst");
        l.children.extend(av.into_iter().map(Node::El));
        g.children.insert(0, Node::El(l));
    }
    Ok(())
}

fn gd(name: &str, fmla: &str) -> Element {
    Element::new("a:gd").with_attr("name", name).with_attr("fmla", fmla)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hanji_package::xml::fragment;

    fn look(kind: Option<&str>, adj: Option<&str>) -> Look {
        Look { kind: kind.map(Into::into), adj: adj.map(Into::into), ..Default::default() }
    }

    #[test]
    fn kinds_read_and_write_what_changes() {
        let sp = fragment("<p:sp><p:nvSpPr/><p:spPr><a:xfrm/><a:prstGeom prst=\"roundRect\"><a:avLst><a:gd name=\"adj\" fmla=\"val 10000\"/></a:avLst></a:prstGeom><a:noFill/></p:spPr></p:sp>");
        assert_eq!(shown(&sp, None, false), (Some("roundRect".into()), Some("10000".into())));
        // The adjustment alone.
        let mut e = sp.clone();
        write(&mut e, &look(Some("roundRect"), Some("20000")), None, false).unwrap();
        assert!(e.to_xml().contains("<a:prstGeom prst=\"roundRect\"><a:avLst><a:gd name=\"adj\" fmla=\"val 20000\"/></a:avLst></a:prstGeom><a:noFill/>"), "{}", e.to_xml());
        // Another kind takes its own adjustments; none is a rectangle.
        write(&mut e, &look(Some("ellipse"), None), None, false).unwrap();
        assert!(e.to_xml().contains("<a:prstGeom prst=\"ellipse\"><a:avLst/></a:prstGeom>"), "{}", e.to_xml());
        write(&mut e, &look(None, None), None, false).unwrap();
        assert_eq!(shown(&e, None, false), (None, None));
        // A shape with no spPr geometry gets one after its xfrm.
        let mut t = fragment("<p:sp><p:nvSpPr/><p:spPr><a:xfrm/><a:solidFill/></p:spPr></p:sp>");
        write(&mut t, &look(Some("hexagon"), Some("adj=25000 vf=115470")), None, false).unwrap();
        assert!(t.to_xml().contains("<a:xfrm/><a:prstGeom prst=\"hexagon\"><a:avLst><a:gd name=\"adj\" fmla=\"val 25000\"/><a:gd name=\"vf\" fmla=\"val 115470\"/></a:avLst></a:prstGeom><a:solidFill/>"), "{}", t.to_xml());
        // A connector: straight is no kind; left out, a bent one is made straight.
        let mut c = fragment("<p:cxnSp><p:nvCxnSpPr/><p:spPr><a:prstGeom prst=\"bentConnector3\"><a:avLst/></a:prstGeom></p:spPr></p:cxnSp>");
        assert_eq!(shown(&c, None, true), (Some("bentConnector3".into()), None));
        write(&mut c, &look(None, None), None, true).unwrap();
        assert!(c.to_xml().contains("prst=\"straightConnector1\""));
        // Custom geometry is kept, and refused a kind.
        let mut g = fragment("<p:sp><p:spPr><a:custGeom/></p:spPr></p:sp>");
        assert_eq!(shown(&g, None, false), (None, None));
        assert!(write(&mut g, &look(Some("ellipse"), None), None, false).is_err());
        // A slot back at its layout's preset loses its own.
        let parent = Geo::Preset { prst: "roundRect".into(), av: vec![] };
        let mut s = fragment("<p:sp><p:spPr><a:prstGeom prst=\"ellipse\"><a:avLst/></a:prstGeom></p:spPr></p:sp>");
        write(&mut s, &look(Some("roundRect"), None), Some(&parent), false).unwrap();
        assert_eq!(s.to_xml(), "<p:sp><p:spPr/></p:sp>");
    }
}
