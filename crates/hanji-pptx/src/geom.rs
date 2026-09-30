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
    // The stored rotation stays when it turns the same way (a file may store
    // -90° as rot="-5400000"); a changed one is written from 0 up to a full turn.
    if !hanji_format::same_turn(o.rot, g.rot) || old.is_none() {
        let rot = g.rot.rem_euclid(hanji_format::FULL_TURN);
        if rot == 0 {
            x.remove_attr("rot");
        } else {
            x.set("rot", &rot.to_string());
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
    // A turned line's ends turn with it, about its box's centre.
    let c = ((g.x as f64) + g.w as f64 / 2.0, (g.y as f64) + g.h as f64 / 2.0);
    let t = |p: (i64, i64)| turned(p, c, g.rot);
    Ends { from: t((x0, y0)), to: t((x1, y1)), ..Default::default() }
}

/// `p` turned `rot` (60,000ths of a degree, clockwise) about `c`.
pub fn turned(p: (i64, i64), c: (f64, f64), rot: i64) -> (i64, i64) {
    if rot.rem_euclid(hanji_format::FULL_TURN) == 0 {
        return p;
    }
    let (s, k) = (rot as f64 / 60_000.0).to_radians().sin_cos();
    let (dx, dy) = (p.0 as f64 - c.0, p.1 as f64 - c.1);
    ((c.0 + dx * k - dy * s).round() as i64, (c.1 + dx * s + dy * k).round() as i64)
}

/// The box and flips of a line from `from` to `to`, turned `rot` (kept):
/// the box about the ends' midpoint whose turned corners they are.
pub fn box_of(e: &Ends, rot: i64) -> Geom {
    let ((x0, y0), (x1, y1)) = (e.from, e.to);
    if rot.rem_euclid(hanji_format::FULL_TURN) == 0 {
        return Geom {
            x: x0.min(x1),
            y: y0.min(y1),
            w: (x1 - x0).abs(),
            h: (y1 - y0).abs(),
            rot,
            flip_h: x1 < x0,
            flip_v: y1 < y0,
        };
    }
    let c = ((x0 + x1) as f64 / 2.0, (y0 + y1) as f64 / 2.0);
    let (s, k) = (-(rot as f64) / 60_000.0).to_radians().sin_cos();
    let (dx, dy) = ((x1 - x0) as f64, (y1 - y0) as f64);
    let (ux, uy) = (dx * k - dy * s, dx * s + dy * k);
    let (w, h) = (ux.abs().round() as i64, uy.abs().round() as i64);
    let x = (c.0 - w as f64 / 2.0).round() as i64;
    let y = (c.1 - h as f64 / 2.0).round() as i64;
    Geom { x, y, w, h, rot, flip_h: ux < -0.5, flip_v: uy < -0.5 }
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

/// A connector's two ends' attachments, each a shape id and site index.
pub type Attachments = [Option<(u32, u32)>; 2];

/// A connector's ends' attachments: the shape id and connection site index
/// of its `a:stCxn` and `a:endCxn`.
pub fn attachments(cxn: &Element) -> Attachments {
    let c = cxn.child("p:nvCxnSpPr").and_then(|n| n.child("p:cNvCxnSpPr"));
    let at = |n: &str| {
        let e = c?.child(n)?;
        Some((e.get("id")?.parse().ok()?, e.get("idx")?.parse().ok()?))
    };
    [at("a:stCxn"), at("a:endCxn")]
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

/// A top-level object's box for snapping (§5.3): the one stored (exact EMU;
/// `None` for a new object) and the one the text writes (`None` without one).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Written {
    pub stored: Option<Geom>,
    pub written: Option<Geom>,
}

fn field(g: &Geom, k: usize) -> i64 {
    [g.x, g.y, g.w, g.h][k]
}

fn field_mut(g: &mut Geom, k: usize) -> &mut i64 {
    match k {
        0 => &mut g.x,
        1 => &mut g.y,
        2 => &mut g.w,
        _ => &mut g.h,
    }
}

/// The stored value of number `k` (x, y, w, h) of `b` when the text leaves it as shown.
fn kept(b: &Written, k: usize) -> Option<i64> {
    let (s, w) = (b.stored?, b.written?);
    let v = field(&s, k);
    (hanji_format::shown_pt(v) * hanji_format::EMU_PER_PT == field(&w, k)).then_some(v)
}

/// The box `boxes[n]` writes, snapped (§5.3): a changed number (x, y, w or h)
/// that another object shows for the same number, left as shown there, takes
/// that object's exact value. Same number, same position. When several do and
/// their exact values differ, the nearest in z-order wins, the one beneath on
/// a tie. A number left as shown is not snapped: it keeps its own exact value.
pub fn snapped(boxes: &[Written], n: usize) -> Option<Geom> {
    let b = boxes.get(n)?;
    let mut w = b.written?;
    for k in 0..4 {
        if kept(b, k).is_some() {
            continue;
        }
        let want = field(&w, k);
        let nearest = (1..boxes.len())
            .flat_map(|d| [n.checked_sub(d), Some(n + d)])
            .flatten()
            .filter_map(|m| boxes.get(m))
            .filter_map(|o| kept(o, k))
            .find(|&v| hanji_format::shown_pt(v) * hanji_format::EMU_PER_PT == want);
        if let Some(v) = nearest {
            *field_mut(&mut w, k) = v;
        }
    }
    Some(w)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PT: i64 = hanji_format::EMU_PER_PT;

    fn b(x: i64, y: i64, w: i64, h: i64) -> Geom {
        Geom { x, y, w, h, ..Default::default() }
    }

    fn kept_as_shown(g: Geom) -> Written {
        Written { stored: Some(g), written: Some(g.shown()) }
    }

    #[test]
    fn a_changed_number_shown_by_another_object_takes_its_exact_value() {
        // Kit file 40 (txt-font-props, P15): TextBox 1's x is 2952093 EMU
        // (232.45 pt, shown 232); TextBox 2 is written at x 232.
        let one = b(2952093, 1_000_000, 3_000_000, 500_000);
        let two = b(1_000_000, 2_000_000, 3_000_000, 500_000);
        let boxes =
            [kept_as_shown(one), Written { stored: Some(two), written: Some(Geom { x: 232 * PT, ..two.shown() }) }];
        let g = snapped(&boxes, 1).unwrap();
        assert_eq!(g.x, 2952093);
        // Its other numbers are its own as shown; place_box keeps them exact.
        assert_eq!((g.y, g.w, g.h), (two.shown().y, two.shown().w, two.shown().h));
        // The same for y, w and h, each against the same number only.
        let other = b(10 * PT, 100 * PT + 3000, 200 * PT + 4000, 50 * PT - 2000);
        let mine = b(400 * PT, 300 * PT, 20 * PT, 20 * PT);
        let boxes = [
            kept_as_shown(other),
            Written { stored: Some(mine), written: Some(b(400 * PT, 100 * PT, 200 * PT, 50 * PT)) },
        ];
        assert_eq!(snapped(&boxes, 1).unwrap(), b(400 * PT, 100 * PT + 3000, 200 * PT + 4000, 50 * PT - 2000));
        // x never snaps to a y, a w or an h.
        let boxes = [
            kept_as_shown(other),
            Written { stored: Some(mine), written: Some(b(100 * PT, 300 * PT, 20 * PT, 20 * PT)) },
        ];
        assert_eq!(snapped(&boxes, 1).unwrap().x, 100 * PT);
        // A new object (nothing stored) snaps too.
        let boxes = [kept_as_shown(one), Written { stored: None, written: Some(b(232 * PT, 0, 10 * PT, 10 * PT)) }];
        assert_eq!(snapped(&boxes, 1).unwrap().x, 2952093);
    }

    #[test]
    fn only_numbers_left_as_shown_are_snapped_to() {
        let one = b(2952093, 0, 100 * PT, 100 * PT);
        let two = b(0, 200 * PT, 100 * PT, 100 * PT);
        // TextBox 1 moves away in the same edit: its old x is nowhere, so 232 is 232.
        let boxes = [
            Written { stored: Some(one), written: Some(Geom { x: 300 * PT, ..one.shown() }) },
            Written { stored: Some(two), written: Some(Geom { x: 232 * PT, ..two.shown() }) },
        ];
        assert_eq!(snapped(&boxes, 1).unwrap().x, 232 * PT);
        // A number left as shown keeps its own exact value, never another's.
        let mine = b(2946400 + 3000, 0, PT, PT);
        let boxes = [kept_as_shown(one), kept_as_shown(mine)];
        assert_eq!(snapped(&boxes, 1).unwrap().x, mine.shown().x);
        // An object without a written box is no anchor.
        let boxes = [
            Written { stored: Some(one), written: None },
            Written { stored: Some(two), written: Some(Geom { x: 232 * PT, ..two.shown() }) },
        ];
        assert_eq!(snapped(&boxes, 1).unwrap().x, 232 * PT);
    }

    #[test]
    fn the_nearest_in_z_order_wins_and_the_one_beneath_on_a_tie() {
        let at = |x: i64| kept_as_shown(b(x, 0, PT, PT));
        let me = Written { stored: Some(b(0, 300 * PT, PT, PT)), written: Some(b(232 * PT, 300 * PT, PT, PT)) };
        let (far, below, above) = (at(232 * PT - 5000), at(232 * PT + 3000), at(232 * PT + 6000));
        assert_eq!(snapped(&[far, below, me, at(9 * PT), above], 2).unwrap().x, 232 * PT + 3000);
        assert_eq!(snapped(&[far, at(9 * PT), me, above], 2).unwrap().x, 232 * PT + 6000);
        assert_eq!(snapped(&[above, me, below], 1).unwrap().x, 232 * PT + 6000);
        assert_eq!(snapped(&[me, at(9 * PT), far], 0).unwrap().x, 232 * PT - 5000);
    }
}
