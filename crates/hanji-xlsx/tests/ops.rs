//! The op test set (§9): every operation kind, alone and combined, on corpus
//! workbooks (and two built to be refused), with cell-level outcomes:
//! every untouched cell's value and style stay byte for byte (at the row it
//! moved to), every range entry lands or is reported removed, the refused
//! operations name their reason, the export re-imports to the text and
//! windows written (PutGet), and every part is well-formed. HANJI_SOFFICE=1
//! also converts every export with LibreOffice and compares its values with
//! the windows'. HANJI_REPORT=1 prints per-case numbers.

use std::collections::HashMap;
use std::path::PathBuf;

use hanji_core::cells::{CellRef, RowShift};
use hanji_core::{Engine, ImportOptions, Remainder};
use hanji_format::sheet::WindowOf;
use hanji_xlsx::book::{Book, SheetKind};
use hanji_xlsx::{package, xml, XlsxEngine};

mod common;
use common::*;

/// Which original cells an operation may change or remove, and where the others go.
#[derive(Clone)]
enum Map {
    /// These cells (sheet, `A1`) change; nothing moves.
    Cells(&'static str, &'static [&'static str]),
    /// Rows of a column span move on a sheet.
    Shift(&'static str, RowShift),
    /// A column's data rows change (`fill_formula`, `set_type`): sheet, column, rows.
    Column(&'static str, u32, u32, u32),
    /// A block is permuted (`sort`): sheet, columns, rows.
    Block(&'static str, (u32, u32), (u32, u32)),
    /// Only new cells (`add_table`, `add_sheet`, `add_column` into empty cells).
    Nothing,
    /// Several operations: only PutGet, entries and well-formedness are checked.
    Many,
}

struct Case {
    name: &'static str,
    file: &'static str,
    ops: &'static str,
    /// `Ok(map)` or `Err(substring of the refusal)`.
    expect: Result<Map, &'static str>,
    /// Substrings the windows must show afterwards, (table or sheet!range, text),
    /// or ("notice", kind) for a notice the report must carry.
    shows: &'static [(&'static str, &'static str)],
}

fn cases() -> Vec<Case> {
    use Map::*;
    let ins = |at, n| RowShift::Insert { cols: (0, 5), at, n };
    vec![
        Case {
            name: "set a number",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "set", "range": "매출!D5", "values": [[18420000]]}]"##,
            expect: Ok(Cells("매출", &["D5"])),
            shows: &[("Sales", "| 5 | 2026-01 | 강남 | 모바일 | 18,420,000 | 6,480,000 | 11,940,000 |")],
        },
        Case {
            name: "set an ID as text",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "set", "range": "담당자!A3", "values": [["00417"]]}]"##,
            expect: Ok(Cells("담당자", &["A3"])),
            shows: &[("Staff", "| 3 | 00417 | 이민준 |")],
        },
        Case {
            name: "set a date",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "set", "range": "매출!A6", "values": [["2026-02"]]}]"##,
            expect: Ok(Cells("매출", &["A6"])),
            shows: &[("Sales", "| 6 | 2026-02 | 강남 | 생활 |")],
        },
        Case {
            name: "set two cells outside tables",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "set", "range": "요약!A7:B7", "values": [["메모", 12]]}]"##,
            expect: Ok(Cells("요약", &["A7", "B7"])),
            shows: &[("요약!A7:B7", "| 7 | 메모 | 12 |")],
        },
        Case {
            name: "text starting with = stays text",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "set", "range": "매출!H3", "values": [["=SUM(A1:A3)"]]}]"##,
            expect: Ok(Cells("매출", &["H3"])),
            shows: &[("매출!H3", "| 3 | =SUM(A1:A3) |"), ("notice", "formula-text")],
        },
        Case {
            name: "set into a merged area",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "set", "range": "매출!B1", "values": [["x"]]}]"##,
            expect: Err("covered by the merged cells A1:F1"),
            shows: &[],
        },
        Case {
            name: "set breaking a validation",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "set", "range": "매출!B5", "values": [["부산"]]}]"##,
            expect: Err("breaks the data validation of B4:B48 (one of 강남, 서초, 송파, 분당, 일산)"),
            shows: &[],
        },
        Case {
            name: "set within a validation",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "set", "range": "매출!B5", "values": [["서초"]]}]"##,
            expect: Ok(Cells("매출", &["B5"])),
            shows: &[("Sales", "| 5 | 2026-01 | 서초 | 모바일 |")],
        },
        Case {
            name: "set a formula column",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "set", "range": "매출!F5", "values": [[1]]}]"##,
            expect: Err("formula column"),
            shows: &[],
        },
        Case {
            name: "set a number where text is due",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "set", "range": "담당자!A3", "values": [[417]]}]"##,
            expect: Err("is a text column; write 417 as a string"),
            shows: &[],
        },
        Case {
            name: "append rows",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "append_rows", "table": "Sales", "rows": [{"월": "2026-04", "지점": "강남", "제품군": "가전", "매출": 12400000, "원가": 9000000}, {"월": "2026-04", "지점": "서초", "제품군": "가전", "매출": 11000000, "원가": 8000000}]}]"##,
            expect: Ok(Shift("매출", ins(49, 2))),
            shows: &[("Sales", "| 50 | 2026-04 | 서초 | 가전 | 11,000,000 | 8,000,000 | 3,000,000 |")],
        },
        Case {
            name: "append above a totals row",
            file: "table-sample.xlsx",
            ops: r##"[{"op": "append_rows", "table": "Tabelle1", "rows": [{"Field 1": "new", "Field 2": 2, "Field 3": 3}]}]"##,
            expect: Ok(Shift("Tabelle1", RowShift::Insert { cols: (2, 6), at: 9, n: 1 })),
            shows: &[("Tabelle1", "| 9 | new | 2 | 3 | 5 |")],
        },
        Case {
            name: "insert rows",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "insert_rows", "table": "Staff", "before": 3, "rows": [{"사번": "00417", "이름": "홍길동", "지점": "강남", "휴대전화": "010-1111-2222", "입사일": "2026-09-01"}]}]"##,
            expect: Ok(Shift("담당자", RowShift::Insert { cols: (0, 4), at: 3, n: 1 })),
            shows: &[("Staff", "| 3 | 00417 | 홍길동 | 강남 | 010-1111-2222 | 2026-09-01 |")],
        },
        Case {
            name: "insert rows under a chart's data",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "insert_rows", "table": "Sales", "before": 10, "rows": [{"월": "2026-01", "지점": "송파", "제품군": "기타", "매출": 1000000, "원가": 500000}]}]"##,
            expect: Ok(Shift("매출", ins(10, 1))),
            shows: &[("Sales", "| 10 | 2026-01 | 송파 | 기타 | 1,000,000 | 500,000 | 500,000 |")],
        },
        Case {
            name: "delete rows",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "delete_rows", "table": "Sales", "rows": "6:7"}]"##,
            expect: Ok(Shift("매출", RowShift::Delete { cols: (0, 5), first: 6, last: 7 })),
            shows: &[("Sales", "| 6 | 2026-01 | 서초 | 모바일 |")],
        },
        Case {
            name: "delete rows below a merge",
            file: "simple-monthly-budget.xlsx",
            ops: r##"[{"op": "delete_rows", "table": "tblIncome", "rows": "6"}]"##,
            expect: Ok(Shift("Simple Monthly Budget", RowShift::Delete { cols: (1, 2), first: 6, last: 6 })),
            shows: &[],
        },
        Case {
            name: "fill a formula",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "fill_formula", "table": "Sales", "column": "이익", "formula": "=ROUND(([@매출]-[@원가])/1000,0)*1000"}]"##,
            expect: Ok(Column("매출", 5, 4, 48)),
            shows: &[("Sales", "| 4 | 2026-01 | 강남 | 가전 | 10,269,000 | 7,410,000 | 2,859,000 |")],
        },
        Case {
            name: "fill a formula over A1 cells",
            file: "ExcelTables.xlsx",
            ops: r##"[{"op": "fill_formula", "table": "TableName", "column": "AgeGroup", "formula": "=ROUNDDOWN([@Age]/10,0)*10"}]"##,
            expect: Ok(Column("ExcelTable", 8, 2, 4)),
            shows: &[("TableName", "| 2 | Anton | 44 | 40 |")],
        },
        Case {
            name: "set a type: ID text to number keeps its display",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "set_type", "table": "Staff", "column": "사번", "type": "number", "format": "00000"}]"##,
            expect: Ok(Column("담당자", 0, 2, 9)),
            shows: &[("Staff", "| 5 | 08024 | 최하윤 |")],
        },
        Case {
            name: "set a format",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "set_type", "table": "Sales", "column": "매출", "type": "number", "format": "#,##0.0"}]"##,
            expect: Ok(Column("매출", 3, 4, 48)),
            shows: &[("Sales", "| 4 | 2026-01 | 강남 | 가전 | 10,269,000.0 |")],
        },
        Case {
            name: "add a column",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "add_column", "table": "Staff", "column": {"name": "이메일", "type": "text", "format": "@"}}, {"op": "set", "range": "담당자!F2", "values": [["kim@example.com"]]}]"##,
            expect: Ok(Cells("담당자", &["F1", "F2"])),
            shows: &[("Staff", "| 2 | 18867 | 김서연 | 강남 | 010-5934-8996 | 2019-01-02 | kim@example.com |")],
        },
        Case {
            name: "add a formula column",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "add_column", "table": "Sales", "column": {"name": "이익률", "type": "number", "format": "0.0%", "formula": "=[@이익]/[@매출]"}}]"##,
            expect: Ok(Nothing),
            shows: &[("Sales", "| 4 | 2026-01 | 강남 | 가전 | 10,269,000 | 7,410,000 | 2,859,000 | 27.8% |")],
        },
        Case {
            name: "add a column to a writer-made table",
            file: "rx_table05.xlsx",
            ops: r##"[{"op": "add_column", "table": "Table1", "column": {"name": "x", "type": "text"}}]"##,
            expect: Ok(Nothing),
            shows: &[],
        },
        Case {
            name: "sort",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "sort", "table": "Staff", "keys": [{"column": "이름", "order": "asc"}]}]"##,
            expect: Ok(Block("담당자", (0, 4), (2, 9))),
            shows: &[("Staff", "| 2 | 34711 | 강서준 |")],
        },
        Case {
            name: "sort by a number, descending",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "sort", "table": "Sales", "keys": [{"column": "매출", "order": "desc"}]}]"##,
            expect: Ok(Block("매출", (0, 5), (4, 48))),
            shows: &[],
        },
        Case {
            name: "add a table",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "add_table", "sheet": "요약", "name": "Targets", "anchor": "D1", "columns": [{"name": "지점", "type": "text", "format": "@"}, {"name": "목표", "type": "number", "format": "#,##0"}, {"name": "달성률", "type": "number", "format": "0.0%", "formula": "=SUMIFS(Sales[매출],Sales[지점],[@지점])/[@목표]"}], "rows": [{"지점": "강남", "목표": 100000000}, {"지점": "서초", "목표": 120000000}]}]"##,
            expect: Ok(Nothing),
            shows: &[("Targets", "| 2 | 강남 | 100,000,000 | 90.0% |")],
        },
        Case {
            name: "add a sheet and a table on it",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "add_sheet", "name": "메모"}, {"op": "add_table", "sheet": "메모", "name": "Notes", "anchor": "A1", "columns": [{"name": "날짜", "type": "date", "format": "yyyy-mm-dd"}, {"name": "내용", "type": "text"}], "rows": [{"날짜": "2026-09-29", "내용": "첫 메모"}]}]"##,
            expect: Ok(Nothing),
            shows: &[("Notes", "| 2 | 2026-09-29 | 첫 메모 |")],
        },
        Case {
            name: "append to a pivot table's source",
            file: "ExcelPivotTableSample.xlsx",
            ops: r##"[{"op": "append_rows", "table": "Tabelle1", "rows": [{"Field1": "w", "Field2": 44, "Field3": "2026-11-14"}]}]"##,
            expect: Ok(Shift("Tabelle1", RowShift::Insert { cols: (4, 6), at: 5, n: 1 })),
            shows: &[("Tabelle1", "| 5 | w | 44 | 14-Nov |"), ("notice", "pivot-refresh")],
        },
        Case {
            name: "delete the only rows of a format and a validation",
            file: "@gone",
            ops: r##"[{"op": "delete_rows", "table": "T", "rows": "3"}]"##,
            expect: Ok(Shift("S", RowShift::Delete { cols: (0, 1), first: 3, last: 3 })),
            shows: &[("T", "| 3 | 3 | 30 |"), ("notice", "removed")],
        },
        Case {
            name: "set within a list validation over cells",
            file: "@dv",
            ops: r##"[{"op": "set", "range": "S!A2", "values": [[2]]}]"##,
            expect: Ok(Cells("S", &["A2"])),
            shows: &[("T", "| 2 | 2 | 10 |")],
        },
        Case {
            name: "set breaking a list validation over cells",
            file: "@dv",
            ops: r##"[{"op": "set", "range": "S!A2", "values": [[7]]}]"##,
            expect: Err("breaks the data validation of A2:A5 (a value of $D$10:$D$12)"),
            shows: &[],
        },
        Case {
            name: "set under a validation that cannot be checked",
            file: "@dv",
            ops: r##"[{"op": "set", "range": "S!B2", "values": [[5]]}]"##,
            expect: Err("the data validation of B2:B3 cannot be checked (it is a custom rule)"),
            shows: &[],
        },
        Case {
            name: "set under an unchecked validation that only warns",
            file: "@dv",
            ops: r##"[{"op": "set", "range": "S!B4", "values": [[5]]}]"##,
            expect: Ok(Cells("S", &["B4"])),
            shows: &[("T", "| 4 | 3 | 5 |"), ("notice", "unchecked-validation")],
        },
        Case {
            name: "a table under a merge that reaches out",
            file: "@merge",
            ops: r##"[{"op": "insert_rows", "table": "T", "before": 3, "rows": [{"a": 1}]}]"##,
            expect: Err("the merged cells B8:D8 on sheet S reaches outside columns A–B"),
            shows: &[],
        },
        Case {
            name: "a table above an array formula that reaches out",
            file: "@array",
            ops: r##"[{"op": "delete_rows", "table": "T", "rows": "3"}]"##,
            expect: Err("the array formula at B8:C8"),
            shows: &[],
        },
        Case {
            name: "a formula that fetches",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "fill_formula", "table": "Sales", "column": "이익", "formula": "=WEBSERVICE(\"https://x.example/?q=\"&[@매출])"}]"##,
            expect: Err("fetches data"),
            shows: &[],
        },
        Case {
            name: "A1 references in a formula",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "fill_formula", "table": "Sales", "column": "이익", "formula": "=D4-E4"}]"##,
            expect: Err("A1 reference D4"),
            shows: &[],
        },
        Case {
            name: "a value set, then its row moved by a delete",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "set", "range": "매출!D10", "values": [[99000000]]}, {"op": "delete_rows", "table": "Sales", "rows": "4:5"}]"##,
            expect: Ok(Many),
            shows: &[("Sales", "| 8 | 2026-01 | 송파 | 가전 | 99,000,000 | 9,900,000 | 89,100,000 |")],
        },
        Case {
            name: "every kind at once",
            file: "korean-sales.xlsx",
            ops: r##"[{"op": "set", "range": "매출!D5", "values": [[18420000]]},
            {"op": "append_rows", "table": "Sales", "rows": [{"월": "2026-04", "지점": "강남", "제품군": "가전", "매출": 12400000, "원가": 9000000}]},
            {"op": "insert_rows", "table": "Staff", "before": 3, "rows": [{"사번": "00417", "이름": "홍길동", "지점": "강남"}]},
            {"op": "delete_rows", "table": "Sales", "rows": "6:7"},
            {"op": "fill_formula", "table": "Sales", "column": "이익", "formula": "=[@매출]-[@원가]"},
            {"op": "set_type", "table": "Staff", "column": "입사일", "type": "text"},
            {"op": "add_column", "table": "Sales", "column": {"name": "비고", "type": "text"}},
            {"op": "sort", "table": "Staff", "keys": [{"column": "사번"}]},
            {"op": "add_sheet", "name": "계획"},
            {"op": "add_table", "sheet": "계획", "name": "Plan", "anchor": "B2", "columns": [{"name": "지점", "type": "text"}, {"name": "합계", "type": "number", "format": "#,##0", "formula": "=SUMIFS(Sales[매출],Sales[지점],[@지점])"}], "rows": [{"지점": "강남"}]}]"##,
            expect: Ok(Many),
            shows: &[("Plan", "| 3 | 강남 | 105,663,000 |"), ("Staff", "| 2 | 00417 | 홍길동 | 강남 |  |  |")],
        },
    ]
}

/// Each worksheet's cells: (sheet, row, col) → (is formula, type, value and inline string, style).
fn cells(rem: &Remainder) -> HashMap<(String, u32, u32), (bool, String, u32)> {
    let mut book = Book::load(rem.parts.clone(), vec![], Default::default()).unwrap();
    let mut out = HashMap::new();
    for i in 0..book.sheets.len() {
        if book.sheets[i].kind != SheetKind::Work {
            continue;
        }
        book.load_store(i).unwrap();
        let name = book.sheets[i].name.clone();
        book.store(i).for_each_cell(&mut |r, c| {
            let body = format!("{}|{:?}|{:?}", c.ty(), c.v, c.is.as_ref().map(|x| x.to_xml()));
            out.insert((name.clone(), r, c.col), (c.f.is_some(), body, c.style()));
        });
    }
    out
}

/// Small workbooks for what the corpus lacks: a table under a merge or above
/// an array formula reaching outside its columns (refused), a format and a
/// validation on one row (removed with it), validations of each kind.
fn fixture(which: &str) -> Vec<u8> {
    use common::fixture::{build, Col, SheetSpec, TableSpec, V};
    let t = TableSpec {
        name: "T".into(),
        col: 0,
        row: 1,
        cols: vec![
            Col { name: "a".into(), format: "0".into(), formula: None },
            Col { name: "b".into(), format: "0".into(), formula: None },
        ],
        rows: (1..=4).map(|k| vec![V::Num(k as f64), V::Num(10.0 * k as f64)]).collect(),
    };
    let pkg = build(&[SheetSpec { name: "S".into(), tables: vec![t] }]);
    let mut parts = package::read(&pkg).unwrap();
    let sheet = parts.iter_mut().find(|p| p.name == "xl/worksheets/sheet1.xml").unwrap();
    let mut s = String::from_utf8(sheet.data.clone()).unwrap();
    match which {
        "@gone" => s = s.replace("</sheetData>", "</sheetData><conditionalFormatting sqref=\"A3:B3\"><cfRule type=\"cellIs\" priority=\"1\" operator=\"greaterThan\"><formula>5</formula></cfRule></conditionalFormatting><dataValidations count=\"1\"><dataValidation type=\"whole\" sqref=\"A3\"><formula1>0</formula1></dataValidation></dataValidations>"),
        "@dv" => s = s.replace("</sheetData>", "<row r=\"10\"><c r=\"D10\"><v>1</v></c></row><row r=\"11\"><c r=\"D11\"><v>2</v></c></row><row r=\"12\"><c r=\"D12\"><v>3</v></c></row></sheetData><dataValidations count=\"3\"><dataValidation type=\"list\" allowBlank=\"1\" sqref=\"A2:A5\"><formula1>$D$10:$D$12</formula1></dataValidation><dataValidation type=\"custom\" sqref=\"B2:B3\"><formula1>ISNUMBER(B2)</formula1></dataValidation><dataValidation type=\"custom\" errorStyle=\"warning\" sqref=\"B4:B5\"><formula1>ISNUMBER(B4)</formula1></dataValidation></dataValidations>"),
        "@merge" => s = s.replace("</sheetData>", "<row r=\"8\"><c r=\"B8\"><v>1</v></c></row></sheetData><mergeCells count=\"1\"><mergeCell ref=\"B8:D8\"/></mergeCells>"),
        _ => s = s.replace("</sheetData>", "<row r=\"8\"><c r=\"B8\"><f t=\"array\" ref=\"B8:C8\">A2:B2*2</f><v>2</v></c><c r=\"C8\"><v>20</v></c></row></sheetData>"),
    }
    sheet.data = s.into_bytes();
    package::write(&parts).unwrap()
}

fn a1(s: &str) -> CellRef {
    CellRef::parse(s).unwrap()
}

#[derive(Default, Debug)]
#[cfg_attr(target_os = "wasi", allow(dead_code))] // the LibreOffice counts
struct Tally {
    cases: usize,
    applied: usize,
    refused: usize,
    putget: usize,
    cells_checked: usize,
    cells_same: usize,
    formula_styles_same: usize,
    formula_cells: usize,
    entries_landed: usize,
    entries_removed: usize,
    entries_lost: usize,
    entries_kept_on_refusal: usize,
    soffice_same: usize,
    soffice_total: usize,
}

#[test]
fn op_set() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus");
    let mut t = Tally::default();
    let mut failures: Vec<String> = vec![];
    #[cfg(not(target_os = "wasi"))]
    let soffice = std::env::var("HANJI_SOFFICE").is_ok();
    for case in cases() {
        t.cases += 1;
        let bytes =
            if case.file.starts_with('@') { fixture(case.file) } else { std::fs::read(dir.join(case.file)).unwrap() };
        let imp = XlsxEngine.import(&bytes, &ImportOptions::default()).unwrap();
        let (rem, _) = XlsxEngine::recalculate(&imp.remainder).unwrap();
        let text = XlsxEngine::text_of(&rem, None).unwrap();
        let fail = |m: String| format!("{}: {m}", case.name);
        let res = XlsxEngine::apply(&text, &rem, case.ops);
        let map = match (&case.expect, res) {
            (Err(want), Err(e)) => {
                t.refused += 1;
                if !e.to_string().contains(want) {
                    failures.push(fail(format!("refused, but not for {want:?}: {e}")));
                }
                t.entries_kept_on_refusal += rem.entries.iter().filter(|e| e.kind == hanji_core::Kind::Range).count();
                continue;
            }
            (Err(want), Ok(_)) => {
                failures.push(fail(format!("applied; expected a refusal ({want})")));
                continue;
            }
            (Ok(_), Err(e)) => {
                failures.push(fail(format!("refused: {e}")));
                continue;
            }
            (Ok(m), Ok(a)) => (m.clone(), a),
        };
        let (map, applied) = map;
        t.applied += 1;
        // The windows show what was written.
        for (target, want) in case.shows {
            if *target == "notice" {
                if !applied.report.notices.iter().any(|n| n.kind == *want) {
                    failures.push(fail(format!("no {want} notice")));
                }
                continue;
            }
            let of = match target.split_once('!') {
                Some((s, r)) => WindowOf::Range { sheet: s.into(), range: r.into() },
                None => WindowOf::Table { name: target.to_string(), rows: None },
            };
            let w = XlsxEngine::window(&applied.remainder, &of).unwrap();
            if !w.contains(want) {
                failures.push(fail(format!("window {target} lacks {want:?}:\n{w}")));
            }
        }
        // Entries: every one lands or is reported removed.
        let removed: Vec<&str> =
            applied.report.notices.iter().filter(|n| n.kind == "removed").map(|n| n.location.as_str()).collect();
        let gone = applied.report.entries.iter().filter(|e| e.after.is_none()).count();
        for e in &applied.report.entries {
            let at = format!("{}!{}", e.sheet, e.before);
            match &e.after {
                Some(_) => t.entries_landed += 1,
                // After several operations an entry may have moved before it went.
                None if removed.contains(&at.as_str()) || (matches!(map, Map::Many) && removed.len() >= gone) => {
                    t.entries_removed += 1
                }
                None => {
                    t.entries_lost += 1;
                    failures.push(fail(format!("entry {} {} {} went without a report", e.id, e.tag, e.before)));
                }
            }
        }
        // Untouched cells: byte for byte at the row they moved to.
        let (before, after) = (cells(&rem), cells(&applied.remainder));
        for ((sheet, r, c), (is_f, body, style)) in &before {
            let target: Option<(u32, bool)> = match &map {
                Map::Many => None,
                Map::Nothing => Some((*r, true)),
                Map::Cells(s, list) => Some((*r, !(s == sheet && list.iter().any(|x| a1(x) == CellRef::new(*c, *r))))),
                Map::Shift(s, sh) => {
                    let s = sheet_named(&rem, s);
                    if &s == sheet {
                        sh.row(*c, *r).map(|nr| (nr, true))
                    } else {
                        Some((*r, true))
                    }
                }
                Map::Column(s, col, a, b) => Some((*r, !(s == sheet && c == col && (*a..=*b).contains(r)))),
                Map::Block(s, (c0, c1), (a, b)) => {
                    Some((*r, !(s == sheet && (*c0..=*c1).contains(c) && (*a..=*b).contains(r))))
                }
            };
            let Some((nr, check)) = target else { continue };
            if !check {
                continue;
            }
            let now = after.get(&(sheet.clone(), nr, *c));
            if *is_f {
                t.formula_cells += 1;
                if now.is_some_and(|x| x.2 == *style) {
                    t.formula_styles_same += 1;
                } else {
                    failures.push(fail(format!(
                        "formula cell {sheet}!{}{r} lost its style",
                        hanji_core::cells::col_letters(*c)
                    )));
                }
                continue;
            }
            t.cells_checked += 1;
            if now.is_some_and(|x| !x.0 && &x.1 == body && x.2 == *style) {
                t.cells_same += 1;
            } else {
                failures.push(fail(format!(
                    "cell {sheet}!{}{r} → row {nr}: {:?} became {now:?}",
                    hanji_core::cells::col_letters(*c),
                    (body, style)
                )));
            }
        }
        // PutGet and well-formed parts.
        let out = XlsxEngine.export(&applied.text, &applied.remainder).unwrap();
        for p in package::read(&out).unwrap() {
            if (p.name.ends_with(".xml") || p.name.ends_with(".rels")) && xml::parse(&p.data).is_err() {
                failures.push(fail(format!("{} is not well-formed", p.name)));
            }
        }
        let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
        if again.text == applied.text
            && XlsxEngine::view(&again.remainder, 500).unwrap() == XlsxEngine::view(&applied.remainder, 500).unwrap()
        {
            t.putget += 1;
        } else {
            failures.push(fail("PutGet: the export reads back differently".into()));
        }
        #[cfg(not(target_os = "wasi"))]
        if soffice {
            let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ops-soffice").join(t.cases.to_string());
            match soffice_agrees(&applied.remainder, &out, &d) {
                Some((same, total, diffs)) => {
                    t.soffice_same += same;
                    t.soffice_total += total;
                    // LibreOffice rebuilds pivot tables with its own labels.
                    if !diffs.is_empty() && case.file != "ExcelPivotTableSample.xlsx" {
                        failures.push(fail(format!("LibreOffice shows otherwise: {}", diffs.join("; "))));
                    }
                }
                None => failures.push(fail("LibreOffice could not convert the export".into())),
            }
        }
        if report() {
            println!(
                "{}: ok; notices {:?}; recalc {:?}",
                case.name,
                applied.report.notices.iter().map(|n| &n.kind).collect::<Vec<_>>(),
                applied.report.recalc
            );
        }
    }
    println!("op set: {t:?}");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(t.entries_lost, 0);
}

fn sheet_named(rem: &Remainder, table_or_sheet: &str) -> String {
    let st = XlsxEngine::structure(rem, None).unwrap();
    match st.table(table_or_sheet) {
        Some((s, _)) => s.name.clone(),
        None => table_or_sheet.to_string(),
    }
}

/// A structure edit in the text (a column's format, a deleted placeholder)
/// is applied as operations are: the cells show the new format, the
/// conditional format goes and is reported, and the export reads back so.
#[test]
fn structure_edits_in_the_text() {
    let bytes = std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus/korean-sales.xlsx")).unwrap();
    let imp = XlsxEngine.import(&bytes, &ImportOptions::default()).unwrap();
    let (rem, _) = XlsxEngine::recalculate(&imp.remainder).unwrap();
    let text = XlsxEngine::text_of(&rem, None).unwrap();
    let cf = text.lines().find(|l| l.contains("kind=\"conditional-format\"")).unwrap().to_string();
    let edited = text
        .replace("| 매출 | number | #,##0 |  |", "| 매출 | number | #,##0.00 |  |")
        .replace(&format!("{cf}\n\n"), "");
    assert_ne!(edited, text);
    let a = XlsxEngine::apply(&edited, &rem, "[]").unwrap();
    assert_eq!(a.text, edited);
    let w = XlsxEngine::window(&a.remainder, &WindowOf::Table { name: "Sales".into(), rows: None }).unwrap();
    assert!(w.contains("| 4 | 2026-01 | 강남 | 가전 | 10,269,000.00 | 7,410,000 | 2,859,000 |"), "{w}");
    assert!(
        a.report.notices.iter().any(|n| n.kind == "removed" && n.location.contains("conditional-format")),
        "{:?}",
        a.report.notices
    );
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    let sheet = package::read(&out).unwrap().into_iter().find(|p| p.name == "xl/worksheets/sheet1.xml").unwrap();
    assert!(!String::from_utf8_lossy(&sheet.data).contains("<conditionalFormatting"));
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(again.text, a.text);
    // A rename is not a structure edit the engine makes: it is refused.
    let renamed = text.replace("<table name=\"Sales\"", "<table name=\"Revenue\"");
    assert!(XlsxEngine::apply(&renamed, &rem, "[]").is_err());
}

/// A formula reading a cell an operation sets gets its new cached value:
/// SUM over the table's column and over A1 cells, in the export as read
/// back and as LibreOffice shows it. In a 1904 workbook the arithmetic is
/// computed too, and a formula that reads dates (YEAR) keeps its cached value
/// and is left to the application, with what reads it.
#[test]
fn dependents_get_new_cached_values() {
    use common::fixture::{build, Col, SheetSpec, TableSpec, V};
    for date1904 in [false, true] {
        let t = TableSpec {
            name: "T".into(),
            col: 0,
            row: 1,
            cols: vec![
                Col { name: "a".into(), format: "0".into(), formula: None },
                Col { name: "b".into(), format: "0".into(), formula: None },
            ],
            rows: (1..=4).map(|k| vec![V::Num(k as f64), V::Num(10.0 * k as f64)]).collect(),
        };
        let mut parts = package::read(&build(&[SheetSpec { name: "S".into(), tables: vec![t] }])).unwrap();
        for p in parts.iter_mut() {
            let mut s = String::from_utf8(p.data.clone()).unwrap();
            if p.name == "xl/worksheets/sheet1.xml" {
                // Row 1 ends with the header; the formulas sit beside it.
                let at = s.find("</row>").unwrap();
                s.insert_str(at, "<c r=\"D1\"><f>SUM(T[b])</f><v>100</v></c><c r=\"E1\"><f>SUM(B2:B5)</f><v>100</v></c><c r=\"F1\"><f>YEAR(B2)+0*B3</f><v>1904</v></c><c r=\"G1\"><f>F1+1</f><v>1905</v></c>");
                s = s.replace("<dimension ref=\"A1:B5\"/>", "<dimension ref=\"A1:G5\"/>");
            }
            if p.name == "xl/workbook.xml" && date1904 {
                s = s.replace("<bookViews>", "<workbookPr date1904=\"1\"/><bookViews>");
            }
            p.data = s.into_bytes();
        }
        let pkg = package::write(&parts).unwrap();
        let imp = XlsxEngine.import(&pkg, &ImportOptions::default()).unwrap();
        let a = XlsxEngine::apply(&imp.text, &imp.remainder, r##"[{"op": "set", "range": "S!B3", "values": [[25]]}]"##)
            .unwrap();
        let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
        let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
        let w = XlsxEngine::window(&again.remainder, &WindowOf::Range { sheet: "S".into(), range: "D1:G1".into() })
            .unwrap();
        let wb = String::from_utf8(
            package::read(&out).unwrap().into_iter().find(|p| p.name == "xl/workbook.xml").unwrap().data,
        )
        .unwrap();
        if date1904 {
            // YEAR is left (its cached 1904 kept), and G1 reads it.
            assert!(w.contains("| 1 | 105 | 105 | 1904 | 1905 |"), "{w}");
            assert_eq!(a.report.recalc.left.len(), 2, "{:?}", a.report.recalc);
            assert!(wb.contains("fullCalcOnLoad=\"1\""), "{wb}");
        } else {
            // IronCalc computes YEAR of serial 10 in 1900: 1900.
            assert!(w.contains("| 1 | 105 | 105 | 1900 | 1901 |"), "{w}");
            assert!(a.report.recalc.left.is_empty(), "{:?}", a.report.recalc);
            assert!(!wb.contains("fullCalcOnLoad"), "{wb}");
        }
        #[cfg(not(target_os = "wasi"))]
        if std::env::var("HANJI_SOFFICE").is_ok() {
            let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ops-dependents").join(date1904.to_string());
            let lo = soffice_csv(&out, &d).expect("LibreOffice converts the export");
            let row1 = &lo[0].1[0];
            assert_eq!(&row1[3..5], ["105", "105"], "{row1:?}");
        }
    }
}
