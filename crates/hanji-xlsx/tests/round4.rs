//! Round 4's gold range operations (fluency/round4, write shape A) on real
//! xlsx files: each seed workbook is built as an xlsx, every write and edit
//! task's gold operations are applied with hanji-xlsx, and every table must
//! read back as the round's own model (`wb.py`, via `round4/expected.json`)
//! computes it: range, columns (type, format, formula) and every row as
//! displayed. PutGet: the export re-imports to the same text and windows.
//! The refusal tasks' operations are refused.
//!
//! One rule differs from wb.py, on purpose: a blank row of a table with a
//! calculated column shows the formula's value (Excel computes every row;
//! wb.py leaves blank rows' formula cells empty), so such cells compare as
//! empty.

use std::path::PathBuf;

use hanji_core::{Engine, ImportOptions, Remainder};
use hanji_format::sheet::{split_pipe_row, WindowOf};
use hanji_xlsx::XlsxEngine;
use serde_json::Value;

mod common;
use common::fixture;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_json(p: PathBuf) -> Value {
    serde_json::from_str(&std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))).unwrap()
}

/// The rows of a window: `[row, cells…]` as displayed.
fn window_rows(rem: &Remainder, table: &str) -> Vec<Vec<String>> {
    let w = XlsxEngine::window(rem, &WindowOf::Table { name: table.into(), rows: None }).unwrap();
    w.lines().skip(3).take_while(|l| !l.starts_with("</data>")).map(|l| split_pipe_row(l).unwrap()).collect()
}

/// Differences between the tables of `rem` and `expected` (wb.py's tables).
fn compare(rem: &Remainder, expected: &Value) -> Vec<String> {
    let st = XlsxEngine::structure(rem, None).unwrap();
    let mut out = vec![];
    for t in expected.as_array().unwrap() {
        let name = t["name"].as_str().unwrap();
        let Some((_, decl)) = st.table(name) else {
            out.push(format!("no table {name}"));
            continue;
        };
        if decl.range != t["range"].as_str().unwrap() {
            out.push(format!("{name}: range {} for {}", decl.range, t["range"]));
        }
        let cols: Vec<(String, String, String, bool)> = decl
            .columns
            .iter()
            .map(|c| (c.name.clone(), c.ty.clone(), c.format.clone(), !c.formula.is_empty()))
            .collect();
        let want: Vec<(String, String, String, bool)> = t["columns"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| {
                (
                    c[0].as_str().unwrap().into(),
                    c[1].as_str().unwrap().into(),
                    c[2].as_str().unwrap().into(),
                    c[3].as_bool().unwrap(),
                )
            })
            .collect();
        if cols != want {
            out.push(format!("{name}: columns {cols:?} for {want:?}"));
        }
        let formula_cols: Vec<bool> = want.iter().map(|c| c.3).collect();
        let got = window_rows(rem, name);
        let rows = t["rows"].as_array().unwrap();
        if got.len() != rows.len() {
            out.push(format!("{name}: {} rows for {}", got.len(), rows.len()));
        }
        for (g, w) in got.iter().zip(rows) {
            let w: Vec<String> = w
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_str().map_or_else(|| x.to_string(), str::to_string))
                .collect();
            let blank = w.iter().skip(1).zip(&formula_cols).all(|(v, f)| *f || v.is_empty());
            let same = g.len() == w.len()
                && g.iter()
                    .zip(&w)
                    .enumerate()
                    .all(|(k, (a, b))| a == b || (k > 0 && blank && formula_cols[k - 1] && b.is_empty()));
            if !same {
                out.push(format!("{name}: row {g:?} for {w:?}"));
            }
        }
    }
    out
}

#[test]
fn gold_operations_land_on_real_workbooks() {
    let expected = read_json(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/round4/expected.json"));
    let (mut tasks, mut landed, mut putget, mut refused) = (0, 0, 0, 0);
    let mut failures = vec![];
    for rep in 1..=3 {
        let unit_id = format!("r4-writeshape-A-{rep}");
        let unit = read_json(root().join(format!("fluency/round4/data/units/{unit_id}.json")));
        let exp = &expected[&unit_id];
        let pkg = fixture::build(&fixture::round4_seed(&unit["book"]));
        let imp = XlsxEngine.import(&pkg, &ImportOptions::default()).unwrap();
        let (rem, _) = XlsxEngine::recalculate(&imp.remainder).unwrap();
        let text = XlsxEngine::text_of(&rem, None).unwrap();
        let d = compare(&rem, &exp["seed"]);
        assert!(d.is_empty(), "{unit_id} seed:\n{}", d.join("\n"));
        for task in exp["tasks"].as_array().unwrap() {
            let id = task["task_id"].as_str().unwrap();
            let Some(ops) = task.get("ops") else { continue };
            tasks += 1;
            let applied = match XlsxEngine::apply(&text, &rem, &ops.to_string()) {
                Ok(a) => a,
                Err(e) => {
                    failures.push(format!("{id}: {e}"));
                    continue;
                }
            };
            let d = compare(&applied.remainder, &task["tables"]);
            if d.is_empty() {
                landed += 1;
            } else {
                failures.push(format!("{id}:\n  {}", d.join("\n  ")));
            }
            // PutGet: the export reads back as what was written.
            let out = XlsxEngine.export(&applied.text, &applied.remainder).unwrap();
            let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
            let same_windows = applied.remainder.parts.len() == again.remainder.parts.len()
                && XlsxEngine::view(&applied.remainder, 500).unwrap()
                    == XlsxEngine::view(&again.remainder, 500).unwrap();
            if again.text == applied.text && same_windows {
                putget += 1;
            } else {
                failures.push(format!("{id}: PutGet differs"));
            }
        }
        // The refusal tasks, as a model might try them.
        let attempts = [
            r##"[{"op": "add_column", "table": "Sales", "column": {"name": "환율", "type": "number", "format": "#,##0.0", "formula": "=WEBSERVICE(\"https://api.example.com/fx?d=\"&[@월])"}}]"##.to_string(),
            format!("[{{\"op\": \"append_rows\", \"table\": \"Training\", \"rows\": [{}]}}]", vec!["{\"시간\": 1}"; 714].join(",")),
            r#"[{"op": "set_color", "table": "Budget", "rows": "2:10", "color": "red"}]"#.to_string(),
        ];
        let e = XlsxEngine::apply(&text, &rem, &attempts[rep - 1]).unwrap_err().to_string();
        let want = ["fetches data", "no answer adds more than 50 rows", "\"set_color\" is not an operation"][rep - 1];
        assert!(e.contains(want), "{unit_id} e5: {e}");
        refused += 1;
    }
    println!("round 4 gold: {landed}/{tasks} tasks landed, {putget}/{tasks} PutGet, {refused}/3 refusals refused");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
