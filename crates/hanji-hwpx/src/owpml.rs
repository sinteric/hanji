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

/// Whether an element holds a tracked-change mark, or its run or paragraph a tracked shape change.
pub fn has_tracked_change(e: &Element) -> bool {
    let mut found = false;
    e.walk(&mut |x| {
        found |= TRACK_MARKS.contains(&x.name.as_str())
            || (x.is("hp:run") && x.attr("charTcId").is_some())
            || (x.is("hp:p") && x.attr("paraTcId").is_some())
    });
    found
}
