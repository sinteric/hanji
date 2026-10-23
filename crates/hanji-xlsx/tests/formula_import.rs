//! §8 on import reads the formulas that may fetch, and the defined names of a
//! workbook whose external links it removes: text the formula tokenizer cannot
//! make sense of (an unclosed `[`) must import and round trip, not panic.

use hanji_core::{Engine, ImportOptions};
use hanji_xlsx::{package, XlsxEngine};

mod common;
use common::fixture::{build, Col, SheetSpec, TableSpec, V};
use common::*;

fn raw() -> ImportOptions {
    ImportOptions {
        neutralise: false,
        template: None,
    }
}

/// A one-table workbook, its parts edited by `edit` (name, text → text).
fn workbook(edit: &dyn Fn(&str, String) -> String) -> Vec<hanji_core::Part> {
    let col = |n: &str| Col {
        name: n.into(),
        format: "General".into(),
        formula: None,
    };
    let base = build(&[SheetSpec {
        name: "Data".into(),
        tables: vec![TableSpec {
            name: "T".into(),
            col: 0,
            row: 1,
            cols: vec![col("a"), col("b")],
            rows: vec![vec![V::Num(1.0), V::Text("x".into())]],
        }],
    }]);
    let mut parts = package::read(&base).unwrap();
    for p in &mut parts {
        p.data = edit(&p.name, String::from_utf8(p.data.clone()).unwrap()).into_bytes();
    }
    parts
}

/// The workbook with an external link (which a neutralising import removes,
/// reading every defined name as it does) and the defined name `x` = `text`.
fn with_external_link_and_name(text: &str) -> Vec<u8> {
    const LINK_REL: &str = "<Relationship Id=\"rId99\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/externalLink\" Target=\"externalLinks/externalLink1.xml\"/>";
    const LINK_TYPE: &str = "<Override PartName=\"/xl/externalLinks/externalLink1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.externalLink+xml\"/>";
    const LINK: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<externalLink xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><externalBook r:id=\"rId1\"/></externalLink>";
    const LINK_RELS: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/externalLinkPath\" Target=\"other.xlsx\" TargetMode=\"External\"/></Relationships>";
    let refs = format!(
        "<externalReferences><externalReference r:id=\"rId99\"/></externalReferences><definedNames><definedName name=\"x\">{text}</definedName></definedNames>"
    );
    let mut parts = workbook(&|name, s| match name {
        "xl/workbook.xml" => s.replace("</sheets>", &format!("</sheets>{refs}")),
        "xl/_rels/workbook.xml.rels" => {
            s.replace("</Relationships>", &format!("{LINK_REL}</Relationships>"))
        }
        "[Content_Types].xml" => s.replace("</Types>", &format!("{LINK_TYPE}</Types>")),
        _ => s,
    });
    let part = |name: &str, data: &str| hanji_core::Part {
        name: name.into(),
        data: data.into(),
        ..parts[0].clone()
    };
    let link = part("xl/externalLinks/externalLink1.xml", LINK);
    let rels = part("xl/externalLinks/_rels/externalLink1.xml.rels", LINK_RELS);
    parts.extend([link, rels]);
    package::write(&parts).unwrap()
}

#[test]
fn defined_names_with_an_unclosed_bracket_import_and_round_trip() {
    for text in ["[", "[가", "SUM([", "T[", "[1"] {
        let bytes = with_external_link_and_name(text);
        // GetPut on the file as it is.
        let imp = XlsxEngine.import(&bytes, &raw()).unwrap();
        getput(
            &bytes,
            &XlsxEngine.export(&imp.text, &imp.remainder).unwrap(),
        )
        .unwrap_or_else(|e| panic!("{text:?}: {e}"));
        // Neutralised: the link goes, the name (which reads nothing) stays.
        let imp = XlsxEngine
            .import(&bytes, &ImportOptions::default())
            .unwrap();
        assert!(
            imp.report
                .neutralised
                .iter()
                .any(|n| n.kind == "external-link"),
            "{text:?}: {:?}",
            imp.report.neutralised
        );
        assert!(
            !imp.report
                .neutralised
                .iter()
                .any(|n| n.location.contains("defined name")),
            "{text:?}"
        );
        let out = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
        let wb = String::from_utf8(
            package::get(&package::read(&out).unwrap(), "xl/workbook.xml")
                .unwrap()
                .to_vec(),
        );
        assert!(
            wb.unwrap().contains(&format!(">{text}</definedName>")),
            "{text:?}"
        );
        let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
        assert!(
            again.report.neutralised.is_empty(),
            "{text:?}: {:?}",
            again.report.neutralised
        );
        getput(
            &out,
            &XlsxEngine.export(&again.text, &again.remainder).unwrap(),
        )
        .unwrap_or_else(|e| panic!("{text:?}: {e}"));
    }
}

#[test]
fn cell_formulas_with_an_unclosed_bracket_import_and_round_trip() {
    // A `|` makes the cell one the formula scan reads.
    for text in [
        "\"|\"&amp;[",
        "\"|\"&amp;[가",
        "\"|\"&amp;SUM([",
        "\"|\"&amp;T[",
    ] {
        let parts = workbook(&|name, s| match name {
            "xl/worksheets/sheet1.xml" => s.replace(
                "</sheetData>",
                &format!("<row r=\"5\"><c r=\"D5\"><f>{text}</f></c></row></sheetData>"),
            ),
            _ => s,
        });
        let bytes = package::write(&parts).unwrap();
        for opts in [raw(), ImportOptions::default()] {
            let imp = XlsxEngine.import(&bytes, &opts).unwrap();
            assert!(
                imp.report.neutralised.is_empty(),
                "{text:?}: {:?}",
                imp.report.neutralised
            );
            getput(
                &bytes,
                &XlsxEngine.export(&imp.text, &imp.remainder).unwrap(),
            )
            .unwrap_or_else(|e| panic!("{text:?} (neutralise {}): {e}", opts.neutralise));
        }
    }
}
