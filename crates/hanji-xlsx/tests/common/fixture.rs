//! Workbooks written for the tests: sheets of tables with typed columns,
//! number formats and calculated columns, as Excel lays them out (shared
//! strings, one cell style per number format, a table part per table).

use std::collections::BTreeMap;

use hanji_core::cells::col_letters;
use hanji_core::Part;
use hanji_xlsx::{numfmt, package};

#[derive(Clone, Debug)]
pub enum V {
    Empty,
    Num(f64),
    Text(String),
    /// `YYYY-MM-DD`
    Date(String),
}

#[derive(Clone, Debug)]
pub struct Col {
    pub name: String,
    pub format: String,
    /// The calculated column's formula as the file stores it (`Sales[[#This Row],[a]]*2`).
    pub formula: Option<String>,
}

#[derive(Clone, Debug)]
pub struct TableSpec {
    pub name: String,
    pub col: u32,
    pub row: u32,
    pub cols: Vec<Col>,
    pub rows: Vec<Vec<V>>,
}

#[derive(Clone, Debug)]
pub struct SheetSpec {
    pub name: String,
    pub tables: Vec<TableSpec>,
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn serial(d: &str) -> f64 {
    let p: Vec<u32> = d.split('-').map(|x| x.parse().unwrap()).collect();
    numfmt::serial_of(p[0] as i64, p[1], *p.get(2).unwrap_or(&1), false)
}

pub fn build(sheets: &[SheetSpec]) -> Vec<u8> {
    let mut sst: Vec<String> = vec![];
    let mut sst_at: BTreeMap<String, usize> = BTreeMap::new();
    let mut intern = |s: &str, sst: &mut Vec<String>| -> usize {
        *sst_at.entry(s.to_string()).or_insert_with(|| {
            sst.push(s.to_string());
            sst.len() - 1
        })
    };
    let mut formats: Vec<String> = vec![];
    let style = |f: &str, formats: &mut Vec<String>| -> usize {
        if f == "General" {
            return 0;
        }
        match formats.iter().position(|x| x == f) {
            Some(k) => k + 1,
            None => {
                formats.push(f.to_string());
                formats.len()
            }
        }
    };
    let mut parts: Vec<(String, Vec<u8>)> = vec![];
    let mut tables_xml: Vec<(usize, String, String)> = vec![];
    let mut refs = 0usize;
    let mut table_id = 0;
    for (si, sh) in sheets.iter().enumerate() {
        let mut rows: BTreeMap<u32, Vec<(u32, String)>> = BTreeMap::new();
        let mut max = (0u32, 0u32, 0u32, 0u32);
        let mut first = true;
        for t in &sh.tables {
            for (ci, c) in t.cols.iter().enumerate() {
                let k = intern(&c.name, &mut sst);
                refs += 1;
                rows.entry(t.row).or_default().push((t.col + ci as u32, format!("t=\"s\"><v>{k}</v>")));
            }
            for (ri, r) in t.rows.iter().enumerate() {
                let rr = t.row + 1 + ri as u32;
                for (ci, c) in t.cols.iter().enumerate() {
                    let s = style(&c.format, &mut formats);
                    let sa = if s == 0 { String::new() } else { format!(" s=\"{s}\"") };
                    let col = t.col + ci as u32;
                    if let Some(f) = &c.formula {
                        rows.entry(rr).or_default().push((col, format!("{sa}><f>{}</f>", esc(f))));
                        continue;
                    }
                    let body = match r.get(ci).unwrap_or(&V::Empty) {
                        V::Empty => continue,
                        V::Num(n) => format!("{sa}><v>{n}</v>"),
                        V::Date(d) => format!("{sa}><v>{}</v>", serial(d)),
                        V::Text(x) => {
                            let k = intern(x, &mut sst);
                            refs += 1;
                            format!("{sa} t=\"s\"><v>{k}</v>")
                        }
                    };
                    rows.entry(rr).or_default().push((col, body));
                }
            }
            let (c1, r1) = (t.col + t.cols.len() as u32 - 1, t.row + (t.rows.len() as u32).max(1));
            if first {
                max = (t.col, t.row, c1, r1);
                first = false;
            } else {
                max = (max.0.min(t.col), max.1.min(t.row), max.2.max(c1), max.3.max(r1));
            }
            table_id += 1;
            let range = format!("{}{}:{}{r1}", col_letters(t.col), t.row, col_letters(c1));
            let cols: String = t
                .cols
                .iter()
                .enumerate()
                .map(|(k, c)| match &c.formula {
                    Some(f) => format!("<tableColumn id=\"{}\" name=\"{}\"><calculatedColumnFormula>{}</calculatedColumnFormula></tableColumn>", k + 1, esc(&c.name), esc(f)),
                    None => format!("<tableColumn id=\"{}\" name=\"{}\"/>", k + 1, esc(&c.name)),
                })
                .collect();
            let xml = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<table xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" id=\"{table_id}\" name=\"{n}\" displayName=\"{n}\" ref=\"{range}\" totalsRowShown=\"0\"><autoFilter ref=\"{range}\"/><tableColumns count=\"{}\">{cols}</tableColumns><tableStyleInfo name=\"TableStyleMedium2\" showFirstColumn=\"0\" showLastColumn=\"0\" showRowStripes=\"1\" showColumnStripes=\"0\"/></table>",
                t.cols.len(),
                n = esc(&t.name)
            );
            tables_xml.push((si, format!("xl/tables/table{table_id}.xml"), xml));
        }
        let mut data = String::new();
        for (r, mut cells) in rows {
            cells.sort_by_key(|c| c.0);
            data.push_str(&format!("<row r=\"{r}\">"));
            for (c, body) in cells {
                data.push_str(&format!("<c r=\"{}{r}\"{body}</c>", col_letters(c)));
            }
            data.push_str("</row>");
        }
        let mine: Vec<&(usize, String, String)> = tables_xml.iter().filter(|t| t.0 == si).collect();
        let parts_xml: String = (1..=mine.len()).map(|k| format!("<tablePart r:id=\"rId{k}\"/>")).collect();
        let dim = if first {
            "A1".to_string()
        } else {
            format!("{}{}:{}{}", col_letters(max.0), max.1, col_letters(max.2), max.3)
        };
        let sheet = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><dimension ref=\"{dim}\"/><sheetViews><sheetView workbookViewId=\"0\"/></sheetViews><sheetFormatPr defaultRowHeight=\"15\"/><sheetData>{data}</sheetData><pageMargins left=\"0.7\" right=\"0.7\" top=\"0.75\" bottom=\"0.75\" header=\"0.3\" footer=\"0.3\"/>{}</worksheet>",
            if mine.is_empty() { String::new() } else { format!("<tableParts count=\"{}\">{parts_xml}</tableParts>", mine.len()) }
        );
        parts.push((format!("xl/worksheets/sheet{}.xml", si + 1), sheet.into_bytes()));
        if !mine.is_empty() {
            let rels: String = mine
                .iter()
                .enumerate()
                .map(|(k, t)| format!("<Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/table\" Target=\"../tables/{}\"/>", k + 1, t.1.rsplit('/').next().unwrap()))
                .collect();
            parts.push((format!("xl/worksheets/_rels/sheet{}.xml.rels", si + 1), rels_doc(&rels)));
        }
    }
    for (_, name, xml) in &tables_xml {
        parts.push((name.clone(), xml.clone().into_bytes()));
    }
    let sst_xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<sst xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" count=\"{refs}\" uniqueCount=\"{}\">{}</sst>",
        sst.len(),
        sst.iter().map(|s| format!("<si><t xml:space=\"preserve\">{}</t></si>", esc(s))).collect::<String>()
    );
    let custom: Vec<(usize, String)> = formats
        .iter()
        .enumerate()
        .filter(|(_, f)| numfmt::builtin_id(f).is_none())
        .map(|(k, f)| (164 + k, f.clone()))
        .collect();
    let num_fmts: String =
        custom.iter().map(|(id, f)| format!("<numFmt numFmtId=\"{id}\" formatCode=\"{}\"/>", esc(f))).collect();
    let xfs: String = formats
        .iter()
        .enumerate()
        .map(|(k, f)| {
            format!(
                "<xf numFmtId=\"{}\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyNumberFormat=\"1\"/>",
                numfmt::builtin_id(f).map_or(164 + k, |b| b as usize)
            )
        })
        .collect();
    let styles = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<styleSheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">{}<fonts count=\"1\"><font><sz val=\"11\"/><name val=\"Calibri\"/></font></fonts><fills count=\"2\"><fill><patternFill patternType=\"none\"/></fill><fill><patternFill patternType=\"gray125\"/></fill></fills><borders count=\"1\"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs><cellXfs count=\"{}\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\"/>{xfs}</cellXfs><cellStyles count=\"1\"><cellStyle name=\"Normal\" xfId=\"0\" builtinId=\"0\"/></cellStyles></styleSheet>",
        if custom.is_empty() { String::new() } else { format!("<numFmts count=\"{}\">{num_fmts}</numFmts>", custom.len()) },
        formats.len() + 1
    );
    let sheets_xml: String = sheets
        .iter()
        .enumerate()
        .map(|(k, s)| format!("<sheet name=\"{}\" sheetId=\"{}\" r:id=\"rId{}\"/>", esc(&s.name), k + 1, k + 1))
        .collect();
    let workbook = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><bookViews><workbookView/></bookViews><sheets>{sheets_xml}</sheets><calcPr calcId=\"191029\"/></workbook>"
    );
    let n = sheets.len();
    let mut wb_rels: String = (1..=n)
        .map(|k| format!("<Relationship Id=\"rId{k}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet{k}.xml\"/>"))
        .collect();
    wb_rels.push_str(&format!("<Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings\" Target=\"sharedStrings.xml\"/>", n + 1));
    wb_rels.push_str(&format!("<Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>", n + 2));
    let mut ct = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/><Override PartName=\"/xl/sharedStrings.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml\"/><Override PartName=\"/xl/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml\"/>");
    for k in 1..=n {
        ct.push_str(&format!("<Override PartName=\"/xl/worksheets/sheet{k}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>"));
    }
    for (_, name, _) in &tables_xml {
        ct.push_str(&format!("<Override PartName=\"/{name}\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml\"/>"));
    }
    ct.push_str("</Types>");
    let mut all: Vec<(String, Vec<u8>)> = vec![
        ("[Content_Types].xml".into(), ct.into_bytes()),
        ("_rels/.rels".into(), rels_doc("<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/>")),
        ("xl/workbook.xml".into(), workbook.into_bytes()),
        ("xl/_rels/workbook.xml.rels".into(), rels_doc(&wb_rels)),
        ("xl/sharedStrings.xml".into(), sst_xml.into_bytes()),
        ("xl/styles.xml".into(), styles.into_bytes()),
    ];
    all.extend(parts);
    let parts: Vec<Part> = all
        .into_iter()
        .map(|(name, data)| Part { name, data, dos_time: 0x5821 << 16, external_attr: 0, deflate: true })
        .collect();
    package::write(&parts).unwrap()
}

fn rels_doc(body: &str) -> Vec<u8> {
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">{body}</Relationships>").into_bytes()
}

/// The sheets of a round 4 unit's seed workbook (`fluency/round4/data/units/*.json`).
pub fn round4_seed(book: &serde_json::Value) -> Vec<SheetSpec> {
    let mut out = vec![];
    for s in book["sheets"].as_array().unwrap() {
        let mut tables = vec![];
        for t in s["tables"].as_array().unwrap() {
            let name = t["name"].as_str().unwrap().to_string();
            let anchor = hanji_core::cells::CellRef::parse(t["anchor"].as_str().unwrap()).unwrap();
            let cols: Vec<Col> = t["columns"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| {
                    let f = c["formula"].as_str().unwrap_or("");
                    Col {
                        name: c["name"].as_str().unwrap().to_string(),
                        format: c["format"].as_str().unwrap().to_string(),
                        formula: (!f.is_empty()).then(|| {
                            let x = hanji_format::formula::to_file_form(f, &name);
                            x.trim_start_matches('=').to_string()
                        }),
                    }
                })
                .collect();
            let types: Vec<String> =
                t["columns"].as_array().unwrap().iter().map(|c| c["type"].as_str().unwrap().to_string()).collect();
            let rows = t["rows"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| {
                    r["cells"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .enumerate()
                        .map(|(k, v)| match v {
                            serde_json::Value::Null => V::Empty,
                            serde_json::Value::Number(n) => V::Num(n.as_f64().unwrap()),
                            serde_json::Value::String(s) if types[k] == "date" => V::Date(s.clone()),
                            serde_json::Value::String(s) => V::Text(s.clone()),
                            other => panic!("{other}"),
                        })
                        .collect()
                })
                .collect();
            tables.push(TableSpec { name, col: anchor.col, row: anchor.row, cols, rows });
        }
        out.push(SheetSpec { name: s["name"].as_str().unwrap().to_string(), tables });
    }
    out
}
