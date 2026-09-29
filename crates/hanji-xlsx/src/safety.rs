//! §8 on import, as for the other engines: active and remote content is
//! neutralised (removed and reported, never preserved) — macros (xlsm,
//! `vbaProject`), Excel 4 macro sheets and dialog sheets, links to other
//! workbooks, DDE and OLE links, embedded OLE objects, ActiveX controls, data
//! connections and query tables, other external relationships, and formulas
//! that fetch (WEBSERVICE, …), run DDE or read another workbook: those keep
//! their cached value and lose the formula. Hidden sheets, rows and columns
//! holding data, notes and comments, HYPERLINK formulas and author metadata
//! are listed to surface before export.

use std::collections::{BTreeMap, BTreeSet};

use hanji_core::{notice, ImportReport, Part};
use hanji_format::formula::{self, FormulaProblem, Tok};
use hanji_package::opc::{self, is_macro_package, rels_part};
use hanji_package::xml::{self, Element, Node};
use hanji_package::{clip, package};

use crate::book::{Book, SheetKind, CT_MAIN};

/// Macro-enabled main parts and what they become.
const MACRO_MAINS: &[(&str, &str)] = &[
    ("application/vnd.ms-excel.sheet.macroEnabled.main+xml", CT_MAIN),
    (
        "application/vnd.ms-excel.template.macroEnabled.main+xml",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.template.main+xml",
    ),
    ("application/vnd.ms-excel.addin.macroEnabled.main+xml", CT_MAIN),
];

/// Worksheet elements that exist only to reach what a removed relationship pointed at.
const SHEET_REACHES: &[&str] = &["oleObjects", "controls"];

/// Neutralise the package before the workbook is read: relationships and
/// the parts they reach, the workbook's sheet list and external references,
/// and the worksheet elements pointing at them.
pub fn neutralise_parts(parts: &mut Vec<Part>, report: &mut ImportReport) -> Result<(), String> {
    let out = &mut report.neutralised;
    let wb_part = Book::main_part(parts)?;
    let macro_parts: BTreeSet<String> = parts.iter().filter(|p| is_macro_package(p)).map(|p| p.name.clone()).collect();
    let mut drop_parts: BTreeSet<String> = BTreeSet::new();
    let mut removed: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for part in parts.iter_mut().filter(|p| p.name.ends_with(".rels")) {
        let Some(source) = opc::source_of_rels(&part.name) else { continue };
        let Ok(mut d) = xml::parse(&part.data) else { continue };
        let before = d.root.children.len();
        let from_workbook = source == wb_part;
        d.root.children.retain(|n| {
            let Node::El(r) = n else { return true };
            let ty = r.get("Type").unwrap_or_default();
            let short = ty.rsplit('/').next().unwrap_or("");
            let external = r.get("TargetMode").as_deref() == Some("External");
            let target = r.get("Target").unwrap_or_default();
            let resolved = opc::resolve_target(&source, &target);
            let kind: Option<&'static str> = match short {
                "hyperlink" => None,
                _ if !external && macro_parts.contains(&resolved) => Some("macro-package"),
                "vbaProject" | "vbaProjectSignature" | "wordVbaData" | "vbaData" => Some("macros"),
                "xlMacrosheet" | "xlIntlMacrosheet" => Some("macro-sheet"),
                "dialogsheet" => Some("macro-sheet"),
                "externalLink" if from_workbook => Some("external-link"),
                "externalLinkPath" | "xlExternalLinkPath/xlPathMissing" => Some("external-link"),
                "control" | "activeXControl" | "activeXControlBinary" | "ctrlProp" => Some("activex-control"),
                "oleObject" if external => Some("linked-object"),
                "oleObject" => Some("ole-object"),
                "package" if source.contains("/worksheets/") => Some("ole-object"),
                "connections" => Some("data-connection"),
                "queryTable" => Some("query-table"),
                "image" if external => Some("linked-image"),
                _ if external => Some("external-relationship"),
                _ => None,
            };
            let Some(kind) = kind else { return true };
            notice(out, kind, format!("{} {}", part.name, r.get("Id").unwrap_or_default()), clip(&target, 80));
            removed.entry(source.clone()).or_default().insert(r.get("Id").unwrap_or_default());
            if !external {
                drop_parts.insert(resolved);
            }
            false
        });
        if d.root.children.len() != before {
            part.data = xml::write_doc(&d);
        }
    }
    for name in &macro_parts {
        if !drop_parts.contains(name) {
            notice(out, "macro-package", name.clone(), "embedded package that can carry macros");
            drop_parts.insert(name.clone());
        }
    }
    // The XML that pointed at removed relationships.
    for (src, ids) in &removed {
        let Some(p) = parts.iter_mut().find(|p| &p.name == src) else { continue };
        let Ok(mut d) = xml::parse(&p.data) else { continue };
        let before = d.root.clone();
        if *src == wb_part {
            clean_workbook(&mut d.root, ids, out);
        } else {
            clean_refs(&mut d.root, ids);
        }
        if d.root != before {
            p.data = xml::write_doc(&d);
        }
    }
    // Tables fed by a removed query table become plain tables.
    for p in parts.iter_mut().filter(|p| p.name.contains("/tables/") && p.name.ends_with(".xml")) {
        let Ok(mut d) = xml::parse(&p.data) else { continue };
        if d.root.get("tableType").as_deref() == Some("queryTable") && opc::rels_part(&p.name) != p.name {
            d.root.remove_attr("tableType");
            d.root.walk_mut(&mut |e| {
                e.remove_attr("queryTableFieldId");
                e.remove_attr("uniqueName");
            });
            p.data = xml::write_doc(&d);
        }
    }
    // Parts that only served removed content, and their content types; a
    // macro-enabled main part becomes a plain one.
    let mut gone = BTreeSet::new();
    let mut todo: Vec<String> = drop_parts.into_iter().collect();
    while let Some(name) = todo.pop() {
        if !gone.insert(name.clone()) {
            continue;
        }
        let rels = rels_part(&name);
        for r in opc::rels_of(parts, &name).into_iter().filter(|r| !r.external) {
            todo.push(opc::resolve_target(&name, &r.target));
        }
        gone.insert(rels);
    }
    // Keep a part something else still reaches (a shared image).
    let reach = {
        let kept: Vec<Part> = parts.iter().filter(|p| !gone.contains(&p.name)).cloned().collect();
        opc::reachable(&kept)
    };
    gone.retain(|g| !reach.contains(g) || g.ends_with(".rels"));
    parts.retain(|p| !gone.contains(&p.name));
    if let Some(ct) = opc::content_types(parts, &gone, &[]) {
        let p = parts.iter_mut().find(|p| p.name == opc::CT_PART).unwrap();
        p.data = ct;
    }
    if let Some(p) = parts.iter_mut().find(|p| p.name == opc::CT_PART) {
        let mut text = String::from_utf8_lossy(&p.data).into_owned();
        let mut changed = false;
        for (from, to) in MACRO_MAINS {
            if text.contains(from) {
                text = text.replace(from, to);
                changed = true;
            }
        }
        if changed {
            p.data = text.into_bytes();
        }
    }
    Ok(())
}

/// Remove the workbook's references to removed relationships: sheets (macro
/// sheets), external references (and the names that read them).
fn clean_workbook(root: &mut Element, ids: &BTreeSet<String>, out: &mut Vec<hanji_core::Notice>) {
    let rid = |e: &Element| e.attrs.iter().find(|a| a.0.ends_with(":id")).map(|a| xml::unescape(&a.1));
    let mut ext_removed = false;
    for c in root.elements_mut() {
        match c.local() {
            "sheets" => c.children.retain(|n| !matches!(n, Node::El(s) if rid(s).is_some_and(|i| ids.contains(&i)))),
            "externalReferences" => {
                let n = c.elements().count();
                c.children.retain(|n| !matches!(n, Node::El(s) if rid(s).is_some_and(|i| ids.contains(&i))));
                ext_removed |= c.elements().count() != n;
            }
            _ => {}
        }
    }
    root.children.retain(|n| !matches!(n, Node::El(e) if e.local() == "externalReferences" && !e.has_elements()));
    // Names that read another workbook, or run a macro sheet's code.
    if let Some(names) = root.elements_mut().find(|e| e.local() == "definedNames") {
        names.children.retain(|n| {
            let Node::El(d) = n else { return true };
            let f = d.text_of(&[d.name.as_str()]);
            let external = ext_removed && reads_other_workbook(&f);
            let auto = d.get("name").is_some_and(|n| {
                n.to_ascii_lowercase().starts_with("auto_open") || n.to_ascii_lowercase().starts_with("auto_close")
            });
            let fetch = formula::safety_problem(&f).is_some_and(|p| !matches!(p, FormulaProblem::DataUrl));
            if external || auto || fetch {
                let kind = if auto {
                    "macro-name"
                } else if fetch {
                    "fetching-function"
                } else {
                    "external-reference"
                };
                notice(out, kind, format!("defined name {}", d.get("name").unwrap_or_default()), clip(&f, 60));
                return false;
            }
            true
        });
    }
    root.children.retain(|n| !matches!(n, Node::El(e) if e.local() == "definedNames" && !e.has_elements()));
}

/// Elements anywhere under `root` that point at a removed relationship.
fn clean_refs(root: &mut Element, ids: &BTreeSet<String>) {
    let points = |e: &Element| {
        e.attrs.iter().any(|a| {
            (a.0.ends_with(":id") || a.0.ends_with(":link") || a.0.ends_with(":embed"))
                && ids.contains(&xml::unescape(&a.1))
        })
    };
    fn deep(e: &Element, points: &dyn Fn(&Element) -> bool) -> bool {
        points(e) || e.elements().any(|c| deep(c, points))
    }
    fn walk(e: &mut Element, points: &dyn Fn(&Element) -> bool) {
        e.children.retain(|n| {
            let Node::El(c) = n else { return true };
            if SHEET_REACHES.contains(&c.local()) {
                return !deep(c, points);
            }
            !points(c) || c.local() == "blip"
        });
        for c in e.elements_mut() {
            if c.local() == "blip" {
                c.attrs.retain(|a| !(a.0.ends_with(":link")));
            }
            walk(c, points);
        }
        // An mc:AlternateContent left holding only its fallback's shell, or nothing.
        e.children.retain(|n| !matches!(n, Node::El(c) if c.local() == "AlternateContent" && !c.elements().any(|x| x.has_elements() || !x.attrs.is_empty())));
    }
    walk(root, &points);
    root.children.retain(|n| !matches!(n, Node::El(c) if SHEET_REACHES.contains(&c.local()) && !c.has_elements()));
}

/// A reference into another workbook (`[1]Sheet1!A1`, `'[Book.xlsx]Sheet'!A1`, `[1]!Name`).
fn reads_other_workbook(f: &str) -> bool {
    formula::tokenize(f).iter().any(|t| matches!(&t.kind, Tok::Ref(r) if r.sheet.as_ref().is_some_and(|s| s.external)))
        || f.contains("]!")
}

/// A row pre-check for [`neutralise_formulas`]: a formula, and a fetching
/// function's name, a DDE `|`, or a `]` and a `!` (a reference into another
/// workbook: `[1]Sheet!A1`, `'C:\[b.xlsx]S'!A1`, `[1]!Name`).
fn may_need_neutralising(row: &[u8]) -> bool {
    let has = |b: u8| memchr::memchr(b, row).is_some();
    crate::store::has_formula(row)
        && (has(b'|') || (has(b']') && has(b'!')) || {
            let upper = row.to_ascii_uppercase();
            formula::FETCH_FUNCTIONS.iter().any(|n| crate::store::holds(&upper, n.as_bytes()))
        })
}

/// Neutralise the package, then the formulas of every worksheet.
pub fn neutralise(book: &mut Book, report: &mut ImportReport) -> Result<(), String> {
    let mut parts = std::mem::take(&mut book.parts);
    neutralise_parts(&mut parts, report)?;
    *book = Book::load(parts, vec![], Default::default())?;
    for i in 0..book.sheets.len() {
        if book.sheets[i].kind == SheetKind::Work {
            neutralise_formulas(book, i, report)?;
        }
    }
    Ok(())
}

/// Formulas that fetch, run DDE or read another workbook keep their cached value only.
fn neutralise_formulas(book: &mut Book, i: usize, report: &mut ImportReport) -> Result<(), String> {
    book.load_store(i)?;
    let sheet = book.sheets[i].name.clone();
    let mut hits: Vec<(u32, u32, &'static str, String)> = vec![];
    let mut shared: BTreeSet<String> = BTreeSet::new();
    book.store(i).for_each_cell_if(&may_need_neutralising, &mut |r, c| {
        let Some(f) = c.formula() else { return };
        let kind = match formula::safety_problem(&f) {
            Some(FormulaProblem::Fetch(_)) => Some("fetching-function"),
            Some(FormulaProblem::Dde(_)) => Some("dde-formula"),
            _ if reads_other_workbook(&f) => Some("external-reference"),
            _ => None,
        };
        if let Some(k) = kind {
            hits.push((r, c.col, k, f));
            if let Some(si) = c.f.as_ref().filter(|x| x.get("t").as_deref() == Some("shared")).and_then(|x| x.get("si"))
            {
                shared.insert(si);
            }
        }
    });
    if hits.is_empty() {
        return Ok(());
    }
    // Every cell of a shared formula that fetches.
    if !shared.is_empty() {
        book.store(i).for_each_cell_if(&crate::store::has_formula, &mut |r, c| {
            let si = c.f.as_ref().filter(|x| x.get("t").as_deref() == Some("shared")).and_then(|x| x.get("si"));
            if si.is_some_and(|s| shared.contains(&s)) && !hits.iter().any(|h| h.0 == r && h.1 == c.col) {
                hits.push((r, c.col, "fetching-function", "(shared formula)".into()));
            }
        });
    }
    let st = book.store_mut(i)?;
    for (r, col, kind, f) in hits {
        let cell = st.row_mut(r).cell_mut(col);
        cell.f = None;
        let at = format!("{sheet}!{}{r}", hanji_core::cells::col_letters(col));
        notice(&mut report.neutralised, kind, at, format!("formula {} removed; its last value is kept", clip(&f, 60)));
    }
    Ok(())
}

/// What a person should see before the file leaves (§8).
pub fn surface(book: &mut Book, report: &mut ImportReport) -> Result<(), String> {
    let out = &mut report.surface;
    for i in 0..book.sheets.len() {
        let s = book.sheets[i].clone();
        if s.state != "visible" {
            notice(out, "hidden-sheet", s.name.clone(), format!("the sheet is {}", s.state));
        }
        if s.kind != SheetKind::Work {
            continue;
        }
        book.load_store(i)?;
        let st = book.store(i);
        let mut hidden_rows = std::collections::BTreeSet::new();
        let mut links = vec![];
        let hidden_or_link = |b: &[u8]| {
            let tag_end = b.iter().position(|&x| x == b'>').unwrap_or(b.len());
            crate::store::holds(&b[..tag_end], b"hidden")
                || crate::store::holds(b, b"HYPERLINK")
                || crate::store::holds(b, b"hyperlink")
        };
        st.for_each_cell_if(&hidden_or_link, &mut |r, c| {
            if let Some(f) = c.formula() {
                if formula::functions(&formula::tokenize(&f)).iter().any(|n| n == "HYPERLINK") {
                    links.push(format!("{}{r}: {}", hanji_core::cells::col_letters(c.col), clip(&f, 60)));
                }
            }
            if !c.is_blank() {
                hidden_rows.insert(r);
            }
        });
        hidden_rows
            .retain(|r| st.row(*r).is_some_and(|row| row.get("hidden").is_some_and(|v| v == "1" || v == "true")));
        if !hidden_rows.is_empty() {
            let list: Vec<String> = hidden_rows.iter().take(20).map(|r| r.to_string()).collect();
            notice(
                out,
                "hidden-rows",
                s.name.clone(),
                format!("{} hidden rows hold data: {}", hidden_rows.len(), list.join(", ")),
            );
        }
        if let Some(cols) = st.child("cols") {
            let hidden: Vec<String> = cols
                .elements()
                .filter(|c| c.get("hidden").is_some_and(|v| v == "1" || v == "true"))
                .map(|c| format!("{}:{}", c.get("min").unwrap_or_default(), c.get("max").unwrap_or_default()))
                .collect();
            if !hidden.is_empty() {
                notice(out, "hidden-columns", s.name.clone(), format!("hidden columns {}", hidden.join(", ")));
            }
        }
        for l in links {
            notice(out, "hyperlink-formula", s.name.clone(), l);
        }
        for r in book.rels_of(&s.part).iter().filter(|r| !r.external) {
            let target = opc::resolve_target(&s.part, &r.target);
            let kind = match r.short_type() {
                "comments" => "comment",
                "threadedComment" => "comment",
                _ => continue,
            };
            let Some(d) = package::get(&book.parts, &target).and_then(|d| xml::parse(d).ok()) else { continue };
            let authors: Vec<String> = d
                .root
                .elements()
                .find(|e| e.local() == "authors")
                .map(|a| a.elements().map(|x| x.text_of(&[x.name.as_str()])).collect())
                .unwrap_or_default();
            d.root.walk(&mut |e| {
                if e.local() == "comment" || e.local() == "threadedComment" {
                    let who = e
                        .get("authorId")
                        .and_then(|k| k.parse::<usize>().ok())
                        .and_then(|k| authors.get(k).cloned())
                        .or_else(|| e.get("personId"))
                        .unwrap_or_default();
                    let text = crate::sst::rich_text(e);
                    notice(
                        out,
                        kind,
                        format!("{} {}", s.name, e.get("ref").unwrap_or_default()),
                        format!("{who}: {}", clip(&text, 60)),
                    );
                }
            });
        }
    }
    for (part, tags) in
        [("docProps/core.xml", &["creator", "lastModifiedBy"][..]), ("docProps/app.xml", &["Company", "Manager"][..])]
    {
        let Some(d) = package::get(&book.parts, part).and_then(|d| xml::parse(d).ok()) else { continue };
        for e in d.root.elements() {
            if tags.contains(&e.local()) {
                let v = e.text_of(&[e.name.as_str()]);
                if !v.trim().is_empty() {
                    notice(out, "metadata", format!("{part} {}", e.local()), v);
                }
            }
        }
    }
    Ok(())
}
