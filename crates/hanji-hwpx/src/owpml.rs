//! OWPML (KS X 6101) names and small helpers shared by import and export.

use hanji_package::xml::{Element, Node};

pub const HEADER_PART: &str = "Contents/header.xml";
pub const CONTENT_PART: &str = "Contents/content.hpf";

/// The section number of `Contents/section{N}.xml`.
pub fn section_number(part: &str) -> Option<u32> {
    part.strip_prefix("Contents/section")?.strip_suffix(".xml")?.parse().ok()
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
pub const TRACK_MARKS: &[&str] = &["hp:insertBegin", "hp:insertEnd", "hp:deleteBegin", "hp:deleteEnd"];

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
        "pic" | "rect" | "ellipse" | "arc" | "polygon" | "curve" | "connectLine" | "line" | "container" | "textart"
        | "chart" => "drawing",
        "equation" => "math",
        "ole" => "object",
        "btn" | "checkBtn" | "radioBtn" | "comboBox" | "listBox" | "edit" | "scrollBar" => "form-control",
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
    e.walk(&mut |x| found |= TRACK_MARKS.contains(&x.name.as_str()) || TC_ATTRS.iter().any(|a| x.attr(a).is_some()));
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
                        Node::El(e) if e.is("hp:lineBreak") || t_char(&e.name).is_some_and(|c| c != '\t') => 1,
                        Node::El(e)
                            if T_MARKERS.contains(&e.name.as_str()) || TRACK_MARKS.contains(&e.name.as_str()) =>
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
    let past = lines.elements().filter_map(|l| l.get("textpos")?.parse::<usize>().ok()).find(|&t| t > len + 1)?;
    Some(format!("a line starts at {past}, past the paragraph's end ({len} characters and its end mark)"))
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
    p.children.retain(|n| !matches!(n, Node::El(e) if e.is("hp:linesegarray")));
}
