//! The Presentation grammar (DESIGN.md §5.3): slides separated by a line
//! `---`, each opened by `layout: Name`, then its objects in z-order: slot
//! markers (`::title box="…"::`), `<shape>`, `<keep/>`, `<line/>` and
//! `<group>` lines. A slot's text is the Document grammar's paragraphs, list
//! items, empty paragraphs and placeholders (§5.1), parsed by the same code.
//! Boxes are in points in the text and in EMU in the AST.

use crate::ast::*;
use crate::diag::{quoted, Diagnostic};
use crate::inline_style::{self as style, TextStyle};
use crate::names::{Layout, Names};
use crate::parse::{chars_of, parse_tag, BlockMap, BlockMapKind, ParaMap, Parser, Tag};
use crate::serialize::{attr, blocks_with, cell_text_with, front_lines, keep_tag};

/// Where a head line is: its byte range, and the offset that stands for it
/// (its line end; for a shape, its `<shape` tag).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HeadMap {
    pub start: usize,
    pub end: usize,
    pub mark: usize,
}

/// Where every slide, slot, shape, line, group and block of a presentation is in its text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PresentationMap {
    pub slides: Vec<SlideMap>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SlideMap {
    /// The `layout:` line.
    pub head: HeadMap,
    pub items: Vec<ItemMap>,
}

/// A slot (its marker line and blocks), a shape (its tag and one block per
/// paragraph), an object (its tag and placeholder), a line (its tag) or a
/// group (all its lines, no blocks).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ItemMap {
    pub head: HeadMap,
    pub blocks: Vec<BlockMap>,
}

#[derive(Clone, Debug)]
pub struct ParsedPresentation {
    pub pres: Presentation,
    pub map: PresentationMap,
}

/// Parse a Presentation, checking layouts, slots, shapes and placeholders
/// against `names` (a `None` list skips its check).
pub fn parse_presentation(text: &str, names: &Names) -> Result<ParsedPresentation, Vec<Diagnostic>> {
    let mut p = Parser::new(text, names);
    let (front, first) = p.front_matter("presentation");
    let Some(front) = front.filter(|_| p.errors.is_empty()) else { return Err(p.errors) };
    let n = p.lines.len();
    let seps: Vec<usize> = (first..n).filter(|&i| is_separator(p.lines[i].text)).collect();
    let mut segs = vec![];
    let mut from = first;
    for &s in &seps {
        segs.push((from, s));
        from = s + 1;
    }
    segs.push((from, n));
    let blank = |p: &Parser, a: usize, b: usize| (a..b).all(|i| p.lines[i].text.trim().is_empty());
    let mut pres = Presentation { front, slides: vec![] };
    let mut map = PresentationMap::default();
    if seps.is_empty() && blank(&p, from, n) {
        // No slides at all.
        p.check_names_after();
        return if p.errors.is_empty() { Ok(ParsedPresentation { pres, map }) } else { Err(p.errors) };
    }
    for (k, &(a, b)) in segs.iter().enumerate() {
        if blank(&p, a, b) {
            let (line, msg) = if k == 0 {
                (seps[0], "a line --- separates two slides, and there is no slide before this one: the first slide begins right after the front matter, with its layout: line. Delete this ---.")
            } else if k + 1 == segs.len() {
                (seps[k - 1], "a line --- separates two slides, and no slide follows this one. Delete this ---.")
            } else {
                (
                    seps[k],
                    "two lines --- with no slide between them; every slide begins with layout: Name. Delete one ---.",
                )
            };
            p.err(line, 1, msg);
            continue;
        }
        // Slidev closes a slide's front matter with a second `---`.
        let slidev = k > 0 && {
            let (pa, pb) = segs[k - 1];
            (pa..pb).any(|i| layout_value(p.lines[i].text.trim()).is_some())
                && (pa..pb).all(|i| p.lines[i].text.trim().is_empty() || key_value(p.lines[i].text.trim()).is_some())
        };
        let before = p.errors.len();
        let parsed = slide(&mut p, a, b, slidev.then(|| seps[k - 1]));
        if let Some((s, m)) = parsed.filter(|_| p.errors.len() == before) {
            pres.slides.push(s);
            map.slides.push(m);
        }
    }
    p.check_names_after();
    if !p.errors.is_empty() {
        return Err(p.errors);
    }
    pres.normalize();
    Ok(ParsedPresentation { pres, map })
}

/// A line `---` (a slide separator).
fn is_separator(line: &str) -> bool {
    line.trim() == "---"
}

/// `layout: Name` (quotes allowed) → the name.
fn layout_value(t: &str) -> Option<String> {
    let v = t.strip_prefix("layout:")?.trim();
    let unq = |q: char| v.len() >= 2 && v.starts_with(q) && v.ends_with(q);
    Some(if unq('"') || unq('\'') { v[1..v.len() - 1].to_string() } else { v.to_string() })
}

/// A `key: value` line: its key.
fn key_value(t: &str) -> Option<&str> {
    let (k, rest) = t.split_once(':')?;
    let ok = k.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    (ok && (rest.is_empty() || rest.starts_with(' '))).then_some(k)
}

/// `::name::` or `::name box="…"::` → what is between the colons
/// (possibly malformed; the caller checks it).
fn marker(t: &str) -> Option<&str> {
    let inner = t.strip_prefix("::")?.strip_suffix("::")?;
    (!t.is_empty() && t.len() >= 4).then_some(inner)
}

/// A marker's inside → its name and the byte offset of its attributes.
fn marker_parts(inner: &str) -> (&str, usize) {
    match inner.find(char::is_whitespace) {
        Some(k) => (&inner[..k], k),
        None => (inner, inner.len()),
    }
}

fn valid_slot_name(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn is_shape_line(t: &str) -> bool {
    t.starts_with("<shape") && t[6..].starts_with([' ', '>', '/'])
}

fn is_line_line(t: &str) -> bool {
    t.starts_with("<line") && t[5..].starts_with([' ', '>', '/'])
}

fn is_picture_line(t: &str) -> bool {
    t.starts_with("<picture") && t[8..].starts_with([' ', '>', '/'])
}

fn is_group_open(t: &str) -> bool {
    t.starts_with("<group") && t[6..].starts_with([' ', '>'])
}

fn is_group_close(t: &str) -> bool {
    t.starts_with("</group")
}

// ---------------------------------------------------------------- geometry

/// A number of points (`36`, `12.5`, `-3`) → EMU.
pub fn parse_pt(s: &str) -> Option<i64> {
    let emu = parse_num(s)? * EMU_PER_PT as f64;
    (emu.abs() < 1e13).then(|| emu.round() as i64)
}

/// A plain decimal number: digits, an optional sign and fraction.
fn parse_num(s: &str) -> Option<f64> {
    let u = s.strip_prefix('-').unwrap_or(s);
    let ok = u.starts_with(|c: char| c.is_ascii_digit())
        && u.chars().all(|c| c.is_ascii_digit() || c == '.')
        && u.matches('.').count() <= 1
        && !u.ends_with('.');
    s.parse().ok().filter(|_| ok)
}

/// EMU → the whole points the text shows.
pub fn pt(emu: i64) -> String {
    shown_pt(emu).to_string()
}

/// `x y` → a point in EMU.
fn parse_point(v: &str) -> Option<(i64, i64)> {
    let n: Vec<&str> = v.split_whitespace().collect();
    match n.as_slice() {
        [x, y] => Some((parse_pt(x)?, parse_pt(y)?)),
        _ => None,
    }
}

const BOX_FORM: &str = "box=\"x y w h\": the left edge, top edge, width and height in points from the slide's top-left corner, such as box=\"36 22 648 90\"";

/// `box`, `rot` and `flip` attributes → the geometry they write; the error
/// is at a column. `None` without a box.
pub(crate) fn geom_of(attrs: &[(String, String, usize)], col: usize) -> Result<Option<Geom>, (usize, String)> {
    let get = |k: &str| attrs.iter().find(|a| a.0 == k);
    let Some((_, b, bcol)) = get("box") else {
        return match attrs.first() {
            Some((k, _, c)) => Err((*c, format!("{k}=\"…\" goes with a box: write {BOX_FORM}, then {k}."))),
            None => Ok(None),
        };
    };
    let n: Vec<Option<i64>> = b.split_whitespace().map(parse_pt).collect();
    let [Some(x), Some(y), Some(w), Some(h)] = n.as_slice() else {
        return Err((*bcol, format!("box=\"{b}\" is not a box; it is {BOX_FORM}.")));
    };
    if *w < 0 || *h < 0 {
        return Err((
            *bcol,
            format!("box=\"{b}\" has a negative width or height; a box's width and height are 0 or more."),
        ));
    }
    let mut g = Geom { x: *x, y: *y, w: *w, h: *h, ..Default::default() };
    if let Some((_, r, rcol)) = get("rot") {
        match parse_num(r) {
            Some(d) if (-360.0..=360.0).contains(&d) => g.rot = ((d * 60_000.0).round() as i64).rem_euclid(FULL_TURN),
            _ => {
                return Err((
                    *rcol,
                    format!("rot=\"{r}\" is not a rotation; it is degrees clockwise, such as rot=\"15\"."),
                ))
            }
        }
    }
    if let Some((_, f, fcol)) = get("flip") {
        match f.as_str() {
            "h" => g.flip_h = true,
            "v" => g.flip_v = true,
            "hv" | "vh" => (g.flip_h, g.flip_v) = (true, true),
            _ => {
                return Err((
                    *fcol,
                    format!("flip=\"{f}\" is not a flip; it is flip=\"h\", flip=\"v\" or flip=\"hv\"."),
                ))
            }
        }
    }
    let _ = col;
    Ok(Some(g))
}

/// A tag's attributes split into the named ones (`id`, `name`, …, each once)
/// and its geometry (`box`, `rot`, `flip`); an unknown one is an error.
#[allow(clippy::type_complexity)]
fn split_attrs(
    tag: &Tag,
    named: &[&str],
    geometry: bool,
    form: &str,
) -> Result<(Vec<Option<String>>, Option<Geom>), (usize, String)> {
    split_attrs_of(&tag.attrs, &tag.name, tag.col, named, geometry, form)
}

/// [`split_attrs`] of a tag's text formatting too (§5.3): the style its
/// `font`, `size` and `color` give.
#[allow(clippy::type_complexity)]
fn split_styled_attrs(
    tag: &Tag,
    named: &[&str],
    form: &str,
) -> Result<(Vec<Option<String>>, Option<Geom>, TextStyle), (usize, String)> {
    let (st, rest) = style::split_text_keys(&tag.attrs)?;
    let (vals, geom) = split_attrs_of(&rest, &tag.name, tag.col, named, true, form)?;
    Ok((vals, geom, st))
}

#[allow(clippy::type_complexity)]
fn split_attrs_of(
    attrs: &[(String, String, usize)],
    tag_name: &str,
    tag_col: usize,
    named: &[&str],
    geometry: bool,
    form: &str,
) -> Result<(Vec<Option<String>>, Option<Geom>), (usize, String)> {
    let mut vals: Vec<Option<String>> = vec![None; named.len()];
    let mut geo = vec![];
    for (k, v, col) in attrs {
        if let Some(n) = named.iter().position(|x| x == k) {
            if vals[n].is_some() {
                return Err((*col, format!("{k}=\"…\" is written twice: {form}.")));
            }
            vals[n] = Some(v.clone());
        } else if geometry && matches!(k.as_str(), "box" | "rot" | "flip") {
            geo.push((k.clone(), v.clone(), *col));
        } else {
            return Err((*col, format!("<{tag_name}> has no attribute \"{k}\": {form}.")));
        }
    }
    Ok((vals, geom_of(&geo, tag_col)?))
}

/// ` box="…"`, and ` rot="…"` and ` flip="…"` when set.
pub fn geom_attrs(g: &Option<Geom>) -> String {
    let Some(g) = g else { return String::new() };
    let mut out = format!(" box=\"{} {} {} {}\"", pt(g.x), pt(g.y), pt(g.w), pt(g.h));
    // Within one turn, as the parse reads it back: a file's rot="-5400000"
    // shows as rot="270" (the stored value stays while it is left so, §5.3).
    let deg = shown_deg(g.rot).rem_euclid(360);
    if deg != 0 {
        out.push_str(&format!(" rot=\"{deg}\""));
    }
    let flip = match (g.flip_h, g.flip_v) {
        (true, true) => "hv",
        (true, false) => "h",
        (false, true) => "v",
        _ => "",
    };
    if !flip.is_empty() {
        out.push_str(&format!(" flip=\"{flip}\""));
    }
    out
}

/// Parse `key="value"` attributes after a marker's name: (key, value, column).
/// Attributes as parsed: (name, value, column).
type Attrs = Vec<(String, String, usize)>;

fn marker_attrs(src: &str, col0: usize) -> Result<Attrs, (usize, String)> {
    match crate::parse::loose_attrs(src) {
        Some(a) => Ok(a.into_iter().map(|(k, v, c)| (k, v, col0 + c - 1)).collect()),
        None => Err((
            col0,
            format!("a slot marker's attributes are written key=\"value\" or key=value, such as ::title {BOX_FORM} size=24pt::."),
        )),
    }
}

/// A line holding one `<keep …/>` tag and nothing else: the tag's id.
fn keep_line_id(p: &Parser, i: usize) -> Option<String> {
    let line = &p.lines[i];
    if !line.text.trim().starts_with("<keep") {
        return None;
    }
    let src = chars_of(line, line.indent());
    let (tag, after) = parse_tag(&src, 0)?;
    let alone = tag.name == "keep" && tag.self_closing && src[after..].iter().all(|c| c.2.is_whitespace());
    alone.then(|| tag.attrs.iter().find(|a| a.0 == "id").map(|a| a.1.clone())).flatten()
}

/// Whether line `i` is a slide object's `<keep/>` line: one with a box, one
/// outside a slot, or one inside a slot when its id is one of the file's
/// objects (a slot's own `<keep/>` is not; with no list of objects, none
/// without a box inside a slot is).
fn is_object_line(p: &Parser, i: usize, in_slot: bool) -> bool {
    // A `<keep/>` with a box is a slide object: a slot's own has its box on the marker.
    let boxed = || line_tag(p, i).is_some_and(|t| t.0.attrs.iter().any(|a| a.0 == "box"));
    match (keep_line_id(p, i), &p.names.objects) {
        (Some(_), _) if boxed() => true,
        (Some(id), Some(objects)) => !in_slot || objects.contains(&id),
        (Some(_), None) => !in_slot,
        (None, _) => false,
    }
}

const SLOT_TEXT: &str = "Slot text is plain lines, - and 1. list items, <p/> empty paragraphs and <keep/> placeholders";

/// A slide object's `<keep/>` line: the head stands on the tag's `<`, the
/// placeholder block on the rest of the line (distinct offsets, so an exact
/// span tells them apart).
/// A slide object's `<keep/>` line: the head stands on the tag's `<`, the
/// placeholder block on the rest of the line (distinct offsets, so an exact
/// span tells them apart).
fn object(p: &mut Parser, i: usize) -> Option<(ObjectItem, ItemMap)> {
    p.object_line = true;
    p.object_geom = None;
    let parsed = p.block(i);
    p.object_line = false;
    let geom = p.object_geom.take();
    let (b, m, _) = parsed?;
    let keep = match b {
        Block::Para(Para { style: ParaStyle::Plain, content, .. }) if content.units.len() == 1 => {
            match &content.units[0].atom {
                Atom::Keep(k) => k.clone(),
                _ => return None,
            }
        }
        Block::Keep(k) => k,
        _ => return None,
    };
    let line = &p.lines[i];
    let tag = line.at + line.indent();
    let head = HeadMap { start: line.at, end: tag + 1, mark: tag };
    let mark = match &m.kind {
        BlockMapKind::Para(pm) => pm.mark,
        _ => line.end,
    };
    let block =
        BlockMap { start: tag + 1, end: m.end, kind: BlockMapKind::Para(ParaMap { units: vec![tag + 1], mark }) };
    Some((ObjectItem { keep, geom }, ItemMap { head, blocks: vec![block] }))
}

fn slot_list(slots: &[String]) -> String {
    slots.iter().map(|s| format!("::{s}::")).collect::<Vec<_>>().join(", ")
}

/// Whether line `j` ends the slot before it: another object or slot.
fn ends_slot(p: &Parser, j: usize) -> bool {
    let t = p.lines[j].text.trim();
    marker(t).is_some()
        || is_shape_line(t)
        || is_line_line(t)
        || is_picture_line(t)
        || is_group_open(t)
        || is_group_close(t)
        || is_object_line(p, j, true)
}

/// One slide from lines `a..b`; `slidev` is the line of a `---` that closed
/// a Slidev-style front matter just before.
fn slide(p: &mut Parser, a: usize, b: usize, slidev: Option<usize>) -> Option<(Slide, SlideMap)> {
    let names = p.names;
    let i0 = (a..b).find(|&i| !p.lines[i].text.trim().is_empty())?;
    let layouts = names.layouts.as_deref();
    let layout_names = || layouts.map(|l| quoted(&l.iter().map(|x| x.name.clone()).collect::<Vec<_>>()));
    let Some(layout) = layout_value(p.lines[i0].text.trim()) else {
        let list = layout_names().map(|l| format!(" Layouts: {l}.")).unwrap_or_default();
        match slidev {
            Some(sep) => p.err(sep, 1, format!("this --- starts a new slide, and a slide begins with layout: Name. Slidev closes a slide's front matter with a second ---; here the layout: line is the slide's first line and needs no closing line. Delete this ---.{list}")),
            None => p.err(i0, 1, format!("every slide begins with a line layout: Name, the name of one of the file's layouts.{list}")),
        }
        return None;
    };
    let lay: Option<&Layout> = layouts.and_then(|l| l.iter().find(|x| x.name == layout));
    if let (Some(_), None) = (layouts, lay) {
        let msg = format!(
            "layout: {layout} is not a layout of this file; the name is written as listed, spaces included. Layouts: {}.",
            layout_names().unwrap()
        );
        p.err(i0, 1, msg);
        return None;
    }
    let slots: Option<Vec<String>> = lay.map(|l| l.slots.iter().cloned().chain(["notes".to_string()]).collect());
    let line = &p.lines[i0];
    let head = HeadMap { start: line.at, end: line.next, mark: line.end };
    let (mut items, mut maps) = (vec![], vec![]);
    let mut seen: Vec<String> = vec![];
    let mut i = i0 + 1;
    while i < b {
        let t = p.lines[i].text.trim();
        if t.is_empty() {
            i += 1;
            continue;
        }
        if let Some(inner) = marker(t) {
            let inner = inner.to_string();
            let end = (i + 1..b).find(|&j| ends_slot(p, j)).unwrap_or(b);
            if let Some((slot, m)) = slot(p, i, end, &inner, slots.as_deref(), &layout, &mut seen) {
                items.push(SlideItem::Slot(slot));
                maps.push(m);
            }
            i = end;
        } else if is_shape_line(t) {
            if let Some((sh, m)) = shape(p, i, &mut seen, false) {
                items.push(SlideItem::Shape(sh));
                maps.push(m);
            }
            i += 1;
        } else if is_line_line(t) {
            if let Some((l, m)) = line_item(p, i, &mut seen, false) {
                items.push(SlideItem::Line(l));
                maps.push(m);
            }
            i += 1;
        } else if is_picture_line(t) {
            if let Some((pic, m)) = picture(p, i, &mut seen, false) {
                items.push(SlideItem::Picture(pic));
                maps.push(m);
            }
            i += 1;
        } else if is_group_open(t) {
            let Some(close) = group_close(p, i, b) else {
                p.err(i, 1, "this <group> is not closed: a group is a <group …> line, its objects' lines, then a line </group>, on one slide.");
                break;
            };
            if let Some((g, m)) = group(p, i, close, &mut seen, false) {
                items.push(SlideItem::Group(g));
                maps.push(m);
            }
            i = close + 1;
        } else if is_group_close(t) {
            p.err(i, 1, "</group> without a <group …> line before it: a group is a <group …> line, its objects' lines, then </group>.");
            i += 1;
        } else if is_object_line(p, i, false) {
            if let Some((k, m)) = object(p, i) {
                items.push(SlideItem::Object(k));
                maps.push(m);
            }
            i += 1;
        } else {
            let msg = match key_value(t) {
                Some("layout") if items.is_empty() => {
                    format!("this slide already has its layout: line (line {}); a slide has one.", i0 + 1)
                }
                Some(k) if items.is_empty() => format!(
                    "\"{k}:\" is not a slide setting: a slide has one key: value line, layout: Name, and it is the slide's first line. Put text in a slot, after a marker such as ::title::."
                ),
                _ => {
                    let list = slots.as_deref().map(|s| format!(" This slide's slots: {}.", slot_list(s))).unwrap_or_default();
                    format!("text outside a slot: every line of a slide after its layout: line belongs to a slot, after its marker line such as ::title:: or ::body::, or is an object's line: <shape>, <picture/>, <keep/>, <line/> or <group>.{list}")
                }
            };
            p.err(i, 1, msg);
            i += 1;
        }
    }
    Some((Slide { layout, items }, SlideMap { head, items: maps }))
}

/// The line `</group>` that closes the group opened on line `i` (before `b`).
fn group_close(p: &Parser, i: usize, b: usize) -> Option<usize> {
    let mut depth = 0;
    for j in i..b {
        let t = p.lines[j].text.trim();
        if is_group_open(t) {
            depth += 1;
        } else if is_group_close(t) {
            depth -= 1;
            if depth == 0 {
                return Some(j);
            }
        }
    }
    None
}

/// The tag a line starts with, the index after it, and the line's chars.
fn line_tag(p: &Parser, i: usize) -> Option<(Tag, usize, Vec<crate::parse::Src>)> {
    let line = &p.lines[i];
    let src = chars_of(line, line.indent());
    let (tag, after) = parse_tag(&src, 0)?;
    Some((tag, after, src))
}

/// Nothing but spaces after `after` on the line, else the error's column.
fn rest_blank(src: &[crate::parse::Src], after: usize) -> Result<(), usize> {
    match src[after..].iter().find(|c| !c.2.is_whitespace()) {
        Some(c) => Err(c.1),
        None => Ok(()),
    }
}
fn slot(
    p: &mut Parser,
    i: usize,
    end: usize,
    inner: &str,
    allowed: Option<&[String]>,
    layout: &str,
    seen: &mut Vec<String>,
) -> Option<(Slot, ItemMap)> {
    let before = p.errors.len();
    let (name, at) = marker_parts(inner);
    let geom = {
        let line = &p.lines[i];
        let lead = line.indent();
        let col0 = line.text[..lead].chars().count() + 2 + inner[..at].chars().count() + 1;
        match marker_attrs(&inner[at..], col0).and_then(|a| style::split_text_keys(&a)) {
            Ok((st, attrs)) => match attrs.iter().find(|a| !matches!(a.0.as_str(), "box" | "rot" | "flip")) {
                Some((k, _, col)) => Err((
                    *col,
                    format!("a slot marker has no attribute \"{k}\"; it is ::{name}:: or ::{name} {BOX_FORM}::, with the text's font, size and color after the box."),
                )),
                None => geom_of(&attrs, col0).map(|g| (g, st)),
            },
            Err(e) => Err(e),
        }
    };
    let (geom, obj_style) = match geom {
        Ok((Some(_), _)) if name == "notes" => {
            p.err(i, 1, "::notes:: has no box: speaker notes are not on the slide. Write the marker ::notes::.");
            (None, TextStyle::default())
        }
        Ok((_, st)) if name == "notes" && !st.is_empty() => {
            p.err(i, 1, "::notes:: shows no formatting: speaker notes keep theirs. Write the marker ::notes::.");
            (None, TextStyle::default())
        }
        Ok(g) => g,
        Err((col, msg)) => {
            p.err(i, col, msg);
            (None, TextStyle::default())
        }
    };
    if !valid_slot_name(name) {
        let lower = name.trim().to_ascii_lowercase();
        let hint = if valid_slot_name(&lower) && lower != name {
            format!(" Slot names are lower case: ::{lower}::.")
        } else {
            String::new()
        };
        p.err(
            i,
            1,
            format!("a slot marker is a line ::name:: holding the slot's name, such as ::title:: or ::body::.{hint}"),
        );
    } else if let Some(allowed) = allowed.filter(|a| !a.iter().any(|s| s == name)) {
        p.err(
            i,
            1,
            format!(
                "::{name}:: is not a slot of layout \"{layout}\": a slide has only its layout's slots. Its slots: {}.",
                slot_list(allowed)
            ),
        );
    } else if seen.iter().any(|s| s == name) {
        p.err(
            i,
            1,
            format!("::{name}:: appears twice on this slide; a slot is written once, all its text after its marker."),
        );
    }
    seen.push(name.to_string());
    let line = &p.lines[i];
    let head = HeadMap { start: line.at, end: line.next, mark: line.end };
    let (mut bl, mut bm) = (vec![], vec![]);
    p.stop = end;
    p.text_styles = name != "notes";
    let mut j = i + 1;
    while j < end {
        let before_block = p.errors.len();
        match p.block(j) {
            Some((b, m, next)) => {
                if p.errors.len() == before_block {
                    if let Some(msg) = not_slot_text(&b) {
                        p.err(j, 1, format!("{msg} {SLOT_TEXT}."));
                    } else {
                        bl.push(b);
                        bm.push(m);
                    }
                }
                j = next;
            }
            None => j += 1,
        }
    }
    p.stop = p.lines.len();
    p.text_styles = false;
    if p.errors.len() > before {
        return None;
    }
    // The marker's formatting is beneath each paragraph's (§5.3).
    let mut inls = slot_inlines(&mut bl);
    style::fold_under(&mut inls, &obj_style);
    style::normalize(&mut inls);
    if bl.is_empty() {
        p.err(i, 1, format!("::{name}:: has no text. An unfilled slot is left out: delete this marker line, or write the slot's text after it."));
        return None;
    }
    Some((Slot { name: name.to_string(), geom, blocks: bl }, ItemMap { head, blocks: bm }))
}

/// Why a block cannot be slot text.
fn not_slot_text(b: &Block) -> Option<&'static str> {
    Some(match b {
        Block::Para(Para { style: ParaStyle::Heading(_), .. }) => {
            "a heading (#) is not slot text: a slide's title is its ::title:: slot."
        }
        Block::Para(Para { style: ParaStyle::Named(_), .. }) => {
            "a slide has no paragraph styles: formatting comes from the layout, so a slot holds no <div style> or <p style>."
        }
        Block::Table(_) => "a table cannot be written in a slot yet; a slide's tables stay in the file as they are.",
        Block::PageBreak => "a slide has no page breaks; slides are separated by a line ---.",
        Block::FootnoteDef(_) => "a presentation has no footnotes.",
        Block::Para(_) | Block::List(_) | Block::Keep(_) => return None,
    })
}

const SHAPE_FORM: &str = "a shape line is <shape id=\"…\" name=\"…\" box=\"…\">text</shape>, its id and name as they are in the file; a shape without text is <shape id=\"…\" name=\"…\" box=\"…\"/>, and a new text box is <shape box=\"…\">text</shape>";

/// A `<shape …>text</shape>` or `<shape …/>` line; `member`: one of a group's objects.
fn shape(p: &mut Parser, i: usize, seen: &mut Vec<String>, member: bool) -> Option<(ShapeText, ItemMap)> {
    let form = SHAPE_FORM;
    let line = &p.lines[i];
    let (at, next, lead) = (line.at, line.next, line.indent());
    let Some((tag, after, src)) = line_tag(p, i).filter(|t| !t.0.closing) else {
        p.err(i, lead + 1, format!("expected {form}."));
        return None;
    };
    let (vals, geom, obj_style) = match split_styled_attrs(&tag, &["id", "name"], form) {
        Ok(v) => v,
        Err((col, msg)) => {
            p.err(i, col, msg);
            return None;
        }
    };
    let (id, name) = match (&vals[0], &vals[1]) {
        (Some(id), Some(name)) => (id.clone(), name.clone()),
        (None, None) if member => {
            p.err(i, tag.col, "a group's objects are never added here: a new text box goes outside the group.");
            return None;
        }
        (None, None) => (String::new(), String::new()),
        _ => {
            p.err(i, tag.col, format!("<shape> needs both id and name, or neither for a new text box: {form}."));
            return None;
        }
    };
    let new = id.is_empty();
    if new && geom.is_none() {
        p.err(i, tag.col, "a new text box needs its box: <shape box=\"x y w h\">text</shape>.");
        return None;
    }
    let (paras, pmaps, open_end) = if tag.self_closing {
        if let Err(col) = rest_blank(&src, after) {
            p.err(i, col, format!("nothing may follow a shape's tag on its line: {form}."));
            return None;
        }
        (vec![], vec![], next)
    } else {
        p.in_cell = true;
        p.text_styles = true;
        let out = p.inline_full(i, &src[after..], Some("shape"));
        p.in_cell = false;
        p.text_styles = false;
        let out = out?;
        let Some(stop) = out.stop else {
            p.err(
                i,
                src.last().map_or(1, |c| c.1),
                format!("<shape> is not closed by </shape> on the same line: {form}."),
            );
            return None;
        };
        let stop = after + stop;
        if let Some(extra) = src[stop..].iter().find(|c| !c.2.is_whitespace()) {
            p.err(i, extra.1, format!("nothing may follow </shape> on its line: {form}."));
            return None;
        }
        let close = (after..stop).rev().find(|&k| src[k].2 == '<').map_or(at, |k| src[k].0);
        let (paras, pmaps, _) = out.cell_paras(close);
        if paras.iter().any(|x| x.style.is_some()) {
            p.err(i, lead + 1, "a slide has no paragraph styles: a shape's paragraphs are started by <p/> alone.");
            return None;
        }
        let open_end = src.get(after).map_or(p.lines[i].end, |c| c.0);
        if paras.iter().all(|x| x.content.is_empty()) {
            (vec![], vec![], next)
        } else {
            (paras, pmaps, open_end)
        }
    };
    if new && paras.is_empty() {
        p.err(i, tag.col, "a new text box holds its text: <shape box=\"x y w h\">text</shape>.");
        return None;
    }
    if !new && !member {
        if let Some(shapes) = &p.names.shapes {
            if !shapes.iter().any(|(a, b)| *a == id && *b == name) {
                let msg = match shapes.iter().find(|(a, _)| *a == id) {
                    Some((_, n)) => {
                        format!("<shape id=\"{id}\"> is named \"{n}\" in the file; keep its id and name as they are.")
                    }
                    None => format!("<shape id=\"{id}\" name=\"{name}\"> is not a shape of this file. Keep the id and name of a shape of the file; a new text box is written without them, <shape box=\"x y w h\">text</shape>, and the write gives it both."),
                };
                p.err(i, tag.col, msg);
                return None;
            }
        }
    }
    if !new {
        let key = format!("<shape id=\"{id}\">");
        if seen.contains(&key) {
            p.err(i, tag.col, format!("{key} appears twice on this slide; a shape is written once."));
            return None;
        }
        seen.push(key);
    }
    let head = HeadMap { start: at, end: open_end, mark: at + lead };
    let n = pmaps.len();
    let mut starts = vec![open_end];
    if n > 0 {
        starts.extend(pmaps[..n - 1].iter().map(|m| m.mark));
    }
    let blocks = pmaps
        .into_iter()
        .enumerate()
        .map(|(k, m)| BlockMap {
            start: starts[k],
            end: starts.get(k + 1).copied().unwrap_or(next),
            kind: BlockMapKind::Para(m),
        })
        .collect();
    let mut paras: Vec<Inline> = paras.into_iter().map(|x| x.content).collect();
    {
        let mut inls: Vec<&mut Inline> = paras.iter_mut().collect();
        style::fold_under(&mut inls, &obj_style);
        style::normalize(&mut inls);
    }
    if paras.is_empty() && !obj_style.is_empty() {
        p.err(
            i,
            tag.col,
            "a shape without text has no text formatting: write its text, or leave out font, size and color.",
        );
        return None;
    }
    let sh = ShapeText { id, name, geom, paras };
    Some((sh, ItemMap { head, blocks }))
}

const LINE_FORM: &str = "a line is <line id=\"…\" name=\"…\" from=\"x y\" to=\"x y\"/>, its two ends in points, its id and name as they are in the file; a new line is <line from=\"x y\" to=\"x y\"/>";

/// A `<line …/>` line.
fn line_item(p: &mut Parser, i: usize, seen: &mut Vec<String>, member: bool) -> Option<(LineItem, ItemMap)> {
    let form = LINE_FORM;
    let line = &p.lines[i];
    let (at, next, lead) = (line.at, line.next, line.indent());
    let Some((tag, after, src)) = line_tag(p, i).filter(|t| t.0.self_closing && !t.0.closing) else {
        p.err(i, lead + 1, format!("expected {form}."));
        return None;
    };
    if let Err(col) = rest_blank(&src, after) {
        p.err(i, col, format!("nothing may follow a line's tag on its line: {form}."));
        return None;
    }
    let (vals, _) = match split_attrs(&tag, &["id", "name", "from", "to"], false, form) {
        Ok(v) => v,
        Err((col, msg)) => {
            p.err(i, col, msg);
            return None;
        }
    };
    let point = |k: usize| vals[k].as_deref().map(|v| (v, parse_point(v)));
    let (from, to) = match (point(2), point(3)) {
        (Some((_, Some(f))), Some((_, Some(t)))) => (f, t),
        (Some((v, None)), _) | (_, Some((v, None))) => {
            p.err(i, tag.col, format!("\"{v}\" is not a point; a line's ends are x y in points: {form}."));
            return None;
        }
        _ => {
            p.err(i, tag.col, format!("a line has both its ends, from and to: {form}."));
            return None;
        }
    };
    let (id, name) = match (&vals[0], &vals[1]) {
        (Some(id), Some(name)) => (id.clone(), name.clone()),
        (None, None) if member => {
            p.err(i, tag.col, "a group's objects are never added here: a new line goes outside the group.");
            return None;
        }
        (None, None) => (String::new(), String::new()),
        _ => {
            p.err(i, tag.col, format!("<line> needs both id and name, or neither for a new line: {form}."));
            return None;
        }
    };
    if !id.is_empty() && !member {
        if let Some(shapes) = &p.names.shapes {
            if !shapes.iter().any(|(a, b)| *a == id && *b == name) {
                p.err(i, tag.col, format!("<line id=\"{id}\" name=\"{name}\"> is not a line of this file: keep a line's id and name as they are, and write a new line without them."));
                return None;
            }
        }
        let key = format!("<line id=\"{id}\">");
        if seen.contains(&key) {
            p.err(i, tag.col, format!("{key} appears twice on this slide; a line is written once."));
            return None;
        }
        seen.push(key);
    }
    let head = HeadMap { start: at, end: next, mark: at + lead };
    Some((LineItem { id, name, ends: Ends { from, to } }, ItemMap { head, blocks: vec![] }))
}

const PICTURE_FORM: &str = "a picture is <picture id=\"…\" name=\"…\" box=\"…\" src=\"…\"/>, its id and name as they are in the file, with crop=\"left top right bottom\" (percent cut off each edge), mask=\"ellipse\" (the preset shape it is cut to) and alt=\"…\" (its alternative text) when it has them; a new picture is <picture box=\"…\" src=\"…\"/>";

/// `crop="l t r b"` in percent → thousandths of a percent.
fn parse_crop(v: &str) -> Result<Crop, String> {
    let n: Vec<Option<f64>> = v.split_whitespace().map(parse_num).collect();
    let [Some(l), Some(t), Some(r), Some(b)] = n.as_slice() else {
        return Err(format!("crop=\"{v}\" is not a crop; it is four numbers, the percent of the image cut off at its left, top, right and bottom edges, such as crop=\"10 0 10 0\"."));
    };
    let k = |x: f64| (x * PER_PERCENT as f64).round() as i64;
    let c = Crop { l: k(*l), t: k(*t), r: k(*r), b: k(*b) };
    if c.l + c.r >= 100 * PER_PERCENT || c.t + c.b >= 100 * PER_PERCENT {
        return Err(format!("crop=\"{v}\" cuts off the whole image: left and right together, and top and bottom together, stay under 100."));
    }
    if [c.l, c.t, c.r, c.b].iter().any(|x| *x < -1000 * PER_PERCENT) {
        return Err(format!("crop=\"{v}\" has a value below -1000; a negative crop leaves space beside the image, at most ten times its size."));
    }
    Ok(c)
}

/// A `<picture …/>` line; `member`: one of a group's objects.
fn picture(p: &mut Parser, i: usize, seen: &mut Vec<String>, member: bool) -> Option<(PictureItem, ItemMap)> {
    let form = PICTURE_FORM;
    let line = &p.lines[i];
    let (at, next, lead) = (line.at, line.next, line.indent());
    let Some((tag, after, src)) = line_tag(p, i).filter(|t| t.0.self_closing && !t.0.closing) else {
        p.err(i, lead + 1, format!("expected {form}."));
        return None;
    };
    if let Err(col) = rest_blank(&src, after) {
        p.err(i, col, format!("nothing may follow a picture's tag on its line: {form}."));
        return None;
    }
    let (vals, geom) = match split_attrs(&tag, &["id", "name", "src", "crop", "mask", "alt"], true, form) {
        Ok(v) => v,
        Err((col, msg)) => {
            p.err(i, col, msg);
            return None;
        }
    };
    let col_of = |k: &str| tag.attrs.iter().find(|a| a.0 == k).map_or(tag.col, |a| a.2);
    let (id, name) = match (&vals[0], &vals[1]) {
        (Some(id), Some(name)) => (id.clone(), name.clone()),
        (None, None) if member => {
            p.err(i, tag.col, "a group's objects are never added here: a new picture goes outside the group.");
            return None;
        }
        (None, None) => (String::new(), String::new()),
        _ => {
            p.err(i, tag.col, format!("<picture> needs both id and name, or neither for a new picture: {form}."));
            return None;
        }
    };
    let Some(src_v) = vals[2].clone().filter(|s| !s.trim().is_empty()) else {
        p.err(i, tag.col, format!("a picture names its image, src=\"…\": {form}."));
        return None;
    };
    if id.is_empty() && geom.is_none() {
        p.err(i, tag.col, "a new picture needs its box: <picture box=\"x y w h\" src=\"…\"/>.");
        return None;
    }
    if geom.is_none() {
        p.err(i, tag.col, format!("a picture shows its box: {form}."));
        return None;
    }
    let crop = match vals[3].as_deref().map(parse_crop) {
        None => None,
        Some(Ok(c)) => Some(c),
        Some(Err(m)) => {
            p.err(i, col_of("crop"), m);
            return None;
        }
    };
    let mask = match vals[4].as_deref() {
        None => None,
        Some(m) if crate::presets::is_preset(m) => Some(m.to_string()),
        Some(m) => {
            let near = crate::presets::near_presets(m);
            let hint = if near.is_empty() {
                " such as ellipse, roundRect, triangle, hexagon or star5".to_string()
            } else {
                format!(": {}", near.join(", "))
            };
            p.err(
                i,
                col_of("mask"),
                format!("mask=\"{m}\" is not a preset shape; a mask is a DrawingML preset name{hint}."),
            );
            return None;
        }
    };
    let alt = vals[5].clone().filter(|a| !a.is_empty());
    if !id.is_empty() && !member {
        if let Some(shapes) = &p.names.shapes {
            if !shapes.iter().any(|(a, b)| *a == id && *b == name) {
                p.err(i, tag.col, format!("<picture id=\"{id}\" name=\"{name}\"> is not a picture of this file: keep a picture's id and name as they are, and write a new picture without them."));
                return None;
            }
        }
        let key = format!("<picture id=\"{id}\">");
        if seen.contains(&key) {
            p.err(i, tag.col, format!("{key} appears twice on this slide; a picture is written once."));
            return None;
        }
        seen.push(key);
    }
    let head = HeadMap { start: at, end: next, mark: at + lead };
    let crop = crop.filter(|c| !c.is_zero());
    let mask = mask.filter(|m| m != "rect");
    Some((PictureItem { id, name, geom, src: src_v, crop, mask, alt }, ItemMap { head, blocks: vec![] }))
}

const GROUP_FORM: &str = "a group is a line <group id=\"…\" name=\"…\" box=\"…\">, its objects' lines, then a line </group>, its id and name as they are in the file";

/// The group opened on line `i` and closed on line `close`.
fn group(p: &mut Parser, i: usize, close: usize, seen: &mut Vec<String>, member: bool) -> Option<(GroupItem, ItemMap)> {
    let form = GROUP_FORM;
    let before = p.errors.len();
    let line = &p.lines[i];
    let (at, lead) = (line.at, line.indent());
    let Some((tag, after, src)) = line_tag(p, i).filter(|t| !t.0.self_closing && !t.0.closing) else {
        p.err(i, lead + 1, format!("expected {form}."));
        return None;
    };
    if let Err(col) = rest_blank(&src, after) {
        p.err(i, col, format!("the group's objects go on the lines after its <group> line: {form}."));
        return None;
    }
    let (vals, geom) = match split_attrs(&tag, &["id", "name"], true, form) {
        Ok(v) => v,
        Err((col, msg)) => {
            p.err(i, col, msg);
            return None;
        }
    };
    let (Some(id), Some(name)) = (vals[0].clone(), vals[1].clone()) else {
        p.err(i, tag.col, format!("groups are never created here, and a group line keeps its id and name: {form}."));
        return None;
    };
    if !member {
        if let Some(shapes) = &p.names.shapes {
            if !shapes.iter().any(|(a, b)| *a == id && *b == name) {
                p.err(i, tag.col, format!("<group id=\"{id}\" name=\"{name}\"> is not a group of this file: groups are never created here; keep a group's id and name as they are."));
                return None;
            }
        }
        let key = format!("<group id=\"{id}\">");
        if seen.contains(&key) {
            p.err(i, tag.col, format!("{key} appears twice on this slide; a group is written once."));
            return None;
        }
        seen.push(key);
    }
    let mut items = vec![];
    let mut local = vec![];
    let mut j = i + 1;
    while j < close {
        let t = p.lines[j].text.trim();
        if t.is_empty() {
            j += 1;
            continue;
        }
        if is_group_open(t) {
            let Some(k) = group_close(p, j, close) else {
                p.err(j, 1, format!("this <group> is not closed inside its group: {form}."));
                return None;
            };
            if let Some((g, _)) = group(p, j, k, &mut local, true) {
                items.push(SlideItem::Group(g));
            }
            j = k + 1;
            continue;
        }
        if is_shape_line(t) {
            if let Some((sh, _)) = shape(p, j, &mut local, true) {
                items.push(SlideItem::Shape(sh));
            }
        } else if is_line_line(t) {
            if let Some((l, _)) = line_item(p, j, &mut local, true) {
                items.push(SlideItem::Line(l));
            }
        } else if is_picture_line(t) {
            if let Some((pic, _)) = picture(p, j, &mut local, true) {
                items.push(SlideItem::Picture(pic));
            }
        } else if t.starts_with("<keep") {
            if let Some(o) = member_keep(p, j) {
                items.push(SlideItem::Object(o));
            }
        } else if marker(t).is_some() {
            p.err(j, 1, "a slot is never in a group: close the group with its line </group> before the slot's marker.");
        } else {
            p.err(
                j,
                1,
                format!(
                    "a group holds its objects' lines only (<shape>, <picture/>, <line/>, <keep/>, <group>): {form}."
                ),
            );
        }
        j += 1;
    }
    if p.lines[close].text.trim() != "</group>" {
        p.err(close, 1, "a group is closed by a line holding only </group>.");
    }
    if p.errors.len() > before {
        return None;
    }
    if items.is_empty() {
        p.err(i, 1, "a group without objects: to delete a group, delete its lines, from <group> to </group>.");
        return None;
    }
    let head = HeadMap { start: at, end: p.lines[close].next, mark: at + lead };
    Some((GroupItem { id, name, geom, items }, ItemMap { head, blocks: vec![] }))
}

/// A group's `<keep id kind summary box/>` line: the id is the object's shape id.
fn member_keep(p: &mut Parser, j: usize) -> Option<ObjectItem> {
    let form = "an object in a group is <keep id=\"…\" kind=\"…\" summary=\"…\" box=\"…\"/>, kept as it is";
    let lead = p.lines[j].indent();
    let Some((tag, after, src)) = line_tag(p, j).filter(|t| t.0.self_closing && t.0.name == "keep") else {
        p.err(j, lead + 1, format!("expected {form}."));
        return None;
    };
    if let Err(col) = rest_blank(&src, after) {
        p.err(j, col, format!("nothing may follow the tag on its line: {form}."));
        return None;
    }
    match split_attrs(&tag, &["id", "kind", "summary"], true, form) {
        Ok((v, geom)) => match (&v[0], &v[1], &v[2]) {
            (Some(id), Some(kind), Some(summary)) => {
                Some(ObjectItem { keep: Keep { id: id.clone(), kind: kind.clone(), summary: summary.clone() }, geom })
            }
            _ => {
                p.err(j, tag.col, format!("{form}."));
                None
            }
        },
        Err((col, msg)) => {
            p.err(j, col, msg);
            None
        }
    }
}

// ---------------------------------------------------------------- serializer

/// Canonical text of a presentation: the front matter, a blank line, then
/// the slides with a line `---` between them (a blank line on either side);
/// slot markers directly after the text before them; blocks in a slot as in
/// a document.
pub fn serialize_presentation(pres: &Presentation) -> String {
    let mut out = front_lines(&pres.front);
    for (k, s) in pres.slides.iter().enumerate() {
        if k > 0 {
            out.extend([String::new(), "---".into()]);
        }
        out.push(String::new());
        out.push(format!("layout: {}", write_layout(&s.layout)));
        item_lines(&mut out, &s.items);
    }
    out.join("\n") + "\n"
}

fn item_lines(out: &mut Vec<String>, items: &[SlideItem]) {
    for it in items {
        match it {
            SlideItem::Slot(sl) => {
                // Lifting (§5.3): what all its text shares on the marker, a
                // paragraph's own at its line's end, the rest in [text]{…}.
                let mut bl = sl.blocks.clone();
                let (obj, ends) = lifted(&mut bl);
                let st = if obj.is_empty() { String::new() } else { format!(" {}", obj.attrs()) };
                out.push(format!("::{}{}{st}::", sl.name, geom_attrs(&sl.geom)));
                let from = out.len();
                blocks_with(out, &bl, true, &ends);
                for l in &mut out[from..] {
                    escape_slot_line(l);
                }
            }
            SlideItem::Shape(sh) => out.push(shape_line(sh)),
            SlideItem::Object(o) => out.push(object_line(o)),
            SlideItem::Line(l) => out.push(line_line(l)),
            SlideItem::Picture(pic) => out.push(picture_line(pic)),
            SlideItem::Group(g) => {
                out.push(format!("<group id=\"{}\" name=\"{}\"{}>", attr(&g.id), attr(&g.name), geom_attrs(&g.geom)));
                item_lines(out, &g.items);
                out.push("</group>".into());
            }
        }
    }
}

fn write_layout(name: &str) -> String {
    let needs = name.is_empty() || name.trim() != name || name.starts_with(['"', '\'']);
    match (needs, name.contains('"')) {
        (false, _) => name.to_string(),
        (true, false) => format!("\"{name}\""),
        (true, true) => format!("'{name}'"),
    }
}

/// A slot line that would read as a marker or a slide separator.
fn escape_slot_line(l: &mut String) {
    let t = l.trim();
    if marker(t).is_some() || is_separator(t) {
        let k = l.len() - l.trim_start().len();
        l.insert(k, '\\');
    }
}

pub fn shape_line(sh: &ShapeText) -> String {
    let refs: Vec<&Inline> = sh.paras.iter().collect();
    let (obj, own, stated) = style::lift(&refs);
    let mut g = geom_attrs(&sh.geom);
    if !obj.is_empty() {
        g.push(' ');
        g.push_str(&obj.attrs());
    }
    let ps: Vec<CellPara> =
        stated.into_iter().map(|c| CellPara { style: None, content: c, props: Default::default() }).collect();
    let ends: Vec<String> =
        own.iter().map(|o| if o.is_empty() { String::new() } else { format!(" {{{}}}", o.attrs()) }).collect();
    let text = cell_text_with(&ps, true, &ends);
    match (sh.id.is_empty(), sh.paras.is_empty()) {
        (true, _) => format!("<shape{g}>{text}</shape>"),
        (false, true) => format!("<shape id=\"{}\" name=\"{}\"{g}/>", attr(&sh.id), attr(&sh.name)),
        (false, false) => format!("<shape id=\"{}\" name=\"{}\"{g}>{text}</shape>", attr(&sh.id), attr(&sh.name)),
    }
}

/// The inlines of a slot's paragraphs and list items, in order.
fn slot_inlines(bl: &mut [Block]) -> Vec<&mut Inline> {
    let mut out = vec![];
    for b in bl.iter_mut() {
        match b {
            Block::Para(p) => out.push(&mut p.content),
            Block::List(items) => out.extend(items.iter_mut().map(|it| &mut it.content)),
            _ => {}
        }
    }
    out
}

/// A slot's blocks lifted in place (see [`style::lift`]): what its marker
/// states, and what each block's lines end with.
fn lifted(bl: &mut [Block]) -> (TextStyle, Vec<Vec<String>>) {
    let refs: Vec<Inline> = {
        let mut v = vec![];
        for b in bl.iter() {
            match b {
                Block::Para(p) => v.push(p.content.clone()),
                Block::List(items) => v.extend(items.iter().map(|it| it.content.clone())),
                _ => {}
            }
        }
        v
    };
    let (obj, own, stated) = style::lift(&refs.iter().collect::<Vec<_>>());
    let end = |o: &TextStyle| if o.is_empty() { String::new() } else { format!(" {{{}}}", o.attrs()) };
    let mut k = 0;
    let mut ends = vec![];
    for b in bl.iter_mut() {
        match b {
            Block::Para(p) => {
                p.content = stated[k].clone();
                ends.push(vec![end(&own[k])]);
                k += 1;
            }
            Block::List(items) => {
                let mut e = vec![];
                for it in items.iter_mut() {
                    it.content = stated[k].clone();
                    e.push(end(&own[k]));
                    k += 1;
                }
                ends.push(e);
            }
            _ => ends.push(vec![]),
        }
    }
    (obj, ends)
}

pub fn object_line(o: &ObjectItem) -> String {
    let t = keep_tag(&o.keep);
    format!("{}{}/>", &t[..t.len() - 2], geom_attrs(&o.geom))
}

pub fn picture_line(pic: &PictureItem) -> String {
    let mut out = String::from("<picture");
    if !pic.id.is_empty() {
        out.push_str(&format!(" id=\"{}\" name=\"{}\"", attr(&pic.id), attr(&pic.name)));
    }
    out.push_str(&geom_attrs(&pic.geom));
    out.push_str(&format!(" src=\"{}\"", attr(&pic.src)));
    if let Some(c) = pic.crop.filter(|c| !c.is_zero()) {
        out.push_str(&format!(" crop=\"{} {} {} {}\"", shown_pct(c.l), shown_pct(c.t), shown_pct(c.r), shown_pct(c.b)));
    }
    if let Some(m) = &pic.mask {
        out.push_str(&format!(" mask=\"{}\"", attr(m)));
    }
    if let Some(a) = &pic.alt {
        // Alternative text may hold line breaks; the line cannot.
        out.push_str(&format!(" alt=\"{}\"", attr(a).replace('\n', "&#10;").replace('\r', "&#13;")));
    }
    out.push_str("/>");
    out
}

pub fn line_line(l: &LineItem) -> String {
    let p = |(x, y): (i64, i64)| format!("{} {}", pt(x), pt(y));
    let ends = format!("from=\"{}\" to=\"{}\"", p(l.ends.from), p(l.ends.to));
    if l.id.is_empty() {
        format!("<line {ends}/>")
    } else {
        format!("<line id=\"{}\" name=\"{}\" {ends}/>", attr(&l.id), attr(&l.name))
    }
}
