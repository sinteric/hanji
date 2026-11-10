//! Drawing shapes as `<shape name anchor>text</shape>` lines and groups as
//! `<group name>` wrappers (DESIGN.md §6 round 7 A, §10 item 12). Unedited
//! shapes leave the drawing part as it was; an edit rewrites only the
//! anchors that changed. Geometry and styling the syntax does not name stay
//! as the original XML. Connectors, charts, pictures, cell-linked text, and
//! groups that hold those stay placeholders.

use hanji_core::cells::{CellRange, CellRef};
use hanji_format::sheet::{DrawingItem, GroupDecl, ShapeDecl, MAX_SHAPE_LINES};
use hanji_package::opc::{self, Rel};
use hanji_package::package;
use hanji_package::xml::{self, Element, Node};

use crate::book::{Book, R_NS};
use crate::drawing::{self, Shown};
use crate::store::SHEET_ORDER;

const XDR_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing";
const A_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const CT_DRAWING: &str = "application/vnd.openxmlformats-officedocument.drawing+xml";

/// A modelled shape/group, or a drawing object that stays a placeholder.
#[derive(Clone, Debug)]
pub enum DrawingLine {
    Item(DrawingItem),
    Keep(Shown),
}

/// This sheet's drawing as structure lines, or all-keeps when there are more
/// than [`MAX_SHAPE_LINES`] modelled shapes.
pub fn lines(book: &Book, i: usize) -> Vec<DrawingLine> {
    let (out, n) = classified(book, i);
    if n > MAX_SHAPE_LINES {
        return drawing_parts(book, i)
            .into_iter()
            .flat_map(|p| drawing::drawing_objects(book, &p))
            .map(DrawingLine::Keep)
            .collect();
    }
    out
}

/// Modelled `<shape>` / `<group>` lines of sheet `i`, or `None` when they
/// stay placeholders (more than [`MAX_SHAPE_LINES`] modelled shapes).
pub fn modelled(book: &Book, i: usize) -> Option<Vec<DrawingItem>> {
    let (out, n) = classified(book, i);
    if n > MAX_SHAPE_LINES {
        return None;
    }
    Some(
        out.into_iter()
            .filter_map(|l| match l {
                DrawingLine::Item(it) => Some(it),
                DrawingLine::Keep(_) => None,
            })
            .collect(),
    )
}

fn classified(book: &Book, i: usize) -> (Vec<DrawingLine>, usize) {
    let mut out = vec![];
    let mut n = 0usize;
    for part in drawing_parts(book, i) {
        out.extend(classify_part(book, &part, &mut n));
    }
    (out, n)
}

/// Apply the drawing lines of `wanted`. A no-op when they already match, so
/// an unedited file keeps its drawing part byte for byte.
pub fn apply(book: &mut Book, i: usize, wanted: &[DrawingItem]) -> Result<(), String> {
    let Some(have) = modelled(book, i) else {
        return if wanted.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "this sheet has more than {MAX_SHAPE_LINES} shapes; they stay placeholders"
            ))
        };
    };
    for w in wanted {
        if let DrawingItem::Group(g) = w {
            group_anchor(g)?;
        }
    }
    if have == wanted {
        return Ok(());
    }
    if wanted.is_empty() {
        return clear_modelled(book, i);
    }
    let part = match drawing_parts(book, i).into_iter().next() {
        Some(p) => p,
        None => create_drawing(book, i)?,
    };
    let data = package::get(&book.parts, &part).ok_or_else(|| format!("no {part}"))?;
    let mut d = xml::parse(data).map_err(|e| format!("{part}: {e}"))?;
    let xdr = ns_prefix(&d.root.name);
    let a = a_prefix(&d.root);
    let orig = take_modelled_anchors(&mut d.root);
    let mut unused = vec![true; orig.len()];
    let mut placed: Vec<Option<usize>> = vec![None; wanted.len()];
    for (wi, w) in wanted.iter().enumerate() {
        let Some(k) = orig
            .iter()
            .enumerate()
            .find(|(k, (_, have))| unused[*k] && have == w)
        else {
            continue;
        };
        unused[k.0] = false;
        placed[wi] = Some(k.0);
    }
    let leftover: Vec<usize> = unused
        .iter()
        .enumerate()
        .filter(|(_, u)| **u)
        .map(|(k, _)| k)
        .collect();
    let mut leftover = leftover.into_iter();
    for slot in &mut placed {
        if slot.is_none() {
            if let Some(k) = leftover.next() {
                *slot = Some(k);
            }
        }
    }
    let mut next_id = max_cnv_id(&d.root);
    for id in orig.iter().map(|(e, _)| max_cnv_id(e)) {
        next_id = next_id.max(id);
    }
    for (w, slot) in wanted.iter().zip(placed) {
        let el = match slot {
            Some(k) => {
                let (mut el, have) = orig[k].clone();
                if have != *w {
                    patch_anchor(&mut el, w, &xdr, &a, &mut next_id)?;
                }
                el
            }
            None => new_anchor(w, &xdr, &a, &mut next_id),
        };
        d.root.children.push(Node::El(el));
    }
    if d.root.elements().next().is_none() {
        drop_drawing(book, i, &part)?;
        return Ok(());
    }
    book.put_part(&part, xml::write_doc(&d));
    Ok(())
}

fn drawing_parts(book: &Book, i: usize) -> Vec<String> {
    let part = &book.sheets[i].part;
    book.rels_of(part)
        .into_iter()
        .filter(|r| r.short_type() == "drawing" && !r.external)
        .map(|r| opc::resolve_target(part, &r.target))
        .collect()
}

fn classify_part(book: &Book, part: &str, modelled: &mut usize) -> Vec<DrawingLine> {
    let Some(d) = package::get(&book.parts, part).and_then(|d| xml::parse(d).ok()) else {
        return vec![];
    };
    let rels = book.rels_of(part);
    let mut out = vec![];
    for a in d.root.elements() {
        match modelled_anchor(a) {
            Some(item) => {
                *modelled += count_shapes(&item);
                out.push(DrawingLine::Item(item));
            }
            None => {
                if let Some(s) = drawing::shown_of(book, part, a, &rels) {
                    out.push(DrawingLine::Keep(s));
                }
            }
        }
    }
    out
}

fn count_shapes(item: &DrawingItem) -> usize {
    match item {
        DrawingItem::Shape(_) => 1,
        DrawingItem::Group(g) => g.items.iter().map(count_shapes).sum(),
    }
}

fn modelled_anchor(a: &Element) -> Option<DrawingItem> {
    let range = anchor_range(a)?;
    let obj = object_of(a)?;
    modelled_object(obj, &range)
}

fn modelled_object(obj: &Element, range: &str) -> Option<DrawingItem> {
    match obj.local() {
        "sp" => modelled_sp(obj, range).map(DrawingItem::Shape),
        "grpSp" => modelled_grp(obj, range).map(DrawingItem::Group),
        _ => None,
    }
}

fn modelled_sp(sp: &Element, range: &str) -> Option<ShapeDecl> {
    if sp.get("textlink").is_some_and(|s| !s.is_empty()) {
        return None;
    }
    Some(ShapeDecl {
        name: cnv_name(sp)?,
        anchor: range.to_string(),
        text: shape_text(sp),
    })
}

fn modelled_grp(grp: &Element, range: &str) -> Option<GroupDecl> {
    let mut items = vec![];
    for c in grp.elements() {
        match c.local() {
            "nvGrpSpPr" | "grpSpPr" => {}
            "sp" | "grpSp" => items.push(modelled_object(c, range)?),
            _ => return None,
        }
    }
    if items.is_empty() {
        return None;
    }
    Some(GroupDecl {
        name: cnv_name(grp)?,
        items,
    })
}

fn object_of(a: &Element) -> Option<&Element> {
    a.elements()
        .find(|e| !matches!(e.local(), "from" | "to" | "pos" | "ext" | "clientData"))
}

fn object_of_mut(a: &mut Element) -> Option<&mut Element> {
    a.elements_mut()
        .find(|e| !matches!(e.local(), "from" | "to" | "pos" | "ext" | "clientData"))
}

fn cnv_name(obj: &Element) -> Option<String> {
    obj.elements()
        .find(|e| e.local().starts_with("nv"))
        .and_then(|nv| nv.elements().find(|e| e.local() == "cNvPr"))
        .and_then(|c| c.get("name"))
        .filter(|n| !n.is_empty())
}

fn cnv_name_mut(obj: &mut Element) -> Option<&mut Element> {
    let nv = obj.elements_mut().find(|e| e.local().starts_with("nv"))?;
    nv.elements_mut().find(|e| e.local() == "cNvPr")
}

fn set_name(obj: &mut Element, name: &str) {
    if let Some(pr) = cnv_name_mut(obj) {
        pr.set("name", name);
    }
}

fn shape_text(sp: &Element) -> String {
    let Some(body) = sp.elements().find(|e| e.local() == "txBody") else {
        return String::new();
    };
    let paras: Vec<String> = body
        .elements()
        .filter(|e| e.local() == "p")
        .map(para_text)
        .filter(|s| !s.is_empty())
        .collect();
    paras.join("\n")
}

fn para_text(p: &Element) -> String {
    let mut s = String::new();
    for c in p.elements() {
        if matches!(c.local(), "r" | "fld") {
            s.push_str(&t_text(c));
        }
    }
    s
}

fn t_text(e: &Element) -> String {
    let mut s = String::new();
    for c in e.elements() {
        if c.local() == "t" {
            for n in &c.children {
                if let Node::Text(t) = n {
                    s.push_str(&xml::unescape(t));
                }
            }
        }
    }
    s
}

fn anchor_range(a: &Element) -> Option<String> {
    if !matches!(a.local(), "twoCellAnchor" | "oneCellAnchor") {
        return None;
    }
    let from = a.elements().find(|e| e.local() == "from")?;
    let start = cell_of(from)?;
    match a.elements().find(|e| e.local() == "to") {
        Some(to) => Some(CellRange::new(start, cell_of(to)?).to_string()),
        None => Some(start.to_string()),
    }
}

fn cell_of(end: &Element) -> Option<CellRef> {
    let col = child_u32(end, "col")?;
    let row = child_u32(end, "row")?;
    Some(CellRef::new(col, row + 1))
}

fn child_u32(e: &Element, local: &str) -> Option<u32> {
    e.elements()
        .find(|x| x.local() == local)
        .map(|x| x.text_of(&[x.name.as_str()]))
        .and_then(|t| t.trim().parse().ok())
}

fn set_child_u32(e: &mut Element, local: &str, v: u32) {
    if let Some(c) = e.elements_mut().find(|x| x.local() == local) {
        c.children = vec![Node::Text(v.to_string())];
    }
}

fn ns_prefix(qname: &str) -> String {
    match qname.rsplit_once(':') {
        Some((p, _)) => format!("{p}:"),
        None => String::new(),
    }
}

fn a_prefix(root: &Element) -> String {
    root.attrs
        .iter()
        .find(|a| a.1 == A_NS)
        .and_then(|a| a.0.strip_prefix("xmlns:"))
        .map(|p| format!("{p}:"))
        .unwrap_or_else(|| "a:".into())
}

fn take_modelled_anchors(root: &mut Element) -> Vec<(Element, DrawingItem)> {
    let mut orig = vec![];
    let mut kept = vec![];
    for n in std::mem::take(&mut root.children) {
        match n {
            Node::El(e) => match modelled_anchor(&e) {
                Some(item) => orig.push((e, item)),
                None => kept.push(Node::El(e)),
            },
            other => kept.push(other),
        }
    }
    root.children = kept;
    orig
}

fn max_cnv_id(e: &Element) -> u32 {
    let mut m = 1u32;
    e.walk(&mut |x| {
        if x.local() == "cNvPr" {
            if let Some(id) = x.get("id").and_then(|s| s.parse().ok()) {
                m = m.max(id);
            }
        }
    });
    m
}

fn shared_anchor(item: &DrawingItem) -> String {
    match item {
        DrawingItem::Shape(s) => s.anchor.clone(),
        DrawingItem::Group(g) => g.items.first().map(shared_anchor).unwrap_or_default(),
    }
}

fn group_anchor(g: &GroupDecl) -> Result<String, String> {
    let want = g
        .items
        .first()
        .map(shared_anchor)
        .ok_or_else(|| format!("group \"{}\" is empty", g.name))?;
    fn check(items: &[DrawingItem], want: &str, name: &str) -> Result<(), String> {
        for it in items {
            match it {
                DrawingItem::Shape(s) if s.anchor != want => {
                    return Err(format!(
                        "grouped shape \"{}\" has anchor {} but the group sits on {want}; every shape in a group uses the group's cells",
                        s.name, s.anchor
                    ));
                }
                DrawingItem::Shape(_) => {}
                DrawingItem::Group(g) => {
                    if g.items.first().map(shared_anchor).unwrap_or_default() != want {
                        return Err(format!(
                            "nested group \"{}\" does not sit on {want}; every shape in group \"{name}\" uses the group's cells",
                            g.name
                        ));
                    }
                    check(&g.items, want, name)?;
                }
            }
        }
        Ok(())
    }
    check(&g.items, &want, &g.name)?;
    Ok(want)
}

fn patch_anchor(
    el: &mut Element,
    wanted: &DrawingItem,
    xdr: &str,
    a: &str,
    next_id: &mut u32,
) -> Result<(), String> {
    match wanted {
        DrawingItem::Shape(s) => {
            set_anchor_range(el, &s.anchor, xdr)?;
            if let Some(obj) = object_of_mut(el) {
                set_name(obj, &s.name);
                set_shape_text(obj, &s.text, a);
            }
            Ok(())
        }
        DrawingItem::Group(g) => {
            let range = group_anchor(g)?;
            set_anchor_range(el, &range, xdr)?;
            if let Some(obj) = object_of_mut(el) {
                set_name(obj, &g.name);
                reconcile_group(obj, g, xdr, a, next_id)?;
            }
            Ok(())
        }
    }
}

fn set_anchor_range(el: &mut Element, range: &str, xdr: &str) -> Result<(), String> {
    let r =
        CellRange::parse(range).ok_or_else(|| format!("anchor \"{range}\" is not an A1 range"))?;
    if el.local() == "oneCellAnchor" && r.first != r.last {
        to_two_cell(el, xdr);
    }
    let from_row = r.first.row.saturating_sub(1);
    let to_row = r.last.row.saturating_sub(1);
    if let Some(from) = el.elements_mut().find(|e| e.local() == "from") {
        set_child_u32(&mut *from, "col", r.first.col);
        set_child_u32(&mut *from, "row", from_row);
    }
    if el.local() == "oneCellAnchor" {
        return Ok(());
    }
    if el.elements().any(|e| e.local() == "to") {
        if let Some(to) = el.elements_mut().find(|e| e.local() == "to") {
            set_child_u32(&mut *to, "col", r.last.col);
            set_child_u32(&mut *to, "row", to_row);
        }
    } else {
        let to = from_to(xdr, "to", r.last.col, to_row, "0", "0");
        insert_after_from(el, to);
    }
    Ok(())
}

fn to_two_cell(el: &mut Element, xdr: &str) {
    el.name = format!("{xdr}twoCellAnchor");
    el.children
        .retain(|n| !matches!(n, Node::El(e) if e.local() == "ext"));
}

fn insert_after_from(el: &mut Element, to: Element) {
    let at = el
        .children
        .iter()
        .position(|n| matches!(n, Node::El(e) if e.local() == "from"))
        .map(|k| k + 1)
        .unwrap_or(0);
    el.children.insert(at, Node::El(to));
}

fn from_to(xdr: &str, end: &str, col: u32, row: u32, col_off: &str, row_off: &str) -> Element {
    let mut e = Element::new(&format!("{xdr}{end}"));
    e.children
        .push(Node::El(text_el(&format!("{xdr}col"), &col.to_string())));
    e.children
        .push(Node::El(text_el(&format!("{xdr}colOff"), col_off)));
    e.children
        .push(Node::El(text_el(&format!("{xdr}row"), &row.to_string())));
    e.children
        .push(Node::El(text_el(&format!("{xdr}rowOff"), row_off)));
    e
}

fn text_el(name: &str, t: &str) -> Element {
    let mut e = Element::new(name);
    e.children.push(Node::Text(t.into()));
    e
}

fn set_shape_text(sp: &mut Element, text: &str, a: &str) {
    if shape_text(sp) == text {
        return;
    }
    let Some(body) = sp.elements_mut().find(|e| e.local() == "txBody") else {
        if text.is_empty() {
            return;
        }
        sp.children.push(Node::El(new_tx_body(a, text)));
        return;
    };
    let template = body
        .elements()
        .find(|e| e.local() == "p")
        .cloned()
        .unwrap_or_else(|| empty_p(a));
    let mut head = vec![];
    for n in std::mem::take(&mut body.children) {
        match &n {
            Node::El(e) if e.local() == "p" => {}
            _ => head.push(n),
        }
    }
    let lines: Vec<&str> = if text.is_empty() {
        vec![]
    } else {
        text.split('\n').collect()
    };
    if lines.is_empty() {
        body.children = head;
        body.children.push(Node::El(empty_para(&template, a)));
        return;
    }
    body.children = head;
    for line in lines {
        body.children
            .push(Node::El(para_with_text(&template, line, a)));
    }
}

fn empty_p(a: &str) -> Element {
    Element::new(&format!("{a}p"))
}

fn empty_para(template: &Element, a: &str) -> Element {
    let mut p = template.shell();
    if let Some(ppr) = template.elements().find(|e| e.local() == "pPr") {
        p.children.push(Node::El(ppr.clone()));
    }
    if let Some(end) = template.elements().find(|e| e.local() == "endParaRPr") {
        p.children.push(Node::El(end.clone()));
    } else {
        p.children
            .push(Node::El(Element::new(&format!("{a}endParaRPr"))));
    }
    p
}

fn para_with_text(template: &Element, text: &str, a: &str) -> Element {
    let mut p = template.shell();
    if let Some(ppr) = template.elements().find(|e| e.local() == "pPr") {
        p.children.push(Node::El(ppr.clone()));
    }
    let run = match template.elements().find(|e| e.local() == "r") {
        Some(r) => {
            let mut nr = r.shell();
            if let Some(rpr) = r.elements().find(|e| e.local() == "rPr") {
                nr.children.push(Node::El(rpr.clone()));
            }
            nr.children.push(Node::El(t_el(a, text)));
            nr
        }
        None => {
            let mut r = Element::new(&format!("{a}r"));
            r.children.push(Node::El(
                Element::new(&format!("{a}rPr"))
                    .with_attr("lang", "en-US")
                    .with_attr("sz", "1100"),
            ));
            r.children.push(Node::El(t_el(a, text)));
            r
        }
    };
    p.children.push(Node::El(run));
    if let Some(end) = template.elements().find(|e| e.local() == "endParaRPr") {
        p.children.push(Node::El(end.clone()));
    }
    p
}

fn t_el(a: &str, text: &str) -> Element {
    let mut t = Element::new(&format!("{a}t"));
    t.children.push(Node::Text(xml::escape_text(text)));
    t
}

fn new_tx_body(a: &str, text: &str) -> Element {
    let mut body = Element::new(&format!("{a}txBody"));
    let mut body_pr = Element::new(&format!("{a}bodyPr"));
    body_pr.set("wrap", "square");
    body_pr.set("rtlCol", "0");
    body.children.push(Node::El(body_pr));
    body.children
        .push(Node::El(Element::new(&format!("{a}lstStyle"))));
    let tmpl = empty_p(a);
    for line in text.split('\n') {
        body.children.push(Node::El(para_with_text(&tmpl, line, a)));
    }
    body
}

fn reconcile_group(
    grp: &mut Element,
    wanted: &GroupDecl,
    xdr: &str,
    a: &str,
    next_id: &mut u32,
) -> Result<(), String> {
    let range = group_anchor(wanted)?;
    let orig = take_group_objects(grp, &range);
    let mut unused = vec![true; orig.len()];
    let mut placed: Vec<Option<usize>> = vec![None; wanted.items.len()];
    for (wi, w) in wanted.items.iter().enumerate() {
        let Some(k) = orig
            .iter()
            .enumerate()
            .find(|(k, (_, have))| unused[*k] && have == w)
        else {
            continue;
        };
        unused[k.0] = false;
        placed[wi] = Some(k.0);
    }
    let leftover: Vec<usize> = unused
        .iter()
        .enumerate()
        .filter(|(_, u)| **u)
        .map(|(k, _)| k)
        .collect();
    let mut leftover = leftover.into_iter();
    for slot in &mut placed {
        if slot.is_none() {
            if let Some(k) = leftover.next() {
                *slot = Some(k);
            }
        }
    }
    for (w, slot) in wanted.items.iter().zip(placed) {
        let el = match slot {
            Some(k) => {
                let (mut el, have) = orig[k].clone();
                if have != *w {
                    patch_object(&mut el, w, xdr, a, next_id)?;
                }
                el
            }
            None => new_object(w, xdr, a, next_id),
        };
        grp.children.push(Node::El(el));
    }
    Ok(())
}

fn take_group_objects(grp: &mut Element, range: &str) -> Vec<(Element, DrawingItem)> {
    let mut orig = vec![];
    let mut kept = vec![];
    for n in std::mem::take(&mut grp.children) {
        match n {
            Node::El(e) if matches!(e.local(), "sp" | "grpSp") => {
                match modelled_object(&e, range) {
                    Some(item) => orig.push((e, item)),
                    None => kept.push(Node::El(e)),
                }
            }
            other => kept.push(other),
        }
    }
    grp.children = kept;
    orig
}

fn patch_object(
    obj: &mut Element,
    wanted: &DrawingItem,
    xdr: &str,
    a: &str,
    next_id: &mut u32,
) -> Result<(), String> {
    match wanted {
        DrawingItem::Shape(s) => {
            set_name(obj, &s.name);
            set_shape_text(obj, &s.text, a);
            Ok(())
        }
        DrawingItem::Group(g) => {
            set_name(obj, &g.name);
            reconcile_group(obj, g, xdr, a, next_id)
        }
    }
}

fn new_anchor(item: &DrawingItem, xdr: &str, a: &str, next_id: &mut u32) -> Element {
    let range = match item {
        DrawingItem::Shape(s) => s.anchor.clone(),
        DrawingItem::Group(g) => g.items.first().map(shared_anchor).unwrap_or_default(),
    };
    let r = CellRange::parse(&range).unwrap_or_else(|| CellRange::cell(CellRef::new(0, 1)));
    let mut anchor = Element::new(&format!("{xdr}twoCellAnchor"));
    anchor.children.push(Node::El(from_to(
        xdr,
        "from",
        r.first.col,
        r.first.row.saturating_sub(1),
        "0",
        "0",
    )));
    anchor.children.push(Node::El(from_to(
        xdr,
        "to",
        r.last.col,
        r.last.row.saturating_sub(1),
        "0",
        "0",
    )));
    anchor
        .children
        .push(Node::El(new_object(item, xdr, a, next_id)));
    anchor
        .children
        .push(Node::El(Element::new(&format!("{xdr}clientData"))));
    anchor
}

fn new_object(item: &DrawingItem, xdr: &str, a: &str, next_id: &mut u32) -> Element {
    *next_id += 1;
    match item {
        DrawingItem::Shape(s) => new_sp(xdr, a, *next_id, &s.name, &s.text, true),
        DrawingItem::Group(g) => new_grp(xdr, a, next_id, g),
    }
}

fn new_sp(xdr: &str, a: &str, id: u32, name: &str, text: &str, tx_box: bool) -> Element {
    let mut sp = Element::new(&format!("{xdr}sp"))
        .with_attr("macro", "")
        .with_attr("textlink", "");
    let mut nv = Element::new(&format!("{xdr}nvSpPr"));
    nv.children.push(Node::El(
        Element::new(&format!("{xdr}cNvPr"))
            .with_attr("id", &id.to_string())
            .with_attr("name", name),
    ));
    let mut cnv = Element::new(&format!("{xdr}cNvSpPr"));
    if tx_box {
        cnv.set("txBox", "1");
    }
    nv.children.push(Node::El(cnv));
    sp.children.push(Node::El(nv));
    let mut sp_pr = Element::new(&format!("{xdr}spPr"));
    let mut xfrm = Element::new(&format!("{a}xfrm"));
    xfrm.children.push(Node::El(
        Element::new(&format!("{a}off"))
            .with_attr("x", "0")
            .with_attr("y", "0"),
    ));
    xfrm.children.push(Node::El(
        Element::new(&format!("{a}ext"))
            .with_attr("cx", "1371600")
            .with_attr("cy", "685800"),
    ));
    sp_pr.children.push(Node::El(xfrm));
    let mut geom = Element::new(&format!("{a}prstGeom")).with_attr("prst", "rect");
    geom.children
        .push(Node::El(Element::new(&format!("{a}avLst"))));
    sp_pr.children.push(Node::El(geom));
    sp.children.push(Node::El(sp_pr));
    if !text.is_empty() {
        sp.children.push(Node::El(new_tx_body(a, text)));
    }
    sp
}

fn new_grp(xdr: &str, a: &str, next_id: &mut u32, g: &GroupDecl) -> Element {
    let id = *next_id;
    let mut grp = Element::new(&format!("{xdr}grpSp"));
    let mut nv = Element::new(&format!("{xdr}nvGrpSpPr"));
    nv.children.push(Node::El(
        Element::new(&format!("{xdr}cNvPr"))
            .with_attr("id", &id.to_string())
            .with_attr("name", &g.name),
    ));
    nv.children
        .push(Node::El(Element::new(&format!("{xdr}cNvGrpSpPr"))));
    grp.children.push(Node::El(nv));
    let mut pr = Element::new(&format!("{xdr}grpSpPr"));
    let mut xfrm = Element::new(&format!("{a}xfrm"));
    xfrm.children.push(Node::El(
        Element::new(&format!("{a}off"))
            .with_attr("x", "0")
            .with_attr("y", "0"),
    ));
    xfrm.children.push(Node::El(
        Element::new(&format!("{a}ext"))
            .with_attr("cx", "1371600")
            .with_attr("cy", "685800"),
    ));
    xfrm.children.push(Node::El(
        Element::new(&format!("{a}chOff"))
            .with_attr("x", "0")
            .with_attr("y", "0"),
    ));
    xfrm.children.push(Node::El(
        Element::new(&format!("{a}chExt"))
            .with_attr("cx", "1371600")
            .with_attr("cy", "685800"),
    ));
    pr.children.push(Node::El(xfrm));
    grp.children.push(Node::El(pr));
    for it in &g.items {
        grp.children.push(Node::El(new_object(it, xdr, a, next_id)));
    }
    grp
}

fn create_drawing(book: &mut Book, i: usize) -> Result<String, String> {
    let sheet = book.sheets[i].part.clone();
    let p = book.free_part("xl/drawings", "drawing");
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<xdr:wsDr xmlns:xdr=\"{XDR_NS}\" xmlns:a=\"{A_NS}\"/>"
    );
    book.add_part(&p, xml.into_bytes(), CT_DRAWING);
    let rid = book.add_rel(&sheet, "drawing", &p);
    ensure_drawing_el(book, i, &rid)?;
    Ok(p)
}

fn ensure_drawing_el(book: &mut Book, i: usize, rid: &str) -> Result<(), String> {
    book.load_store(i)?;
    if book.store(i).child("drawing").is_some() {
        return Ok(());
    }
    let st = book.store_mut(i)?;
    let prefix = st.prefix.clone();
    let rp = rel_prefix(&mut st.skel.root);
    xml::insert_ordered(
        &mut st.skel.root,
        Element::new(&format!("{prefix}drawing")).with_attr(&format!("{rp}:id"), rid),
        SHEET_ORDER,
    );
    Ok(())
}

fn rel_prefix(root: &mut Element) -> String {
    match root
        .attrs
        .iter()
        .find(|x| x.1 == R_NS)
        .and_then(|x| x.0.strip_prefix("xmlns:"))
    {
        Some(p) => p.to_string(),
        None => {
            root.attrs.push(("xmlns:r".into(), R_NS.into()));
            "r".into()
        }
    }
}

fn clear_modelled(book: &mut Book, i: usize) -> Result<(), String> {
    let parts = drawing_parts(book, i);
    if parts.is_empty() {
        return Ok(());
    }
    for part in parts {
        let Some(data) = package::get(&book.parts, &part) else {
            continue;
        };
        let Ok(mut d) = xml::parse(data) else {
            continue;
        };
        let orig_len = d.root.elements().count();
        let _ = take_modelled_anchors(&mut d.root);
        if d.root.elements().next().is_none() {
            drop_drawing(book, i, &part)?;
        } else if d.root.elements().count() != orig_len {
            book.put_part(&part, xml::write_doc(&d));
        }
    }
    Ok(())
}

fn drop_drawing(book: &mut Book, i: usize, part: &str) -> Result<(), String> {
    let sheet = book.sheets[i].part.clone();
    let still = drawing_parts(book, i).into_iter().any(|p| p != part);
    drop_sheet_rel(book, &sheet, "drawing", part);
    book.remove_part(part);
    if still {
        return Ok(());
    }
    book.load_store(i)?;
    if book.store(i).child("drawing").is_none() {
        return Ok(());
    }
    let st = book.store_mut(i)?;
    st.skel
        .root
        .children
        .retain(|n| !matches!(n, Node::El(e) if e.local() == "drawing"));
    Ok(())
}

fn drop_sheet_rel(book: &mut Book, sheet: &str, ty: &str, target: &str) {
    let rels: Vec<Rel> = book
        .rels_of(sheet)
        .into_iter()
        .filter(|r| {
            !(r.short_type() == ty
                && !r.external
                && opc::resolve_target(sheet, &r.target) == target)
        })
        .collect();
    book.set_rels(sheet, rels);
}
