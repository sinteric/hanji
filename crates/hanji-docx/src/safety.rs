//! §8 on import: active and remote content is neutralised (removed, and
//! reported, never preserved); hidden text, comments, tracked deletions and
//! author metadata are listed to surface before export.

use hanji_core::{ImportReport, Notice, Part};

use crate::import::clip;
use crate::ooxml::DOC_PART;
use crate::xml::{self, Element, Node};

/// Field instructions that fetch or run something when the file opens or updates.
const FETCHING_FIELDS: &[&str] = &["DDE", "DDEAUTO", "INCLUDEPICTURE", "INCLUDETEXT", "LINK", "IMPORT", "RD"];

/// Extensions of macro-enabled Office packages.
const MACRO_PACKAGES: &[&str] =
    &[".docm", ".dotm", ".xlsm", ".xltm", ".xlam", ".xlsb", ".pptm", ".potm", ".ppsm", ".ppam", ".sldm"];

/// An embedded package that can carry macros: a macro-enabled extension, or
/// a zip holding a VBA project.
fn is_macro_package(p: &Part) -> bool {
    let lower = p.name.to_ascii_lowercase();
    MACRO_PACKAGES.iter().any(|x| lower.ends_with(x))
        || (p.data.starts_with(b"PK")
            && crate::package::read(&p.data)
                .is_ok_and(|inner| inner.iter().any(|q| q.name.to_ascii_lowercase().ends_with("vbaproject.bin"))))
}

fn notice(report: &mut Vec<Notice>, kind: &str, location: impl Into<String>, detail: impl Into<String>) {
    report.push(Notice { kind: kind.into(), location: location.into(), detail: detail.into() });
}

/// `word/_rels/document.xml.rels` → `word/document.xml`.
fn source_of_rels(rels: &str) -> Option<String> {
    let (dir, file) = rels.rsplit_once("_rels/")?;
    Some(format!("{dir}{}", file.strip_suffix(".rels")?))
}

/// A relationship target resolved against its source part.
fn resolve_target(source: &str, target: &str) -> String {
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

struct Removed {
    source: String,
    id: String,
    kind: &'static str,
}

pub fn neutralise(parts: &mut Vec<Part>, doc: &mut Element, report: &mut ImportReport) -> Result<(), String> {
    let out = &mut report.neutralised;
    let mut removed_rels: Vec<Removed> = vec![];
    let mut drop_parts: Vec<String> = vec![];
    let macro_parts: std::collections::BTreeSet<String> =
        parts.iter().filter(|p| p.name != DOC_PART && is_macro_package(p)).map(|p| p.name.clone()).collect();
    // Relationships that fetch (external, not a hyperlink) or run (OLE, ActiveX,
    // macros, packages that carry macros).
    for part in parts.iter_mut() {
        if !part.name.ends_with(".rels") {
            continue;
        }
        let Some(source) = source_of_rels(&part.name) else { continue };
        let Ok(mut d) = xml::parse(&part.data) else { continue };
        let before = d.root.children.len();
        let mut kept = vec![];
        for n in std::mem::take(&mut d.root.children) {
            let Node::El(r) = &n else {
                kept.push(n);
                continue;
            };
            let ty = r.get("Type").unwrap_or_default();
            let ty_short = ty.rsplit('/').next().unwrap_or("").to_string();
            let external = r.get("TargetMode").as_deref() == Some("External");
            let target = r.get("Target").unwrap_or_default();
            let kind: Option<&'static str> = match ty_short.as_str() {
                "hyperlink" => None,
                "attachedTemplate" if external => Some("remote-template"),
                "image" if external => Some("linked-image"),
                "oleObject" if external => Some("linked-object"),
                _ if !external && macro_parts.contains(&resolve_target(&source, &target)) => Some("macro-package"),
                "oleObject" | "package" if source == DOC_PART => Some("ole-object"),
                "control" | "activeXControl" => Some("activex-control"),
                "vbaProject" | "wordVbaData" | "keyMapCustomizations" => Some("macros"),
                _ if external => Some("external-relationship"),
                _ => None,
            };
            match kind {
                Some(kind) => {
                    notice(out, kind, format!("{} {}", part.name, r.get("Id").unwrap_or_default()), target.clone());
                    removed_rels.push(Removed { source: source.clone(), id: r.get("Id").unwrap_or_default(), kind });
                    if !external {
                        drop_parts.push(resolve_target(&source, &target));
                    }
                }
                None => kept.push(n),
            }
        }
        d.root.children = kept;
        if d.root.children.len() != before {
            part.data = xml::write_doc(&d);
        }
    }
    // The XML that points at them.
    for src in removed_rels.iter().map(|r| r.source.clone()).collect::<std::collections::BTreeSet<_>>() {
        let ids: Vec<&Removed> = removed_rels.iter().filter(|r| r.source == src).collect();
        if src == DOC_PART {
            strip_refs(doc, &ids, out, DOC_PART);
            continue;
        }
        let Some(p) = parts.iter_mut().find(|p| p.name == src) else { continue };
        let Ok(mut d) = xml::parse(&p.data) else { continue };
        strip_refs(&mut d.root, &ids, out, &src);
        p.data = xml::write_doc(&d);
    }
    // ActiveX controls and embedded objects not reached through a relationship above.
    neutralise_objects(doc, out);
    // Fields that fetch: keep their shown result, drop the instruction.
    neutralise_fields(doc, out);
    // Macro-carrying packages that no relationship reached go too.
    for name in &macro_parts {
        if !drop_parts.contains(name) {
            notice(out, "macro-package", name.clone(), "embedded package that can carry macros");
            drop_parts.push(name.clone());
        }
    }
    // Parts that only served removed content, and their content types.
    let mut gone = std::collections::BTreeSet::new();
    for name in drop_parts {
        let rels = match name.rsplit_once('/') {
            Some((d, f)) => format!("{d}/_rels/{f}.rels"),
            None => format!("_rels/{name}.rels"),
        };
        gone.insert(name);
        gone.insert(rels);
    }
    parts.retain(|p| !gone.contains(&p.name));
    if let Some(ct) = parts.iter_mut().find(|p| p.name == "[Content_Types].xml") {
        if let Ok(mut d) = xml::parse(&ct.data) {
            let n = d.root.children.len();
            d.root.children.retain(|c| !matches!(c, Node::El(e) if e.is("Override") && e.get("PartName").is_some_and(|p| gone.contains(p.trim_start_matches('/')))));
            let mut changed = d.root.children.len() != n;
            if gone.iter().any(|g| g.ends_with("vbaProject.bin")) {
                for e in d.root.elements_mut() {
                    if e.get("ContentType").as_deref() == Some("application/vnd.ms-word.document.macroEnabled.main+xml")
                    {
                        e.set(
                            "ContentType",
                            "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml",
                        );
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

/// Remove attributes and elements that reference removed relationships.
fn strip_refs(root: &mut Element, ids: &[&Removed], out: &mut Vec<Notice>, part: &str) {
    fn rec(e: &mut Element, ids: &[&Removed], out: &mut Vec<Notice>, part: &str) {
        e.children.retain(|n| {
            let Node::El(c) = n else { return true };
            // Whole elements that only exist to reach the target.
            let reaches = matches!(
                c.name.as_str(),
                "w:attachedTemplate" | "o:OLEObject" | "w:control" | "w:subDoc" | "c:externalData"
            );
            !(reaches
                && ids.iter().any(|r| c.attrs.iter().any(|a| a.0.starts_with("r:") && xml::unescape(&a.1) == r.id)))
        });
        for c in e.elements_mut() {
            let before = c.attrs.len();
            c.attrs.retain(|a| {
                !(a.0.starts_with("r:") && ids.iter().any(|r| xml::unescape(&a.1) == r.id && r.kind != "ole-object"))
            });
            if c.attrs.len() != before {
                notice(out, "reference-removed", format!("{part} <{}>", c.name), "points at a removed relationship");
            }
            rec(c, ids, out, part);
        }
    }
    rec(root, ids, out, part);
}

fn neutralise_objects(doc: &mut Element, out: &mut Vec<Notice>) {
    fn rec(e: &mut Element, out: &mut Vec<Notice>) {
        let n = e.children.len();
        e.children.retain(|c| !matches!(c, Node::El(x) if x.is("o:OLEObject") || x.is("w:control")));
        if e.children.len() != n {
            let detail = "embedded object or control removed; its preview picture stays";
            notice(out, "ole-object", format!("{DOC_PART} <{}>", e.name), detail);
        }
        for c in e.elements_mut() {
            rec(c, out);
        }
    }
    rec(doc, out);
}

/// `fldSimple` and complex fields whose instruction fetches content.
fn neutralise_fields(doc: &mut Element, out: &mut Vec<Notice>) {
    fn fetches(instr: &str) -> bool {
        let first = instr.split_whitespace().next().unwrap_or("").to_ascii_uppercase();
        FETCHING_FIELDS.contains(&first.as_str())
    }
    // fldSimple → its result runs.
    fn simple(e: &mut Element, out: &mut Vec<Notice>) {
        let mut k = 0;
        while k < e.children.len() {
            if let Node::El(c) = &e.children[k] {
                if c.is("w:fldSimple") && fetches(&c.get("w:instr").unwrap_or_default()) {
                    notice(out, "fetching-field", DOC_PART, clip(&c.get("w:instr").unwrap_or_default(), 60));
                    let Node::El(c) = e.children.remove(k) else { unreachable!() };
                    for (j, x) in c.children.into_iter().enumerate() {
                        e.children.insert(k + j, x);
                    }
                    continue;
                }
            }
            if let Node::El(c) = &mut e.children[k] {
                simple(c, out);
            }
            k += 1;
        }
    }
    simple(doc, out);
    // Complex fields: find (in document order) each field's parts, then drop
    // begin..separate and end of the fetching ones.
    let mut toks: Vec<(Vec<usize>, Tok)> = vec![];
    collect(doc, &mut vec![], &mut toks);
    let mut drop = std::collections::BTreeSet::new();
    let mut stack: Vec<(usize, String, bool)> = vec![]; // (begin token, instruction, separated)
    let mut bad_fields: Vec<(usize, String)> = vec![];
    for i in 0..toks.len() {
        match &toks[i].1 {
            Tok::Begin => stack.push((i, String::new(), false)),
            Tok::Instr(t) => {
                if let Some(top) = stack.last_mut().filter(|t| !t.2) {
                    top.1.push_str(t);
                }
            }
            Tok::Separate => {
                if let Some(top) = stack.last_mut() {
                    top.2 = true;
                }
            }
            Tok::End => {
                let Some((start, instr, _)) = stack.pop() else { continue };
                if !fetches(&instr) {
                    continue;
                }
                bad_fields.push((start, instr));
                // Its begin, instruction (nested fields included), separate and end go; the result stays.
                let (mut depth, mut sep) = (0, false);
                for tok in &toks[start..=i] {
                    let in_instr = !sep;
                    match tok.1 {
                        Tok::Begin => depth += 1,
                        Tok::Separate if depth == 1 => sep = true,
                        _ => {}
                    }
                    let own_end = matches!(tok.1, Tok::End) && depth == 1;
                    if in_instr || own_end {
                        drop.insert(tok.0.clone());
                    }
                    if matches!(tok.1, Tok::End) {
                        depth -= 1;
                    }
                }
            }
        }
    }
    for (_, instr) in &bad_fields {
        notice(out, "fetching-field", DOC_PART, clip(instr, 60));
    }
    for path in drop.into_iter().rev() {
        remove_at(doc, &path);
    }
}

enum Tok {
    Begin,
    Separate,
    End,
    Instr(String),
}

fn collect(e: &Element, path: &mut Vec<usize>, out: &mut Vec<(Vec<usize>, Tok)>) {
    for (k, n) in e.children.iter().enumerate() {
        let Node::El(c) = n else { continue };
        path.push(k);
        let tok = match c.name.as_str() {
            "w:fldChar" => match c.get("w:fldCharType").as_deref() {
                Some("begin") => Some(Tok::Begin),
                Some("separate") => Some(Tok::Separate),
                Some("end") => Some(Tok::End),
                _ => None,
            },
            "w:instrText" => Some(Tok::Instr(c.text_of(&["w:instrText"]))),
            _ => None,
        };
        match tok {
            Some(t) => out.push((path.clone(), t)),
            None => collect(c, path, out),
        }
        path.pop();
    }
}

fn remove_at(e: &mut Element, path: &[usize]) {
    if path.len() == 1 {
        e.children.remove(path[0]);
        return;
    }
    if let Some(Node::El(c)) = e.children.get_mut(path[0]) {
        remove_at(c, &path[1..]);
    }
}

// ---------------------------------------------------------------- surface before export

pub fn surface(parts: &[Part], doc: &Element, report: &mut ImportReport) {
    let out = &mut report.surface;
    let mut para = 0;
    doc.walk(&mut |e| {
        if e.is("w:p") {
            para += 1;
        }
        if e.is("w:r") {
            let hidden = e.child("w:rPr").and_then(|p| p.child("w:vanish")).is_some_and(|v| crate::ooxml::on(Some(v)));
            let t = e.text_of(&["w:t"]);
            if hidden && !t.trim().is_empty() {
                notice(out, "hidden-text", format!("paragraph {para}"), clip(&t, 60));
            }
        }
        if e.is("w:del") || e.is("w:moveFrom") {
            let t = e.text_of(&["w:delText", "w:t"]);
            if !t.trim().is_empty() {
                let who = e.get("w:author").unwrap_or_else(|| "?".into());
                notice(out, "tracked-deletion", format!("paragraph {para}"), format!("{who}: {}", clip(&t, 60)));
            }
        }
    });
    if let Some(d) = crate::package::get(parts, "word/comments.xml").and_then(|d| xml::parse(d).ok()) {
        for c in d.root.elements().filter(|e| e.is("w:comment")) {
            let who = c.get("w:author").unwrap_or_else(|| "?".into());
            let location = format!("comment {}", c.get("w:id").unwrap_or_default());
            notice(out, "comment", location, format!("{who}: {}", clip(&c.text_of(&["w:t"]), 60)));
        }
    }
    for (part, tags) in [
        ("docProps/core.xml", &["dc:creator", "cp:lastModifiedBy"][..]),
        ("docProps/app.xml", &["Company", "Manager"][..]),
    ] {
        let Some(d) = crate::package::get(parts, part).and_then(|d| xml::parse(d).ok()) else { continue };
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
