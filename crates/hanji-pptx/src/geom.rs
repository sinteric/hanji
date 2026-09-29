//! Geometry of slide objects (DESIGN.md §5.3): reading and writing `a:xfrm`
//! (`p:xfrm` on a graphic frame), a group's child coordinates, a line's ends,
//! and the connections of connectors.

use hanji_format::{Ends, Geom};
use hanji_package::xml::{Element, Node};

/// The element that holds an object's `a:xfrm`: `p:spPr` of a shape,
/// connector or picture, `p:grpSpPr` of a group, the graphic frame itself.
fn holder(el: &Element) -> Option<&Element> {
    match el.name.as_str() {
        "p:graphicFrame" => Some(el),
        "p:grpSp" => el.child("p:grpSpPr"),
        _ => el.child("p:spPr"),
    }
}

fn xfrm_name(el: &Element) -> &'static str {
    if el.is("p:graphicFrame") {
        "p:xfrm"
    } else {
        "a:xfrm"
    }
}

/// An object's own `a:xfrm` (`p:xfrm` of a graphic frame), when it has one.
pub fn xfrm(el: &Element) -> Option<&Element> {
    holder(el)?.child(xfrm_name(el))
}

fn int(e: Option<&Element>, k: &str) -> Option<i64> {
    e?.get(k)?.trim().parse().ok()
}

/// The box an `a:xfrm` stores; `None` without both `a:off` and `a:ext`.
pub fn read_xfrm(x: &Element) -> Option<Geom> {
    let (off, ext) = (x.child("a:off"), x.child("a:ext"));
    let on = |k: &str| x.get(k).is_some_and(|v| v == "1" || v == "true");
    Some(Geom {
        x: int(off, "x")?,
        y: int(off, "y")?,
        w: int(ext, "cx")?,
        h: int(ext, "cy")?,
        rot: x.get("rot").and_then(|v| v.parse().ok()).unwrap_or(0),
        flip_h: on("flipH"),
        flip_v: on("flipV"),
    })
}

/// An object's own box.
pub fn own(el: &Element) -> Option<Geom> {
    read_xfrm(xfrm(el)?)
}

/// Write `g` into an object's `a:xfrm`, making the element when it has none
/// (first in `p:spPr`, where the schema puts it). Only what differs is set,
/// so an attribute the file writes its own way stays.
pub fn write(el: &mut Element, g: &Geom) -> Result<(), String> {
    let name = xfrm_name(el);
    let frame = el.is("p:graphicFrame");
    let h: &mut Element = match el.name.as_str() {
        "p:graphicFrame" => el,
        "p:grpSp" => el.child_mut("p:grpSpPr").ok_or("a group without p:grpSpPr")?,
        _ => {
            if el.child("p:spPr").is_none() {
                return Err(format!("<{}> has no p:spPr to hold its position", el.name));
            }
            el.child_mut("p:spPr").unwrap()
        }
    };
    if h.child(name).is_none() {
        let x = Element::new(name);
        let at = if frame {
            // p:xfrm follows p:nvGraphicFramePr.
            h.children.iter().position(|n| matches!(n, Node::El(e) if e.is("p:nvGraphicFramePr"))).map_or(0, |k| k + 1)
        } else {
            0
        };
        h.children.insert(at, Node::El(x));
    }
    let x = h.child_mut(name).unwrap();
    let old = read_xfrm(x);
    let set_int = |x: &mut Element, child: &str, k: &str, v: i64| {
        if x.child(child).is_none() {
            let order = ["a:off", "a:ext", "a:chOff", "a:chExt"];
            let pos = order.iter().position(|o| *o == child).unwrap();
            let at = x
                .children
                .iter()
                .position(|n| matches!(n, Node::El(e) if order.iter().position(|o| e.is(o)).is_some_and(|q| q > pos)))
                .unwrap_or(x.children.len());
            x.children.insert(at, Node::El(Element::new(child)));
        }
        let c = x.child_mut(child).unwrap();
        if c.get(k).and_then(|s| s.parse::<i64>().ok()) != Some(v) {
            c.set(k, &v.to_string());
        }
    };
    set_int(x, "a:off", "x", g.x);
    set_int(x, "a:off", "y", g.y);
    set_int(x, "a:ext", "cx", g.w);
    set_int(x, "a:ext", "cy", g.h);
    let o = old.unwrap_or_default();
    if o.rot != g.rot || old.is_none() {
        if g.rot == 0 {
            x.remove_attr("rot");
        } else {
            x.set("rot", &g.rot.to_string());
        }
    }
    for (k, want, had) in [("flipH", g.flip_h, o.flip_h), ("flipV", g.flip_v, o.flip_v)] {
        if want != had || old.is_none() {
            if want {
                x.set(k, "1");
            } else {
                x.remove_attr(k);
            }
        }
    }
    Ok(())
}

/// Remove an object's own `a:xfrm`: a placeholder then sits where its layout puts it.
pub fn remove(el: &mut Element) {
    let name = xfrm_name(el);
    if let Some(h) = match el.name.as_str() {
        "p:graphicFrame" => Some(el),
        "p:grpSp" => el.child_mut("p:grpSpPr"),
        _ => el.child_mut("p:spPr"),
    } {
        h.children.retain(|n| !matches!(n, Node::El(e) if e.is(name)));
    }
}

/// A line's ends from its box: the flips say which corner it starts at.
pub fn ends_of(g: &Geom) -> Ends {
    let (x0, x1) = if g.flip_h { (g.x + g.w, g.x) } else { (g.x, g.x + g.w) };
    let (y0, y1) = if g.flip_v { (g.y + g.h, g.y) } else { (g.y, g.y + g.h) };
    Ends { from: (x0, y0), to: (x1, y1) }
}

/// The box and flips of a line from `from` to `to` (its rotation kept).
pub fn box_of(e: &Ends, rot: i64) -> Geom {
    let ((x0, y0), (x1, y1)) = (e.from, e.to);
    Geom { x: x0.min(x1), y: y0.min(y1), w: (x1 - x0).abs(), h: (y1 - y0).abs(), rot, flip_h: x1 < x0, flip_v: y1 < y0 }
}

/// A group's mapping from child coordinates to its parent's: `a:off`,
/// `a:ext`, `a:chOff`, `a:chExt` of its `a:xfrm`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub off: (i64, i64),
    pub ext: (i64, i64),
    pub ch_off: (i64, i64),
    pub ch_ext: (i64, i64),
}

impl Frame {
    /// The identity (the slide itself).
    pub const SLIDE: Frame = Frame { off: (0, 0), ext: (1, 1), ch_off: (0, 0), ch_ext: (1, 1) };

    /// A group's frame; `None` without a complete `a:xfrm` or with a zero child extent.
    pub fn of(grp: &Element) -> Option<Frame> {
        let x = xfrm(grp)?;
        let g = read_xfrm(x)?;
        let (co, ce) = (x.child("a:chOff"), x.child("a:chExt"));
        let f = Frame {
            off: (g.x, g.y),
            ext: (g.w, g.h),
            ch_off: (int(co, "x")?, int(co, "y")?),
            ch_ext: (int(ce, "cx")?, int(ce, "cy")?),
        };
        (f.ch_ext.0 > 0 && f.ch_ext.1 > 0).then_some(f)
    }

    fn scale(&self) -> (f64, f64) {
        (self.ext.0 as f64 / self.ch_ext.0 as f64, self.ext.1 as f64 / self.ch_ext.1 as f64)
    }

    /// A child box in the parent's coordinates.
    pub fn out(&self, g: &Geom) -> Geom {
        let (sx, sy) = self.scale();
        let r = |v: f64| v.round() as i64;
        Geom {
            x: self.off.0 + r((g.x - self.ch_off.0) as f64 * sx),
            y: self.off.1 + r((g.y - self.ch_off.1) as f64 * sy),
            w: r(g.w as f64 * sx),
            h: r(g.h as f64 * sy),
            ..*g
        }
    }

    /// A box in the parent's coordinates in child coordinates.
    pub fn back(&self, g: &Geom) -> Geom {
        let (sx, sy) = self.scale();
        let r = |v: f64| v.round() as i64;
        Geom {
            x: self.ch_off.0 + r((g.x - self.off.0) as f64 / sx),
            y: self.ch_off.1 + r((g.y - self.off.1) as f64 / sy),
            w: r(g.w as f64 / sx),
            h: r(g.h as f64 / sy),
            ..*g
        }
    }

    /// This frame seen from the slide, when its parent is `parent`.
    pub fn within(&self, parent: &Frame) -> Frame {
        let o = parent.out(&Geom { x: self.off.0, y: self.off.1, w: self.ext.0, h: self.ext.1, ..Default::default() });
        Frame { off: (o.x, o.y), ext: (o.w, o.h), ..*self }
    }
}

/// The smallest box around `boxes` (no rotation or flip).
pub fn union(boxes: &[Geom]) -> Option<Geom> {
    let x0 = boxes.iter().map(|g| g.x).min()?;
    let y0 = boxes.iter().map(|g| g.y).min()?;
    let x1 = boxes.iter().map(|g| g.x + g.w).max()?;
    let y1 = boxes.iter().map(|g| g.y + g.h).max()?;
    Some(Geom { x: x0, y: y0, w: x1 - x0, h: y1 - y0, ..Default::default() })
}

/// A shape's `p:cNvPr` id.
pub fn shape_id(el: &Element) -> Option<u32> {
    crate::pml::c_nv_pr(el)?.get("id")?.parse().ok()
}

/// The shape ids a connector's ends are attached to (`a:stCxn`, `a:endCxn`).
pub fn connections(cxn: &Element) -> Vec<(&'static str, u32)> {
    let Some(c) = cxn.child("p:nvCxnSpPr").and_then(|n| n.child("p:cNvCxnSpPr")) else { return vec![] };
    [("start", "a:stCxn"), ("end", "a:endCxn")]
        .into_iter()
        .filter_map(|(end, n)| Some((end, c.child(n)?.get("id")?.parse().ok()?)))
        .collect()
}

/// Scale a table's column widths and row heights to a frame of `w` × `h`
/// from `ow` × `oh` (§5.3: resizing a table scales its grid).
pub fn scale_table(frame: &mut Element, (ow, oh): (i64, i64), (w, h): (i64, i64)) {
    let mut tbl = None;
    frame.walk_mut(&mut |e| {
        if e.is("a:tbl") && tbl.is_none() {
            tbl = Some(());
            let f = |v: i64, a: i64, b: i64| if a > 0 { (v as f64 * b as f64 / a as f64).round() as i64 } else { v };
            for c in e.elements_mut() {
                if c.is("a:tblGrid") {
                    for col in c.elements_mut().filter(|x| x.is("a:gridCol")) {
                        if let Some(v) = col.get("w").and_then(|v| v.parse::<i64>().ok()) {
                            col.set("w", &f(v, ow, w).to_string());
                        }
                    }
                } else if c.is("a:tr") && h != oh {
                    if let Some(v) = c.get("h").and_then(|v| v.parse::<i64>().ok()) {
                        c.set("h", &f(v, oh, h).to_string());
                    }
                }
            }
        }
    });
}

/// An object without its position, size and table grid: what its
/// placeholder id is made from, so a moved or resized object keeps its id.
pub fn without_geometry(el: &Element) -> Element {
    let mut e = el.clone();
    e.walk_mut(&mut |x| {
        x.children.retain(|n| !matches!(n, Node::El(c) if c.is("a:xfrm") || c.is("p:xfrm")));
        if x.is("a:gridCol") {
            x.remove_attr("w");
        } else if x.is("a:tr") {
            x.remove_attr("h");
        }
    });
    e
}
