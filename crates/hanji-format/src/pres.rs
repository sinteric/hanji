//! The Presentation grammar (DESIGN.md §5.3): slides separated by a line
//! `---`, each opened by `layout: Name`, then slot markers (`::title::`) and
//! `<shape>` lines. A slot's text is the Document grammar's paragraphs, list
//! items, empty paragraphs and placeholders (§5.1), parsed by the same code.

use crate::ast::*;
use crate::diag::{quoted, Diagnostic};
use crate::names::{Layout, Names};
use crate::parse::{chars_of, parse_tag, BlockMap, BlockMapKind, ParaMap, Parser};
use crate::serialize::{attr, blocks, cell_text, front_lines, keep_tag};

/// Where a head line is: its byte range, and the offset that stands for it
/// (its line end; for a shape, its `<shape` tag).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HeadMap {
    pub start: usize,
    pub end: usize,
    pub mark: usize,
}

/// Where every slide, slot, shape and block of a presentation is in its text.
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

/// A slot (its marker line and blocks) or a shape (its tag and one block per paragraph).
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

/// `::name::` → the name (possibly malformed; the caller checks it).
fn marker(t: &str) -> Option<&str> {
    let inner = t.strip_prefix("::")?.strip_suffix("::")?;
    (!t.is_empty() && t.len() >= 4).then_some(inner)
}

fn valid_slot_name(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn is_shape_line(t: &str) -> bool {
    t.starts_with("<shape") && t[6..].starts_with([' ', '>', '/'])
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

/// Whether line `i` is a slide object's `<keep/>` line: one outside a slot,
/// or inside one when its id is one of the file's objects (a slot's own
/// `<keep/>` is not; with no list of objects, none inside a slot is).
fn is_object_line(p: &Parser, i: usize, in_slot: bool) -> bool {
    match (keep_line_id(p, i), &p.names.objects) {
        (Some(id), Some(objects)) => !in_slot || objects.contains(&id),
        (Some(_), None) => !in_slot,
        (None, _) => false,
    }
}

const SLOT_TEXT: &str = "Slot text is plain lines, - and 1. list items, <p/> empty paragraphs and <keep/> placeholders";

/// A slide object's `<keep/>` line: the head stands on the tag's `<`, the
/// placeholder block on the rest of the line (distinct offsets, so an exact
/// span tells them apart).
fn object(p: &mut Parser, i: usize) -> Option<(Keep, ItemMap)> {
    let (b, m, _) = p.block(i)?;
    let keep = match b {
        Block::Para(Para { style: ParaStyle::Plain, content }) if content.units.len() == 1 => {
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
    Some((keep, ItemMap { head, blocks: vec![block] }))
}

fn slot_list(slots: &[String]) -> String {
    slots.iter().map(|s| format!("::{s}::")).collect::<Vec<_>>().join(", ")
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
        if let Some(name) = marker(t) {
            let name = name.to_string();
            let end = (i + 1..b)
                .find(|&j| {
                    let t = p.lines[j].text.trim();
                    marker(t).is_some() || is_shape_line(t) || is_object_line(p, j, true)
                })
                .unwrap_or(b);
            if let Some((slot, m)) = slot(p, i, end, &name, slots.as_deref(), &layout, &mut seen) {
                items.push(SlideItem::Slot(slot));
                maps.push(m);
            }
            i = end;
        } else if is_shape_line(t) {
            if let Some((sh, m)) = shape(p, i, &mut seen) {
                items.push(SlideItem::Shape(sh));
                maps.push(m);
            }
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
                    format!("text outside a slot: every line of a slide after its layout: line belongs to a slot, after its marker line such as ::title:: or ::body::, or is a <shape> line or an object's <keep/> line.{list}")
                }
            };
            p.err(i, 1, msg);
            i += 1;
        }
    }
    Some((Slide { layout, items }, SlideMap { head, items: maps }))
}

/// The slot whose marker is line `i` and whose text is lines `i + 1..end`.
#[allow(clippy::too_many_arguments)]
fn slot(
    p: &mut Parser,
    i: usize,
    end: usize,
    name: &str,
    allowed: Option<&[String]>,
    layout: &str,
    seen: &mut Vec<String>,
) -> Option<(Slot, ItemMap)> {
    let before = p.errors.len();
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
    if p.errors.len() > before {
        return None;
    }
    if bl.is_empty() {
        p.err(i, 1, format!("::{name}:: has no text. An unfilled slot is left out: delete this marker line, or write the slot's text after it."));
        return None;
    }
    Some((Slot { name: name.to_string(), blocks: bl }, ItemMap { head, blocks: bm }))
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

/// A `<shape id="…" name="…">text</shape>` line.
fn shape(p: &mut Parser, i: usize, seen: &mut Vec<String>) -> Option<(ShapeText, ItemMap)> {
    let form = "a shape line is <shape id=\"…\" name=\"…\">text</shape>, its id and name as they are in the file";
    let line = &p.lines[i];
    let (at, next, lead) = (line.at, line.next, line.indent());
    let src = chars_of(line, lead);
    let Some((tag, after)) = parse_tag(&src, 0).filter(|t| !t.0.closing && !t.0.self_closing) else {
        p.err(i, lead + 1, format!("expected {form}."));
        return None;
    };
    let (mut id, mut name) = (None, None);
    for (k, v, col) in &tag.attrs {
        match k.as_str() {
            "id" if id.is_none() => id = Some(v.clone()),
            "name" if name.is_none() => name = Some(v.clone()),
            _ => {
                p.err(i, *col, format!("<shape> has the attributes id and name only, once each: {form}."));
                return None;
            }
        }
    }
    let (Some(id), Some(name)) = (id, name) else {
        p.err(i, tag.col, format!("<shape> needs both id and name: {form}."));
        return None;
    };
    p.in_cell = true;
    let out = p.inline_full(i, &src[after..], Some("shape"));
    p.in_cell = false;
    let out = out?;
    let Some(stop) = out.stop else {
        p.err(i, src.last().map_or(1, |c| c.1), format!("<shape> is not closed by </shape> on the same line: {form}."));
        return None;
    };
    let stop = after + stop;
    if let Some(extra) = src[stop..].iter().find(|c| !c.2.is_whitespace()) {
        p.err(i, extra.1, format!("nothing may follow </shape> on its line: {form}."));
        return None;
    }
    let close = (after..stop).rev().find(|&k| src[k].2 == '<').map_or(at, |k| src[k].0);
    let (paras, pmaps) = out.cell_paras(close);
    if paras.iter().any(|x| x.style.is_some()) {
        p.err(i, lead + 1, "a slide has no paragraph styles: a shape's paragraphs are started by <p/> alone.");
        return None;
    }
    if paras.iter().all(|x| x.content.is_empty()) {
        p.err(i, lead + 1, "a shape line holds the shape's text, and a shape without text is not shown. To remove the shape, delete its line.");
        return None;
    }
    if let Some(shapes) = &p.names.shapes {
        if !shapes.iter().any(|(a, b)| *a == id && *b == name) {
            let msg = match shapes.iter().find(|(a, _)| *a == id) {
                Some((_, n)) => format!("<shape id=\"{id}\"> is named \"{n}\" in the file; keep its id and name as they are."),
                None => format!("<shape id=\"{id}\" name=\"{name}\"> is not a shape of this file. Shapes come from the file: edit their text, move or delete them, but never create one; new text goes in a slot."),
            };
            p.err(i, tag.col, msg);
            return None;
        }
    }
    let key = format!("<shape id=\"{id}\">");
    if seen.contains(&key) {
        p.err(i, tag.col, format!("{key} appears twice on this slide; a shape is written once."));
        return None;
    }
    seen.push(key);
    let open_end = src.get(after).map_or(p.lines[i].end, |c| c.0);
    let head = HeadMap { start: at, end: open_end, mark: at + lead };
    let n = pmaps.len();
    let mut starts = vec![open_end];
    starts.extend(pmaps[..n - 1].iter().map(|m| m.mark));
    let blocks = pmaps
        .into_iter()
        .enumerate()
        .map(|(k, m)| BlockMap {
            start: starts[k],
            end: starts.get(k + 1).copied().unwrap_or(next),
            kind: BlockMapKind::Para(m),
        })
        .collect();
    let sh = ShapeText { id, name, paras: paras.into_iter().map(|x| x.content).collect() };
    Some((sh, ItemMap { head, blocks }))
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
        for it in &s.items {
            match it {
                SlideItem::Slot(sl) => {
                    out.push(format!("::{}::", sl.name));
                    let from = out.len();
                    blocks(&mut out, &sl.blocks);
                    for l in &mut out[from..] {
                        escape_slot_line(l);
                    }
                }
                SlideItem::Shape(sh) => out.push(shape_line(sh)),
                SlideItem::Object(k) => out.push(keep_tag(k)),
            }
        }
    }
    out.join("\n") + "\n"
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
    let ps: Vec<CellPara> = sh.paras.iter().map(|c| CellPara { style: None, content: c.clone() }).collect();
    format!("<shape id=\"{}\" name=\"{}\">{}</shape>", attr(&sh.id), attr(&sh.name), cell_text(&ps))
}
