//! The workbook as the engine holds it between import, operations and
//! export: the package's parts, the workbook part parsed, the shared strings,
//! the styles, the tables, and each worksheet as a [`Store`] loaded when it
//! is first needed. Parts nothing changed are written back as they were.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use hanji_core::cells::CellRange;
use hanji_core::{Entry, Part};
use hanji_package::opc::{self, Rel};
use hanji_package::package;
use hanji_package::xml::{self, Element, Node};

use crate::numfmt;
use crate::sst::{self, Sst};
use crate::store::{Cell, Store};
use crate::styles::Styles;
use crate::value::CellValue;

pub const MAIN_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
pub const STRICT_NS: &str = "http://purl.oclc.org/ooxml/spreadsheetml/main";
pub const R_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
pub const REL_BASE: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/";
pub const CT_SHEET: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml";
pub const CT_TABLE: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml";
pub const CT_SST: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml";
pub const CT_MAIN: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SheetKind {
    Work,
    Chart,
    /// Dialog sheets and Excel 4 macro sheets.
    Other(String),
}

#[derive(Clone, Debug)]
pub struct SheetInfo {
    pub name: String,
    pub part: String,
    pub rid: String,
    pub kind: SheetKind,
    /// `visible`, `hidden` or `veryHidden`.
    pub state: String,
}

#[derive(Clone, Debug)]
pub struct TableCol {
    pub name: String,
    /// `calculatedColumnFormula`, as the file stores it (no leading `=`).
    pub calc: Option<String>,
    pub dxf: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct TableInfo {
    pub part: String,
    pub sheet: usize,
    pub doc: xml::Doc,
    pub name: String,
    pub range: CellRange,
    pub header: bool,
    pub totals: u32,
    pub cols: Vec<TableCol>,
    pub changed: bool,
    /// A new table: its part is written from scratch.
    pub new: bool,
}

impl TableInfo {
    /// The first and last data row (`first > last` for a table without data rows).
    pub fn data_rows(&self) -> (u32, u32) {
        let first = self.range.first.row + u32::from(self.header);
        (first, self.range.last.row - self.totals)
    }
    pub fn col_index(&self, name: &str) -> Option<usize> {
        self.cols.iter().position(|c| c.name == name)
    }
    pub fn cols_span(&self) -> (u32, u32) {
        (self.range.first.col, self.range.last.col)
    }
}

/// What export needs besides parts and entries, stored once in the remainder.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Shell {
    /// Per worksheet part: the start tags of the containers whose children
    /// became entries (`dataValidations` → `<dataValidations disablePrompts="1">`).
    pub containers: BTreeMap<String, BTreeMap<String, String>>,
    /// Formulas the calculator could not compute: the file asks to be recalculated when opened.
    pub recalc_on_open: bool,
}

pub struct Book {
    pub parts: Vec<Part>,
    pub wb_part: String,
    pub wb: xml::Doc,
    pub wb_changed: bool,
    pub date1904: bool,
    pub sheets: Vec<SheetInfo>,
    pub sst: Option<Sst>,
    sst_new: bool,
    pub styles: Styles,
    pub tables: Vec<TableInfo>,
    stores: Vec<Option<Store>>,
    dirty: Vec<bool>,
    pub entries: Vec<Entry>,
    pub shell: Shell,
    /// Parts added since load: (name, content type).
    added: Vec<(String, String)>,
    /// Parts removed since load.
    gone: BTreeSet<String>,
    /// Relationship parts changed: part → rels.
    rels: BTreeMap<String, Vec<Rel>>,
}

fn parse_part(parts: &[Part], name: &str) -> Result<xml::Doc, String> {
    let d = package::get(parts, name).ok_or_else(|| format!("no {name}"))?;
    xml::parse(d).map_err(|e| format!("{name}: {e}"))
}

impl Book {
    /// The main part of a package (`xl/workbook.xml`).
    pub fn main_part(parts: &[Part]) -> Result<String, String> {
        let root = opc::parse_rels(package::get(parts, "_rels/.rels").ok_or("the package has no _rels/.rels")?);
        root.iter()
            .find(|r| r.short_type() == "officeDocument" && !r.external)
            .map(|r| opc::resolve_target("", &r.target))
            .ok_or_else(|| "the package has no main part".into())
    }

    pub fn load(parts: Vec<Part>, entries: Vec<Entry>, shell: Shell) -> Result<Book, String> {
        let wb_part = Book::main_part(&parts)?;
        let wb = parse_part(&parts, &wb_part)?;
        let ns = wb.root.attrs.iter().find(|a| a.0 == "xmlns" || a.0.starts_with("xmlns:")).map(|a| a.1.clone());
        if wb.root.local() != "workbook" {
            return Err(format!("{wb_part} is not a workbook"));
        }
        if wb.root.attrs.iter().any(|a| a.1 == STRICT_NS) || ns.as_deref() == Some(STRICT_NS) {
            return Err("Strict Open XML workbooks are not supported yet".into());
        }
        let rels = opc::rels_of(&parts, &wb_part);
        let date1904 = wb
            .root
            .elements()
            .find(|e| e.local() == "workbookPr")
            .and_then(|p| p.get("date1904"))
            .is_some_and(|v| v == "1" || v == "true");
        let mut sheets = vec![];
        if let Some(list) = wb.root.elements().find(|e| e.local() == "sheets") {
            for s in list.elements().filter(|e| e.local() == "sheet") {
                let rid = s
                    .attrs
                    .iter()
                    .find(|a| a.0.ends_with(":id") && a.0 != "xml:id")
                    .map(|a| xml::unescape(&a.1))
                    .unwrap_or_default();
                let rel = rels
                    .iter()
                    .find(|r| r.id == rid && !r.external)
                    .ok_or_else(|| format!("sheet {:?} has no part", s.get("name")))?;
                let kind = match rel.short_type() {
                    "worksheet" => SheetKind::Work,
                    "chartsheet" => SheetKind::Chart,
                    other => SheetKind::Other(other.to_string()),
                };
                sheets.push(SheetInfo {
                    name: s.get("name").unwrap_or_default(),
                    part: opc::resolve_target(&wb_part, &rel.target),
                    rid,
                    kind,
                    state: s.get("state").unwrap_or_else(|| "visible".into()),
                });
            }
        }
        let target = |ty: &str| {
            rels.iter().find(|r| r.short_type() == ty && !r.external).map(|r| opc::resolve_target(&wb_part, &r.target))
        };
        let sst = match target("sharedStrings") {
            Some(p) => match package::get(&parts, &p) {
                Some(d) => Some(Sst::parse(&p, d)?),
                None => None,
            },
            None => None,
        };
        let styles_part = target("styles");
        let styles =
            Styles::load(styles_part.as_deref(), styles_part.as_deref().and_then(|p| package::get(&parts, p)))?;
        let mut tables = vec![];
        for (k, s) in sheets.iter().enumerate() {
            if s.kind != SheetKind::Work {
                continue;
            }
            for r in opc::rels_of(&parts, &s.part).iter().filter(|r| r.short_type() == "table" && !r.external) {
                let part = opc::resolve_target(&s.part, &r.target);
                let doc = parse_part(&parts, &part)?;
                tables.push(read_table(part, k, doc)?);
            }
        }
        let n = sheets.len();
        Ok(Book {
            parts,
            wb_part,
            wb,
            wb_changed: false,
            date1904,
            sheets,
            sst,
            sst_new: false,
            styles,
            tables,
            stores: vec![None; n],
            dirty: vec![false; n],
            entries,
            shell,
            added: vec![],
            gone: BTreeSet::new(),
            rels: BTreeMap::new(),
        })
    }

    /// Load worksheet `i` (a no-op when loaded).
    pub fn load_store(&mut self, i: usize) -> Result<(), String> {
        if self.stores[i].is_some() {
            return Ok(());
        }
        let part = &self.sheets[i].part;
        let data = package::get(&self.parts, part).ok_or_else(|| format!("no {part}"))?.to_vec();
        self.stores[i] = Some(Store::load(data).map_err(|e| format!("{part}: {e}"))?);
        Ok(())
    }

    pub fn store(&self, i: usize) -> &Store {
        self.stores[i].as_ref().expect("the sheet is loaded")
    }

    /// Worksheet `i`, loaded, marked changed.
    pub fn store_mut(&mut self, i: usize) -> Result<&mut Store, String> {
        self.load_store(i)?;
        self.dirty[i] = true;
        Ok(self.stores[i].as_mut().unwrap())
    }

    pub fn is_dirty(&self, i: usize) -> bool {
        self.dirty[i]
    }

    pub fn sheet_index(&self, name: &str) -> Option<usize> {
        self.sheets.iter().position(|s| s.name.eq_ignore_ascii_case(name))
    }

    pub fn table_index(&self, name: &str) -> Option<usize> {
        self.tables.iter().position(|t| t.name.eq_ignore_ascii_case(name))
    }

    /// A cell's value (a formula cell: its cached value).
    pub fn value(&self, c: &Cell) -> CellValue {
        let v = c.v.as_deref().map(xml::unescape);
        match c.ty() {
            "s" => v
                .and_then(|i| i.trim().parse::<usize>().ok())
                .and_then(|i| self.sst.as_ref()?.get(i).map(str::to_string))
                .map_or(CellValue::Empty, CellValue::Text),
            "str" => v.map_or(CellValue::Empty, |s| CellValue::Text(sst::decode_xstring(&s))),
            "inlineStr" => c.is.as_ref().map_or(CellValue::Empty, |is| CellValue::Text(sst::rich_text(is))),
            "b" => v.map_or(CellValue::Empty, |s| {
                CellValue::Bool(s.trim() == "1" || s.trim().eq_ignore_ascii_case("true"))
            }),
            "e" => v.map_or(CellValue::Empty, CellValue::Error),
            "d" => v.map_or(CellValue::Empty, CellValue::Text),
            _ => match v {
                Some(s) if !s.trim().is_empty() => {
                    s.trim().parse::<f64>().map_or(CellValue::Text(s), CellValue::Number)
                }
                _ => CellValue::Empty,
            },
        }
    }

    /// A cell as it is displayed.
    pub fn display(&self, c: &Cell) -> String {
        numfmt::display(&self.value(c), &self.styles.format_of(c.style()), self.date1904)
    }

    /// Rels of `part`, as changed so far.
    pub fn rels_of(&self, part: &str) -> Vec<Rel> {
        self.rels.get(part).cloned().unwrap_or_else(|| opc::rels_of(&self.parts, part))
    }

    pub fn set_rels(&mut self, part: &str, rels: Vec<Rel>) {
        self.rels.insert(part.to_string(), rels);
    }

    /// Add a relationship from `source` to `target_part`; its id.
    pub fn add_rel(&mut self, source: &str, ty: &str, target_part: &str) -> String {
        let mut rels = self.rels_of(source);
        let id = opc::free_rel_id(&rels);
        rels.push(Rel {
            id: id.clone(),
            ty: format!("{REL_BASE}{ty}"),
            target: opc::relative_target(source, target_part),
            external: false,
        });
        self.set_rels(source, rels);
        id
    }

    /// Add a part with its content type.
    pub fn add_part(&mut self, name: &str, data: Vec<u8>, ct: &str) {
        let dos_time = self.parts.iter().find(|p| p.name == self.wb_part).map_or(0, |p| p.dos_time);
        self.parts.push(Part { name: name.into(), data, dos_time, external_attr: 0, deflate: true });
        self.added.push((name.into(), ct.into()));
    }

    pub fn remove_part(&mut self, name: &str) {
        self.parts.retain(|p| p.name != name);
        self.gone.insert(name.to_string());
        let rels = opc::rels_part(name);
        if self.parts.iter().any(|p| p.name == rels) {
            self.parts.retain(|p| p.name != rels);
            self.gone.insert(rels);
        }
        self.added.retain(|a| a.0 != name);
    }

    /// A free part name `dir/stemN.xml`.
    pub fn free_part(&self, dir: &str, stem: &str) -> String {
        (1..).map(|k| format!("{dir}/{stem}{k}.xml")).find(|n| !self.parts.iter().any(|p| &p.name == n)).unwrap()
    }

    /// The shared string table, made when the workbook has none.
    pub fn sst_mut(&mut self) -> &mut Sst {
        if self.sst.is_none() {
            let dir = self.wb_part.rsplit_once('/').map_or("", |x| x.0).to_string();
            let part =
                if dir.is_empty() { "sharedStrings.xml".to_string() } else { format!("{dir}/sharedStrings.xml") };
            self.sst = Some(Sst::new(&part));
            self.sst_new = true;
        }
        self.sst.as_mut().unwrap()
    }

    fn set_part(&mut self, name: &str, data: Vec<u8>) {
        match self.parts.iter_mut().find(|p| p.name == name) {
            Some(p) => p.data = data,
            None => self.parts.push(Part { name: name.into(), data, dos_time: 0, external_attr: 0, deflate: true }),
        }
    }

    /// Write every changed part back; the parts, entries and shell of the new revision.
    pub fn finish(mut self) -> (Vec<Part>, Vec<Entry>, Shell) {
        for i in 0..self.sheets.len() {
            if self.dirty[i] {
                if let Some(st) = &self.stores[i] {
                    let data = st.write();
                    let part = self.sheets[i].part.clone();
                    self.set_part(&part, data);
                }
            }
        }
        if let Some(s) = &self.sst {
            if self.sst_new {
                let part = s.part.clone();
                let data = s.write(None);
                self.add_part(&part, data, CT_SST);
                let wb = self.wb_part.clone();
                self.add_rel(&wb, "sharedStrings", &part);
            } else if s.changed() {
                let data = s.write(package::get(&self.parts, &s.part));
                let part = s.part.clone();
                self.set_part(&part, data);
            }
        }
        if let (Some(data), Some(part)) = (self.styles.write(), self.styles.part.clone()) {
            self.set_part(&part, data);
        }
        for k in 0..self.tables.len() {
            if self.tables[k].changed || self.tables[k].new {
                let data = write_table(&mut self.tables[k]);
                let part = self.tables[k].part.clone();
                if self.tables[k].new {
                    self.add_part(&part, data, CT_TABLE);
                } else {
                    self.set_part(&part, data);
                }
            }
        }
        if self.wb_changed {
            let data = xml::write_doc(&self.wb);
            let part = self.wb_part.clone();
            self.set_part(&part, data);
        }
        let rels = std::mem::take(&mut self.rels);
        for (part, rels) in rels {
            let name = opc::rels_part(&part);
            let data = opc::write_rels(&rels);
            if self.parts.iter().any(|p| p.name == name) {
                self.set_part(&name, data);
            } else {
                self.add_part(&name, data, "");
            }
        }
        let added: Vec<(String, String)> = self.added.iter().filter(|a| !a.1.is_empty()).cloned().collect();
        if let Some(ct) = opc::content_types(&self.parts, &self.gone, &added) {
            self.set_part(opc::CT_PART, ct);
        }
        (self.parts, self.entries, self.shell)
    }
}

fn read_table(part: String, sheet: usize, doc: xml::Doc) -> Result<TableInfo, String> {
    let r = &doc.root;
    let range =
        r.get("ref").and_then(|v| CellRange::parse(&v)).ok_or_else(|| format!("{part}: the table has no ref"))?;
    let header = r.get("headerRowCount").is_none_or(|v| v != "0");
    let totals = r.get("totalsRowCount").and_then(|v| v.parse().ok()).unwrap_or(0);
    let cols = r
        .elements()
        .find(|e| e.local() == "tableColumns")
        .map(|tc| {
            tc.elements()
                .filter(|e| e.local() == "tableColumn")
                .map(|c| TableCol {
                    name: sst::decode_xstring(&c.get("name").unwrap_or_default()),
                    calc: c
                        .elements()
                        .find(|x| x.local() == "calculatedColumnFormula")
                        .map(|f| f.text_of(&[f.name.as_str()])),
                    dxf: c.get("dataDxfId").and_then(|v| v.parse().ok()),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if cols.len() as u32 != range.cols() {
        return Err(format!("{part}: the table has {} columns for a range of {}", cols.len(), range.cols()));
    }
    let name = r.get("displayName").or_else(|| r.get("name")).unwrap_or_default();
    Ok(TableInfo { part, sheet, doc, name, range, header, totals, cols, changed: false, new: false })
}

/// The table part for a table's current fields.
fn write_table(t: &mut TableInfo) -> Vec<u8> {
    let p = t.doc.root.name.strip_suffix("table").unwrap_or("").to_string();
    let r = &mut t.doc.root;
    let range = t.range.to_string();
    r.set("ref", &range);
    let filter_ref = if t.totals > 0 {
        CellRange {
            first: t.range.first,
            last: hanji_core::cells::CellRef::new(t.range.last.col, t.range.last.row - t.totals),
        }
        .to_string()
    } else {
        range.clone()
    };
    if let Some(af) = r.elements_mut().find(|e| e.local() == "autoFilter") {
        af.set("ref", &filter_ref);
    }
    let old: Vec<Element> = r
        .elements()
        .find(|e| e.local() == "tableColumns")
        .map(|tc| tc.elements().cloned().collect())
        .unwrap_or_default();
    let mut next_id = old.iter().filter_map(|c| c.get("id").and_then(|v| v.parse::<u32>().ok())).max().unwrap_or(0) + 1;
    let mut cols = vec![];
    for c in &t.cols {
        let mut e = old
            .iter()
            .find(|o| sst::decode_xstring(&o.get("name").unwrap_or_default()) == c.name)
            .cloned()
            .unwrap_or_else(|| {
                let e = Element::new(&format!("{p}tableColumn")).with_attr("id", &next_id.to_string());
                next_id += 1;
                e
            });
        if e.get("name").map(|n| sst::decode_xstring(&n)).as_deref() != Some(c.name.as_str()) {
            e.set("name", &sst::encode_xstring_raw(&c.name));
        }
        match c.dxf {
            Some(d) => e.set("dataDxfId", &d.to_string()),
            None => {
                e.remove_attr("dataDxfId");
            }
        }
        e.children.retain(|n| !matches!(n, Node::El(x) if x.local() == "calculatedColumnFormula"));
        if let Some(f) = &c.calc {
            let mut cf = Element::new(&format!("{p}calculatedColumnFormula"));
            cf.children.push(Node::Text(xml::escape_text(f)));
            e.children.insert(0, Node::El(cf));
        }
        cols.push(Node::El(e));
    }
    if let Some(tc) = r.elements_mut().find(|e| e.local() == "tableColumns") {
        tc.children = cols;
        tc.set("count", &t.cols.len().to_string());
    }
    xml::write_doc(&t.doc)
}
