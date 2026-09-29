//! §8 on import, as for docx: active and remote content is neutralised
//! (removed and reported, never preserved) — macros (pptm, `vbaProject`),
//! OLE objects and ActiveX controls, click actions that run a program or a
//! macro, and external links other than hyperlinks (linked pictures, video,
//! sound, OLE and chart data). Hidden slides and shapes, comments and author
//! metadata are listed to surface before export.

use std::collections::{BTreeMap, BTreeSet};

use hanji_core::{notice, ImportReport, Notice, Part};
use hanji_package::opc::{self, rels_part};
use hanji_package::xml::{self, Element, Node};
use hanji_package::{clip, package};

use crate::pml::CT_MAIN;

/// Extensions of macro-enabled Office packages.
const MACRO_PACKAGES: &[&str] =
    &[".docm", ".dotm", ".xlsm", ".xltm", ".xlam", ".xlsb", ".pptm", ".potm", ".ppsm", ".ppam", ".sldm"];

/// Macro-enabled main parts and what they become.
const MACRO_MAINS: &[(&str, &str)] = &[
    ("application/vnd.ms-powerpoint.presentation.macroEnabled.main+xml", CT_MAIN),
    (
        "application/vnd.ms-powerpoint.slideshow.macroEnabled.main+xml",
        "application/vnd.openxmlformats-officedocument.presentationml.slideshow.main+xml",
    ),
    (
        "application/vnd.ms-powerpoint.template.macroEnabled.main+xml",
        "application/vnd.openxmlformats-officedocument.presentationml.template.main+xml",
    ),
];

/// Elements that exist only to reach what a removed relationship pointed at.
const REACHES: &[&str] = &["p:control", "a:videoFile", "a:audioFile", "a:quickTimeFile", "p14:media", "c:externalData"];

/// An embedded package that can carry macros: a macro-enabled extension, or a zip holding a VBA project.
fn is_macro_package(p: &Part) -> bool {
    let lower = p.name.to_ascii_lowercase();
    MACRO_PACKAGES.iter().any(|x| lower.ends_with(x))
        || (p.data.starts_with(b"PK")
            && package::read(&p.data)
                .is_ok_and(|inner| inner.iter().any(|q| q.name.to_ascii_lowercase().ends_with("vbaproject.bin"))))
}

struct Removed {
    source: String,
    id: String,
}

/// Neutralise `parts`; `split` holds the parsed roots of the parts the
/// engine splits (slides, notes pages, `presentation.xml`), changed in place.
pub fn neutralise(
    parts: &mut Vec<Part>,
    split: &mut BTreeMap<String, Element>,
    report: &mut ImportReport,
) -> Result<(), String> {
    let out = &mut report.neutralised;
    let mut removed: Vec<Removed> = vec![];
    let mut drop_parts: BTreeSet<String> = BTreeSet::new();
    let macro_parts: BTreeSet<String> = parts.iter().filter(|p| is_macro_package(p)).map(|p| p.name.clone()).collect();
    for part in parts.iter_mut().filter(|p| p.name.ends_with(".rels")) {
        let Some(source) = opc::source_of_rels(&part.name) else { continue };
        let Ok(mut d) = xml::parse(&part.data) else { continue };
        let before = d.root.children.len();
        let from_slide = source.starts_with("ppt/slides/")
            || source.starts_with("ppt/slideLayouts/")
            || source.starts_with("ppt/slideMasters/")
            || source.starts_with("ppt/notesSlides/");
        d.root.children.retain(|n| {
            let Node::El(r) = n else { return true };
            let ty = r.get("Type").unwrap_or_default();
            let short = ty.rsplit('/').next().unwrap_or("");
            let external = r.get("TargetMode").as_deref() == Some("External");
            let target = r.get("Target").unwrap_or_default();
            let resolved = opc::resolve_target(&source, &target);
            let kind: Option<&'static str> = match short {
                "hyperlink" | "slide" => None,
                _ if !external && macro_parts.contains(&resolved) => Some("macro-package"),
                "vbaProject" => Some("macros"),
                "control" | "activeXControl" | "activeXControlBinary" => Some("activex-control"),
                "oleObject" if external => Some("linked-object"),
                "oleObject" => Some("ole-object"),
                "package" if from_slide => Some("ole-object"),
                "image" if external => Some("linked-image"),
                "video" | "audio" | "media" if external => Some("linked-media"),
                _ if external => Some("external-relationship"),
                _ => None,
            };
            let Some(kind) = kind else { return true };
            notice(out, kind, format!("{} {}", part.name, r.get("Id").unwrap_or_default()), target.clone());
            removed.push(Removed { source: source.clone(), id: r.get("Id").unwrap_or_default() });
            if !external {
                drop_parts.insert(resolved);
            }
            false
        });
        if d.root.children.len() != before {
            part.data = xml::write_doc(&d);
        }
    }
    // The XML that pointed at them, and OLE objects, ActiveX controls and
    // click actions wherever they are.
    let sources: BTreeSet<String> = removed.iter().map(|r| r.source.clone()).chain(split.keys().cloned()).collect();
    for src in &sources {
        let ids: BTreeSet<&str> = removed.iter().filter(|r| &r.source == src).map(|r| r.id.as_str()).collect();
        match split.get_mut(src) {
            Some(root) => clean(root, &ids, out, src),
            None => {
                let Some(p) = parts.iter_mut().find(|p| &p.name == src) else { continue };
                let Ok(mut d) = xml::parse(&p.data) else { continue };
                let before = d.root.clone();
                clean(&mut d.root, &ids, out, src);
                if d.root != before {
                    p.data = xml::write_doc(&d);
                }
            }
        }
    }
    for name in &macro_parts {
        if !drop_parts.contains(name) {
            notice(out, "macro-package", name.clone(), "embedded package that can carry macros");
            drop_parts.insert(name.clone());
        }
    }
    // Parts that only served removed content, and their content types; a
    // macro-enabled main part becomes a plain one.
    let mut gone = BTreeSet::new();
    for name in drop_parts {
        gone.insert(rels_part(&name));
        gone.insert(name);
    }
    parts.retain(|p| !gone.contains(&p.name));
    if let Some(ct) = parts.iter_mut().find(|p| p.name == opc::CT_PART) {
        if let Ok(mut d) = xml::parse(&ct.data) {
            let n = d.root.children.len();
            d.root.children.retain(|c| !matches!(c, Node::El(e) if e.local() == "Override" && e.get("PartName").is_some_and(|p| gone.contains(p.trim_start_matches('/')))));
            let mut changed = d.root.children.len() != n;
            if gone.iter().any(|g| g.to_ascii_lowercase().ends_with("vbaproject.bin")) {
                for e in d.root.elements_mut() {
                    let ct = e.get("ContentType").unwrap_or_default();
                    if let Some((_, plain)) = MACRO_MAINS.iter().find(|m| m.0 == ct) {
                        e.set("ContentType", plain);
                        changed = true;
                    }
                }
            }
            if changed {
                ct.data = xml::write_doc(&d);
            }
        }
    }
    Ok(())
}

/// Remove from `root` the references to relationships `ids`, embedded
/// objects (their preview picture stays), ActiveX controls and click actions
/// that run something.
fn clean(root: &mut Element, ids: &BTreeSet<&str>, out: &mut Vec<Notice>, part: &str) {
    fn refers(e: &Element, ids: &BTreeSet<&str>) -> bool {
        e.attrs.iter().any(|a| a.0.starts_with("r:") && ids.contains(xml::unescape(&a.1).as_str()))
    }
    fn rec(e: &mut Element, ids: &BTreeSet<&str>, out: &mut Vec<Notice>, part: &str) {
        let mut k = 0;
        while k < e.children.len() {
            let Node::El(c) = &e.children[k] else {
                k += 1;
                continue;
            };
            // An embedded object: its preview picture takes its place.
            if let Some(pic) = ole_preview(c) {
                match pic {
                    Some(p) => {
                        notice(
                            out,
                            "ole-object",
                            format!("{part} <{}>", c.name),
                            "embedded object removed; its preview picture stays",
                        );
                        e.children[k] = Node::El(p);
                    }
                    None => {
                        notice(
                            out,
                            "ole-object",
                            format!("{part} <{}>", c.name),
                            "embedded object removed, with no preview picture",
                        );
                        e.children.remove(k);
                        continue;
                    }
                }
                k += 1;
                continue;
            }
            let drop = (REACHES.contains(&c.name.as_str()) && refers(c, ids))
                || c.is("p:controls")
                || (matches!(c.name.as_str(), "a:hlinkClick" | "a:hlinkHover" | "a:hlinkMouseOver")
                    && runs_something(c))
                || (c.is("mc:AlternateContent")
                    && c.descendants("p:controls").len() + c.descendants("p:control").len() > 0);
            if drop {
                let kind = if c.name.starts_with("a:hlink") {
                    "click-action"
                } else if c.name.contains("control") || c.is("mc:AlternateContent") {
                    "activex-control"
                } else {
                    "reference-removed"
                };
                let detail = c.get("action").map_or_else(|| format!("<{}> removed", c.name), |a| clip(&a, 60));
                notice(out, kind, format!("{part} <{}>", c.name), detail);
                e.children.remove(k);
                continue;
            }
            k += 1;
        }
        for c in e.elements_mut() {
            let before = c.attrs.len();
            c.attrs.retain(|a| !(a.0.starts_with("r:") && ids.contains(xml::unescape(&a.1).as_str())));
            if c.attrs.len() != before {
                notice(out, "reference-removed", format!("{part} <{}>", c.name), "points at a removed relationship");
            }
            rec(c, ids, out, part);
        }
    }
    rec(root, ids, out, part);
}

/// A click action that runs a program, a macro or an object's verb.
fn runs_something(h: &Element) -> bool {
    h.get("action").is_some_and(|a| {
        a.starts_with("ppaction://program") || a.starts_with("ppaction://macro") || a.starts_with("ppaction://ole")
    })
}

/// An embedded or linked OLE object (a `p:graphicFrame` holding `p:oleObj`,
/// or an `mc:AlternateContent` choosing between two): `Some` with its
/// preview picture, named and placed as the frame was, when it has one.
fn ole_preview(e: &Element) -> Option<Option<Element>> {
    let frame_of = |e: &Element| -> Option<Element> {
        if !e.is("p:graphicFrame") {
            return None;
        }
        e.descendants("p:oleObj").first().map(|_| e.clone())
    };
    let frames: Vec<Element> = if e.is("mc:AlternateContent") {
        e.elements().flat_map(|c| c.elements().filter_map(frame_of).collect::<Vec<_>>()).collect()
    } else {
        frame_of(e).into_iter().collect()
    };
    let first = frames.first()?;
    let pic = frames
        .iter()
        .find_map(|f| f.descendants("p:oleObj").first().and_then(|o| o.child("p:pic")).cloned())
        .map(|mut p| {
            if let (Some(src), Some(dst)) = (
                first.child("p:nvGraphicFramePr").and_then(|n| n.child("p:cNvPr")),
                p.child_mut("p:nvPicPr").and_then(|n| n.child_mut("p:cNvPr")),
            ) {
                *dst = src.clone();
            }
            if let (Some(xfrm), Some(sp)) = (first.child("p:xfrm"), p.child_mut("p:spPr")) {
                if sp.child("a:xfrm").is_none() {
                    let mut x = xfrm.clone();
                    x.name = "a:xfrm".into();
                    sp.children.insert(0, Node::El(x));
                }
            }
            p
        });
    Some(pic)
}

/// Content to show a person before export (§8).
pub fn surface(parts: &[Part], slides: &[(String, &Element)], report: &mut ImportReport) {
    let out = &mut report.surface;
    for (k, (name, root)) in slides.iter().enumerate() {
        if root.get("show").as_deref() == Some("0") {
            notice(out, "hidden-slide", format!("slide {} ({name})", k + 1), "the slide is hidden in the slide show");
        }
        root.walk(&mut |e| {
            if e.is("p:cNvPr") && e.get("hidden").as_deref() == Some("1") {
                notice(out, "hidden-shape", format!("slide {} ({name})", k + 1), e.get("name").unwrap_or_default());
            }
        });
    }
    let authors: BTreeMap<String, String> = package::get(parts, "ppt/commentAuthors.xml")
        .and_then(|d| xml::parse(d).ok())
        .map(|d| d.root.elements().filter_map(|a| Some((a.get("id")?, a.get("name").unwrap_or_default()))).collect())
        .unwrap_or_default();
    for p in parts.iter().filter(|p| p.name.starts_with("ppt/comments/") && p.name.ends_with(".xml")) {
        let Ok(d) = xml::parse(&p.data) else { continue };
        d.root.walk(&mut |e| {
            if e.local() == "cm" {
                let who =
                    e.get("authorId").and_then(|a| authors.get(&a).cloned().or(Some(a))).unwrap_or_else(|| "?".into());
                let text = e.text_of(&["p:text", "a:t"]);
                notice(out, "comment", p.name.clone(), format!("{who}: {}", clip(&text, 60)));
            }
        });
    }
    for (part, tags) in [
        ("docProps/core.xml", &["dc:creator", "cp:lastModifiedBy"][..]),
        ("docProps/app.xml", &["Company", "Manager"][..]),
    ] {
        let Some(d) = package::get(parts, part).and_then(|d| xml::parse(d).ok()) else { continue };
        for tag in tags {
            for e in d.root.elements().filter(|e| e.name == *tag) {
                let v = e.text_of(&[tag]);
                if !v.trim().is_empty() {
                    notice(out, "metadata", format!("{part} {tag}"), v);
                }
            }
        }
    }
}
