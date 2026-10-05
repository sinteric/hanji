//! Sparse cells below a table must not be shifted beyond Excel's last row.
use hanji_core::cells::{CellRef, MAX_ROW};
use hanji_core::{Engine, ImportOptions};
use hanji_xlsx::{package, xml, XlsxEngine};

mod common;
use common::fixture::{build, Col, SheetSpec, TableSpec, V};

fn workbook(row: u32, col: &str) -> Vec<u8> {
    let table = TableSpec {
        name: "T".into(),
        col: 0,
        row: 1,
        cols: vec![Col { name: "Value".into(), format: "0".into(), formula: None }],
        rows: vec![vec![V::Num(1.0)], vec![V::Num(2.0)]],
    };
    let mut parts = package::read(&build(&[SheetSpec { name: "Data".into(), tables: vec![table] }])).unwrap();
    let sheet = parts.iter_mut().find(|p| p.name == "xl/worksheets/sheet1.xml").unwrap();
    let text = String::from_utf8(sheet.data.clone()).unwrap();
    sheet.data = text
        .replace("</sheetData>", &format!("<row r=\"{row}\"><c r=\"{col}{row}\"><v>42</v></c></row></sheetData>"))
        .into_bytes();
    package::write(&parts).unwrap()
}

fn insert(n: usize) -> String {
    let rows = vec![r#"{"Value":9}"#; n].join(",");
    format!(r#"[{{"op":"insert_rows","table":"T","before":2,"rows":[{rows}]}}]"#)
}

#[test]
fn bottom_cells_refuse_insert_atomically() {
    for (row, n) in [(MAX_ROW, 1), (MAX_ROW - 1, 2), (MAX_ROW, 3)] {
        let input = workbook(row, "A");
        let imp = XlsxEngine.import(&input, &ImportOptions::default()).unwrap();
        let before = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
        let snapshot = format!("{:?}", imp.remainder);
        // A preceding valid edit in the batch must not escape the refusal.
        let ops = insert(n).replacen(
            '[',
            r#"[{"op":"set","range":"Data!A2","values":[[99]]},"#,
            1,
        );
        let err = XlsxEngine::apply(&imp.text, &imp.remainder, &ops).unwrap_err().to_string();
        assert!(err.contains("worksheet row limit 1048576"), "{err}");
        assert!(err.contains(&format!("A{row}")), "{err}");
        assert_eq!(snapshot, format!("{:?}", imp.remainder));
        assert_eq!(before, XlsxEngine.export(&imp.text, &imp.remainder).unwrap());
    }
}

#[test]
fn exact_limit_and_cells_outside_the_shifted_columns_succeed() {
    for (row, col, n, expected) in [
        (MAX_ROW - 1, "A", 1, MAX_ROW),
        (MAX_ROW - 3, "A", 3, MAX_ROW),
        (MAX_ROW, "B", 3, MAX_ROW),
    ] {
        let imp = XlsxEngine.import(&workbook(row, col), &ImportOptions::default()).unwrap();
        let a = XlsxEngine::apply(&imp.text, &imp.remainder, &insert(n)).unwrap();
        let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
        let parts = package::read(&out).unwrap();
        let sheet = package::get(&parts, "xl/worksheets/sheet1.xml").unwrap();
        let doc = xml::parse(sheet).unwrap();
        let mut found = false;
        doc.root.walk(&mut |e| {
            if e.local() == "row" {
                assert!(e.get("r").unwrap().parse::<u32>().unwrap() <= MAX_ROW);
            }
            if e.local() == "c" {
                let addr = e.get("r").unwrap();
                assert!(CellRef::parse(&addr).is_some(), "out-of-bounds cell: {addr}");
                found |= addr == format!("{col}{expected}");
            }
        });
        assert!(found);
        let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
        assert_eq!(a.text, again.text);
    }
}

#[test]
fn empty_insert_preserves_a_bottom_cell() {
    let imp = XlsxEngine.import(&workbook(MAX_ROW, "A"), &ImportOptions::default()).unwrap();
    let a = XlsxEngine::apply(&imp.text, &imp.remainder, &insert(0)).unwrap();
    assert_eq!(imp.text, a.text);
    common::getput(
        &XlsxEngine.export(&imp.text, &imp.remainder).unwrap(),
        &XlsxEngine.export(&a.text, &a.remainder).unwrap(),
    )
    .unwrap();
}
