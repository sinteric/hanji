//! The Spreadsheet structure (DESIGN.md §5.4).

use hanji_format::sheet::{parse_spreadsheet, serialize_spreadsheet, window_text, SheetItem, WindowOf};
use hanji_format::{diag, Names};

#[test]
fn the_design_md_example_parses() {
    // DESIGN.md §5.4's first example, read from the spec itself so the two cannot drift.
    let spec = include_str!("../../../DESIGN.md");
    let body = &spec[spec.find("### 5.4 Spreadsheet").unwrap()..];
    let a = body.find("```\n").unwrap() + 4;
    let b = a + body[a..].find("```\n").unwrap();
    // The block is indented under its list item.
    let text: String =
        body[a..b].trim_end_matches(' ').lines().map(|l| format!("{}\n", l.strip_prefix("  ").unwrap_or(l))).collect();
    let s = parse_spreadsheet(&text, &Names::default()).unwrap_or_else(|e| panic!("{}", diag::render(&e)));
    assert_eq!(serialize_spreadsheet(&s), text, "the example is in canonical form");
    let sheet = &s.sheets[0];
    assert_eq!((sheet.name.as_str(), sheet.range.as_deref()), ("매출", Some("A1:G1201")));
    let items: Vec<String> = sheet
        .items
        .iter()
        .map(|i| match i {
            SheetItem::Table(t) => {
                format!("table {} {}", t.name, t.columns.iter().map(|c| c.ty.as_str()).collect::<Vec<_>>().join(","))
            }
            SheetItem::Chart(c) => format!("chart {}", c.ty),
            SheetItem::Keep(k) => format!("keep {}", k.kind),
        })
        .collect();
    assert_eq!(items, ["table Sales date,number,number,number", "chart bar", "keep data-validation"]);
}

#[test]
fn the_design_md_windows_are_what_the_window_writes() {
    // DESIGN.md §5.4's second example: a table window with `rows`, and a range window.
    let spec = include_str!("../../../DESIGN.md");
    let body = &spec[spec.find("### 5.4 Spreadsheet").unwrap()..];
    let mut at = 0;
    let mut blocks = vec![];
    while blocks.len() < 2 {
        let k = body[at..].find("```\n").unwrap();
        let a = at + k + 4;
        let b = a + body[a..].find("```\n").unwrap();
        blocks.push(
            body[a..b]
                .trim_end_matches(' ')
                .lines()
                .map(|l| format!("{}\n", l.strip_prefix("  ").unwrap_or(l)))
                .collect::<String>(),
        );
        at = b + 4;
    }
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    let table = window_text(
        &WindowOf::Table { name: "Sales".into(), rows: Some((2, 3)) },
        &s(&["월", "매출", "원가", "이익"]),
        &[
            (2, s(&["2026-01", "12,000,000", "8,400,000", "3,600,000"])),
            (3, s(&["2026-02", "11,200,000", "7,900,000", "3,300,000"])),
        ],
    );
    let range = window_text(
        &WindowOf::Range { sheet: "매출".into(), range: "F1:G2".into() },
        &s(&["F", "G"]),
        &[(1, s(&["목표", "150,000,000"])), (2, s(&["", ""]))],
    );
    assert_eq!(blocks[1], table + &range);
}
