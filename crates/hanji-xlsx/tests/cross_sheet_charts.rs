//! A chart's owning drawing does not determine which worksheet supplies its data.
use hanji_core::{Engine, ImportOptions};
use hanji_package::opc::Rel;
use hanji_xlsx::book::{Book, Shell};
use hanji_xlsx::{package, xml, XlsxEngine};

mod common;
use common::fixture::{build, Col, SheetSpec, TableSpec, V};

fn chart(sheet: &str) -> Vec<u8> {
    format!(
        r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart><c:plotArea><c:pieChart><c:ser><c:idx val="0"/><c:order val="0"/><c:cat><c:strRef><c:f>{sheet}!$B$2:$B$3</c:f><c:strCache><c:ptCount val="2"/><c:pt idx="0"><c:v>1</c:v></c:pt><c:pt idx="1"><c:v>2</c:v></c:pt></c:strCache></c:strRef></c:cat><c:val><c:numRef><c:f>{sheet}!$B$2:$B$3</c:f><c:numCache><c:formatCode>0</c:formatCode><c:ptCount val="2"/><c:pt idx="0"><c:v>1</c:v></c:pt><c:pt idx="1"><c:v>2</c:v></c:pt></c:numCache></c:numRef></c:val></c:ser></c:pieChart></c:plotArea></c:chart></c:chartSpace>"#
    )
    .into_bytes()
}

fn workbook(name: &str, formula_sheet: &str) -> Vec<u8> {
    let table = TableSpec {
        name: "T".into(),
        col: 1,
        row: 1,
        cols: vec![Col { name: "Value".into(), format: "0".into(), formula: None }],
        rows: vec![vec![V::Num(1.0)], vec![V::Num(2.0)]],
    };
    let bytes = build(&[
        SheetSpec { name: name.into(), tables: vec![table] },
        SheetSpec { name: "Summary".into(), tables: vec![] },
    ]);
    let mut book = Book::load(package::read(&bytes).unwrap(), vec![], Shell::default()).unwrap();
    for (i, source) in [(1, formula_sheet), (2, formula_sheet), (3, "Summary")] {
        book.add_part(
            &format!("xl/charts/chart{i}.xml"),
            chart(source),
            "application/vnd.openxmlformats-officedocument.drawingml.chart+xml",
        );
    }
    for sheet in 1..=2 {
        let drawing = format!("xl/drawings/drawing{sheet}.xml");
        let cp = if sheet == 1 { 2 } else { 1 };
        let rid = book.add_rel(&drawing, "chart", &format!("xl/charts/chart{cp}.xml"));
        let anchor = format!(
            r#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><xdr:twoCellAnchor><xdr:from><xdr:col>1</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>4</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from><xdr:to><xdr:col>1</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>8</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:to><xdr:graphicFrame><xdr:nvGraphicFramePr><xdr:cNvPr id="1" name="Chart"/><xdr:cNvGraphicFramePr/></xdr:nvGraphicFramePr><xdr:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/></xdr:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart r:id="{rid}"/></a:graphicData></a:graphic></xdr:graphicFrame><xdr:clientData/></xdr:twoCellAnchor></xdr:wsDr>"#
        );
        book.add_part(&drawing, anchor.into_bytes(), "application/vnd.openxmlformats-officedocument.drawing+xml");
        let rid = book.add_rel(&format!("xl/worksheets/sheet{sheet}.xml"), "drawing", &drawing);
        let part = book.parts.iter_mut().find(|p| p.name == format!("xl/worksheets/sheet{sheet}.xml")).unwrap();
        let text = String::from_utf8(part.data.clone()).unwrap();
        let at = text.find("<tableParts").unwrap_or_else(|| text.find("</worksheet>").unwrap());
        part.data = format!("{}<drawing r:id=\"{rid}\"/>{}", &text[..at], &text[at..]).into_bytes();
    }
    // A shared target must be shifted only once. An unrelated chart and an
    // external relationship must retain their original data.
    book.add_rel("xl/drawings/drawing1.xml", "chart", "xl/charts/chart1.xml");
    book.add_rel("xl/drawings/drawing2.xml", "chart", "xl/charts/chart3.xml");
    book.add_part(
        "xl/charts/chart4.xml",
        chart(formula_sheet),
        "application/vnd.openxmlformats-officedocument.drawingml.chart+xml",
    );
    let mut rels = book.rels_of("xl/drawings/drawing2.xml");
    rels.push(Rel {
        id: "externalChart".into(),
        ty: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart".into(),
        target: "../charts/chart4.xml".into(),
        external: true,
    });
    book.set_rels("xl/drawings/drawing2.xml", rels);
    package::write(&book.finish().0).unwrap()
}

fn chart_state(bytes: &[u8], part: &str) -> (Vec<String>, usize) {
    let parts = package::read(bytes).unwrap();
    let d = xml::parse(package::get(&parts, part).unwrap()).unwrap();
    let mut formulas = vec![];
    let mut caches = 0;
    d.root.walk(&mut |e| {
        if e.local() == "f" {
            formulas.push(e.text_of(&[e.name.as_str()]));
        }
        if matches!(e.local(), "numCache" | "strCache" | "multiLvlStrCache") {
            caches += 1;
        }
    });
    (formulas, caches)
}

#[test]
fn charts_on_other_sheets_follow_insertions_and_deletions() {
    for (name, source) in [("Data", "Data"), ("Data's values", "'Data''s values'")] {
        for (ops, range) in [
            (r#"[{"op":"insert_rows","table":"T","before":2,"rows":[{"Value":9}]}]"#, "$B$3:$B$4"),
            (r#"[{"op":"insert_rows","table":"T","before":3,"rows":[{"Value":9}]}]"#, "$B$2:$B$4"),
            (r#"[{"op":"delete_rows","table":"T","rows":"2"}]"#, "$B$2:$B$2"),
        ] {
            let input = workbook(name, source);
            let imp = XlsxEngine.import(&input, &ImportOptions::default()).unwrap();
            let a = XlsxEngine::apply(&imp.text, &imp.remainder, ops).unwrap();
            let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
            for part in ["xl/charts/chart1.xml", "xl/charts/chart2.xml"] {
                let (formulas, caches) = chart_state(&out, part);
                assert_eq!(formulas, vec![format!("{source}!{range}"); 2], "{part}: {ops}");
                assert_eq!(caches, 0, "moved sources must not retain stale caches");
            }
            let old = package::read(&input).unwrap();
            let new = package::read(&out).unwrap();
            for untouched in ["xl/charts/chart3.xml", "xl/charts/chart4.xml", "xl/drawings/drawing2.xml"] {
                assert_eq!(package::get(&old, untouched), package::get(&new, untouched), "{untouched}");
            }
            XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
        }
    }
}

#[test]
fn no_op_preserves_chart_parts_and_caches() {
    let input = workbook("Data", "Data");
    let imp = XlsxEngine.import(&input, &ImportOptions::default()).unwrap();
    let a = XlsxEngine::apply(&imp.text, &imp.remainder, "[]").unwrap();
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    let old = package::read(&input).unwrap();
    let new = package::read(&out).unwrap();
    for part in ["xl/charts/chart1.xml", "xl/charts/chart2.xml", "xl/charts/chart3.xml"] {
        assert_eq!(package::get(&old, part), package::get(&new, part));
        assert_eq!(chart_state(&out, part).1, 2);
    }
}

#[test]
fn chart_sources_that_would_split_refuse_atomically() {
    let input = workbook("Data", "Data");
    let mut parts = package::read(&input).unwrap();
    let chart = parts.iter_mut().find(|p| p.name == "xl/charts/chart1.xml").unwrap();
    chart.data = String::from_utf8(chart.data.clone()).unwrap().replace("$B$2:$B$3", "$A$2:$B$3").into_bytes();
    let imp = XlsxEngine.import(&package::write(&parts).unwrap(), &ImportOptions::default()).unwrap();
    let before = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
    let err = XlsxEngine::apply(
        &imp.text,
        &imp.remainder,
        r#"[{"op":"insert_rows","table":"T","before":2,"rows":[{"Value":9}]}]"#,
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("chart xl/charts/chart1.xml source Data!$A$2:$B$3 would be split"), "{err}");
    assert_eq!(before, XlsxEngine.export(&imp.text, &imp.remainder).unwrap());
}
