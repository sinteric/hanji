//! `word/styles.xml` → the file's style set.

use hanji_core::{StyleDef, StyleSet};

use crate::xml;

pub fn read(data: Option<&[u8]>) -> StyleSet {
    let mut s = StyleSet::default();
    let mut default_p = None;
    let mut default_t = None;
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
                    s.paragraph.push(StyleDef { id, name });
                }
                Some("table") => {
                    if default && default_t.is_none() {
                        default_t = Some(name.clone());
                    }
                    s.table.push(StyleDef { id, name });
                }
                _ => {}
            }
        }
    }
    let default_p = default_p.unwrap_or_else(|| {
        if !s.paragraph.iter().any(|p| p.id == "Normal") {
            s.paragraph.push(StyleDef { id: "Normal".into(), name: "Normal".into() });
        }
        s.paragraph_name("Normal").unwrap().to_string()
    });
    s.default_paragraph = default_p;
    s.default_table = default_t;
    for p in &s.paragraph {
        let lower = p.name.to_lowercase();
        if let Some(n) = lower.strip_prefix("heading ").and_then(|d| d.parse::<usize>().ok()) {
            if (1..=6).contains(&n) && s.headings[n - 1].is_none() {
                s.headings[n - 1] = Some(p.name.clone());
            }
        }
    }
    s
}
