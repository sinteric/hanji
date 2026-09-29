//! Edits to the structure text (§5.4: the structure is text the model may
//! edit): a column's type, format or formula, a new column at the end of a
//! table, a new table, a new sheet at the end, a deleted placeholder. They
//! become range operations, so the text and the operations share one
//! validator and one implementation. Any other change is refused with the
//! form it should take.

use hanji_format::ops::{ColumnSpec, RangeOp};
use hanji_format::sheet::{SheetItem, Spreadsheet, TableDecl};

/// Whether two structures are the same (front matter aside).
pub fn same(a: &Spreadsheet, b: &Spreadsheet) -> bool {
    a.sheets == b.sheets
}

fn spec(c: &hanji_format::sheet::ColumnDecl) -> ColumnSpec {
    ColumnSpec {
        name: c.name.clone(),
        ty: c.ty.clone(),
        format: Some(c.format.clone()),
        formula: (!c.formula.is_empty()).then(|| c.formula.clone()),
    }
}

fn tables(items: &[SheetItem]) -> Vec<&TableDecl> {
    items.iter().filter_map(|i| if let SheetItem::Table(t) = i { Some(t) } else { None }).collect()
}

/// The operations `new` asks for against `old`, and the placeholders it deleted.
pub fn reconcile(new: &Spreadsheet, old: &Spreadsheet) -> Result<(Vec<RangeOp>, Vec<String>), String> {
    let mut ops = vec![];
    let mut drops = vec![];
    if new.sheets.len() < old.sheets.len() {
        let gone: Vec<&str> = old
            .sheets
            .iter()
            .filter(|s| !new.sheets.iter().any(|n| n.name == s.name))
            .map(|s| s.name.as_str())
            .collect();
        return Err(format!(
            "sheet {} is missing from the text; deleting a sheet is not supported yet — keep its <sheet> block",
            gone.join(", ")
        ));
    }
    for (k, ns) in new.sheets.iter().enumerate() {
        let Some(os) = old.sheets.get(k) else {
            ops.push(RangeOp::AddSheet { name: ns.name.clone() });
            for t in tables(&ns.items) {
                ops.push(new_table(&ns.name, t)?);
            }
            if ns.items.iter().any(|i| matches!(i, SheetItem::Chart(_))) {
                return Err(chart_refusal());
            }
            continue;
        };
        if ns.name != os.name {
            return Err(format!(
                "sheet {} is written as {}; renaming or reordering sheets is not supported yet — keep the sheets as the file has them, in order, and add new ones at the end",
                os.name, ns.name
            ));
        }
        let (nt, ot) = (tables(&ns.items), tables(&os.items));
        for t in &ot {
            if !nt.iter().any(|x| x.name == t.name) {
                return Err(format!(
                    "table {} is missing from the text; deleting a table is not supported yet — keep its <table> block",
                    t.name
                ));
            }
        }
        for t in &nt {
            let Some(o) = ot.iter().find(|x| x.name == t.name) else {
                ops.push(new_table(&ns.name, t)?);
                continue;
            };
            if t.range != o.range {
                return Err(format!("table {}: range=\"{}\" is written as \"{}\"; a table's range is read-only and follows its rows and columns — change them with range operations", t.name, o.range, t.range));
            }
            for (ci, c) in t.columns.iter().enumerate() {
                let Some(oc) = o.columns.get(ci) else {
                    if !c.formula.is_empty() || c.ty != "mixed" {
                        ops.push(RangeOp::AddColumn { table: t.name.clone(), column: spec(c) });
                        continue;
                    }
                    ops.push(RangeOp::AddColumn { table: t.name.clone(), column: spec(c) });
                    continue;
                };
                if c.name != oc.name {
                    return Err(format!("table {}: column {} is written as {}; renaming or reordering columns is not supported yet — new columns go at the end", t.name, oc.name, c.name));
                }
                if c.formula != oc.formula {
                    if c.formula.is_empty() {
                        return Err(format!("table {}: column {} loses its formula; turning a formula column into values is not supported yet", t.name, c.name));
                    }
                    ops.push(RangeOp::FillFormula {
                        table: t.name.clone(),
                        column: c.name.clone(),
                        formula: c.formula.clone(),
                    });
                }
                if c.ty != oc.ty || c.format != oc.format {
                    ops.push(RangeOp::SetType {
                        table: t.name.clone(),
                        column: c.name.clone(),
                        ty: c.ty.clone(),
                        format: Some(c.format.clone()),
                    });
                }
            }
            if t.columns.len() < o.columns.len() {
                return Err(format!(
                    "table {}: column {} is missing; deleting a column is not supported yet",
                    t.name,
                    o.columns[t.columns.len()].name
                ));
            }
        }
        let nk: Vec<&str> = ns
            .items
            .iter()
            .filter_map(|i| if let SheetItem::Keep(k) = i { Some(k.id.as_str()) } else { None })
            .collect();
        for i in &os.items {
            if let SheetItem::Keep(k) = i {
                if !nk.contains(&k.id.as_str()) {
                    drops.push(k.id.clone());
                }
            }
        }
        if ns.items.iter().any(|i| matches!(i, SheetItem::Chart(_))) {
            return Err(chart_refusal());
        }
        if ns.range != os.range && ns.range.is_some() {
            return Err(format!("sheet {}: range=\"{}\" is its used range, which is read-only; write it as the file has it (\"{}\") or leave it out", ns.name, ns.range.clone().unwrap_or_default(), os.range.clone().unwrap_or_default()));
        }
    }
    Ok((ops, drops))
}

fn chart_refusal() -> String {
    "adding a chart with <chart …/> is not supported yet; the workbook's charts are placeholders".into()
}

fn new_table(sheet: &str, t: &TableDecl) -> Result<RangeOp, String> {
    let anchor = t.range.split(':').next().unwrap_or(&t.range).to_string();
    Ok(RangeOp::AddTable {
        sheet: sheet.to_string(),
        name: t.name.clone(),
        anchor,
        columns: t.columns.iter().map(spec).collect(),
        rows: vec![],
    })
}
