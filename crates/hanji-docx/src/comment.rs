//! DOCX comments whose body is ordinary text: shown as `<comment>…</comment>`
//! at the balloon (`w:commentReference`). Range start/end stay markers; author,
//! date, initials, threading and resolved state stay in `comments.xml` and the
//! extended parts.

use std::collections::{HashMap, HashSet};

use hanji_core::model::paras;
use hanji_core::{Block, Entry, Kind, Part};
use hanji_format::Atom;
use hanji_package::package;
use hanji_package::xml::{self, Element, Node};

const COMMENTS: &str = "word/comments.xml";
const COMMENTS_EX: &str = "word/commentsExtended.xml";
const COMMENTS_IDS: &str = "word/commentsIds.xml";
const COMMENTS_EXT: &str = "word/commentsExtensible.xml";

/// Locals a shown comment body may hold. Anything else (a table, a drawing,
/// tracked changes, a nested comment) stays a placeholder.
const SHOWN: &[&str] = &[
    "comment",
    "p",
    "pPr",
    "r",
    "rPr",
    "t",
    "annotationRef",
    "proofErr",
    "pStyle",
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
    "bidi",
    "overflowPunct",
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
    "keepNext",
    "keepLines",
    "widowControl",
    "autoSpaceDE",
    "autoSpaceDN",
    "adjustRightInd",
    "contextualSpacing",
];

/// Comment ids whose body the text can show, mapped to that body.
pub fn shown(parts: &[Part]) -> HashMap<String, String> {
    let Some(d) = package::get(parts, COMMENTS).and_then(|b| xml::parse(b).ok()) else {
        return HashMap::new();
    };
    let replies = reply_para_ids(parts);
    let mut out = HashMap::new();
    for c in d.root.elements().filter(|e| e.local() == "comment") {
        let Some(id) = c.get("w:id") else { continue };
        if let Some(body) = shown_body(c, &replies) {
            out.insert(id, body);
        }
    }
    out
}

fn reply_para_ids(parts: &[Part]) -> HashSet<String> {
    let mut out = HashSet::new();
    let Some(d) = package::get(parts, COMMENTS_EX).and_then(|b| xml::parse(b).ok()) else {
        return out;
    };
    for e in d.root.elements() {
        if e.local() != "commentEx" {
            continue;
        }
        if e.get("w15:paraIdParent")
            .filter(|s| !s.is_empty())
            .is_some()
        {
            if let Some(id) = e.get("w15:paraId") {
                out.insert(id);
            }
        }
    }
    out
}

fn shown_body(c: &Element, replies: &HashSet<String>) -> Option<String> {
    let paras: Vec<&Element> = c.elements().filter(|e| e.local() == "p").collect();
    if paras.len() != 1 {
        return None;
    }
    if let Some(pid) = paras[0].get("w14:paraId") {
        if replies.contains(&pid) {
            return None;
        }
    }
    let mut ok = true;
    c.walk(&mut |e| {
        if !SHOWN.contains(&e.local()) {
            ok = false;
        }
    });
    if !ok {
        return None;
    }
    let body = c.text_of(&["w:t"]);
    if body.contains("</comment>") || body.contains('\n') || body.contains('\r') {
        return None;
    }
    Some(body)
}

/// Rewrite `comments.xml` (and extended parts) from the live remainder: update
/// shown bodies, drop comments whose balloon is gone. Unchanged parts keep
/// their bytes.
pub fn apply(parts: &mut [Part], blocks: &[Block], entries: &[Entry]) -> Result<(), String> {
    let Some(raw) = package::get(parts, COMMENTS) else {
        return Ok(());
    };
    let mut doc = xml::parse(raw).map_err(|e| format!("{COMMENTS}: {e}"))?;
    let live_refs = live_references(entries);
    let bodies = shown_bodies(blocks, entries);
    let mut removed_para = HashSet::new();
    let mut removed_durable = HashSet::new();
    let mut changed = false;
    let children = std::mem::take(&mut doc.root.children);
    let mut kept = Vec::with_capacity(children.len());
    for n in children {
        let Node::El(mut c) = n else {
            kept.push(n);
            continue;
        };
        if c.local() != "comment" {
            kept.push(Node::El(c));
            continue;
        }
        let id = c.get("w:id").unwrap_or_default();
        if !live_refs.contains(&id) {
            if let Some(p) = first_para_id(&c) {
                removed_para.insert(p);
            }
            changed = true;
            continue;
        }
        if let Some(body) = bodies.get(&id) {
            if c.text_of(&["w:t"]) != *body {
                set_body(&mut c, body);
                changed = true;
            }
        }
        kept.push(Node::El(c));
    }
    doc.root.children = kept;
    if changed {
        put(parts, COMMENTS, xml::write_doc(&doc));
        strip_extended(parts, &removed_para, &mut removed_durable)?;
    }
    Ok(())
}

fn live_references(entries: &[Entry]) -> HashSet<String> {
    let mut out = HashSet::new();
    for e in entries {
        for x in e.xml.iter().chain(e.meta.aux.iter()) {
            if x.contains("commentReference") {
                if let Some(id) = w_id(x) {
                    out.insert(id);
                }
            }
        }
    }
    out
}

fn shown_bodies(blocks: &[Block], entries: &[Entry]) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for e in entries {
        if e.kind != Kind::Wrap || e.meta.tag != "w:commentReference" {
            continue;
        }
        let Some(id) = e.xml.first().and_then(|x| w_id(x)) else {
            continue;
        };
        let Some(inl) = paras(blocks)
            .into_iter()
            .find(|(p, _)| *p == e.path)
            .and_then(|(_, i)| i)
        else {
            continue;
        };
        let Some(s) = e.start else { continue };
        if let Some(Atom::Comment(body)) = inl.units.get(s).map(|u| &u.atom) {
            out.insert(id, body.clone());
        }
    }
    out
}

fn first_para_id(c: &Element) -> Option<String> {
    c.elements()
        .find(|e| e.local() == "p")
        .and_then(|p| p.get("w14:paraId"))
}

fn set_body(comment: &mut Element, body: &str) {
    let mut used = false;
    set_t(comment, body, &mut used);
    if !used {
        let mut t = Element::new("w:t");
        if needs_preserve(body) {
            t.set("xml:space", "preserve");
        }
        t.children.push(Node::Text(xml::escape_text(body)));
        let mut r = Element::new("w:r");
        r.children.push(Node::El(t));
        comment.children.push(Node::El(r));
    }
}

fn set_t(e: &mut Element, body: &str, used: &mut bool) {
    if e.local() == "t" {
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

fn strip_extended(
    parts: &mut [Part],
    removed_para: &HashSet<String>,
    removed_durable: &mut HashSet<String>,
) -> Result<(), String> {
    if removed_para.is_empty() {
        return Ok(());
    }
    drop_matching(parts, COMMENTS_EX, "commentEx", "w15:paraId", removed_para)?;
    if let Some(raw) = package::get(parts, COMMENTS_IDS) {
        let mut doc = xml::parse(raw).map_err(|e| format!("{COMMENTS_IDS}: {e}"))?;
        let children = std::mem::take(&mut doc.root.children);
        let mut kept = Vec::with_capacity(children.len());
        let mut changed = false;
        for n in children {
            let Node::El(e) = n else {
                kept.push(n);
                continue;
            };
            if e.local() == "commentId" {
                if let Some(pid) = e.get("w16cid:paraId") {
                    if removed_para.contains(&pid) {
                        if let Some(d) = e.get("w16cid:durableId") {
                            removed_durable.insert(d);
                        }
                        changed = true;
                        continue;
                    }
                }
            }
            kept.push(Node::El(e));
        }
        if changed {
            doc.root.children = kept;
            put(parts, COMMENTS_IDS, xml::write_doc(&doc));
        }
    }
    if !removed_durable.is_empty() {
        drop_matching(
            parts,
            COMMENTS_EXT,
            "commentExtensible",
            "w16cex:durableId",
            removed_durable,
        )?;
    }
    Ok(())
}

fn drop_matching(
    parts: &mut [Part],
    name: &str,
    local: &str,
    attr: &str,
    ids: &HashSet<String>,
) -> Result<(), String> {
    let Some(raw) = package::get(parts, name) else {
        return Ok(());
    };
    let mut doc = xml::parse(raw).map_err(|e| format!("{name}: {e}"))?;
    let n = doc.root.children.len();
    doc.root.children.retain(|node| match node {
        Node::El(e) if e.local() == local => e.get(attr).is_none_or(|v| !ids.contains(&v)),
        _ => true,
    });
    if doc.root.children.len() != n {
        put(parts, name, xml::write_doc(&doc));
    }
    Ok(())
}

fn put(parts: &mut [Part], name: &str, data: Vec<u8>) {
    if let Some(p) = parts.iter_mut().find(|p| p.name == name) {
        p.data = data;
    }
}

pub(crate) fn w_id(xml: &str) -> Option<String> {
    let key = "w:id=\"";
    let i = xml.find(key)?;
    let rest = &xml[i + key.len()..];
    Some(rest[..rest.find('"')?].to_string())
}

/// A run that holds only a `w:commentReference` (and optional `w:rPr`).
pub fn comment_only_run(r: &Element) -> Option<&Element> {
    if !r.is("w:r") {
        return None;
    }
    let content: Vec<&Element> = r.elements().filter(|c| !c.is("w:rPr")).collect();
    match content.as_slice() {
        [c] if c.is("w:commentReference") => Some(*c),
        _ => None,
    }
}

/// The `w:id` of a comment-only run's reference, when that comment is shown.
pub fn shown_run<'a>(r: &'a Element, shown: &HashMap<String, String>) -> Option<&'a Element> {
    let cref = comment_only_run(r)?;
    let id = cref.get("w:id")?;
    shown.contains_key(&id).then_some(r)
}
