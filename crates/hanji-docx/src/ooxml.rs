//! WordprocessingML names and small helpers shared by import and export.

use crate::xml::Element;
pub use crate::xml::{fp, insert_ordered, remove_child};

pub const W_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
pub const W_NS_STRICT: &str = "http://purl.oclc.org/ooxml/wordprocessingml/main";
pub const R_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
pub const DOC_PART: &str = "word/document.xml";

/// Inline wrappers whose text the model edits.
pub const WRAPPERS: &[&str] = &["w:hyperlink", "w:smartTag", "w:customXml", "w:dir", "w:bdo"];

/// Zero-width inline elements.
pub const MARKERS: &[&str] = &[
    "w:bookmarkStart",
    "w:bookmarkEnd",
    "w:commentRangeStart",
    "w:commentRangeEnd",
    "w:proofErr",
    "w:permStart",
    "w:permEnd",
    "w:moveFromRangeStart",
    "w:moveFromRangeEnd",
    "w:moveToRangeStart",
    "w:moveToRangeEnd",
    "w:customXmlInsRangeStart",
    "w:customXmlInsRangeEnd",
    "w:customXmlDelRangeStart",
    "w:customXmlDelRangeEnd",
    "w:customXmlMoveFromRangeStart",
    "w:customXmlMoveFromRangeEnd",
    "w:customXmlMoveToRangeStart",
    "w:customXmlMoveToRangeEnd",
];

/// Markers that mean something to a person: never dropped with nearby text.
pub const DURABLE_MARKERS: &[&str] = &[
    "w:bookmarkStart",
    "w:bookmarkEnd",
    "w:commentRangeStart",
    "w:commentRangeEnd",
    "w:permStart",
    "w:permEnd",
    "w:moveFromRangeStart",
    "w:moveFromRangeEnd",
    "w:moveToRangeStart",
    "w:moveToRangeEnd",
];

pub const RUN_MARKERS: &[&str] = &["w:lastRenderedPageBreak"];

/// Placeholder kind by element.
pub fn keep_kind(name: &str) -> String {
    let local = name.rsplit(':').next().unwrap_or(name);
    match local {
        "drawing" | "pict" | "AlternateContent" => "drawing",
        "object" => "object",
        "footnoteReference" => "footnote",
        "endnoteReference" => "endnote",
        "commentReference" => "comment",
        "fldSimple" => "field",
        "fldChar" | "instrText" => "field-part",
        "ins" => "tracked-insert",
        "del" => "tracked-delete",
        "moveFrom" | "moveTo" => "tracked-move",
        "sdt" => "content-control",
        "oMath" | "oMathPara" => "math",
        "sym" => "symbol",
        "ptab" => "tab",
        "noBreakHyphen" | "softHyphen" => "hyphen",
        "cr" | "br" => "break",
        "t" => "text",
        other => other,
    }
    .to_string()
}

/// Order of `rPr` children (ECMA-376 §17.3.2), for inserting a flag.
pub const RPR_ORDER: &[&str] = &[
    "rStyle",
    "rFonts",
    "b",
    "bCs",
    "i",
    "iCs",
    "caps",
    "smallCaps",
    "strike",
    "dstrike",
    "outline",
    "shadow",
    "emboss",
    "imprint",
    "noProof",
    "snapToGrid",
    "vanish",
    "webHidden",
    "color",
    "spacing",
    "w",
    "kern",
    "position",
    "sz",
    "szCs",
    "highlight",
    "u",
    "effect",
    "bdr",
    "shd",
    "fitText",
    "vertAlign",
    "rtl",
    "cs",
    "em",
    "lang",
    "eastAsianLayout",
    "specVanish",
    "oMath",
];

/// Order of `pPr` children (ECMA-376 §17.3.1.26), for inserting `numPr`.
pub const PPR_ORDER: &[&str] = &[
    "pStyle",
    "keepNext",
    "keepLines",
    "pageBreakBefore",
    "framePr",
    "widowControl",
    "numPr",
    "suppressLineNumbers",
    "pBdr",
    "shd",
    "tabs",
    "suppressAutoHyphens",
    "kinsoku",
    "wordWrap",
    "overflowPunct",
    "topLinePunct",
    "autoSpaceDE",
    "autoSpaceDN",
    "bidi",
    "adjustRightInd",
    "snapToGrid",
    "spacing",
    "ind",
    "contextualSpacing",
    "mirrorIndents",
    "suppressOverlap",
    "jc",
    "textDirection",
    "textAlignment",
    "textboxTightWrap",
    "outlineLvl",
    "divId",
    "cnfStyle",
    "rPr",
    "sectPr",
    "pPrChange",
];

pub const TCPR_ORDER: &[&str] = &[
    "cnfStyle",
    "tcW",
    "gridSpan",
    "hMerge",
    "vMerge",
    "tcBorders",
    "shd",
    "noWrap",
    "tcMar",
    "textDirection",
    "tcFitText",
    "vAlign",
    "hideMark",
];

/// `w:gridSpan` of a cell (1 when absent).
pub fn grid_span(tcpr: Option<&Element>) -> usize {
    tcpr.and_then(|t| t.child("w:gridSpan")).and_then(|g| g.get("w:val")).and_then(|v| v.parse().ok()).unwrap_or(1)
}

/// A `w:vMerge` cell other than `restart`: covered by the cell above.
pub fn v_merged(tcpr: Option<&Element>) -> bool {
    tcpr.and_then(|t| t.child("w:vMerge")).is_some_and(|v| v.get("w:val").as_deref() != Some("restart"))
}

/// `w:b`-style toggle: present and not `0`/`false`/`off`.
pub fn on(e: Option<&Element>) -> bool {
    e.is_some_and(|e| !matches!(e.get("w:val").as_deref(), Some("0") | Some("false") | Some("off")))
}

/// Underline is on unless absent or `none`.
pub fn underline_on(e: Option<&Element>) -> bool {
    e.is_some_and(|e| e.get("w:val").as_deref() != Some("none"))
}
