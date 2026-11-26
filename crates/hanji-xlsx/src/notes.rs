//! Cell notes as `<note range/>` lines: comments parts, VML balloons and
//! worksheet relationships. Unedited notes leave those parts as they were;
//! an edit rewrites the comments XML and only the note shapes in the VML.

use hanji_core::cells::CellRange;
use hanji_format::sheet::{NoteDecl, MAX_NOTE_LINES};
use hanji_package::opc::{self, Rel};
use hanji_package::package;
use hanji_package::xml::{self, Element, Node};

use crate::book::{Book, MAIN_NS, R_NS};
use crate::store::SHEET_ORDER;

const CT_COMMENTS: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.comments+xml";
const CT_VML: &str = "application/vnd.openxmlformats-officedocument.vmlDrawing";

const VML_NS: &str = "urn:schemas-microsoft-com:vml";
const VML_O: &str = "urn:schemas-microsoft-com:office:office";
const VML_X: &str = "urn:schemas-microsoft-com:office:excel";

/// This sheet's notes as `<note/>` lines, or `None` when they stay a
/// placeholder: more than [`MAX_NOTE_LINES`], an unparsable ref, overlapping
/// ranges, or a comments part that does not parse.
pub fn modelled(book: &Book, i: usize) -> Option<Vec<NoteDecl>> {
    let Some(target) = comments_part(book, i) else {
        return Some(vec![]);
    };
    let data = package::get(&book.parts, &target)?;
    let d = xml::parse(data).ok()?;
    let authors = authors_of(&d.root);
    let list = d.root.elements().find(|e| e.local() == "commentList")?;
    let mut out = vec![];
    let mut seen: Vec<CellRange> = vec![];
    for c in list.elements().filter(|e| e.local() == "comment") {
        let r = CellRange::parse(&c.get("ref")?)?;
        if seen.iter().any(|o| o.intersects(&r)) {
            return None;
        }
        seen.push(r);
        let author = c
            .get("authorId")
            .and_then(|id| id.parse::<usize>().ok())
            .and_then(|k| authors.get(k).cloned())
            .unwrap_or_default();
        out.push(NoteDecl {
            range: r.to_string(),
            author,
            text: comment_text(c),
        });
    }
    if out.len() > MAX_NOTE_LINES {
        return None;
    }
    Some(out)
}

/// Apply the `<note/>` lines of `wanted`. A no-op when they already match
/// the comments part, so an unedited file keeps that part (and its VML)
/// byte for byte.
pub fn apply(book: &mut Book, i: usize, wanted: &[NoteDecl]) -> Result<(), String> {
    let have = modelled(book, i).unwrap_or_default();
    if have == wanted {
        return Ok(());
    }
    let old_cells: Vec<String> = have.iter().map(|n| n.range.clone()).collect();
    let new_cells: Vec<String> = wanted.iter().map(|n| n.range.clone()).collect();
    set_comments(book, i, wanted)?;
    if old_cells != new_cells {
        set_vml(book, i, wanted)?;
    }
    Ok(())
}

/// Remove every note on the sheet (the comments part, note VML shapes, and
/// the comments relationship). Other VML shapes stay.
pub fn clear(book: &mut Book, i: usize) -> Result<(), String> {
    apply(book, i, &[])
}

fn comments_part(book: &Book, i: usize) -> Option<String> {
    rel_target(book, i, "comments")
}

fn vml_part(book: &Book, i: usize) -> Option<String> {
    rel_target(book, i, "vmlDrawing")
}

fn rel_target(book: &Book, i: usize, ty: &str) -> Option<String> {
    let part = &book.sheets[i].part;
    book.rels_of(part)
        .into_iter()
        .find(|r| r.short_type() == ty && !r.external)
        .map(|r| opc::resolve_target(part, &r.target))
}

fn authors_of(root: &Element) -> Vec<String> {
    root.elements()
        .find(|e| e.local() == "authors")
        .map(|a| {
            a.elements()
                .filter(|e| e.local() == "author")
                .map(|e| e.text_of(&[e.name.as_str()]))
                .collect()
        })
        .unwrap_or_default()
}

fn comment_text(c: &Element) -> String {
    let mut s = String::new();
    c.walk(&mut |e| {
        if e.local() == "t" {
            for n in &e.children {
                match n {
                    Node::Text(t) => s.push_str(&xml::unescape(t)),
                    Node::CData(t) => s.push_str(t),
                    _ => {}
                }
            }
        }
    });
    s
}

fn set_comments(book: &mut Book, i: usize, wanted: &[NoteDecl]) -> Result<(), String> {
    let sheet = book.sheets[i].part.clone();
    if wanted.is_empty() {
        if let Some(part) = comments_part(book, i) {
            drop_sheet_rel(book, &sheet, "comments", &part);
            book.remove_part(&part);
        }
        return Ok(());
    }
    let existing = comments_part(book, i);
    let (mut doc, part, prefix) = match existing {
        Some(part) => {
            let data = package::get(&book.parts, &part).ok_or_else(|| format!("no {part}"))?;
            let d = xml::parse(data).map_err(|e| format!("{part}: {e}"))?;
            let prefix = d
                .root
                .name
                .strip_suffix("comments")
                .unwrap_or("")
                .to_string();
            (d, part, prefix)
        }
        None => {
            let part = book.free_part("xl", "comments");
            let src = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<comments xmlns=\"{MAIN_NS}\"><authors/><commentList/></comments>"
            );
            let d = xml::parse(src.as_bytes()).map_err(|e| e.to_string())?;
            book.add_part(&part, src.into_bytes(), CT_COMMENTS);
            book.add_rel(&sheet, "comments", &part);
            (d, part, String::new())
        }
    };
    let mut authors = authors_of(&doc.root);
    let old: Vec<Element> = doc
        .root
        .elements()
        .find(|e| e.local() == "commentList")
        .map(|l| {
            l.elements()
                .filter(|e| e.local() == "comment")
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let mut unused = vec![true; old.len()];
    let mut placed: Vec<Option<Element>> = vec![None; wanted.len()];
    for (wi, w) in wanted.iter().enumerate() {
        let Some(k) = old.iter().enumerate().find(|(k, c)| {
            unused[*k]
                && c.get("ref")
                    .and_then(|r| CellRange::parse(&r))
                    .is_some_and(|r| r.to_string() == w.range)
        }) else {
            continue;
        };
        unused[k.0] = false;
        placed[wi] = Some(k.1.clone());
    }
    let leftover: Vec<Element> = old
        .into_iter()
        .enumerate()
        .filter(|(k, _)| unused[*k])
        .map(|(_, e)| e)
        .collect();
    let mut leftover = leftover.into_iter();
    let mut out = vec![];
    for (w, slot) in wanted.iter().zip(placed) {
        let mut c = match slot.or_else(|| leftover.next()) {
            Some(c) => c,
            None => Element::new(&format!("{prefix}comment")),
        };
        let same = c.get("ref").and_then(|r| CellRange::parse(&r)) == CellRange::parse(&w.range)
            && comment_text(&c) == w.text
            && c.get("authorId")
                .and_then(|id| id.parse::<usize>().ok())
                .and_then(|k| authors.get(k).map(String::as_str))
                == Some(w.author.as_str());
        if !same {
            c.set("ref", &w.range);
            let aid = author_id(&mut authors, &w.author);
            c.set("authorId", &aid.to_string());
            set_comment_text(&mut c, &prefix, &w.text);
        }
        out.push(c);
    }
    ensure_child(&mut doc.root, &format!("{prefix}authors"));
    ensure_child(&mut doc.root, &format!("{prefix}commentList"));
    if let Some(a) = doc.root.elements_mut().find(|e| e.local() == "authors") {
        a.children = authors
            .iter()
            .map(|name| Node::El(text_el(&format!("{prefix}author"), &xml::escape_text(name))))
            .collect();
    }
    if let Some(list) = doc.root.elements_mut().find(|e| e.local() == "commentList") {
        list.children = out.into_iter().map(Node::El).collect();
    }
    book.put_part(&part, xml::write_doc(&doc));
    Ok(())
}

fn author_id(authors: &mut Vec<String>, name: &str) -> usize {
    if let Some(k) = authors.iter().position(|a| a == name) {
        return k;
    }
    authors.push(name.to_string());
    authors.len() - 1
}

fn set_comment_text(c: &mut Element, prefix: &str, text: &str) {
    c.children
        .retain(|n| !matches!(n, Node::El(e) if e.local() == "text"));
    let t = Element::new(&format!("{prefix}t"))
        .with_attr("xml:space", "preserve")
        .with_text(&xml::escape_text(text));
    c.children
        .push(Node::El(Element::new(&format!("{prefix}text")).with_el(t)));
}

fn ensure_child(root: &mut Element, qname: &str) {
    let local = xml::local_name(qname);
    if root.elements().any(|e| e.local() == local) {
        return;
    }
    root.children.push(Node::El(Element::new(qname)));
}

fn text_el(name: &str, escaped: &str) -> Element {
    Element::new(name).with_text(escaped)
}

fn set_vml(book: &mut Book, i: usize, wanted: &[NoteDecl]) -> Result<(), String> {
    let sheet = book.sheets[i].part.clone();
    let wanted_cells: Vec<(u32, u32)> = wanted
        .iter()
        .filter_map(|w| {
            CellRange::parse(&w.range).map(|r| (r.first.row.saturating_sub(1), r.first.col))
        })
        .collect();
    if wanted.is_empty() {
        if let Some(p) = vml_part(book, i) {
            let data = package::get(&book.parts, &p).ok_or_else(|| format!("no {p}"))?;
            let text = String::from_utf8_lossy(data).into_owned();
            let (rest, _) = strip_note_shapes(&text);
            if vml_has_other_shapes(&rest) {
                book.put_part(&p, rest.into_bytes());
            } else {
                drop_sheet_rel(book, &sheet, "vmlDrawing", &p);
                book.remove_part(&p);
                drop_legacy_drawing(book, i)?;
            }
        } else {
            drop_legacy_drawing(book, i)?;
        }
        return Ok(());
    }
    let part = match vml_part(book, i) {
        Some(p) => p,
        None => {
            let p = book.free_named("xl/drawings", "vmlDrawing", "vml");
            book.add_part(&p, empty_vml().into_bytes(), CT_VML);
            let rid = book.add_rel(&sheet, "vmlDrawing", &p);
            ensure_legacy_drawing(book, i, &rid)?;
            p
        }
    };
    book.load_store(i)?;
    if book.store(i).child("legacyDrawing").is_none() {
        let rid = book
            .rels_of(&sheet)
            .into_iter()
            .find(|r| r.short_type() == "vmlDrawing")
            .map(|r| r.id)
            .ok_or_else(|| format!("{sheet}: the vmlDrawing relationship was just added"))?;
        ensure_legacy_drawing(book, i, &rid)?;
    }
    let data = package::get(&book.parts, &part).ok_or_else(|| format!("no {part}"))?;
    let text = String::from_utf8_lossy(data).into_owned();
    let (mut rest, leftover) = strip_note_shapes(&text);
    let mut unused = vec![true; leftover.len()];
    let mut placed: Vec<Option<String>> = vec![None; wanted_cells.len()];
    for (wi, &(row, col)) in wanted_cells.iter().enumerate() {
        let Some(k) = leftover
            .iter()
            .enumerate()
            .find(|(k, (r, c, _))| unused[*k] && *r == row && *c == col)
        else {
            continue;
        };
        unused[k.0] = false;
        placed[wi] = Some(k.1 .2.clone());
    }
    for (k, (_, _, xml)) in leftover.iter().enumerate() {
        if !unused[k] {
            continue;
        }
        if let Some(slot) = placed.iter_mut().find(|s| s.is_none()) {
            unused[k] = false;
            *slot = Some(xml.clone());
        }
    }
    let mut shapes = String::new();
    for (cell, slot) in wanted_cells.iter().zip(placed) {
        let (row, col) = *cell;
        let xml = match slot {
            Some(s) => set_vml_rc(&s, row, col),
            None => new_vml_shape(
                next_shape_id(&rest) + shapes.matches("_x0000_s").count() as u32,
                row,
                col,
            ),
        };
        shapes.push_str(&xml);
    }
    if let Some(at) = rest.rfind("</xml>") {
        rest.insert_str(at, &shapes);
    } else {
        rest.push_str(&shapes);
    }
    book.put_part(&part, rest.into_bytes());
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

fn ensure_legacy_drawing(book: &mut Book, i: usize, rid: &str) -> Result<(), String> {
    book.load_store(i)?;
    if book.store(i).child("legacyDrawing").is_some() {
        return Ok(());
    }
    let st = book.store_mut(i)?;
    let p = st.prefix.clone();
    let rp = rel_prefix(&mut st.skel.root);
    xml::insert_ordered(
        &mut st.skel.root,
        Element::new(&format!("{p}legacyDrawing")).with_attr(&format!("{rp}:id"), rid),
        SHEET_ORDER,
    );
    Ok(())
}

fn drop_legacy_drawing(book: &mut Book, i: usize) -> Result<(), String> {
    if vml_part(book, i).is_some() {
        return Ok(());
    }
    book.load_store(i)?;
    if book.store(i).child("legacyDrawing").is_none() {
        return Ok(());
    }
    let st = book.store_mut(i)?;
    st.skel
        .root
        .children
        .retain(|n| !matches!(n, Node::El(e) if e.local() == "legacyDrawing"));
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

struct VmlNote {
    start: usize,
    end: usize,
    row: u32,
    col: u32,
}

fn strip_note_shapes(text: &str) -> (String, Vec<(u32, u32, String)>) {
    let shapes = vml_note_shapes(text);
    let mut notes = vec![];
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for s in &shapes {
        out.push_str(&text[at..s.start]);
        notes.push((s.row, s.col, text[s.start..s.end].to_string()));
        at = s.end;
    }
    out.push_str(&text[at..]);
    (out, notes)
}

fn vml_note_shapes(text: &str) -> Vec<VmlNote> {
    let mut out = vec![];
    let mut at = 0;
    while let Some(k) = text[at..].find("ObjectType=\"Note\"") {
        let hit = at + k;
        let Some(cd) = text[..hit].rfind('<') else {
            at = hit + 1;
            continue;
        };
        let Some(cd_end_rel) = text[hit..].find("</") else {
            break;
        };
        let after = &text[hit + cd_end_rel..];
        let Some(gt) = after.find('>') else {
            break;
        };
        let cd_end = hit + cd_end_rel + gt + 1;
        let Some((start, qname)) = last_open_shape(&text[..cd]) else {
            at = cd_end;
            continue;
        };
        let close = format!("</{qname}>");
        let Some(end_rel) = text[cd_end..].find(&close) else {
            at = cd_end;
            continue;
        };
        let end = cd_end + end_rel + close.len();
        let block = &text[start..end];
        out.push(VmlNote {
            start,
            end,
            row: tagged_num(block, "Row").unwrap_or(0),
            col: tagged_num(block, "Column").unwrap_or(0),
        });
        at = end;
    }
    out
}

fn last_open_shape(text: &str) -> Option<(usize, String)> {
    let bytes = text.as_bytes();
    let mut i = bytes.len();
    while i > 0 {
        i -= 1;
        if bytes[i] != b'<' {
            continue;
        }
        let rest = &text[i + 1..];
        if rest.starts_with('/') {
            continue;
        }
        let name_end = rest.find(|c: char| c.is_whitespace() || matches!(c, '>' | '/'))?;
        let qname = &rest[..name_end];
        if xml::local_name(qname) == "shape" {
            return Some((i, qname.to_string()));
        }
    }
    None
}

fn tagged_num(block: &str, local: &str) -> Option<u32> {
    let needle = format!("{local}>");
    let k = block.find(&needle)?;
    let rest = &block[k + needle.len()..];
    let end = rest.find('<')?;
    rest[..end].trim().parse().ok()
}

fn set_vml_rc(block: &str, row: u32, col: u32) -> String {
    let s = set_tagged_num(block, "Row", row);
    set_tagged_num(&s, "Column", col)
}

fn set_tagged_num(block: &str, local: &str, v: u32) -> String {
    let needle = format!("{local}>");
    let Some(k) = block.find(&needle) else {
        return block.to_string();
    };
    let start = k + needle.len();
    let Some(end) = block[start..].find('<') else {
        return block.to_string();
    };
    format!("{}{}{}", &block[..start], v, &block[start + end..])
}

fn next_shape_id(text: &str) -> u32 {
    let mut max = 1024u32;
    let mut rest = text;
    while let Some(k) = rest.find("_x0000_s") {
        rest = &rest[k + 8..];
        let n: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(v) = n.parse::<u32>() {
            max = max.max(v);
        }
    }
    max + 1
}

fn new_vml_shape(id: u32, row: u32, col: u32) -> String {
    format!(
        "<v:shape xmlns:v=\"{VML_NS}\" xmlns:o=\"{VML_O}\" xmlns:x=\"{VML_X}\" id=\"_x0000_s{id}\" type=\"#_x0000_t202\" style=\"position:absolute;margin-left:59.25pt;margin-top:1.5pt;width:96pt;height:55.5pt;z-index:1;visibility:hidden\" fillcolor=\"#ffffe1\" o:insetmode=\"auto\"><v:fill color2=\"#ffffe1\"/><v:shadow on=\"t\" color=\"black\" obscured=\"t\"/><v:path o:connecttype=\"none\"/><v:textbox style=\"mso-direction-alt:auto\"><div style=\"text-align:left\"/></v:textbox><x:ClientData ObjectType=\"Note\"><x:MoveWithCells/><x:SizeWithCells/><x:AutoFill>False</x:AutoFill><x:Row>{row}</x:Row><x:Column>{col}</x:Column></x:ClientData></v:shape>"
    )
}

fn empty_vml() -> String {
    format!(
        "<xml xmlns:v=\"{VML_NS}\" xmlns:o=\"{VML_O}\" xmlns:x=\"{VML_X}\"><o:shapelayout v:ext=\"edit\"><o:idmap v:ext=\"edit\" data=\"1\"/></o:shapelayout><v:shapetype id=\"_x0000_t202\" coordsize=\"21600,21600\" o:spt=\"202\" path=\"m,l,21600r21600,l21600,xe\"><v:stroke joinstyle=\"miter\"/><v:path gradientshapeok=\"t\" o:connecttype=\"rect\"/></v:shapetype></xml>"
    )
}

fn vml_has_other_shapes(text: &str) -> bool {
    let mut rest = text;
    while let Some(k) = rest.find('<') {
        let s = &rest[k + 1..];
        if s.starts_with('/') {
            rest = s;
            continue;
        }
        let name_end = s
            .find(|c: char| c.is_whitespace() || matches!(c, '>' | '/'))
            .unwrap_or(0);
        if xml::local_name(&s[..name_end]) == "shape" {
            return true;
        }
        rest = &s[name_end.max(1)..];
    }
    false
}

trait WithKids {
    fn with_text(self, escaped: &str) -> Element;
    fn with_el(self, child: Element) -> Element;
}

impl WithKids for Element {
    fn with_text(mut self, escaped: &str) -> Element {
        self.children = vec![Node::Text(escaped.into())];
        self
    }
    fn with_el(mut self, child: Element) -> Element {
        self.children = vec![Node::El(child)];
        self
    }
}
