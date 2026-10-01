//! Connector rerouting (DESIGN.md §5.3): a connector's end attached to an
//! object is where that object's connection site is. When the object moves
//! or is resized, or the text attaches the end to another site, the end is
//! put on the site again: a straight connector exactly, an elbow one
//! (`bentConnector2`, `bentConnector3`) by its ends with its bend at the
//! midpoint. What cannot be rerouted is refused with the reason.

use hanji_format::{Attach, Ends, Geom};
use hanji_package::xml::{insert_ordered, Element, Node};

use crate::geom::{self, Frame};
use crate::kind::{self, Geo};

/// Presets whose four sites are the middles of their box's sides, top, left,
/// bottom, right (their `a:cxnLst` in DrawingML's preset definitions).
const SIDES: &[&str] = &[
    "rect",
    "roundRect",
    "diamond",
    "flowChartProcess",
    "flowChartDecision",
    "flowChartAlternateProcess",
    "flowChartPredefinedProcess",
];

/// Presets whose eight sites go round an ellipse from its top, anticlockwise.
const ROUND: &[&str] = &["ellipse", "flowChartConnector"];

/// The connection sites of a `w` × `h` shape of geometry `g`, in its own
/// coordinates (before its flips and rotation).
fn sites(g: &Geo, w: f64, h: f64) -> Result<Vec<(f64, f64)>, String> {
    match g {
        Geo::Preset { prst, .. } if SIDES.contains(&prst.as_str()) => {
            Ok(vec![(w / 2.0, 0.0), (0.0, h / 2.0), (w / 2.0, h), (w, h / 2.0)])
        }
        Geo::Preset { prst, .. } if ROUND.contains(&prst.as_str()) => {
            let k = std::f64::consts::FRAC_1_SQRT_2;
            let (il, ir) = (w / 2.0 * (1.0 - k), w / 2.0 * (1.0 + k));
            let (it, ib) = (h / 2.0 * (1.0 - k), h / 2.0 * (1.0 + k));
            Ok(vec![(w / 2.0, 0.0), (il, it), (0.0, h / 2.0), (il, ib), (w / 2.0, h), (ir, ib), (w, h / 2.0), (ir, it)])
        }
        Geo::Preset { prst, .. } => Err(format!("the connection sites of kind=\"{prst}\" are not known here")),
        Geo::Custom => Err("its connection sites are custom geometry, which is not evaluated here".into()),
    }
}

/// Where site `site` of `el` (at `frame`, its parent's mapping to the slide) is on the slide.
fn site_point(el: &Element, site: u32, frame: &Frame) -> Result<(i64, i64), String> {
    if el.is("p:grpSp") {
        return Err("a group's connection sites are not known here".into());
    }
    let g = geom::own(el).ok_or("it has no box")?;
    let geo = kind::own(el).unwrap_or(Geo::Preset { prst: "rect".into(), av: vec![] });
    let all = sites(&geo, g.w as f64, g.h as f64)?;
    let &(mut x, mut y) = all.get(site as usize).ok_or_else(|| format!("it has no connection site {site}"))?;
    if g.flip_h {
        x = g.w as f64 - x;
    }
    if g.flip_v {
        y = g.h as f64 - y;
    }
    let p = ((g.x as f64 + x).round() as i64, (g.y as f64 + y).round() as i64);
    let c = (g.x as f64 + g.w as f64 / 2.0, g.y as f64 + g.h as f64 / 2.0);
    let (px, py) = geom::turned(p, c, g.rot);
    let s = frame.out(&Geom { x: px, y: py, ..Default::default() });
    Ok((s.x, s.y))
}

/// Where the object with shape id `id` is among `items` (top-level
/// elements and groups' objects).
struct Found<'a> {
    el: &'a Element,
    /// Its parent's frame on the slide.
    frame: Frame,
    /// The ids of the groups it is in.
    up: Vec<u32>,
    /// Whether one of them is turned or flipped.
    turned: bool,
}

fn find<'a>(items: &[&'a Element], id: u32, frame: Frame) -> Option<Found<'a>> {
    for &e in items {
        if geom::shape_id(e) == Some(id) {
            return Some(Found { el: e, frame, up: vec![], turned: false });
        }
        if e.is("p:grpSp") {
            let kids: Vec<&Element> = e.elements().collect();
            if let Some(mut f) = find(&kids, id, Frame::of(e).map_or(frame, |f| f.within(&frame))) {
                let g = geom::own(e).unwrap_or_default();
                f.turned |= g.rot.rem_euclid(hanji_format::FULL_TURN) != 0 || g.flip_h || g.flip_v;
                f.up.extend(geom::shape_id(e));
                return Some(f);
            }
        }
    }
    None
}

/// A connector without its ends' attachments: what its fingerprint holds,
/// so a re-attached connector is found again.
pub fn without_attachments(el: &Element) -> Element {
    let mut e = el.clone();
    if let Some(c) = e.child_mut("p:nvCxnSpPr").and_then(|n| n.child_mut("p:cNvCxnSpPr")) {
        c.children.retain(|n| !matches!(n, Node::El(x) if x.is("a:stCxn") || x.is("a:endCxn")));
    }
    e
}

/// `cNvCxnSpPr`'s `a:stCxn` or `a:endCxn`.
fn set_cxn(cxn: &mut Element, name: &str, at: Option<(u32, u32)>) {
    let Some(nv) = cxn.child_mut("p:nvCxnSpPr") else { return };
    if nv.child("p:cNvCxnSpPr").is_none() {
        let at = nv.children.iter().position(|n| matches!(n, Node::El(c) if c.is("p:cNvPr"))).map_or(0, |k| k + 1);
        nv.children.insert(at, Node::El(Element::new("p:cNvCxnSpPr")));
    }
    let c = nv.child_mut("p:cNvCxnSpPr").expect("p:cNvCxnSpPr");
    match at {
        None => c.children.retain(|n| !matches!(n, Node::El(x) if x.is(name))),
        Some((id, idx)) => {
            let (id, idx) = (id.to_string(), idx.to_string());
            match c.child_mut(name) {
                Some(x) => {
                    if x.get("id").as_deref() != Some(id.as_str()) {
                        x.set("id", &id);
                    }
                    if x.get("idx").as_deref() != Some(idx.as_str()) {
                        x.set("idx", &idx);
                    }
                }
                None => insert_ordered(
                    c,
                    Element::new(name).with_attr("id", &id).with_attr("idx", &idx),
                    &["cxnSpLocks", "stCxn", "endCxn", "extLst"],
                ),
            }
        }
    }
}

/// Write connector `cxn`'s ends as the text gives them, `w`: an attached
/// end on its object's site (the stored end kept where neither the
/// attachment nor the object's box changed); a point as the text writes it.
/// `items` are the slide's written objects, `moved` the ids whose boxes
/// changed, `visible` the ids the text shows (an attachment to one the text
/// does not show stays as it is). Returns whether the connector's box was
/// written.
pub fn write(
    cxn: &mut Element,
    w: &Ends,
    items: &[&Element],
    moved: &[u32],
    visible: &[u32],
    created: bool,
) -> Result<bool, String> {
    let st = geom::own(cxn).unwrap_or_default();
    let stored = geom::ends_of(&st);
    let had = geom::attachments(cxn);
    let mut ends = if created { *w } else { stored.merged(w) };
    let mut rerouted = false;
    for (i, (want, name)) in [(w.from_at, "a:stCxn"), (w.to_at, "a:endCxn")].into_iter().enumerate() {
        let end = if i == 0 { "from" } else { "to" };
        match want {
            None => {
                // Written as a point: an end the text showed attached comes loose.
                if had[i].is_some_and(|(id, _)| visible.contains(&id)) {
                    set_cxn(cxn, name, None);
                }
            }
            Some(Attach { id, site, .. }) => {
                let same = had[i] == Some((id, site));
                let stored_end = if i == 0 { stored.from } else { stored.to };
                let found = find(items, id, Frame::SLIDE);
                let still =
                    !moved.contains(&id) && !found.as_ref().is_some_and(|f| f.up.iter().any(|g| moved.contains(g)));
                let p = if same && still && !created {
                    stored_end
                } else {
                    let why = |m: &str| format!("its {end} end is attached to s{id}.{site}, but {m}");
                    let f = found.ok_or_else(|| why(&format!("there is no object s{id} on this slide")))?;
                    if f.turned {
                        return Err(why(
                            "it is in a turned or flipped group, whose objects' sites are not placed here",
                        ));
                    }
                    let p = site_point(f.el, site, &f.frame).map_err(|m| why(&m))?;
                    if !same {
                        set_cxn(cxn, name, Some((id, site)));
                    }
                    rerouted = true;
                    p
                };
                if i == 0 {
                    ends.from = p;
                } else {
                    ends.to = p;
                }
            }
        }
    }
    if (ends.from, ends.to) == (stored.from, stored.to) && !created {
        return Ok(false);
    }
    if rerouted {
        match kind::own(cxn) {
            None => {}
            Some(Geo::Preset { prst, .. })
                if matches!(prst.as_str(), "line" | "straightConnector1" | "bentConnector2") => {}
            // An elbow's bend goes back to the midpoint.
            Some(Geo::Preset { prst, av }) if prst == "bentConnector3" => {
                if !av.is_empty() {
                    let g = cxn.child_mut("p:spPr").and_then(|s| s.child_mut("a:prstGeom")).expect("a:prstGeom");
                    g.children.retain(|n| !matches!(n, Node::El(c) if c.is("a:avLst")));
                    g.children.insert(0, Node::El(Element::new("a:avLst")));
                }
            }
            Some(Geo::Preset { prst, .. }) => {
                return Err(format!("it is kind=\"{prst}\", and only straight and elbow connectors (bentConnector2, bentConnector3) are rerouted here: attach it where its object is, write its ends as points, or leave the object where it is"));
            }
            Some(Geo::Custom) => return Err("it is drawn with custom geometry, which is not rerouted here".into()),
        }
    }
    geom::write(cxn, &geom::box_of(&ends, st.rot))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hanji_package::xml::fragment;

    #[test]
    fn sites_are_placed_through_flips_turns_and_groups() {
        let sp = |x: &str| {
            fragment(&format!("<p:sp><p:nvSpPr><p:cNvPr id=\"3\" name=\"a\"/></p:nvSpPr><p:spPr>{x}</p:spPr></p:sp>"))
        };
        let r =
            sp("<a:xfrm><a:off x=\"100\" y=\"200\"/><a:ext cx=\"40\" cy=\"20\"/></a:xfrm><a:prstGeom prst=\"rect\"/>");
        assert_eq!(site_point(&r, 1, &Frame::SLIDE).unwrap(), (100, 210));
        assert_eq!(site_point(&r, 3, &Frame::SLIDE).unwrap(), (140, 210));
        // Turned a quarter: its left site is at the top.
        let t = sp("<a:xfrm rot=\"5400000\"><a:off x=\"100\" y=\"200\"/><a:ext cx=\"40\" cy=\"20\"/></a:xfrm><a:prstGeom prst=\"rect\"/>");
        assert_eq!(site_point(&t, 1, &Frame::SLIDE).unwrap(), (120, 190));
        let e = sp("<a:xfrm flipH=\"1\"><a:off x=\"0\" y=\"0\"/><a:ext cx=\"200\" cy=\"100\"/></a:xfrm><a:prstGeom prst=\"ellipse\"/>");
        assert_eq!(site_point(&e, 7, &Frame::SLIDE).unwrap(), (29, 15));
        assert!(site_point(
            &sp("<a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"2\" cy=\"2\"/></a:xfrm><a:prstGeom prst=\"star5\"/>"),
            0,
            &Frame::SLIDE
        )
        .is_err());
        assert!(site_point(&r, 4, &Frame::SLIDE).is_err());
    }
}
