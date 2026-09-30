//! `word/styles.xml` → the file's style set.

use std::collections::HashMap;

use hanji_core::{StyleDef, StyleSet};

use crate::xml::{self, Element};

/// The file's style set, and the id of the table style `w:default` marks
/// (what a table without `w:tblStyle` is drawn with).
pub fn read(data: Option<&[u8]>) -> (StyleSet, Option<String>) {
    let mut s = StyleSet::default();
    let mut default_p = None;
    let mut default_t = None;
    // Table style id → (draws borders itself, the style it is based on).
    let mut tables: HashMap<String, (bool, Option<String>)> = HashMap::new();
    if let Some(root) = data.and_then(|d| xml::parse(d).ok()).map(|d| d.root) {
        for st in root.elements().filter(|e| e.is("w:style")) {
            let Some(id) = st.get("w:styleId") else { continue };
            let name = st.child("w:name").and_then(|n| n.get("w:val")).unwrap_or_else(|| id.clone());
            let default = matches!(st.get("w:default").as_deref(), Some("1") | Some("true") | Some("on"));
            match st.get("w:type").as_deref() {
                Some("paragraph") => {
                    if default && default_p.is_none() {
                        default_p = Some(name.clone());
                    }
                    s.paragraph.push(StyleDef::new(id, name));
                }
                Some("table") => {
                    if default && default_t.is_none() {
                        default_t = Some(id.clone());
                    }
                    let based = st.child("w:basedOn").and_then(|b| b.get("w:val"));
                    tables.insert(id.clone(), (draws_borders(st), based));
                    s.table.push(StyleDef::new(id, name));
                }
                _ => {}
            }
        }
    }
    let default_p = default_p.unwrap_or_else(|| {
        if !s.paragraph.iter().any(|p| p.id == "Normal") {
            s.paragraph.push(StyleDef::new("Normal", "Normal"));
        }
        s.paragraph_name("Normal").unwrap().to_string()
    });
    s.default_paragraph = default_p;
    // A new table takes the default table style when it draws borders;
    // when it does not (Word's Normal Table), Table Grid, as Word inserts one.
    let bordered = |id: &str| {
        let mut cur = tables.get(id);
        for _ in 0..16 {
            let Some((borders, based)) = cur else { return false };
            if *borders {
                return true;
            }
            cur = based.as_ref().and_then(|b| tables.get(b));
        }
        false
    };
    let name = |id: &str| s.table_name(id).map(str::to_string);
    let default_table = match default_t.as_deref() {
        Some(id) if bordered(id) => name(id),
        d => s.table_id("Table Grid").map(|_| "Table Grid".to_string()).or_else(|| d.and_then(name)),
    };
    s.default_table = default_table;
    for p in &s.paragraph {
        let lower = p.name.to_lowercase();
        if let Some(n) = lower.strip_prefix("heading ").and_then(|d| d.parse::<usize>().ok()) {
            if (1..=6).contains(&n) && s.headings[n - 1].is_none() {
                s.headings[n - 1] = Some(p.name.clone());
            }
        }
    }
    (s, default_t)
}

/// A table style whose own table properties draw a border.
fn draws_borders(st: &Element) -> bool {
    let borders = st.child("w:tblPr").and_then(|p| p.child("w:tblBorders"));
    borders.is_some_and(|b| b.elements().any(|x| !matches!(x.get("w:val").as_deref(), Some("nil" | "none"))))
}
