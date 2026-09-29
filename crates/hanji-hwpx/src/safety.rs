//! §8 on import, for hwpx: scripts (HWPX macros), embedded OLE objects,
//! linked (not embedded) files and a linked source document are
//! neutralised (removed, and reported, never preserved); tracked deletions,
//! hidden comments, memos, author metadata and the stored text preview are
//! listed to surface before export. The report has hanji-docx's shape.

use std::collections::BTreeSet;

use hanji_core::{notice, ImportReport, Part};
use hanji_package::clip;
use hanji_package::xml::{self, Doc, Element, Node};

use crate::owpml::{CONTENT_PART, HEADER_PART};

/// Remove every element (at any depth) that `drop` selects; returns what went.
fn remove_where(e: &mut Element, drop: &dyn Fn(&Element) -> bool) -> Vec<Element> {
    let mut gone = vec![];
    let mut kept = vec![];
    for n in std::mem::take(&mut e.children) {
        match n {
            Node::El(c) if drop(&c) => gone.push(c),
            Node::El(mut c) => {
                gone.extend(remove_where(&mut c, drop));
                kept.push(Node::El(c));
            }
            n => kept.push(n),
        }
    }
    e.children = kept;
    gone
}

fn edit_part(parts: &mut [Part], name: &str, f: &mut dyn FnMut(&mut Element) -> bool) {
    let Some(p) = parts.iter_mut().find(|p| p.name == name) else { return };
    let Ok(mut d) = xml::parse(&p.data) else { return };
    if f(&mut d.root) {
        p.data = xml::write_doc(&d);
    }
}

pub fn neutralise(parts: &mut Vec<Part>, sections: &mut [(String, Doc)], report: &mut ImportReport) {
    let out = &mut report.neutralised;
    // Embedded OLE objects: the object and its data go.
    let mut ole_items = BTreeSet::new();
    for (name, d) in sections.iter_mut() {
        for o in remove_where(&mut d.root, &|e| e.is("hp:ole")) {
            let item = o.get("binaryItemIDRef").unwrap_or_default();
            notice(out, "ole-object", format!("{name} <hp:ole>"), format!("embedded object {item} removed"));
            ole_items.insert(item);
        }
    }
    // The package manifest: scripts, linked files and the OLE data.
    let mut gone_parts: BTreeSet<String> =
        parts.iter().filter(|p| p.name.starts_with("Scripts/")).map(|p| p.name.clone()).collect();
    let mut gone_ids = BTreeSet::new();
    edit_part(parts, CONTENT_PART, &mut |root| {
        let mut changed = false;
        for m in root.elements_mut().filter(|e| e.is("opf:manifest")) {
            let items = remove_where(m, &|e| {
                let href = e.get("href").unwrap_or_default();
                let media = e.get("media-type").unwrap_or_default();
                e.is("opf:item")
                    && (href.starts_with("Scripts/")
                        || media.contains("javascript")
                        || e.get("isEmbeded").as_deref() == Some("0")
                        || e.get("id").is_some_and(|id| ole_items.contains(&id)))
            });
            for it in items {
                changed = true;
                let (id, href) = (it.get("id").unwrap_or_default(), it.get("href").unwrap_or_default());
                let media = it.get("media-type").unwrap_or_default();
                if href.starts_with("Scripts/") || media.contains("javascript") {
                    notice(out, "macros", format!("{CONTENT_PART} {id}"), href.clone());
                } else if it.get("isEmbeded").as_deref() == Some("0") {
                    let kind = if media.starts_with("image/") { "linked-image" } else { "linked-object" };
                    notice(out, kind, format!("{CONTENT_PART} {id}"), href.clone());
                }
                gone_parts.insert(href);
                gone_ids.insert(id);
            }
        }
        for s in root.elements_mut().filter(|e| e.is("opf:spine")) {
            changed |= !remove_where(s, &|e| e.get("idref").is_some_and(|r| gone_ids.contains(&r))).is_empty();
        }
        changed
    });
    for p in parts.iter().filter(|p| p.name.starts_with("Scripts/")) {
        if !out.iter().any(|n| n.detail == p.name) {
            notice(out, "macros", p.name.clone(), "script part");
        }
    }
    edit_part(parts, "META-INF/manifest.xml", &mut |root| {
        !remove_where(root, &|e| e.get("manifest:full-path").is_some_and(|f| gone_parts.contains(&f))).is_empty()
    });
    parts.retain(|p| !gone_parts.contains(&p.name));
    // A linked source document (`hh:linkinfo path`) is fetched to inherit pages.
    edit_part(parts, HEADER_PART, &mut |root| {
        let mut changed = false;
        root.walk_mut(&mut |e| {
            if e.is("hh:linkinfo") && e.get("path").is_some_and(|p| !p.is_empty()) {
                notice(
                    out,
                    "linked-document",
                    format!("{HEADER_PART} <hh:linkinfo>"),
                    e.get("path").unwrap_or_default(),
                );
                e.set("path", "");
                changed = true;
            }
        });
        changed
    });
}

pub fn surface(parts: &[Part], sections: &[(String, Doc)], report: &mut ImportReport) {
    let out = &mut report.surface;
    for (name, d) in sections {
        let mut para = 0;
        let mut deleting: BTreeSet<String> = BTreeSet::new();
        let mut deleted = String::new();
        d.root.walk(&mut |e| {
            if e.is("hp:p") {
                para += 1;
            }
            if e.is("hp:ctrl") {
                if let Some(h) = e.child("hp:hiddenComment") {
                    notice(out, "hidden-text", format!("{name} paragraph {para}"), clip(&h.text_of(&["hp:t"]), 60));
                }
                if let Some(f) = e.child("hp:fieldBegin").filter(|f| f.get("type").as_deref() == Some("MEMO")) {
                    notice(out, "comment", format!("{name} paragraph {para}"), clip(&f.text_of(&["hp:t"]), 60));
                }
            }
            if e.is("hp:t") {
                for n in &e.children {
                    match n {
                        Node::El(x) if x.is("hp:deleteBegin") => {
                            deleting.insert(x.get("Id").unwrap_or_default());
                        }
                        Node::El(x) if x.is("hp:deleteEnd") => {
                            deleting.remove(&x.get("Id").unwrap_or_default());
                            if deleting.is_empty() && !deleted.trim().is_empty() {
                                notice(out, "tracked-deletion", format!("{name} paragraph {para}"), clip(&deleted, 60));
                                deleted.clear();
                            }
                        }
                        Node::Text(t) if !deleting.is_empty() => deleted.push_str(&xml::unescape(t)),
                        _ => {}
                    }
                }
            }
        });
    }
    if let Some(d) = hanji_package::package::get(parts, CONTENT_PART).and_then(|d| xml::parse(d).ok()) {
        d.root.walk(&mut |e| {
            let name = e.get("name").unwrap_or_default();
            if e.is("opf:meta") && ["creator", "lastsaveby"].contains(&name.as_str()) {
                let v = e.text_of(&["opf:meta"]);
                if !v.trim().is_empty() {
                    notice(out, "metadata", format!("{CONTENT_PART} {name}"), v);
                }
            }
        });
    }
    if hanji_package::package::get(parts, "Preview/PrvText.txt").is_some_and(|d| !d.is_empty()) {
        let detail = "a text preview of the original; export does not rewrite it";
        notice(out, "preview-text", "Preview/PrvText.txt", detail);
    }
}
