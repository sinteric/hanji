//! DOCX tracked insertions and deletions whose inner text the model can
//! edit: shown as `<ins>…</ins>` / `<del>…</del>`. Author, date, id and the
//! original run XML stay in the wrap; `w:moveFrom` / `w:moveTo`, revisions
//! that wrap paragraphs or tables, and inner content the tags cannot hold
//! stay `<keep/>`.

use hanji_package::xml::{self, Element, Node};

/// Locals a shown revision may hold besides the `w:ins` / `w:del` itself.
/// Nested revisions, drawings, notes, math and content controls stay
/// placeholders.
const SHOWN: &[&str] = &[
    "ins",
    "del",
    "r",
    "rPr",
    "t",
    "delText",
    "tab",
    "br",
    "proofErr",
    "rStyle",
    "rFonts",
    "sz",
    "szCs",
    "lang",
    "b",
    "bCs",
    "i",
    "iCs",
    "u",
    "color",
    "shd",
    "spacing",
    "ind",
    "jc",
    "kern",
    "highlight",
    "vertAlign",
    "w",
    "position",
    "em",
    "vanish",
    "noProof",
    "snapToGrid",
    "smallCaps",
    "caps",
    "strike",
    "dstrike",
    "rtl",
    "cs",
    "fldChar",
    "instrText",
    "delInstrText",
    "rPrChange",
    "hyperlink",
    "bookmarkStart",
    "bookmarkEnd",
    "lastRenderedPageBreak",
];

/// `(is_insert, body)` when `el` is an inline `w:ins` / `w:del` the text
/// can show. Property marks (empty), moves, and anything else stay `None`.
pub fn shown(el: &Element) -> Option<(bool, String)> {
    let ins = match el.local() {
        "ins" => true,
        "del" => false,
        _ => return None,
    };
    el.elements().next()?;
    let mut outer = true;
    let mut ok = true;
    el.walk(&mut |e| {
        if !outer && matches!(e.local(), "ins" | "del" | "moveFrom" | "moveTo") {
            ok = false;
        }
        outer = false;
        if !SHOWN.contains(&e.local()) {
            ok = false;
        }
    });
    if !ok {
        return None;
    }
    let body = el.text_of(&["w:t", "w:delText"]);
    if body.contains('\n') || body.contains('\r') {
        return None;
    }
    let close = if ins { "</ins>" } else { "</del>" };
    if body.contains(close) {
        return None;
    }
    Some((ins, body))
}

/// `xml` of a stored `w:ins` / `w:del`, with its `w:t` / `w:delText` set to
/// `body`. Unchanged text keeps the original fragment.
pub fn with_body(xml: &str, body: &str) -> String {
    let mut e = xml::fragment(xml);
    if e.text_of(&["w:t", "w:delText"]) == body {
        return xml.to_string();
    }
    set_body(&mut e, body);
    e.to_xml()
}

fn set_body(e: &mut Element, body: &str) {
    let mut used = false;
    set_t(e, body, &mut used);
    if used {
        return;
    }
    let tag = if e.local() == "del" {
        "w:delText"
    } else {
        "w:t"
    };
    let mut t = Element::new(tag);
    if needs_preserve(body) {
        t.set("xml:space", "preserve");
    }
    t.children.push(Node::Text(xml::escape_text(body)));
    let mut r = Element::new("w:r");
    r.children.push(Node::El(t));
    e.children.push(Node::El(r));
}

fn set_t(e: &mut Element, body: &str, used: &mut bool) {
    if e.local() == "t" || e.local() == "delText" {
        if !*used {
            e.children = vec![Node::Text(xml::escape_text(body))];
            if needs_preserve(body) && e.attr("xml:space").is_none() {
                e.set("xml:space", "preserve");
            }
            *used = true;
        } else {
            e.children.clear();
        }
        return;
    }
    for c in e.elements_mut() {
        set_t(c, body, used);
    }
}

fn needs_preserve(s: &str) -> bool {
    s.trim() != s || s.contains("  ")
}
