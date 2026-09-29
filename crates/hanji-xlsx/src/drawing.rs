//! What a sheet shows that the text does not model (§2 rule 8): the objects
//! of its drawing (charts, pictures, shapes), its notes, its pivot tables and
//! its 2010 extensions, summarised for their placeholders.

use hanji_package::opc;
use hanji_package::xml::{self, Element};
use hanji_package::{clip, package};

use crate::book::Book;

/// An object shown as a placeholder.
#[derive(Clone, Debug)]
pub struct Shown {
    pub kind: String,
    pub summary: String,
}

fn text_of(e: &Element) -> String {
    let mut s = String::new();
    fn walk(e: &Element, s: &mut String) {
        for c in e.elements() {
            if c.local() == "t" {
                for n in &c.children {
                    if let xml::Node::Text(t) = n {
                        s.push_str(&xml::unescape(t));
                    }
                }
            } else {
                walk(c, s);
                if c.local() == "p" {
                    s.push(' ');
                }
            }
        }
    }
    walk(e, &mut s);
    hanji_package::squash(&s)
}

/// `xdr:from` of an anchor as a cell (`B2`).
fn anchor_cell(a: &Element) -> Option<String> {
    let from = a.elements().find(|e| e.local() == "from")?;
    let num = |n: &str| {
        from.elements()
            .find(|e| e.local() == n)
            .map(|e| e.text_of(&[e.name.as_str()]))
            .and_then(|t| t.trim().parse::<u32>().ok())
    };
    Some(format!("{}{}", hanji_core::cells::col_letters(num("col")?), num("row")? + 1))
}

/// A chart part summarised: `bar chart "월별 이익": Sheet1!$A$2:$A$13, …`.
pub fn chart_summary(parts: &[hanji_core::Part], part: &str) -> String {
    let Some(d) = package::get(parts, part).and_then(|d| xml::parse(d).ok()) else { return "chart".into() };
    let mut kinds = vec![];
    let mut refs = vec![];
    let mut title = None;
    d.root.walk(&mut |e| {
        let l = e.local();
        if l.ends_with("Chart") && l != "chart" && !kinds.contains(&l.to_string()) {
            let dir = e.elements().find(|x| x.local() == "barDir").and_then(|x| x.get("val"));
            kinds.push(match (l, dir.as_deref()) {
                ("barChart" | "bar3DChart", Some("col")) => "column".to_string(),
                _ => l.trim_end_matches("Chart").trim_end_matches("3D").to_string(),
            });
        }
        if l == "f" && refs.len() < 6 {
            refs.push(e.text_of(&[e.name.as_str()]));
        }
        if l == "title" && title.is_none() {
            let t = text_of(e);
            if !t.is_empty() {
                title = Some(t);
            }
        }
    });
    let mut s = format!("{} chart", if kinds.is_empty() { "a".to_string() } else { kinds.join("+") });
    if let Some(t) = title {
        s.push_str(&format!(" “{}”", clip(&t, 40)));
    }
    if !refs.is_empty() {
        s.push_str(": ");
        s.push_str(&refs.join(", "));
    }
    clip(&s, 160)
}

/// The objects of drawing part `part` (a worksheet's or a chart sheet's).
pub fn drawing_objects(book: &Book, part: &str) -> Vec<Shown> {
    let Some(d) = package::get(&book.parts, part).and_then(|d| xml::parse(d).ok()) else { return vec![] };
    let rels = book.rels_of(part);
    let mut out = vec![];
    for a in d.root.elements() {
        let at = anchor_cell(a).map(|c| format!(" at {c}")).unwrap_or_default();
        let Some(obj) = a.elements().find(|e| !matches!(e.local(), "from" | "to" | "pos" | "ext" | "clientData"))
        else {
            continue;
        };
        let name = obj
            .elements()
            .find(|e| e.local().starts_with("nv"))
            .and_then(|nv| nv.elements().find(|e| e.local() == "cNvPr"))
            .map(|c| c.get("descr").filter(|d| !d.is_empty()).or_else(|| c.get("name")).unwrap_or_default())
            .unwrap_or_default();
        let (kind, summary) = match obj.local() {
            "graphicFrame" => {
                let mut chart = None;
                let mut other = None;
                obj.walk(&mut |e| {
                    if e.local() == "chart" {
                        chart = e.attrs.iter().find(|x| x.0.ends_with(":id")).map(|x| x.1.clone());
                    }
                    if e.local() == "graphicData" {
                        other = e.get("uri");
                    }
                });
                match chart
                    .and_then(|id| rels.iter().find(|r| r.id == id).map(|r| opc::resolve_target(part, &r.target)))
                {
                    Some(cp) => ("chart", format!("{}{at}", chart_summary(&book.parts, &cp))),
                    None if other.as_deref().is_some_and(|u| u.contains("diagram")) => {
                        ("smartart", format!("SmartArt {name}{at}"))
                    }
                    None => ("object", format!("{name}{at}")),
                }
            }
            "pic" => ("picture", format!("{name}{at}")),
            "sp" => {
                let t = text_of(obj);
                ("shape", if t.is_empty() { format!("{name}{at}") } else { format!("{name}{at}: {t}") })
            }
            "grpSp" => ("group", format!("{name}{at}: {}", text_of(obj))),
            "cxnSp" => ("connector", format!("{name}{at}")),
            "AlternateContent" => {
                // A slicer, a timeline or a 2010 shape with its fallback.
                let t = text_of(obj);
                ("object", format!("{name}{at} {t}"))
            }
            other => ("object", format!("{other} {name}{at}")),
        };
        out.push(Shown { kind: kind.into(), summary: clip(summary.trim(), 160) });
    }
    out
}

/// Everything sheet `i` shows as placeholders besides its range entries.
pub fn sheet_objects(book: &Book, i: usize) -> Vec<Shown> {
    let part = &book.sheets[i].part;
    let rels = book.rels_of(part);
    let mut out = vec![];
    for r in rels.iter().filter(|r| !r.external) {
        let target = opc::resolve_target(part, &r.target);
        match r.short_type() {
            "drawing" => out.extend(drawing_objects(book, &target)),
            "comments" => {
                let Some(d) = package::get(&book.parts, &target).and_then(|d| xml::parse(d).ok()) else { continue };
                let refs: Vec<String> =
                    d.root.descendants_local("comment").iter().filter_map(|c| c.get("ref")).collect();
                let first = d.root.descendants_local("comment").first().map(|c| text_of(c)).unwrap_or_default();
                out.push(Shown {
                    kind: "notes".into(),
                    summary: clip(
                        &format!(
                            "{} note{} at {}: {first}",
                            refs.len(),
                            if refs.len() == 1 { "" } else { "s" },
                            refs.join(", ")
                        ),
                        160,
                    ),
                });
            }
            "threadedComment" => {
                let Some(d) = package::get(&book.parts, &target).and_then(|d| xml::parse(d).ok()) else { continue };
                let refs: Vec<String> = d
                    .root
                    .descendants_local("threadedComment")
                    .iter()
                    .filter(|c| c.get("parentId").is_none())
                    .filter_map(|c| c.get("ref"))
                    .collect();
                out.push(Shown {
                    kind: "comments".into(),
                    summary: clip(&format!("{} comment thread(s) at {}", refs.len(), refs.join(", ")), 160),
                });
            }
            "pivotTable" => {
                let Some(d) = package::get(&book.parts, &target).and_then(|d| xml::parse(d).ok()) else { continue };
                let loc =
                    d.root.elements().find(|e| e.local() == "location").and_then(|l| l.get("ref")).unwrap_or_default();
                out.push(Shown {
                    kind: "pivot-table".into(),
                    summary: format!("{} at {loc}", d.root.get("name").unwrap_or_default()),
                });
            }
            _ => {}
        }
    }
    out
}

/// A worksheet's 2010+ extensions whose ranges the engine does not model: their names.
pub fn extensions(skel: &Element) -> Vec<String> {
    let mut out = vec![];
    if let Some(ext) = skel.elements().find(|e| e.local() == "extLst") {
        for e in ext.elements() {
            for c in e.elements() {
                let n = match c.local() {
                    "sparklineGroups" => "sparklines",
                    "slicerList" => "slicers",
                    "conditionalFormattings" => "conditional formats (2010)",
                    "dataValidations" => "data validations (2010)",
                    "timelineRefs" => "timelines",
                    "protectedRanges" => "protected ranges (2010)",
                    "ignoredErrors" => "ignored errors (2010)",
                    "webExtensions" => "web extensions",
                    _ => continue,
                };
                if !out.contains(&n.to_string()) {
                    out.push(n.to_string());
                }
            }
        }
    }
    out
}

trait Descend {
    fn descendants_local(&self, l: &str) -> Vec<&Element>;
}

impl Descend for Element {
    fn descendants_local(&self, l: &str) -> Vec<&Element> {
        let mut out = vec![];
        self.walk(&mut |e| {
            if e.local() == l {
                out.push(e)
            }
        });
        out
    }
}
