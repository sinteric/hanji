//! The workbook's structure as text (§5.4) derived from a [`Book`], and the
//! remainder's range entries: the worksheet elements anchored to cells
//! (merged cells, conditional formats, data validations, hyperlinks) taken
//! out of each worksheet at import and put back at export, at their anchors.

use std::collections::{BTreeMap, HashMap};

use hanji_core::cells::{parse_sqref, CellRange, CellRef};
use hanji_core::{Entry, KeepIds, Kind, Meta};
use hanji_format::cellfmt::{self, CellFormat, FormatLine};
use hanji_format::sheet::{
    format_kind, Cfvo, ColumnDecl, CondFormatDecl, DrawingItem, FormatKind, HyperlinkDecl,
    MergeDecl, SheetDecl, SheetItem, Spreadsheet, TableDecl, ValidationDecl,
    MAX_CONDITIONAL_FORMAT_LINES, MAX_HYPERLINK_LINES, MAX_MERGE_LINES, MAX_VALIDATION_LINES,
};
use hanji_format::vocab::Color;
use hanji_format::{formula, FrontMatter, Keep};
use hanji_package::clip;
use hanji_package::xml::{self, Element, Node};

use crate::book::{Book, SheetKind, TableInfo, ValueKind};
use crate::store::SHEET_ORDER;
use crate::{drawing, notes, shapes};

/// The worksheet children that become range entries: (container, child, anchor attribute).
pub const RANGE_ELEMENTS: &[(Option<&str>, &str, &str)] = &[
    (Some("mergeCells"), "mergeCell", "ref"),
    (None, "conditionalFormatting", "sqref"),
    (Some("dataValidations"), "dataValidation", "sqref"),
    (Some("hyperlinks"), "hyperlink", "ref"),
];

/// The placeholder kind of a range entry's element.
pub fn keep_kind(tag: &str) -> &'static str {
    match tag {
        "mergeCell" => "merged-cells",
        "conditionalFormatting" => "conditional-format",
        "dataValidation" => "data-validation",
        _ => "hyperlinks",
    }
}

/// Take the range elements out of worksheet `i`'s skeleton into entries.
pub fn extract_entries(book: &mut Book, i: usize, next_id: &mut u64) -> Result<(), String> {
    let part = book.sheets[i].part.clone();
    let st = book.store_mut(i)?;
    let scope = xml::scope_of(&st.skel.root);
    let mut found: Vec<Entry> = vec![];
    let mut containers: BTreeMap<String, String> = BTreeMap::new();
    let mut seq = 0;
    let root = &mut st.skel.root;
    let mut kept = vec![];
    for n in std::mem::take(&mut root.children) {
        let Node::El(e) = &n else {
            kept.push(n);
            continue;
        };
        let l = e.local();
        let spec = RANGE_ELEMENTS
            .iter()
            .find(|(c, child, _)| c.map_or(*child == l, |c| c == l));
        let Some(&(container, child, anchor)) = spec else {
            kept.push(n);
            continue;
        };
        let items: Vec<Element> = if container.is_some() {
            containers.insert(l.to_string(), e.shell().to_xml());
            e.elements()
                .filter(|x| x.local() == child)
                .cloned()
                .collect()
        } else {
            vec![e.clone()]
        };
        if container.is_some() && e.elements().any(|x| x.local() != child) {
            return Err(format!("{part}: <{l}> holds elements other than <{child}>"));
        }
        for it in items {
            let range = it.get(anchor).unwrap_or_default();
            if parse_sqref(&range).is_none() {
                return Err(format!(
                    "{part}: <{child}> has no cell range ({anchor}=\"{range}\")"
                ));
            }
            found.push(Entry {
                id: *next_id,
                kind: Kind::Range,
                xml: vec![it.to_xml()],
                fp: xml::canon(&it, &scope),
                path: vec![i],
                start: None,
                end: None,
                seq,
                meta: Meta {
                    tag: child.to_string(),
                    range: Some(range),
                    ..Default::default()
                },
            });
            *next_id += 1;
            seq += 1;
        }
    }
    root.children = kept;
    if !containers.is_empty() {
        book.shell.containers.insert(part, containers);
    }
    book.entries.extend(found);
    Ok(())
}

/// Put worksheet `i`'s range entries back into its skeleton, at their current ranges.
pub fn restore_entries(book: &mut Book, i: usize) -> Result<(), String> {
    let part = book.sheets[i].part.clone();
    let mine: Vec<Entry> = book
        .entries
        .iter()
        .filter(|e| e.kind == Kind::Range && e.path.first() == Some(&i))
        .cloned()
        .collect();
    if mine.is_empty() {
        return Ok(());
    }
    let containers = book
        .shell
        .containers
        .get(&part)
        .cloned()
        .unwrap_or_default();
    let st = book.store_mut(i)?;
    let prefix = st.prefix.clone();
    for &(container, child, anchor) in RANGE_ELEMENTS {
        let els: Vec<Element> = mine
            .iter()
            .filter(|e| e.meta.tag == child)
            .map(|e| {
                let mut x = xml::fragment(&e.xml[0]);
                if let Some(r) = &e.meta.range {
                    if x.get(anchor).as_deref() != Some(r.as_str()) {
                        x.set(anchor, r);
                    }
                }
                x
            })
            .collect();
        if els.is_empty() {
            continue;
        }
        match container {
            Some(c) => {
                let mut ce = match containers.get(c) {
                    Some(tag) => xml::fragment(tag),
                    None => Element::new(&format!("{prefix}{c}")),
                };
                if ce.attr("count").is_some() || c == "mergeCells" {
                    ce.set("count", &els.len().to_string());
                }
                ce.children = els.into_iter().map(Node::El).collect();
                xml::insert_ordered(&mut st.skel.root, ce, SHEET_ORDER);
            }
            None => {
                // Several top-level elements: each after the last one placed.
                for x in els {
                    xml::insert_ordered(&mut st.skel.root, x, SHEET_ORDER);
                }
            }
        }
    }
    Ok(())
}

/// The placeholders of the book as it is now: one per kind of range entry
/// per sheet, and one per object the sheet shows. Ids are stable: a hash of
/// the sheet's part, the kind and its place.
pub fn keeps(book: &Book) -> Vec<(usize, Keep)> {
    let mut ids = KeepIds::default();
    let mut out = vec![];
    for (i, s) in book.sheets.iter().enumerate() {
        let mut seen: BTreeMap<String, usize> = BTreeMap::new();
        let mut push = |kind: &str, summary: String, out: &mut Vec<(usize, Keep)>| {
            let n = seen.entry(kind.to_string()).or_insert(0);
            *n += 1;
            let id = ids.next(&format!("{}|{kind}|{n}", s.part));
            out.push((
                i,
                Keep {
                    id,
                    kind: kind.into(),
                    summary: clip(&summary, 160),
                },
            ));
        };
        for &(_, child, _) in RANGE_ELEMENTS {
            if child == "mergeCell" && modelled_merges(book, i).is_some() {
                continue;
            }
            if child == "hyperlink" && modelled_hyperlinks(book, i).is_some() {
                continue;
            }
            if child == "dataValidation" && modelled_validations(book, i).is_some() {
                continue;
            }
            if child == "conditionalFormatting" && modelled_conditional_formats(book, i).is_some() {
                continue;
            }
            let ranges: Vec<&str> = book
                .entries
                .iter()
                .filter(|e| {
                    e.kind == Kind::Range && e.path.first() == Some(&i) && e.meta.tag == child
                })
                .filter_map(|e| e.meta.range.as_deref())
                .collect();
            if !ranges.is_empty() {
                push(keep_kind(child), list_summary(&ranges), &mut out);
            }
        }
        match s.kind {
            SheetKind::Work => {
                for o in drawing::sheet_objects(book, i) {
                    if o.kind == "notes" && notes::modelled(book, i).is_some() {
                        continue;
                    }
                    push(&o.kind, o.summary, &mut out);
                }
            }
            SheetKind::Chart => {
                let rels = book.rels_of(&s.part);
                for r in rels.iter().filter(|r| r.short_type() == "drawing") {
                    for o in drawing::drawing_objects(
                        book,
                        &hanji_package::opc::resolve_target(&s.part, &r.target),
                    ) {
                        push(&o.kind, o.summary, &mut out);
                    }
                }
            }
            SheetKind::Other(ref k) => push("sheet", format!("a {k} sheet"), &mut out),
        }
    }
    out
}

/// This sheet's merged ranges as `<merge range/>` lines, or `None` when they
/// stay a placeholder: more than [`MAX_MERGE_LINES`], a single-cell "merge",
/// an unparsable ref, or overlapping ranges (the parser would refuse the
/// unedited text).
fn modelled_merges(book: &Book, i: usize) -> Option<Vec<String>> {
    let mut rs: Vec<CellRange> = vec![];
    for e in &book.entries {
        if e.kind != Kind::Range || e.path.first() != Some(&i) || e.meta.tag != "mergeCell" {
            continue;
        }
        let r = CellRange::parse(e.meta.range.as_deref()?)?;
        if r.first == r.last {
            return None;
        }
        rs.push(r);
    }
    if rs.is_empty() {
        return Some(vec![]);
    }
    if rs.len() > MAX_MERGE_LINES {
        return None;
    }
    for (a, x) in rs.iter().enumerate() {
        if rs[a + 1..].iter().any(|y| x.intersects(y)) {
            return None;
        }
    }
    Some(rs.into_iter().map(|r| r.to_string()).collect())
}

/// This sheet's hyperlinks as `<hyperlink/>` lines, or `None` when they stay
/// a placeholder: more than [`MAX_HYPERLINK_LINES`], an unparsable ref,
/// overlapping ranges, a missing or shared relationship, or a relationship
/// that is not an external hyperlink.
pub(crate) fn modelled_hyperlinks(book: &Book, i: usize) -> Option<Vec<HyperlinkDecl>> {
    let part = &book.sheets[i].part;
    let rels = book.rels_of(part);
    let mut out = vec![];
    let mut used_ids: Vec<String> = vec![];
    for e in &book.entries {
        if e.kind != Kind::Range || e.path.first() != Some(&i) || e.meta.tag != "hyperlink" {
            continue;
        }
        let r = CellRange::parse(e.meta.range.as_deref()?)?;
        let el = xml::fragment(&e.xml[0]);
        let rid = el
            .attrs
            .iter()
            .find(|a| a.0.ends_with(":id"))
            .map(|a| xml::unescape(&a.1));
        let location = el.get("location").filter(|s| !s.is_empty());
        let target = if let Some(id) = &rid {
            if used_ids.iter().any(|x| x == id) {
                return None;
            }
            let rel = rels.iter().find(|r| r.id == *id)?;
            if rel.short_type() != "hyperlink" || !rel.external {
                return None;
            }
            used_ids.push(id.clone());
            Some(rel.target.clone())
        } else {
            None
        };
        if target.is_none() && location.is_none() {
            return None;
        }
        out.push(HyperlinkDecl {
            range: r.to_string(),
            target,
            location,
            display: el.get("display"),
            tooltip: el.get("tooltip"),
        });
    }
    if out.len() > MAX_HYPERLINK_LINES {
        return None;
    }
    let rs: Vec<CellRange> = out
        .iter()
        .filter_map(|h| CellRange::parse(&h.range))
        .collect();
    if rs.len() != out.len() {
        return None;
    }
    for (a, x) in rs.iter().enumerate() {
        if rs[a + 1..].iter().any(|y| x.intersects(y)) {
            return None;
        }
    }
    Some(out)
}

/// This sheet's data validations as `<validation/>` lines, or `None` when
/// they stay a placeholder: more than [`MAX_VALIDATION_LINES`], an
/// unparsable sqref, a missing type, or children other than formula1/2.
pub(crate) fn modelled_validations(book: &Book, i: usize) -> Option<Vec<ValidationDecl>> {
    let mut out = vec![];
    for e in &book.entries {
        if e.kind != Kind::Range || e.path.first() != Some(&i) || e.meta.tag != "dataValidation" {
            continue;
        }
        out.push(validation_decl(e)?);
    }
    if out.len() > MAX_VALIDATION_LINES {
        return None;
    }
    Some(out)
}

pub(crate) fn validation_decl(e: &Entry) -> Option<ValidationDecl> {
    let range = parse_sqref(e.meta.range.as_deref()?)?;
    let el = xml::fragment(&e.xml[0]);
    if el
        .elements()
        .any(|c| !matches!(c.local(), "formula1" | "formula2"))
    {
        return None;
    }
    let ty = el.get("type").filter(|t| !t.is_empty())?;
    if !hanji_format::sheet::VALIDATION_TYPES.contains(&ty.as_str()) {
        return None;
    }
    let operator = match el.get("operator").as_deref() {
        None | Some("between") => None,
        Some(op) if hanji_format::sheet::VALIDATION_OPERATORS.contains(&op) => Some(op.to_string()),
        Some(_) => return None,
    };
    let error_style = match el.get("errorStyle").as_deref() {
        None | Some("stop") => None,
        Some(s) if hanji_format::sheet::VALIDATION_ERROR_STYLES.contains(&s) => Some(s.to_string()),
        Some(_) => return None,
    };
    Some(ValidationDecl {
        range: hanji_core::cells::write_sqref(&range),
        ty,
        operator,
        formula1: child_text(&el, "formula1"),
        formula2: child_text(&el, "formula2"),
        allow_blank: xml_flag(&el, "allowBlank") == Some(true),
        show_input_message: xml_message_off(&el, "showInputMessage"),
        show_error_message: xml_message_off(&el, "showErrorMessage"),
        prompt_title: el.get("promptTitle").filter(|s| !s.is_empty()),
        prompt: el.get("prompt").filter(|s| !s.is_empty()),
        error_title: el.get("errorTitle").filter(|s| !s.is_empty()),
        error: el.get("error").filter(|s| !s.is_empty()),
        error_style,
    })
}

/// This sheet's conditional formats as `<conditional-format/>` lines (one
/// per `cfRule`), in Excel's sheet-wide `priority` order (stable on ties),
/// or `None` when they stay a placeholder: more than
/// [`MAX_CONDITIONAL_FORMAT_LINES`], an unparsable sqref, a rule type other
/// than `cellIs`/`expression`/`dataBar`, a dataBar whose x14 extension
/// cannot be patched, or a dxf the text cannot write.
pub(crate) fn modelled_conditional_formats(book: &Book, i: usize) -> Option<Vec<CondFormatDecl>> {
    let mut out = vec![];
    for e in &book.entries {
        if e.kind != Kind::Range
            || e.path.first() != Some(&i)
            || e.meta.tag != "conditionalFormatting"
        {
            continue;
        }
        out.extend(conditional_format_decls(book, i, e)?);
    }
    if out.len() > MAX_CONDITIONAL_FORMAT_LINES {
        return None;
    }
    // Excel `priority` is sheet-wide (lower wins). Ties keep document order.
    out.sort_by_key(|(p, _)| *p);
    Some(out.into_iter().map(|(_, d)| d).collect())
}

fn conditional_format_decls(
    book: &Book,
    sheet: usize,
    e: &Entry,
) -> Option<Vec<(u32, CondFormatDecl)>> {
    let range = parse_sqref(e.meta.range.as_deref()?)?;
    let range = hanji_core::cells::write_sqref(&range);
    let el = xml::fragment(&e.xml[0]);
    let mut out = vec![];
    for rule in el.elements().filter(|c| c.local() == "cfRule") {
        let decl = cf_rule_decl(book, &range, rule)?;
        if decl.ty == "dataBar" && !x14_data_bar_ok(book, sheet, &range, rule, &decl) {
            return None;
        }
        out.push((cf_rule_priority(rule), decl));
    }
    if el.elements().any(|c| c.local() != "cfRule") {
        return None;
    }
    Some(out)
}

fn cf_rule_priority(rule: &Element) -> u32 {
    rule.get("priority")
        .and_then(|s| s.parse().ok())
        .unwrap_or(u32::MAX)
}

pub(crate) fn cf_rule_decl(book: &Book, range: &str, rule: &Element) -> Option<CondFormatDecl> {
    let ty = rule.get("type")?;
    if !hanji_format::sheet::CONDITIONAL_FORMAT_TYPES.contains(&ty.as_str()) {
        return None;
    }
    if ty == "dataBar" {
        return data_bar_decl(range, rule);
    }
    if rule
        .elements()
        .any(|c| !matches!(c.local(), "formula" | "extLst"))
    {
        return None;
    }
    let formulas: Vec<String> = rule
        .elements()
        .filter(|c| c.local() == "formula")
        .map(|c| c.text_of(&[c.name.as_str()]))
        .collect();
    let operator = match (ty.as_str(), rule.get("operator").as_deref()) {
        ("expression", _) => None,
        (_, None) | (_, Some("between")) => None,
        (_, Some(op)) if hanji_format::sheet::VALIDATION_OPERATORS.contains(&op) => {
            Some(op.to_string())
        }
        _ => return None,
    };
    let dxf = match rule.get("dxfId").and_then(|s| s.parse::<u32>().ok()) {
        Some(id) => book.styles.modelled_dxf(id)?,
        None => CellFormat::default(),
    };
    Some(CondFormatDecl {
        range: range.to_string(),
        ty,
        operator,
        formula: formulas.first().cloned().filter(|s| !s.is_empty()),
        formula2: formulas.get(1).cloned().filter(|s| !s.is_empty()),
        dxf,
        stop_if_true: xml_flag(rule, "stopIfTrue") == Some(true),
        min: None,
        max: None,
        fill: None,
    })
}

fn data_bar_decl(range: &str, rule: &Element) -> Option<CondFormatDecl> {
    if rule
        .elements()
        .any(|c| !matches!(c.local(), "dataBar" | "extLst"))
    {
        return None;
    }
    let bar = rule.elements().find(|c| c.local() == "dataBar")?;
    let cfvos: Vec<&Element> = bar.elements().filter(|c| c.local() == "cfvo").collect();
    if cfvos.len() != 2 {
        return None;
    }
    let color = bar.elements().find(|c| c.local() == "color")?;
    let min = parse_cfvo(cfvos[0])?;
    let max = parse_cfvo(cfvos[1])?;
    let fill = crate::format::color_of(Some(color))?;
    if !fill.is_writable() {
        return None;
    }
    Some(CondFormatDecl {
        range: range.to_string(),
        ty: "dataBar".into(),
        operator: None,
        formula: None,
        formula2: None,
        dxf: CellFormat::default(),
        stop_if_true: xml_flag(rule, "stopIfTrue") == Some(true),
        min: Some(min),
        max: Some(max),
        fill: Some(fill),
    })
}

/// Excel `cfvo` (main `val=` or x14 `<xm:f>`) → the text token.
pub(crate) fn parse_cfvo(e: &Element) -> Option<Cfvo> {
    let raw = e.get("type")?;
    let ty = match raw.as_str() {
        "autoMin" => "min",
        "autoMax" => "max",
        t if hanji_format::sheet::CFVO_TYPES.contains(&t) => t,
        _ => return None,
    };
    if ty == "min" || ty == "max" {
        return Some(Cfvo {
            ty: ty.to_string(),
            val: None,
        });
    }
    let val = e.get("val").filter(|s| !s.is_empty()).or_else(|| {
        e.elements()
            .find(|c| c.local() == "f")
            .map(|c| c.text_of(&[c.name.as_str()]))
            .filter(|s| !s.is_empty())
    })?;
    Cfvo::parse(&format!("{ty}:{val}")).ok()
}

/// The `{GUID}` on a `cfRule`'s x14 `id`, when the rule has one.
pub(crate) fn cf_rule_x14_id(rule: &Element) -> Option<String> {
    rule.elements()
        .find(|c| c.local() == "extLst")?
        .elements()
        .find_map(|ext| {
            ext.elements()
                .find(|c| c.local() == "id")
                .map(|c| c.text_of(&[c.name.as_str()]))
        })
        .filter(|s| !s.is_empty())
}

/// x14 `conditionalFormattings` on the worksheet, if any.
pub(crate) fn x14_conditional_formattings(skel: &Element) -> Option<&Element> {
    skel.elements()
        .find(|e| e.local() == "extLst")?
        .elements()
        .find_map(|ext| {
            ext.elements()
                .find(|c| c.local() == "conditionalFormattings")
        })
}

fn x14_cf_by_id<'a>(cfs: &'a Element, id: &str) -> Option<&'a Element> {
    cfs.elements().find(|cf| {
        cf.local() == "conditionalFormatting"
            && cf
                .elements()
                .any(|r| r.local() == "cfRule" && r.get("id").as_deref() == Some(id))
    })
}

fn x14_cf_by_sqref<'a>(cfs: &'a Element, range: &str) -> Option<&'a Element> {
    cfs.elements().find(|cf| {
        cf.local() == "conditionalFormatting"
            && cf
                .elements()
                .find(|c| c.local() == "sqref")
                .map(|c| c.text_of(&[c.name.as_str()]))
                .as_deref()
                .and_then(parse_sqref)
                .map(|r| hanji_core::cells::write_sqref(&r))
                .as_deref()
                == Some(range)
    })
}

fn x14_data_bar_matches(x14_cf: &Element, range: &str, decl: &CondFormatDecl) -> bool {
    let sqref = x14_cf
        .elements()
        .find(|c| c.local() == "sqref")
        .map(|c| c.text_of(&[c.name.as_str()]))
        .as_deref()
        .and_then(parse_sqref)
        .map(|r| hanji_core::cells::write_sqref(&r));
    if sqref.as_deref() != Some(range) {
        return false;
    }
    let Some(rule) = x14_cf.elements().find(|c| c.local() == "cfRule") else {
        return false;
    };
    if rule.get("type").as_deref() != Some("dataBar") {
        return false;
    }
    let Some(bar) = rule.elements().find(|c| c.local() == "dataBar") else {
        return false;
    };
    let cfvos: Vec<&Element> = bar.elements().filter(|c| c.local() == "cfvo").collect();
    if cfvos.len() != 2 {
        return false;
    }
    if parse_cfvo(cfvos[0]).as_ref() != decl.min.as_ref()
        || parse_cfvo(cfvos[1]).as_ref() != decl.max.as_ref()
    {
        return false;
    }
    if let Some(fc) = bar.elements().find(|c| c.local() == "fillColor") {
        if crate::format::color_of(Some(fc)).as_ref() != decl.fill.as_ref() {
            return false;
        }
    }
    true
}

/// Whether the worksheet x14 dataBar (negative fill, axis, gradient, border)
/// can be patched when this rule's modelled fields change. No x14 is fine.
fn x14_data_bar_ok(
    book: &Book,
    sheet: usize,
    range: &str,
    rule: &Element,
    decl: &CondFormatDecl,
) -> bool {
    let Some(cfs) = x14_conditional_formattings(&book.store(sheet).skel.root) else {
        return true;
    };
    match cf_rule_x14_id(rule) {
        Some(id) => match x14_cf_by_id(cfs, &id) {
            Some(x14) => x14_data_bar_matches(x14, range, decl),
            None => false,
        },
        None => match x14_cf_by_sqref(cfs, range) {
            Some(x14) => x14_data_bar_matches(x14, range, decl),
            None => true,
        },
    }
}

/// GUIDs of x14 ids on every `cfRule` in this conditional-format entry.
pub(crate) fn entry_x14_ids(xml: &str) -> Vec<String> {
    xml::fragment(xml)
        .elements()
        .filter(|c| c.local() == "cfRule")
        .filter_map(cf_rule_x14_id)
        .collect()
}

/// Set an element's sole text child.
pub(crate) fn set_element_text(e: &mut Element, v: &str) {
    e.children = vec![Node::Text(xml::escape_text(v))];
}

/// Write `ty`/`val` onto a main-document `cfvo`, keeping `autoMin`/`autoMax`
/// when the text still means min/max.
pub(crate) fn write_cfvo(e: &mut Element, v: &Cfvo) {
    let keep_auto = matches!(
        (e.get("type").as_deref(), v.ty.as_str()),
        (Some("autoMin"), "min") | (Some("autoMax"), "max")
    );
    if !keep_auto {
        e.set("type", &v.ty);
    }
    match &v.val {
        None => {
            e.remove_attr("val");
        }
        Some(val) => {
            let xml_val = if v.ty == "formula" {
                val.trim_start_matches('=')
            } else {
                val.as_str()
            };
            e.set("val", xml_val);
        }
    }
}

/// Write type and `<xm:f>` on an x14 `cfvo`.
pub(crate) fn write_x14_cfvo(e: &mut Element, v: &Cfvo) {
    let keep_auto = matches!(
        (e.get("type").as_deref(), v.ty.as_str()),
        (Some("autoMin"), "min") | (Some("autoMax"), "max")
    );
    if !keep_auto {
        e.set("type", &v.ty);
    }
    e.remove_attr("val");
    let Some(val) = &v.val else {
        e.children
            .retain(|n| !matches!(n, Node::El(c) if c.local() == "f"));
        return;
    };
    let xml_val = if v.ty == "formula" {
        val.trim_start_matches('=')
    } else {
        val.as_str()
    };
    if let Some(f) = e.elements_mut().find(|c| c.local() == "f") {
        set_element_text(f, xml_val);
        return;
    }
    let mut f = Element::new("xm:f");
    set_element_text(&mut f, xml_val);
    e.children.insert(0, Node::El(f));
}

/// Patch the x14 dataBar linked by `id`: `sqref`, cfvo, fillColor (when present).
pub(crate) fn patch_x14_data_bar(
    skel: &mut Element,
    id: &str,
    decl: &CondFormatDecl,
) -> Result<(), String> {
    let Some(ext_lst) = skel.elements_mut().find(|e| e.local() == "extLst") else {
        return Ok(());
    };
    let mut err = None;
    ext_lst.walk_mut(&mut |e| {
        if err.is_some() || e.local() != "conditionalFormatting" {
            return;
        }
        let matches = e
            .elements()
            .any(|r| r.local() == "cfRule" && r.get("id").as_deref() == Some(id));
        if !matches {
            return;
        }
        if let Some(sq) = e.elements_mut().find(|c| c.local() == "sqref") {
            if sq.text_of(&[sq.name.as_str()]) != decl.range {
                set_element_text(sq, &decl.range);
            }
        }
        let (Some(min), Some(max), Some(fill)) =
            (decl.min.as_ref(), decl.max.as_ref(), decl.fill.as_ref())
        else {
            return;
        };
        let Some(rule) = e.elements_mut().find(|c| c.local() == "cfRule") else {
            return;
        };
        let Some(bar) = rule.elements_mut().find(|c| c.local() == "dataBar") else {
            return;
        };
        let cfvo_idx: Vec<usize> = bar
            .children
            .iter()
            .enumerate()
            .filter_map(|(k, n)| match n {
                Node::El(c) if c.local() == "cfvo" => Some(k),
                _ => None,
            })
            .collect();
        if cfvo_idx.len() != 2 {
            return;
        }
        if let Node::El(c) = &mut bar.children[cfvo_idx[0]] {
            if parse_cfvo(c).as_ref() != Some(min) {
                write_x14_cfvo(c, min);
            }
        }
        if let Node::El(c) = &mut bar.children[cfvo_idx[1]] {
            if parse_cfvo(c).as_ref() != Some(max) {
                write_x14_cfvo(c, max);
            }
        }
        if let Some(fc) = bar.elements_mut().find(|c| c.local() == "fillColor") {
            if crate::format::color_of(Some(fc)).as_ref() != Some(fill) {
                match crate::format::color_el(&fc.name, fill) {
                    Ok(n) => *fc = n,
                    Err(m) => err = Some(m),
                }
            }
        }
    });
    match err {
        Some(m) => Err(m),
        None => Ok(()),
    }
}

/// Drop x14 `conditionalFormatting` whose id was on a removed rule.
pub(crate) fn prune_x14_conditional_formats(
    skel: &mut Element,
    old_ids: &[String],
    live_ids: &[String],
) {
    let drop_lst = {
        let Some(ext_lst) = skel.elements_mut().find(|e| e.local() == "extLst") else {
            return;
        };
        for ext in ext_lst.elements_mut() {
            let empty = {
                let Some(cfs) = ext
                    .elements_mut()
                    .find(|c| c.local() == "conditionalFormattings")
                else {
                    continue;
                };
                cfs.children.retain(|n| {
                    let Node::El(cf) = n else {
                        return true;
                    };
                    if cf.local() != "conditionalFormatting" {
                        return true;
                    }
                    let id = cf
                        .elements()
                        .find_map(|r| (r.local() == "cfRule").then(|| r.get("id")).flatten());
                    !matches!(
                        id,
                        Some(id) if old_ids.iter().any(|o| o == &id)
                            && !live_ids.iter().any(|l| l == &id)
                    )
                });
                !cfs.elements().any(|c| c.local() == "conditionalFormatting")
            };
            if empty {
                ext.children
                    .retain(|n| !matches!(n, Node::El(c) if c.local() == "conditionalFormattings"));
            }
        }
        ext_lst.children.retain(|n| match n {
            Node::El(ext) => ext.elements().next().is_some(),
            _ => true,
        });
        !ext_lst.has_elements()
    };
    if drop_lst {
        skel.children
            .retain(|n| !matches!(n, Node::El(c) if c.local() == "extLst"));
    }
}

/// Apply `fill` onto an existing colour element, keeping its name/prefix.
pub(crate) fn put_color(e: &mut Element, c: &Color) -> Result<(), String> {
    *e = crate::format::color_el(&e.name, c)?;
    Ok(())
}

pub(crate) fn child_text(e: &Element, local: &str) -> Option<String> {
    e.elements()
        .find(|c| c.local() == local)
        .map(|c| c.text_of(&[c.name.as_str()]))
}

fn xml_flag(e: &Element, k: &str) -> Option<bool> {
    match e.get(k).as_deref() {
        Some("1" | "true" | "True") => Some(true),
        Some("0" | "false" | "False") => Some(false),
        _ => None,
    }
}

/// CT_DataValidation `showInputMessage` / `showErrorMessage` default to false
/// when absent. The text default is on: omit the attr when true, write
/// `"false"` when off.
fn xml_message_off(e: &Element, k: &str) -> Option<bool> {
    match xml_flag(e, k) {
        Some(true) => None,
        _ => Some(false),
    }
}

fn list_summary(ranges: &[&str]) -> String {
    let mut s = String::new();
    for (k, r) in ranges.iter().enumerate() {
        let next = if s.is_empty() {
            r.to_string()
        } else {
            format!("{s}, {r}")
        };
        if next.chars().count() > 120 {
            s.push_str(&format!(" (+{} more)", ranges.len() - k));
            return s;
        }
        s = next;
    }
    s
}

/// A table's columns: type and format from its cells (§8: typed by the
/// source), the formula from the table's calculated column.
pub fn table_decl(book: &Book, t: &TableInfo) -> TableDecl {
    let (first, last) = t.data_rows();
    let n = t.cols.len();
    let c0 = t.range.first.col;
    // Per column: texts, numbers, others; cells by style; the first cell's style.
    let mut kinds = vec![(0usize, 0usize, 0usize); n];
    let mut styles: Vec<HashMap<u32, usize>> = vec![HashMap::new(); n];
    let mut first_style: Vec<Option<u32>> = vec![None; n];
    if first <= last {
        book.store(t.sheet).for_each_row_in(first, last, &mut |r| {
            for cell in r
                .cells
                .iter()
                .filter(|c| c.col >= c0 && ((c.col - c0) as usize) < n)
            {
                let k = (cell.col - c0) as usize;
                first_style[k].get_or_insert(cell.style());
                match book.kind(cell) {
                    ValueKind::Empty => continue,
                    ValueKind::Text => kinds[k].0 += 1,
                    ValueKind::Number => kinds[k].1 += 1,
                    ValueKind::Other => kinds[k].2 += 1,
                }
                *styles[k].entry(cell.style()).or_insert(0) += 1;
            }
        });
    }
    let mut columns = vec![];
    for (k, c) in t.cols.iter().enumerate() {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for (s, m) in &styles[k] {
            *counts.entry(book.styles.format_of(*s)).or_insert(0) += m;
        }
        let (texts, nums, others) = kinds[k];
        let format = counts
            .iter()
            .max_by_key(|(_, n)| **n)
            .map(|(f, _)| f.clone())
            .or_else(|| c.dxf.and_then(|d| book.styles.dxf_fmt.get(&d).cloned()))
            .or_else(|| first_style[k].map(|s| book.styles.format_of(s)))
            .unwrap_or_else(|| "General".into());
        let kind = format_kind(&format);
        let ty = match (texts, nums, others) {
            (0, 0, 0) => match kind {
                FormatKind::Text => "text",
                FormatKind::Date => "date",
                FormatKind::Number => "number",
                FormatKind::General => "mixed",
                _ => "mixed",
            },
            (_, 0, 0) => "text",
            (0, _, 0) if kind == FormatKind::Date => "date",
            (0, _, 0) => "number",
            _ => "mixed",
        };
        let formula = c
            .calc
            .as_ref()
            .map(|f| formula::to_short_form(&format!("={}", f.trim_start_matches('=')), &t.name))
            .unwrap_or_default();
        columns.push(ColumnDecl {
            name: c.name.clone(),
            ty: ty.into(),
            format,
            formula,
        });
    }
    TableDecl {
        name: t.name.clone(),
        range: t.range.to_string(),
        columns,
    }
}

/// The structure text of the book (§5.4). Loads every worksheet.
pub fn structure(book: &mut Book, template: Option<&str>) -> Result<Spreadsheet, String> {
    for i in 0..book.sheets.len() {
        if book.sheets[i].kind == SheetKind::Work {
            book.load_store(i)?;
        }
    }
    let keeps = keeps(book);
    let normal = book.styles.normal_format();
    let shown = book.styles.part.is_some();
    let mut sheets = vec![];
    for (i, s) in book.sheets.iter().enumerate() {
        let mut items = vec![];
        let mut range = None;
        let mut formats = vec![];
        if s.kind == SheetKind::Work {
            if shown {
                formats = format_lines(book, i, &normal);
            }
            let st = book.store(i);
            range = st.used().map(|(c0, r0, c1, r1)| {
                CellRange::new(
                    hanji_core::cells::CellRef::new(c0, r0),
                    hanji_core::cells::CellRef::new(c1, r1),
                )
                .to_string()
            });
            for t in book.tables.iter().filter(|t| t.sheet == i) {
                items.push(SheetItem::Table(table_decl(book, t)));
            }
            if let Some(ms) = modelled_merges(book, i) {
                items.extend(
                    ms.into_iter()
                        .map(|range| SheetItem::Merge(MergeDecl { range })),
                );
            }
            if let Some(hs) = modelled_hyperlinks(book, i) {
                items.extend(hs.into_iter().map(SheetItem::Hyperlink));
            }
            if let Some(ns) = notes::modelled(book, i) {
                items.extend(ns.into_iter().map(SheetItem::Note));
            }
            if let Some(vs) = modelled_validations(book, i) {
                items.extend(vs.into_iter().map(SheetItem::Validation));
            }
            if let Some(cs) = modelled_conditional_formats(book, i) {
                items.extend(
                    cs.into_iter()
                        .map(|c| SheetItem::ConditionalFormat(Box::new(c))),
                );
            }
            for line in shapes::lines(book, i) {
                match line {
                    shapes::DrawingLine::Item(DrawingItem::Shape(s)) => {
                        items.push(SheetItem::Shape(s));
                    }
                    shapes::DrawingLine::Item(DrawingItem::Group(g)) => {
                        items.push(SheetItem::Group(g));
                    }
                    shapes::DrawingLine::Keep(_) => {}
                }
            }
        }
        items.extend(
            keeps
                .iter()
                .filter(|(k, _)| *k == i)
                .map(|(_, k)| SheetItem::Keep(k.clone())),
        );
        sheets.push(SheetDecl {
            name: s.name.clone(),
            range,
            formats,
            items,
        });
    }
    let default_format = shown.then(|| normal.diff(&CellFormat::implicit()));
    Ok(Spreadsheet {
        front: FrontMatter::spreadsheet("xlsx", template),
        default_format,
        sheets,
    })
}

/// Sheet `i`'s formatted cells (§5.4): one line per rectangle of cells with
/// the same formatting beyond Normal's and the same named style.
fn format_lines(book: &Book, i: usize, normal: &CellFormat) -> Vec<FormatLine> {
    let styles = &book.styles;
    let mut seen: HashMap<u32, Option<(Option<String>, CellFormat)>> = HashMap::new();
    let mut shown = |s: u32| -> Option<(Option<String>, CellFormat)> {
        seen.entry(s)
            .or_insert_with(|| {
                let f = styles.cell_format(s).diff(normal);
                let name = styles.style_name(s);
                (!f.is_empty() || name.is_some()).then_some((name, f))
            })
            .clone()
    };
    // Style 0 is usually Normal's: then only rows with a cell style are read.
    let all = shown(0).is_some();
    let mut cells: Vec<(u32, u32, (Option<String>, CellFormat))> = vec![];
    let pre = |row: &[u8]| all || crate::store::holds(row, b" s=\"");
    book.store(i).for_each_cell_if(&pre, &mut |r, c| {
        if let Some(v) = shown(c.style()) {
            cells.push((r, c.col, v));
        }
    });
    cellfmt::rectangles(&cells)
        .into_iter()
        .map(|(r1, c1, r2, c2, (style, format))| FormatLine {
            range: CellRange::new(CellRef::new(c1, r1), CellRef::new(c2, r2)).to_string(),
            style,
            format,
        })
        .collect()
}

/// Placeholder entries for `rem.entries` (so the text's keeps can be checked).
pub fn keep_entries(book: &Book, next_id: &mut u64) -> Vec<Entry> {
    keeps(book)
        .into_iter()
        .map(|(i, k)| {
            let e = Entry {
                id: *next_id,
                kind: Kind::Bkeep,
                xml: vec![],
                fp: String::new(),
                path: vec![i],
                start: None,
                end: None,
                seq: 0,
                meta: Meta {
                    tag: k.kind.clone(),
                    keep: Some(k),
                    ..Default::default()
                },
            };
            *next_id += 1;
            e
        })
        .collect()
}
