//! DOCX footnotes and endnotes whose body is ordinary text in one paragraph:
//! shown as `[^label]` at the reference and `[^label]: body` for the
//! definition (DESIGN.md §5.1). Separator and continuation notices stay in
//! the note parts; notes that hold a table, drawing, content control or
//! several paragraphs stay `<keep/>`.

use std::collections::{HashMap, HashSet};

use hanji_core::model::paras;
use hanji_core::{Block, Entry, Kind, Part};
use hanji_format::{Atom, Note, NoteKind};
use hanji_package::package;
use hanji_package::xml::{self, Element, Node};

const FOOTNOTES: &str = "word/footnotes.xml";
const ENDNOTES: &str = "word/endnotes.xml";

/// Locals a shown note body may hold. Anything else (a table, a drawing, a
/// content control, nested notes) stays a placeholder.
const SHOWN: &[&str] = &[
    "footnote",
    "endnote",
    "p",
    "pPr",
    "r",
    "rPr",
    "t",
    "footnoteRef",
    "endnoteRef",
    "tab",
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
    "lastRenderedPageBreak",
];

/// Note ids whose body the text can show, mapped to that body. Key is
/// (`"footnote"` / `"endnote"`, `w:id`).
pub fn shown(parts: &[Part]) -> HashMap<(String, String), String> {
    let mut out = HashMap::new();
    for (part, tag, kind) in [
        (FOOTNOTES, "footnote", "footnote"),
        (ENDNOTES, "endnote", "endnote"),
    ] {
        let Some(d) = package::get(parts, part).and_then(|b| xml::parse(b).ok()) else {
            continue;
        };
        for n in d.root.elements().filter(|e| e.local() == tag) {
            if n.get("w:type").is_some() {
                continue;
            }
            let Some(id) = n.get("w:id") else { continue };
            if let Some(body) = shown_body(n) {
                out.insert((kind.into(), id), body);
            }
        }
    }
    out
}

fn shown_body(n: &Element) -> Option<String> {
    let paras: Vec<&Element> = n.elements().filter(|e| e.local() == "p").collect();
    if paras.len() != 1 {
        return None;
    }
    let mut ok = true;
    n.walk(&mut |e| {
        if !SHOWN.contains(&e.local()) {
            ok = false;
        }
    });
    if !ok {
        return None;
    }
    let body = n.text_of(&["w:t"]);
    if body.contains('\n') || body.contains('\r') {
        return None;
    }
    Some(body)
}

/// Rewrite `footnotes.xml` / `endnotes.xml` from the live remainder: update
/// shown bodies, drop notes whose reference is gone (separator and
/// continuation notices stay). Unchanged parts keep their bytes.
pub fn apply(parts: &mut [Part], blocks: &[Block], entries: &[Entry]) -> Result<(), String> {
    apply_part(
        parts,
        FOOTNOTES,
        "footnote",
        "w:footnoteReference",
        blocks,
        entries,
    )?;
    apply_part(
        parts,
        ENDNOTES,
        "endnote",
        "w:endnoteReference",
        blocks,
        entries,
    )?;
    Ok(())
}

fn apply_part(
    parts: &mut [Part],
    name: &str,
    local: &str,
    tag: &str,
    blocks: &[Block],
    entries: &[Entry],
) -> Result<(), String> {
    let Some(raw) = package::get(parts, name) else {
        return Ok(());
    };
    let mut doc = xml::parse(raw).map_err(|e| format!("{name}: {e}"))?;
    let live = live_ids(entries, tag);
    let bodies = shown_bodies(blocks, entries, tag);
    let mut changed = false;
    let children = std::mem::take(&mut doc.root.children);
    let mut kept = Vec::with_capacity(children.len());
    for n in children {
        let Node::El(mut c) = n else {
            kept.push(n);
            continue;
        };
        if c.local() != local {
            kept.push(Node::El(c));
            continue;
        }
        if c.get("w:type").is_some() {
            kept.push(Node::El(c));
            continue;
        }
        let id = c.get("w:id").unwrap_or_default();
        if !live.contains(&id) {
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
        put(parts, name, xml::write_doc(&doc));
    }
    Ok(())
}

fn live_ids(entries: &[Entry], tag: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let needle = tag.rsplit(':').next().unwrap_or(tag);
    for e in entries {
        for x in e.xml.iter().chain(e.meta.aux.iter()) {
            if x.contains(needle) {
                if let Some(id) = crate::comment::w_id(x) {
                    out.insert(id);
                }
            }
        }
    }
    out
}

fn shown_bodies(blocks: &[Block], entries: &[Entry], tag: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for e in entries {
        if e.kind != Kind::Wrap || e.meta.tag != tag {
            continue;
        }
        let Some(id) = e.xml.first().and_then(|x| crate::comment::w_id(x)) else {
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
        if let Some(Atom::NoteRef(Note { body, .. })) = inl.units.get(s).map(|u| &u.atom) {
            out.insert(id, body.clone());
        }
    }
    out
}

fn set_body(note: &mut Element, body: &str) {
    let mut used = false;
    set_t(note, body, &mut used);
    if !used {
        let mut t = Element::new("w:t");
        if needs_preserve(body) {
            t.set("xml:space", "preserve");
        }
        t.children.push(Node::Text(xml::escape_text(body)));
        let mut r = Element::new("w:r");
        r.children.push(Node::El(t));
        note.children.push(Node::El(r));
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

fn put(parts: &mut [Part], name: &str, data: Vec<u8>) {
    if let Some(p) = parts.iter_mut().find(|p| p.name == name) {
        p.data = data;
    }
}

/// A run that holds only a `w:footnoteReference` or `w:endnoteReference`
/// (and optional `w:rPr`).
pub fn note_only_run(r: &Element) -> Option<(&Element, NoteKind)> {
    if !r.is("w:r") {
        return None;
    }
    let content: Vec<&Element> = r.elements().filter(|c| !c.is("w:rPr")).collect();
    match content.as_slice() {
        [c] if c.is("w:footnoteReference") => Some((*c, NoteKind::Footnote)),
        [c] if c.is("w:endnoteReference") => Some((*c, NoteKind::Endnote)),
        _ => None,
    }
}

/// A note-only run whose body the text can show: the run, its kind, and that body.
pub fn shown_run<'a>(
    r: &'a Element,
    shown: &HashMap<(String, String), String>,
) -> Option<(&'a Element, NoteKind, String)> {
    let (nref, kind) = note_only_run(r)?;
    let id = nref.get("w:id")?;
    let body = shown.get(&(kind.as_str().into(), id.clone()))?;
    Some((r, kind, body.clone()))
}
