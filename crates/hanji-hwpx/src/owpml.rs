//! OWPML (KS X 6101) names and small helpers shared by import and export.

use hanji_format::NoteKind;
use hanji_package::xml::{escape_text, is_blank, unescape, Element, Node};

pub const HEADER_PART: &str = "Contents/header.xml";
pub const CONTENT_PART: &str = "Contents/content.hpf";

/// The section number of `Contents/section{N}.xml`.
pub fn section_number(part: &str) -> Option<u32> {
    part.strip_prefix("Contents/section")?
        .strip_suffix(".xml")?
        .parse()
        .ok()
}

/// `hp:ctrl` children that take no room in the text: kept as markers in
/// their run. They set up the page (columns, header, footer, page numbers)
/// or mark a place (bookmark, field begin and end), so they are never
/// dropped with nearby text.
pub const MARKER_CTRLS: &[&str] = &[
    "hp:colPr",
    "hp:bookmark",
    "hp:fieldBegin",
    "hp:fieldEnd",
    "hp:header",
    "hp:footer",
    "hp:pageNum",
    "hp:pageNumCtrl",
    "hp:pageHiding",
    "hp:newNum",
    "hp:indexmark",
    "hp:hiddenComment",
];

/// Zero-width elements inside `hp:t` (not tracked-change marks).
pub const T_MARKERS: &[&str] = &["hp:markpenBegin", "hp:markpenEnd", "hp:titleMark"];

/// Tracked-change marks (`TrackChangeTag`), inside `hp:t`.
pub const TRACK_MARKS: &[&str] = &[
    "hp:insertBegin",
    "hp:insertEnd",
    "hp:deleteBegin",
    "hp:deleteEnd",
];

/// Characters an `hp:t` child element stands for.
pub fn t_char(name: &str) -> Option<char> {
    Some(match name {
        "hp:tab" => '\t',
        "hp:nbSpace" => '\u{a0}',
        "hp:fwSpace" => '\u{2007}',
        "hp:hyphen" => '\u{ad}',
        _ => return None,
    })
}

/// The element a character is written as inside `hp:t`, if it is one.
pub fn char_element(c: char) -> Option<&'static str> {
    Some(match c {
        '\t' => "hp:tab",
        '\u{a0}' => "hp:nbSpace",
        '\u{2007}' => "hp:fwSpace",
        '\u{ad}' => "hp:hyphen",
        _ => return None,
    })
}

/// Placeholder kind by element (or `hp:ctrl` child).
pub fn keep_kind(name: &str) -> String {
    let local = name.rsplit(':').next().unwrap_or(name);
    match local {
        "tbl" => "table",
        "pic" | "rect" | "ellipse" | "arc" | "polygon" | "curve" | "connectLine" | "line"
        | "container" | "textart" | "chart" => "drawing",
        "equation" => "math",
        "ole" => "object",
        "btn" | "checkBtn" | "radioBtn" | "comboBox" | "listBox" | "edit" | "scrollBar" => {
            "form-control"
        }
        "footNote" => "footnote",
        "endNote" => "endnote",
        "autoNum" => "number",
        "compose" | "dutmal" | "t" => "text",
        "video" => "media",
        "p" => "paragraph",
        other => other,
    }
    .to_string()
}

/// Locals a shown footnote or endnote body may hold. A table, drawing or
/// nested object stays a placeholder.
const NOTE_SHOWN: &[&str] = &[
    "footNote",
    "endNote",
    "subList",
    "p",
    "run",
    "ctrl",
    "autoNum",
    "autoNumFormat",
    "t",
    "linesegarray",
    "lineseg",
];

/// An `hp:ctrl` whose `hp:footNote` / `hp:endNote` body the text can show:
/// one paragraph of ordinary text.
pub fn shown_note(ctrl: &Element) -> Option<(NoteKind, String)> {
    if !ctrl.is("hp:ctrl") {
        return None;
    }
    let inner = ctrl_kind(ctrl)?;
    let kind = if inner.is("hp:footNote") {
        NoteKind::Footnote
    } else if inner.is("hp:endNote") {
        NoteKind::Endnote
    } else {
        return None;
    };
    let paras: Vec<&Element> = inner
        .child("hp:subList")
        .map(|s| s.elements().filter(|e| e.is("hp:p")).collect())
        .unwrap_or_default();
    if paras.len() != 1 {
        return None;
    }
    let mut ok = true;
    inner.walk(&mut |e| {
        if !NOTE_SHOWN.contains(&e.local()) {
            ok = false;
        }
    });
    if !ok {
        return None;
    }
    let body = inner.text_of(&["hp:t"]);
    if body.contains('\n') || body.contains('\r') {
        return None;
    }
    Some((kind, body))
}

/// Writes `body` into the first `hp:t` of a note control, clearing the rest.
/// Adds a `hp:t` when the note has none. Returns whether the stored text changed.
pub fn set_note_body(ctrl: &mut Element, body: &str) -> bool {
    let Some(inner) = ctrl
        .elements_mut()
        .find(|e| e.local() == "footNote" || e.local() == "endNote")
    else {
        return false;
    };
    if inner.text_of(&["hp:t"]) == body {
        return false;
    }
    let mut used = false;
    set_note_t(inner, body, &mut used);
    if !used {
        let mut t = Element::new("hp:t");
        t.children.push(Node::Text(escape_text(body)));
        let mut r = Element::new("hp:run");
        r.children.push(Node::El(t));
        if let Some(p) = inner
            .child_mut("hp:subList")
            .and_then(|s| s.child_mut("hp:p"))
        {
            p.children.push(Node::El(r));
        } else {
            inner.children.push(Node::El(r));
        }
    }
    true
}

fn set_note_t(e: &mut Element, body: &str, used: &mut bool) {
    if e.local() == "t" {
        if !*used {
            e.children = vec![Node::Text(escape_text(body))];
            *used = true;
        } else {
            e.children.clear();
        }
        return;
    }
    for c in e.elements_mut() {
        set_note_t(c, body, used);
    }
}

/// A piece of a shown `hp:drawText`: outer run text, a paragraph break, or
/// a footnote/endnote the text already models as `[^n]`.
pub(crate) enum DrawBit<'a> {
    Text(String),
    Break,
    Note(&'a Element),
}

/// The `hp:drawText` body a drawing object can show: ordinary text in one
/// or more paragraphs, and shown notes. Paragraphs join with `\n` (`<br/>`
/// in the text). Nested pictures, tables, equations and unshown controls
/// stay placeholders.
pub fn shown_draw_text(el: &Element) -> Option<String> {
    let bits = draw_bits(el)?;
    let mut body = String::new();
    let mut notes = false;
    for b in &bits {
        match b {
            DrawBit::Text(s) => body.push_str(s),
            DrawBit::Break => body.push('\n'),
            DrawBit::Note(_) => notes = true,
        }
    }
    if body.contains('\r') {
        return None;
    }
    if body.is_empty() && !notes {
        return None;
    }
    Some(body)
}

/// Pieces of a shown drawing, in order, or `None` when the object stays a
/// placeholder.
pub(crate) fn draw_bits(el: &Element) -> Option<Vec<DrawBit<'_>>> {
    if keep_kind(&el.name) != "drawing" {
        return None;
    }
    let dt = el.descendants("hp:drawText").into_iter().next()?;
    let sub = dt.child("hp:subList")?;
    let mut paras = Vec::new();
    for n in &sub.children {
        match n {
            Node::El(e) if e.is("hp:p") => {
                if !draw_para_showable(e) {
                    return None;
                }
                paras.push(e);
            }
            n if is_blank(n) => {}
            Node::El(e) if e.is("hp:textMargin") => {}
            _ => return None,
        }
    }
    if paras.is_empty() {
        return None;
    }
    let mut bits = Vec::new();
    let mut any = false;
    for (i, p) in paras.iter().enumerate() {
        if i > 0 {
            bits.push(DrawBit::Break);
        }
        for b in draw_para_bits(p) {
            match &b {
                DrawBit::Text(s) if !s.is_empty() => any = true,
                DrawBit::Note(_) => any = true,
                _ => {}
            }
            bits.push(b);
        }
    }
    any.then_some(bits)
}

fn draw_para_showable(p: &Element) -> bool {
    for n in &p.children {
        match n {
            Node::El(e) if e.is("hp:run") => {
                for c in e.elements() {
                    if c.is("hp:t") {
                        if !draw_t_plain(c) {
                            return false;
                        }
                    } else if !(c.is("hp:ctrl") && shown_note(c).is_some()) {
                        return false;
                    }
                }
            }
            Node::El(e) if e.is("hp:linesegarray") => {}
            n if is_blank(n) => {}
            _ => return false,
        }
    }
    true
}

fn draw_t_plain(t: &Element) -> bool {
    t.children.iter().all(|n| match n {
        Node::Text(s) => {
            let u = unescape(s);
            !u.contains('\n') && !u.contains('\r')
        }
        Node::CData(s) => !s.contains('\n') && !s.contains('\r'),
        Node::Comment(_) => true,
        _ => false,
    })
}

fn draw_para_bits(p: &Element) -> Vec<DrawBit<'_>> {
    let mut bits = Vec::new();
    for r in p.elements().filter(|e| e.is("hp:run")) {
        for c in r.elements() {
            if c.is("hp:t") {
                let mut s = String::new();
                for n in &c.children {
                    match n {
                        Node::Text(t) => s.push_str(&unescape(t)),
                        Node::CData(t) => s.push_str(t),
                        _ => {}
                    }
                }
                bits.push(DrawBit::Text(s));
            } else if c.is("hp:ctrl") {
                bits.push(DrawBit::Note(c));
            }
        }
    }
    bits
}

fn draw_note_bodies(el: &Element) -> Vec<String> {
    draw_bits(el)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|b| match b {
            DrawBit::Note(c) => shown_note(c).map(|(_, body)| body),
            _ => None,
        })
        .collect()
}

/// `xml` of a stored drawing, with outer `hp:drawText` `hp:t` set to `body`
/// (`\n` is a paragraph). Unchanged text keeps the original fragment.
pub fn with_draw_text(xml: &str, body: &str) -> String {
    with_draw_contents(xml, body, &[])
}

/// Like [`with_draw_text`], and writes shown notes inside the box from
/// `notes` in order. Extra stored notes are dropped; the original fragment
/// is kept when body and notes match.
pub fn with_draw_contents(xml: &str, body: &str, notes: &[&str]) -> String {
    let mut e = hanji_package::xml::fragment(xml);
    let same_body = shown_draw_text(&e).as_deref() == Some(body);
    let same_notes = draw_note_bodies(&e)
        .iter()
        .map(String::as_str)
        .eq(notes.iter().copied());
    if same_body && same_notes {
        return xml.to_string();
    }
    if !same_body {
        set_draw_paras(&mut e, body);
    }
    if !same_notes {
        set_draw_notes(&mut e, notes);
    }
    drop_stale_layouts(&mut e);
    e.to_xml()
}

/// Writes `body` into the outer `hp:t` of `hp:drawText` (one paragraph per
/// `\n`). Unchanged text is a no-op. Does not touch notes.
pub fn set_draw_text(el: &mut Element, body: &str) -> bool {
    if shown_draw_text(el).as_deref() == Some(body) {
        return false;
    }
    set_draw_paras(el, body);
    true
}

fn set_draw_paras(el: &mut Element, body: &str) {
    let Some(dt) = find_draw_text_mut(el) else {
        return;
    };
    let parts: Vec<&str> = body.split('\n').collect();
    let Some(sub) = dt.child_mut("hp:subList") else {
        let mut sub = Element::new("hp:subList");
        for part in &parts {
            sub.children.push(Node::El(draw_para(None, part)));
        }
        dt.children.push(Node::El(sub));
        return;
    };
    let model = sub.elements().find(|e| e.is("hp:p")).map(Element::shell);
    let mut kept = Vec::new();
    let mut pi = 0usize;
    for n in std::mem::take(&mut sub.children) {
        match n {
            Node::El(mut p) if p.is("hp:p") => {
                if pi < parts.len() {
                    set_para_outer_t(&mut p, parts[pi]);
                    drop_layout(&mut p);
                    kept.push(Node::El(p));
                    pi += 1;
                }
            }
            other => kept.push(other),
        }
    }
    sub.children = kept;
    while pi < parts.len() {
        sub.children
            .push(Node::El(draw_para(model.as_ref(), parts[pi])));
        pi += 1;
    }
}

fn draw_para(model: Option<&Element>, body: &str) -> Element {
    let mut t = Element::new("hp:t");
    t.children.push(Node::Text(escape_text(body)));
    let mut run = Element::new("hp:run");
    run.children.push(Node::El(t));
    let mut p = model.cloned().unwrap_or_else(|| Element::new("hp:p"));
    p.children.clear();
    p.children.push(Node::El(run));
    p
}

fn set_para_outer_t(p: &mut Element, body: &str) {
    let mut used = false;
    for n in &mut p.children {
        let Node::El(r) = n else { continue };
        if !r.is("hp:run") {
            continue;
        }
        for c in r.elements_mut() {
            if !c.is("hp:t") {
                continue;
            }
            if !used {
                c.children = vec![Node::Text(escape_text(body))];
                used = true;
            } else {
                c.children.clear();
            }
        }
    }
    if !used {
        p.children.push(Node::El({
            let mut t = Element::new("hp:t");
            t.children.push(Node::Text(escape_text(body)));
            let mut run = Element::new("hp:run");
            run.children.push(Node::El(t));
            run
        }));
    }
}

fn set_draw_notes(el: &mut Element, notes: &[&str]) {
    let Some(dt) = find_draw_text_mut(el) else {
        return;
    };
    let mut i = 0usize;
    set_draw_note_ctrls(dt, notes, &mut i);
    let mut keep = notes.len();
    retain_draw_notes(dt, &mut keep);
}

fn set_draw_note_ctrls(e: &mut Element, notes: &[&str], i: &mut usize) {
    if e.is("hp:ctrl") && shown_note(e).is_some() {
        if *i < notes.len() {
            set_note_body(e, notes[*i]);
            *i += 1;
        }
        return;
    }
    for c in e.elements_mut() {
        set_draw_note_ctrls(c, notes, i);
    }
}

fn retain_draw_notes(e: &mut Element, keep: &mut usize) {
    if e.is("hp:run") {
        e.children.retain(|n| match n {
            Node::El(c) if c.is("hp:ctrl") && shown_note(c).is_some() => {
                if *keep > 0 {
                    *keep -= 1;
                    true
                } else {
                    false
                }
            }
            _ => true,
        });
        return;
    }
    for c in e.elements_mut() {
        retain_draw_notes(c, keep);
    }
}

fn find_draw_text_mut(el: &mut Element) -> Option<&mut Element> {
    if el.is("hp:drawText") {
        return Some(el);
    }
    for c in el.elements_mut() {
        if let Some(d) = find_draw_text_mut(c) {
            return Some(d);
        }
    }
    None
}

/// A copy whose draw-text runs are empty, so the wrap's fingerprint is the
/// object, not the words.
pub fn fingerprint_drawing(el: &Element) -> Element {
    let mut x = el.clone();
    clear_draw_t(&mut x);
    x
}

fn clear_draw_t(e: &mut Element) {
    if e.is("hp:drawText") {
        e.walk_mut(&mut |x| {
            if x.is("hp:t") {
                x.children.clear();
            }
        });
        return;
    }
    for c in e.elements_mut() {
        clear_draw_t(c);
    }
}

/// The equation script the text can show: ordinary one-line text, not empty
/// and not holding `</math>` (which would close `<math>`).
pub fn shown_script(el: &Element) -> Option<String> {
    if !el.is("hp:equation") {
        return None;
    }
    let body = el.text_of(&["hp:script"]);
    if body.is_empty() || body.contains('\n') || body.contains('\r') || body.contains("</math>") {
        return None;
    }
    Some(body)
}

/// Writes `script` into `hp:script`, adding the child when the object has
/// none. Returns whether the stored text changed.
pub fn set_script(el: &mut Element, script: &str) -> bool {
    if el.text_of(&["hp:script"]) == script {
        return false;
    }
    if let Some(s) = el.child_mut("hp:script") {
        s.children = vec![Node::Text(escape_text(script))];
        return true;
    }
    let mut s = Element::new("hp:script");
    s.children.push(Node::Text(escape_text(script)));
    el.children.push(Node::El(s));
    true
}

/// A new `hp:equation` for a `$…$` that was not in the file: Hancom's
/// in-line defaults, a guessed box (`hp:sz`) Hancom replaces when it
/// lays the formula out.
pub fn new_equation(id: u32, z_order: u32, script: &str) -> Element {
    let mut eq = Element::new("hp:equation")
        .with_attr("id", &id.to_string())
        .with_attr("zOrder", &z_order.to_string())
        .with_attr("numberingType", "EQUATION")
        .with_attr("textWrap", "TOP_AND_BOTTOM")
        .with_attr("textFlow", "BOTH_SIDES")
        .with_attr("lock", "0")
        .with_attr("dropcapstyle", "None")
        .with_attr("version", "Equation Version 60")
        .with_attr("baseLine", "85")
        .with_attr("textColor", "#000000")
        .with_attr("baseUnit", "1000")
        .with_attr("lineMode", "CHAR")
        .with_attr("font", "HYhwpEQ");
    eq.children.extend([
        Node::El(
            Element::new("hp:sz")
                .with_attr("width", "1000")
                .with_attr("widthRelTo", "ABSOLUTE")
                .with_attr("height", "1000")
                .with_attr("heightRelTo", "ABSOLUTE")
                .with_attr("protect", "0"),
        ),
        Node::El(
            Element::new("hp:pos")
                .with_attr("treatAsChar", "1")
                .with_attr("affectLSpacing", "0")
                .with_attr("flowWithText", "1")
                .with_attr("allowOverlap", "0")
                .with_attr("holdAnchorAndSO", "0")
                .with_attr("vertRelTo", "PARA")
                .with_attr("horzRelTo", "PARA")
                .with_attr("vertAlign", "TOP")
                .with_attr("horzAlign", "LEFT")
                .with_attr("vertOffset", "0")
                .with_attr("horzOffset", "0"),
        ),
        Node::El(
            Element::new("hp:outMargin")
                .with_attr("left", "56")
                .with_attr("right", "56")
                .with_attr("top", "0")
                .with_attr("bottom", "0"),
        ),
        Node::El({
            let mut s = Element::new("hp:script");
            s.children.push(Node::Text(escape_text(script)));
            s
        }),
    ]);
    eq
}

/// Order of `hh:charPr` children (KS X 6101 `CharShapeType`), for inserting a flag.
pub const CHARPR_ORDER: &[&str] = &[
    "fontRef",
    "ratio",
    "spacing",
    "relSz",
    "offset",
    "italic",
    "bold",
    "underline",
    "strikeout",
    "outline",
    "shadow",
    "emboss",
    "engrave",
    "supscript",
    "subscript",
];

/// The first element child of `hp:ctrl`, which says what the control is.
pub fn ctrl_kind(ctrl: &Element) -> Option<&Element> {
    ctrl.elements().next()
}

/// An `hp:ctrl` (or bare `hp:autoNum`) whose formatted number the text can
/// show: `prefix + num + suffix` when the format is `DIGIT`. Other formats
/// (circled digits, Hangul, Roman) stay placeholders.
pub fn shown_auto_num(el: &Element) -> Option<String> {
    let auto = auto_num_el(el)?;
    let num = auto.get("num")?;
    if num.is_empty() || num.contains('\n') || num.contains('\r') {
        return None;
    }
    let (prefix, suffix, ty) = auto_num_format(auto);
    if ty != "DIGIT" {
        return None;
    }
    Some(format!("{prefix}{num}{suffix}"))
}

/// `xml` of a stored auto-number when `body` is still that formatted number,
/// or when it still parses as prefix + digits + suffix (the `num` is
/// updated). Any other body is ordinary text (the caller writes `hp:t`).
pub fn with_auto_num(xml: &str, body: &str) -> Option<String> {
    let mut e = hanji_package::xml::fragment(xml);
    let shown = shown_auto_num(&e)?;
    if shown == body {
        return Some(xml.to_string());
    }
    let (prefix, suffix, ty) = auto_num_format(auto_num_el(&e)?);
    if ty != "DIGIT" {
        return None;
    }
    let inner = body.strip_prefix(&prefix)?.strip_suffix(&suffix)?;
    if inner.is_empty() || !inner.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    auto_num_el_mut(&mut e)?.set("num", inner);
    Some(e.to_xml())
}

/// A copy whose `num` is empty, so the wrap's fingerprint is the format,
/// not the shown digits.
pub fn fingerprint_auto_num(el: &Element) -> Element {
    let mut x = el.clone();
    x.walk_mut(&mut |e| {
        if e.is("hp:autoNum") {
            e.remove_attr("num");
        }
    });
    x
}

fn auto_num_el(el: &Element) -> Option<&Element> {
    if el.is("hp:autoNum") {
        return Some(el);
    }
    if !el.is("hp:ctrl") {
        return None;
    }
    let mut auto = None;
    for c in el.elements() {
        if c.is("hp:autoNum") {
            if auto.is_some() {
                return None;
            }
            auto = Some(c);
        } else {
            return None;
        }
    }
    auto
}

fn auto_num_el_mut(el: &mut Element) -> Option<&mut Element> {
    if el.is("hp:autoNum") {
        return Some(el);
    }
    if !el.is("hp:ctrl") {
        return None;
    }
    let mut found = None;
    for (i, n) in el.children.iter().enumerate() {
        let Node::El(c) = n else { continue };
        if c.is("hp:autoNum") {
            if found.is_some() {
                return None;
            }
            found = Some(i);
        } else {
            return None;
        }
    }
    match found {
        Some(i) => match &mut el.children[i] {
            Node::El(c) => Some(c),
            _ => None,
        },
        None => None,
    }
}

fn auto_num_format(auto: &Element) -> (String, String, String) {
    let fmt = auto.child("hp:autoNumFormat");
    let ty = fmt
        .and_then(|f| f.get("type"))
        .unwrap_or_else(|| "DIGIT".into());
    let prefix = fmt.and_then(|f| f.get("prefixChar")).unwrap_or_default();
    let suffix = fmt.and_then(|f| f.get("suffixChar")).unwrap_or_default();
    (prefix, suffix, ty)
}

/// A child's qualified name, `#text` for character data.
pub fn node_name(n: &Node) -> &str {
    match n {
        Node::El(e) => &e.name,
        _ => "#text",
    }
}

/// Attributes that refer to a tracked change (`hh:trackChange id`): on the
/// marks, on a run (a character shape change) and on a paragraph.
const TC_ATTRS: [&str; 3] = ["TcId", "charTcId", "paraTcId"];

/// Each tracked-change id `e` and its descendants refer to, in document order.
pub fn tracked_ids(e: &Element, f: &mut dyn FnMut(String)) {
    e.walk(&mut |x| {
        for a in TC_ATTRS {
            if let Some(v) = x.get(a) {
                f(v);
            }
        }
    });
}

/// Whether an element holds a tracked-change mark or a tracked shape change.
pub fn has_tracked_change(e: &Element) -> bool {
    let mut found = false;
    e.walk(&mut |x| {
        found |=
            TRACK_MARKS.contains(&x.name.as_str()) || TC_ATTRS.iter().any(|a| x.attr(a).is_some())
    });
    found
}

// ------------------------------------------------------------ layout cache

/// How long a paragraph is on the axis its layout cache counts in
/// (`hp:lineseg textpos`, HWP's character positions): UTF-16 units of its
/// text; 8 for a tab and for each control or object (`hp:secPr`, each
/// `hp:ctrl` child, a table, a picture…); 1 for a line break and the fixed
/// characters (`hp:nbSpace`, `hp:fwSpace`, `hp:hyphen`); nothing for the
/// zero-width marks. An element this does not know counts 8, so the length
/// errs long rather than flag a cache Hancom wrote (none in the corpus, nor
/// in rhwp's own sample files but for a few converted from HWP 3).
pub fn char_count(p: &Element) -> usize {
    let u16s = |s: &str| s.chars().map(char::len_utf16).sum::<usize>();
    let mut n = 0;
    for r in p.elements().filter(|r| r.is("hp:run")) {
        for c in r.elements() {
            if c.is("hp:t") {
                for k in &c.children {
                    n += match k {
                        Node::Text(t) => u16s(&hanji_package::xml::unescape(t)),
                        Node::CData(t) => u16s(t),
                        Node::El(e)
                            if e.is("hp:lineBreak")
                                || t_char(&e.name).is_some_and(|c| c != '\t') =>
                        {
                            1
                        }
                        Node::El(e)
                            if T_MARKERS.contains(&e.name.as_str())
                                || TRACK_MARKS.contains(&e.name.as_str()) =>
                        {
                            0
                        }
                        Node::El(_) => 8,
                        _ => 0,
                    };
                }
            } else if c.is("hp:ctrl") {
                n += 8 * c.elements().count();
            } else {
                n += 8;
            }
        }
    }
    n
}

/// Why a paragraph's own layout cache (`hp:linesegarray`) cannot be the
/// layout of its content: a line starting past the paragraph's end (its
/// last character, then its end mark). Hancom does not open such a file
/// without repairing it (rhwp's serializer notes the same of Hangul 2022).
pub fn stale_layout(p: &Element) -> Option<String> {
    let lines = p.child("hp:linesegarray")?;
    let len = char_count(p);
    let past = lines
        .elements()
        .filter_map(|l| l.get("textpos")?.parse::<usize>().ok())
        .find(|&t| t > len + 1)?;
    Some(format!(
        "a line starts at {past}, past the paragraph's end ({len} characters and its end mark)"
    ))
}

/// Every paragraph under `e` (itself included, cells and text boxes too)
/// whose layout cache is stale ([`stale_layout`]), with its text.
pub fn layout_problems(e: &Element) -> Vec<String> {
    let mut out = vec![];
    e.walk(&mut |x| {
        if x.is("hp:p") {
            if let Some(why) = stale_layout(x) {
                let text: String = x.text_of(&["hp:t"]).chars().take(30).collect();
                out.push(format!("paragraph {text:?}: {why}"));
            }
        }
    });
    out
}

/// Drops the layout cache of every paragraph under `e` whose cache is
/// stale; Hancom lays those paragraphs out again.
pub fn drop_stale_layouts(e: &mut Element) {
    e.walk_mut(&mut |x| {
        if x.is("hp:p") && stale_layout(x).is_some() {
            drop_layout(x);
        }
    });
}

/// Drops a paragraph's own layout cache (`hp:linesegarray`), which Hancom
/// recomputes when it opens the file.
pub fn drop_layout(p: &mut Element) {
    p.children
        .retain(|n| !matches!(n, Node::El(e) if e.is("hp:linesegarray")));
}

#[cfg(test)]
mod tests {
    use super::*;
    use hanji_package::xml::fragment;

    fn rect(inner: &str) -> String {
        format!(
            r#"<hp:rect id="1"><hp:drawText><hp:subList>{inner}</hp:subList></hp:drawText></hp:rect>"#
        )
    }

    fn p(text: &str) -> String {
        format!("<hp:p><hp:run><hp:t>{text}</hp:t></hp:run></hp:p>")
    }

    #[test]
    fn a_one_paragraph_draw_text_is_shown() {
        let e = fragment(&rect(&p("글상자")));
        assert_eq!(shown_draw_text(&e).as_deref(), Some("글상자"));
    }

    #[test]
    fn two_paragraphs_are_shown_joined() {
        let e = fragment(&rect(&format!("{}{}", p("a"), p("b"))));
        assert_eq!(shown_draw_text(&e).as_deref(), Some("a\nb"));
    }

    #[test]
    fn a_shown_note_in_draw_text_is_not_in_the_body() {
        let note = "<hp:ctrl><hp:footNote><hp:subList><hp:p><hp:run><hp:t> 각주</hp:t></hp:run></hp:p></hp:subList></hp:footNote></hp:ctrl>";
        let inner = format!("<hp:p><hp:run><hp:t>앞</hp:t>{note}<hp:t>뒤</hp:t></hp:run></hp:p>");
        let e = fragment(&rect(&inner));
        assert_eq!(shown_draw_text(&e).as_deref(), Some("앞뒤"));
        assert_eq!(draw_note_bodies(&e), vec![" 각주"]);
    }

    #[test]
    fn with_draw_contents_keeps_the_fragment_when_notes_match() {
        let note = "<hp:ctrl><hp:footNote><hp:subList><hp:p><hp:run><hp:t> 각주</hp:t></hp:run></hp:p></hp:subList></hp:footNote></hp:ctrl>";
        let inner = format!("<hp:p><hp:run><hp:t>앞</hp:t>{note}<hp:t>뒤</hp:t></hp:run></hp:p>");
        let xml = rect(&inner);
        assert_eq!(with_draw_contents(&xml, "앞뒤", &[" 각주"]), xml);
    }

    #[test]
    fn with_draw_contents_rewrites_outer_t_and_keeps_the_note() {
        let note = "<hp:ctrl><hp:footNote><hp:subList><hp:p><hp:run><hp:t> 각주</hp:t></hp:run></hp:p></hp:subList></hp:footNote></hp:ctrl>";
        let inner = format!("<hp:p><hp:run><hp:t>앞</hp:t>{note}<hp:t>뒤</hp:t></hp:run></hp:p>");
        let out = with_draw_contents(&rect(&inner), "고침", &[" 각주"]);
        assert_eq!(shown_draw_text(&fragment(&out)).as_deref(), Some("고침"));
        assert!(out.contains("<hp:t> 각주</hp:t>"), "{out}");
        assert!(out.contains("<hp:footNote"), "{out}");
    }

    #[test]
    fn a_nested_picture_stays_unshown() {
        let inner = "<hp:p><hp:run><hp:pic/><hp:t>alt</hp:t></hp:run></hp:p>";
        assert!(shown_draw_text(&fragment(&rect(inner))).is_none());
    }

    #[test]
    fn with_draw_text_keeps_the_fragment_when_unchanged() {
        let xml = rect(&p("글상자"));
        assert_eq!(with_draw_text(&xml, "글상자"), xml);
    }

    #[test]
    fn with_draw_text_rewrites_t() {
        let xml = rect(&p("글상자"));
        let out = with_draw_text(&xml, "고침");
        assert_eq!(shown_draw_text(&fragment(&out)).as_deref(), Some("고침"));
        assert!(!out.contains("글상자"), "{out}");
    }

    #[test]
    fn with_draw_text_rewrites_each_paragraph() {
        let xml = rect(&format!("{}{}", p("a"), p("b")));
        assert_eq!(with_draw_text(&xml, "a\nb"), xml);
        let out = with_draw_text(&xml, "x\ny");
        assert_eq!(shown_draw_text(&fragment(&out)).as_deref(), Some("x\ny"));
        assert!(!out.contains(">a<"), "{out}");
    }

    fn auto_num_xml(num: &str, prefix: &str, suffix: &str, ty: &str) -> String {
        format!(
            r#"<hp:ctrl><hp:autoNum num="{num}" numType="ENDNOTE"><hp:autoNumFormat type="{ty}" userChar="" prefixChar="{prefix}" suffixChar="{suffix}" supscript="0"/></hp:autoNum></hp:ctrl>"#
        )
    }

    #[test]
    fn a_digit_auto_num_is_shown() {
        let e = fragment(&auto_num_xml("11", "문", "）", "DIGIT"));
        assert_eq!(shown_auto_num(&e).as_deref(), Some("문11）"));
    }

    #[test]
    fn a_circled_auto_num_stays_unshown() {
        let e = fragment(&auto_num_xml("1", "", "", "CIRCLED_DIGIT"));
        assert!(shown_auto_num(&e).is_none());
    }

    #[test]
    fn with_auto_num_keeps_the_fragment_when_unchanged() {
        let xml = auto_num_xml("11", "문", "）", "DIGIT");
        assert_eq!(with_auto_num(&xml, "문11）").as_deref(), Some(xml.as_str()));
    }

    #[test]
    fn with_auto_num_updates_the_number() {
        let xml = auto_num_xml("11", "문", "）", "DIGIT");
        let out = with_auto_num(&xml, "문12）").expect("parses");
        assert_eq!(shown_auto_num(&fragment(&out)).as_deref(), Some("문12）"));
        assert!(out.contains(r#"num="12""#), "{out}");
        assert!(out.contains(r#"prefixChar="문""#), "{out}");
    }

    #[test]
    fn with_auto_num_is_none_when_the_text_is_not_that_format() {
        let xml = auto_num_xml("11", "문", "）", "DIGIT");
        assert!(with_auto_num(&xml, "hello").is_none());
        assert!(with_auto_num(&xml, "11").is_none());
    }
}
