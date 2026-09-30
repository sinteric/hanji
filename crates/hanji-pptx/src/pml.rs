//! PresentationML names and small helpers shared by import and export.

use hanji_package::xml::Element;

pub const P_NS: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
pub const A_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
pub const R_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
pub const P14_NS: &str = "http://schemas.microsoft.com/office/powerpoint/2010/main";

pub const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
pub const REL_SLIDE: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide";
pub const REL_LAYOUT: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout";
pub const REL_NOTES: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide";
pub const REL_IMAGE: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";
pub const REL_NOTES_MASTER: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesMaster";

pub const CT_SLIDE: &str = "application/vnd.openxmlformats-officedocument.presentationml.slide+xml";
pub const CT_NOTES: &str = "application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml";
pub const CT_MAIN: &str = "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml";

/// Stand-in, in a slide's or notes page's stored skeleton, for a modelled
/// shape: `<hanji-item k="3"/>`.
pub const ITEM: &str = "hanji-item";
/// A placeholder with no text, kept in the skeleton until a slot fills it:
/// `<hanji-latent slot="body">…</hanji-latent>`.
pub const LATENT: &str = "hanji-latent";

/// A placeholder's slot name by its type (`ST_PlaceholderType`); body-like
/// ones (`body`, `obj`) are named by the layout (`body`, `left`/`right`, …).
pub fn slot_of_type(ty: &str) -> Option<&'static str> {
    Some(match ty {
        "title" | "ctrTitle" => "title",
        "subTitle" => "subtitle",
        "pic" => "picture",
        "chart" => "chart",
        "tbl" => "table",
        "dgm" => "diagram",
        "media" => "media",
        "clipArt" => "clipart",
        "dt" => "date",
        "ftr" => "footer",
        "sldNum" => "number",
        "hdr" => "header",
        "sldImg" => "image",
        "body" | "obj" => return None,
        _ => return None,
    })
}

/// A body-like placeholder type (`body`, `obj`, or none given).
pub fn is_body_type(ty: &str) -> bool {
    matches!(ty, "body" | "obj")
}

/// Which master text style a placeholder's text inherits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TextClass {
    Title,
    Body,
    Other,
    Notes,
}

pub fn class_of_type(ty: &str) -> TextClass {
    match ty {
        "title" | "ctrTitle" => TextClass::Title,
        "dt" | "ftr" | "sldNum" | "hdr" => TextClass::Other,
        _ => TextClass::Body,
    }
}

/// `p:ph` of a shape (`p:sp`, `p:pic`, `p:graphicFrame`): its type
/// (`obj` when absent) and idx (0 when absent).
pub fn placeholder(shape: &Element) -> Option<(String, u32, &Element)> {
    let nv = shape.elements().find(|e| e.name.starts_with("p:nv"))?;
    let ph = nv.child("p:nvPr")?.child("p:ph")?;
    let ty = ph.get("type").unwrap_or_else(|| "obj".into());
    let idx = ph.get("idx").and_then(|v| v.parse().ok()).unwrap_or(0);
    Some((ty, idx, ph))
}

/// `p:cNvPr` of a shape.
pub fn c_nv_pr(shape: &Element) -> Option<&Element> {
    shape.elements().find(|e| e.name.starts_with("p:nv"))?.child("p:cNvPr")
}

pub fn c_nv_pr_mut(shape: &mut Element) -> Option<&mut Element> {
    shape.elements_mut().find(|e| e.name.starts_with("p:nv"))?.child_mut("p:cNvPr")
}

/// A bullet setting of one paragraph level.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Bu {
    /// Not set here: inherited.
    #[default]
    Unset,
    /// `a:buNone`.
    None,
    /// `a:buChar`, `a:buBlip`.
    Bullet,
    /// `a:buAutoNum`.
    Number,
}

impl Bu {
    /// As read where the text has no list items: never a bullet.
    pub fn filter_lists(self, lists: bool) -> Bu {
        if lists {
            self
        } else {
            Bu::None
        }
    }
}

pub const BU: &[&str] = &["a:buNone", "a:buAutoNum", "a:buChar", "a:buBlip"];

/// The bullet a `a:pPr` or `a:lvlNpPr` sets.
pub fn bu_of(ppr: Option<&Element>) -> Bu {
    let Some(p) = ppr else { return Bu::Unset };
    for e in p.elements() {
        match e.name.as_str() {
            "a:buNone" => return Bu::None,
            "a:buAutoNum" => return Bu::Number,
            "a:buChar" | "a:buBlip" => return Bu::Bullet,
            _ => {}
        }
    }
    Bu::Unset
}

/// Per level (0–8), the bullet an `a:lstStyle` (or `p:titleStyle`, …) sets.
pub fn levels_of(lst: Option<&Element>) -> [Bu; 9] {
    let mut out = [Bu::Unset; 9];
    if let Some(l) = lst {
        for (k, o) in out.iter_mut().enumerate() {
            *o = bu_of(l.child(&format!("a:lvl{}pPr", k + 1)));
        }
    }
    out
}

/// `over` where it sets a bullet, else `under`.
pub fn over(over: [Bu; 9], under: [Bu; 9]) -> [Bu; 9] {
    let mut out = under;
    for k in 0..9 {
        if over[k] != Bu::Unset {
            out[k] = over[k];
        }
    }
    out
}

/// Order of `a:pPr` children (CT_TextParagraphProperties), for inserting a bullet.
pub const PPR_ORDER: &[&str] = &[
    "lnSpc",
    "spcBef",
    "spcAft",
    "buClrTx",
    "buClr",
    "buSzTx",
    "buSzPct",
    "buSzPts",
    "buFontTx",
    "buFont",
    "buNone",
    "buAutoNum",
    "buChar",
    "buBlip",
    "tabLst",
    "defRPr",
    "extLst",
];

/// A run's bold, italic, underline and strike from its `a:rPr` attributes.
pub fn marks_of(rpr: Option<&Element>) -> hanji_format::Marks {
    use hanji_format::Marks;
    let get = |k: &str| rpr.and_then(|r| r.get(k));
    let on = |k: &str| get(k).is_some_and(|v| v == "1" || v == "true");
    Marks::NONE
        .with(Marks::BOLD, on("b"))
        .with(Marks::ITALIC, on("i"))
        .with(Marks::UNDERLINE, get("u").is_some_and(|v| v != "none"))
        .with(Marks::STRIKE, get("strike").is_some_and(|v| v != "noStrike"))
}

/// The `a:rPr` attributes the marks are written in.
pub const MARK_ATTRS: &[&str] = &["b", "i", "u", "strike"];

/// Every relationship id an attribute of `root` names (`r:id`, `r:embed`, …).
pub fn named_rel_ids(root: &Element) -> std::collections::BTreeSet<String> {
    let mut ids = std::collections::BTreeSet::new();
    root.walk(&mut |e| ids.extend(e.attrs.iter().filter(|a| a.0.starts_with("r:")).map(|a| a.1.clone())));
    ids
}
