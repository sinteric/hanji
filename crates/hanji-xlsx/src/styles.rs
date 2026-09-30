//! `styles.xml` as far as the model needs it (§5.1: formatting by name only):
//! the number format of each cell style, and new cell styles that change
//! only the number format of an existing one. Everything else in the part
//! (fonts, fills, borders, cell styles, dxfs, table styles) is copied.

use std::collections::BTreeMap;

use hanji_package::xml::{self, Element, Node};

use crate::numfmt;

#[derive(Clone, Debug, Default)]
pub struct Styles {
    pub part: Option<String>,
    pub(crate) doc: Option<xml::Doc>,
    /// Custom number formats: id → code.
    pub fmts: BTreeMap<u32, String>,
    /// The number format id of each cell style (`cellXfs`, by index).
    pub xf_fmt: Vec<u32>,
    /// dxf index → its number format code, for the dxfs that have one.
    pub dxf_fmt: BTreeMap<u32, String>,
    pub changed: bool,
}

fn prefix_of(root: &Element) -> String {
    root.name.strip_suffix("styleSheet").unwrap_or("").to_string()
}

impl Styles {
    pub fn load(part: Option<&str>, data: Option<&[u8]>) -> Result<Styles, String> {
        let (Some(part), Some(data)) = (part, data) else { return Ok(Styles::default()) };
        let doc = xml::parse(data).map_err(|e| format!("{part}: {e}"))?;
        let mut s = Styles { part: Some(part.into()), ..Default::default() };
        for e in doc.root.elements() {
            match e.local() {
                "numFmts" => {
                    for f in e.elements().filter(|f| f.local() == "numFmt") {
                        if let (Some(id), Some(code)) =
                            (f.get("numFmtId").and_then(|v| v.parse().ok()), f.get("formatCode"))
                        {
                            s.fmts.insert(id, code);
                        }
                    }
                }
                "cellXfs" => {
                    s.xf_fmt = e
                        .elements()
                        .filter(|x| x.local() == "xf")
                        .map(|x| x.get("numFmtId").and_then(|v| v.parse().ok()).unwrap_or(0))
                        .collect();
                }
                "dxfs" => {
                    for (k, d) in e.elements().filter(|x| x.local() == "dxf").enumerate() {
                        if let Some(code) =
                            d.elements().find(|x| x.local() == "numFmt").and_then(|f| f.get("formatCode"))
                        {
                            s.dxf_fmt.insert(k as u32, code);
                        }
                    }
                }
                _ => {}
            }
        }
        s.doc = Some(doc);
        Ok(s)
    }

    /// The code of number format `id` (built-in or the file's own).
    pub fn code(&self, id: u32) -> String {
        self.fmts
            .get(&id)
            .cloned()
            .or_else(|| numfmt::builtin(id).map(str::to_string))
            .unwrap_or_else(|| "General".into())
    }

    /// The number format code of cell style `s`.
    pub fn format_of(&self, s: u32) -> String {
        self.code(self.xf_fmt.get(s as usize).copied().unwrap_or(0))
    }

    /// Every number format code the workbook can name: built-ins and its own.
    pub fn all_codes(&self) -> Vec<String> {
        let mut v: Vec<String> = (0..=49).filter_map(numfmt::builtin).map(str::to_string).collect();
        for c in self.fmts.values() {
            if !v.contains(c) {
                v.push(c.clone());
            }
        }
        v
    }

    /// The id of format `code`, adding it to `numFmts` when the file has no such format.
    pub fn format_id(&mut self, code: &str) -> Result<u32, String> {
        if let Some(id) = self.fmts.iter().find(|(_, c)| *c == code).map(|(k, _)| *k) {
            return Ok(id);
        }
        if let Some(id) = numfmt::builtin_id(code) {
            return Ok(id);
        }
        let id = self.fmts.keys().copied().max().unwrap_or(163).max(163) + 1;
        let doc = self.doc.as_mut().ok_or("the workbook has no styles part")?;
        let p = prefix_of(&doc.root);
        let f =
            Element::new(&format!("{p}numFmt")).with_attr("numFmtId", &id.to_string()).with_attr("formatCode", code);
        if !doc.root.elements().any(|e| e.local() == "numFmts") {
            doc.root.children.insert(0, Node::El(Element::new(&format!("{p}numFmts")).with_attr("count", "0")));
        }
        let list = doc.root.elements_mut().find(|e| e.local() == "numFmts").unwrap();
        list.children.push(Node::El(f));
        let n = list.elements().count();
        list.set("count", &n.to_string());
        self.fmts.insert(id, code.to_string());
        self.changed = true;
        Ok(id)
    }

    /// A cell style like `s` with number format `code` (an existing one when
    /// one differs from `s` in nothing else).
    pub fn with_format(&mut self, s: u32, code: &str) -> Result<u32, String> {
        if self.format_of(s) == code {
            return Ok(s);
        }
        let id = self.format_id(code)?;
        let doc = self.doc.as_mut().ok_or("the workbook has no styles part")?;
        let xfs = doc.root.elements_mut().find(|e| e.local() == "cellXfs").ok_or("styles.xml has no cellXfs")?;
        let list: Vec<Element> = xfs.elements().cloned().collect();
        let base = list.get(s as usize).or(list.first()).cloned().ok_or("styles.xml has no cell style")?;
        let mut want = base.clone();
        want.set("numFmtId", &id.to_string());
        want.set("applyNumberFormat", "1");
        let canon = |e: &Element| xml::canon(e, &xml::Scope::new());
        if let Some(k) = list.iter().position(|x| canon(x) == canon(&want)) {
            return Ok(k as u32);
        }
        xfs.children.push(Node::El(want));
        let n = xfs.elements().count();
        xfs.set("count", &n.to_string());
        self.xf_fmt.push(id);
        self.changed = true;
        Ok(n as u32 - 1)
    }

    /// A dxf holding number format `code` (for a table column's data format).
    pub fn dxf_with_format(&mut self, code: &str) -> Result<u32, String> {
        if let Some(k) = self.dxf_fmt.iter().find(|(_, c)| *c == code).map(|(k, _)| *k) {
            return Ok(k);
        }
        let id = self.format_id(code)?;
        let doc = self.doc.as_mut().ok_or("the workbook has no styles part")?;
        let p = prefix_of(&doc.root);
        let order = [
            "numFmts",
            "fonts",
            "fills",
            "borders",
            "cellStyleXfs",
            "cellXfs",
            "cellStyles",
            "dxfs",
            "tableStyles",
            "colors",
            "extLst",
        ];
        if !doc.root.elements().any(|e| e.local() == "dxfs") {
            xml::insert_ordered(&mut doc.root, Element::new(&format!("{p}dxfs")).with_attr("count", "0"), &order);
        }
        let dxfs = doc.root.elements_mut().find(|e| e.local() == "dxfs").unwrap();
        let mut dxf = Element::new(&format!("{p}dxf"));
        dxf.children.push(Node::El(
            Element::new(&format!("{p}numFmt")).with_attr("numFmtId", &id.to_string()).with_attr("formatCode", code),
        ));
        dxfs.children.push(Node::El(dxf));
        let n = dxfs.elements().count();
        dxfs.set("count", &n.to_string());
        self.dxf_fmt.insert(n as u32 - 1, code.to_string());
        self.changed = true;
        Ok(n as u32 - 1)
    }

    pub fn write(&self) -> Option<Vec<u8>> {
        self.changed.then(|| xml::write_doc(self.doc.as_ref().unwrap()))
    }
}
