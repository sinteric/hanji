//! Open Packaging Conventions: relationship parts, targets and content
//! types, as the OOXML engines read and change them.

use std::collections::BTreeSet;

use hanji_core::Part;

use crate::package;
use crate::xml::{self, Element, Node};

pub const RELS_NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
pub const CT_PART: &str = "[Content_Types].xml";

/// One relationship of a part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rel {
    pub id: String,
    /// Full type URI.
    pub ty: String,
    /// As written (relative to the source part, or absolute, or a URL).
    pub target: String,
    pub external: bool,
}

impl Rel {
    /// The last segment of the type URI (`slideLayout`).
    pub fn short_type(&self) -> &str {
        self.ty.rsplit('/').next().unwrap_or("")
    }
}

/// `ppt/slides/slide1.xml` → `ppt/slides/_rels/slide1.xml.rels`.
pub fn rels_part(part: &str) -> String {
    match part.rsplit_once('/') {
        Some((d, f)) => format!("{d}/_rels/{f}.rels"),
        None => format!("_rels/{part}.rels"),
    }
}

/// `word/_rels/document.xml.rels` → `word/document.xml` (`_rels/.rels` → the package root, ``).
pub fn source_of_rels(rels: &str) -> Option<String> {
    let (dir, file) = rels.rsplit_once("_rels/")?;
    Some(format!("{dir}{}", file.strip_suffix(".rels")?))
}

/// A relationship target resolved against its source part.
pub fn resolve_target(source: &str, target: &str) -> String {
    if let Some(abs) = target.strip_prefix('/') {
        return abs.to_string();
    }
    let mut segs: Vec<&str> = source.split('/').collect();
    segs.pop();
    for t in target.split('/') {
        match t {
            ".." => {
                segs.pop();
            }
            "." | "" => {}
            t => segs.push(t),
        }
    }
    segs.join("/")
}

/// The target a relationship from `source` to `part` writes (`../slideLayouts/slideLayout2.xml`).
pub fn relative_target(source: &str, part: &str) -> String {
    let from: Vec<&str> = source.split('/').collect();
    let to: Vec<&str> = part.split('/').collect();
    let dir = &from[..from.len() - 1];
    let common = dir.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut segs: Vec<&str> = vec![".."; dir.len() - common];
    segs.extend(&to[common..]);
    segs.join("/")
}

/// The relationships of `rels` data (a `.rels` part).
pub fn parse_rels(data: &[u8]) -> Vec<Rel> {
    let Ok(d) = xml::parse(data) else { return vec![] };
    d.root
        .elements()
        .filter(|e| e.local() == "Relationship")
        .map(|e| Rel {
            id: e.get("Id").unwrap_or_default(),
            ty: e.get("Type").unwrap_or_default(),
            target: e.get("Target").unwrap_or_default(),
            external: e.get("TargetMode").as_deref() == Some("External"),
        })
        .collect()
}

/// The relationships of `part` in `parts`.
pub fn rels_of(parts: &[Part], part: &str) -> Vec<Rel> {
    package::get(parts, &rels_part(part)).map(parse_rels).unwrap_or_default()
}

/// The internal part `part`'s relationship `id` points at.
pub fn target_of(parts: &[Part], part: &str, id: &str) -> Option<String> {
    rels_of(parts, part).into_iter().find(|r| r.id == id && !r.external).map(|r| resolve_target(part, &r.target))
}

/// A `.rels` part holding `rels`.
pub fn write_rels(rels: &[Rel]) -> Vec<u8> {
    let mut root = Element::new("Relationships").with_attr("xmlns", RELS_NS);
    for r in rels {
        let mut e =
            Element::new("Relationship").with_attr("Id", &r.id).with_attr("Type", &r.ty).with_attr("Target", &r.target);
        if r.external {
            e = e.with_attr("TargetMode", "External");
        }
        root.children.push(Node::El(e));
    }
    let d = xml::Doc {
        prolog: "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n".into(),
        root,
        epilog: String::new(),
    };
    xml::write_doc(&d)
}

/// A relationship id not in `rels` (`rId1`, `rId2`, …).
pub fn free_rel_id(rels: &[Rel]) -> String {
    (1..).map(|k| format!("rId{k}")).find(|id| !rels.iter().any(|r| &r.id == id)).unwrap()
}

/// Every internal part reachable from the package root through relationships.
pub fn reachable(parts: &[Part]) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut todo = vec![String::new()];
    while let Some(p) = todo.pop() {
        let rels = if p.is_empty() {
            package::get(parts, "_rels/.rels").map(parse_rels).unwrap_or_default()
        } else {
            rels_of(parts, &p)
        };
        for r in rels.iter().filter(|r| !r.external) {
            let t = resolve_target(&p, &r.target);
            if package::get(parts, &t).is_some() && seen.insert(t.clone()) {
                todo.push(t);
            }
        }
    }
    seen
}

/// `[Content_Types].xml` with the overrides of `gone` removed and an
/// override `(part, content type)` added for each of `added`; `None` when
/// nothing changes.
pub fn content_types(parts: &[Part], gone: &BTreeSet<String>, added: &[(String, String)]) -> Option<Vec<u8>> {
    let data = package::get(parts, CT_PART)?;
    let mut d = xml::parse(data).ok()?;
    let n = d.root.children.len();
    d.root.children.retain(|c| {
        !matches!(c, Node::El(e) if e.local() == "Override" && e.get("PartName").is_some_and(|p| gone.contains(p.trim_start_matches('/'))))
    });
    let mut changed = d.root.children.len() != n;
    for (part, ct) in added {
        let name = format!("/{part}");
        if d.root.elements().any(|e| e.local() == "Override" && e.get("PartName").as_deref() == Some(name.as_str())) {
            continue;
        }
        d.root
            .children
            .push(Node::El(Element::new("Override").with_attr("PartName", &name).with_attr("ContentType", ct)));
        changed = true;
    }
    changed.then(|| xml::write_doc(&d))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_resolve_and_relativise() {
        assert_eq!(
            resolve_target("ppt/slides/slide1.xml", "../slideLayouts/slideLayout2.xml"),
            "ppt/slideLayouts/slideLayout2.xml"
        );
        assert_eq!(resolve_target("ppt/presentation.xml", "slides/slide1.xml"), "ppt/slides/slide1.xml");
        assert_eq!(resolve_target("", "ppt/presentation.xml"), "ppt/presentation.xml");
        assert_eq!(resolve_target("ppt/slides/slide1.xml", "/ppt/media/a.png"), "ppt/media/a.png");
        assert_eq!(
            relative_target("ppt/slides/slide1.xml", "ppt/slideLayouts/slideLayout2.xml"),
            "../slideLayouts/slideLayout2.xml"
        );
        assert_eq!(relative_target("ppt/presentation.xml", "ppt/slides/slide9.xml"), "slides/slide9.xml");
        assert_eq!(rels_part("ppt/slides/slide1.xml"), "ppt/slides/_rels/slide1.xml.rels");
        assert_eq!(source_of_rels("ppt/slides/_rels/slide1.xml.rels").as_deref(), Some("ppt/slides/slide1.xml"));
        assert_eq!(source_of_rels("_rels/.rels").as_deref(), Some(""));
    }

    #[test]
    fn rels_round_trip() {
        let rels = vec![
            Rel { id: "rId1".into(), ty: "http://x/slideLayout".into(), target: "../a.xml".into(), external: false },
            Rel {
                id: "rId3".into(),
                ty: "http://x/hyperlink".into(),
                target: "https://e.com/?a=1&b=2".into(),
                external: true,
            },
        ];
        assert_eq!(parse_rels(&write_rels(&rels)), rels);
        assert_eq!(free_rel_id(&rels), "rId2");
        assert_eq!(rels[0].short_type(), "slideLayout");
    }
}
