//! §2 rule 11 on a large sheet: a generated workbook of 100,000 table rows
//! (five columns, one of them calculated) is imported, a window of it read,
//! range operations applied and the result exported, in bounded time and
//! memory. The structure text does not grow with the rows, and a window
//! costs what it shows. HANJI_REPORT=1 prints the numbers.

use std::time::Instant;

use hanji_core::{Engine, ImportOptions};
use hanji_format::sheet::WindowOf;
use hanji_xlsx::XlsxEngine;

mod common;
use common::fixture::{build, Col, SheetSpec, TableSpec, V};
use common::report;

const ROWS: usize = 100_000;

/// The process's peak resident memory in MB (Linux; 0 elsewhere).
fn peak_mb() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines().find(|l| l.starts_with("VmHWM:")).and_then(|l| l.split_whitespace().nth(1)?.parse::<u64>().ok())
        })
        .map_or(0, |kb| kb / 1024)
}

fn workbook() -> Vec<u8> {
    let branches = ["강남", "서초", "송파", "분당", "일산"];
    let col = |name: &str, format: &str, formula: Option<&str>| Col {
        name: name.into(),
        format: format.into(),
        formula: formula.map(Into::into),
    };
    let rows = (0..ROWS)
        .map(|k| {
            vec![
                V::Date(format!("2026-{:02}-{:02}", k % 12 + 1, k % 28 + 1)),
                V::Text(branches[k % 5].into()),
                V::Text(format!("item {}", k % 997)),
                V::Num((k * 37 % 100_000) as f64 * 100.0),
                V::Num((k * 23 % 70_000) as f64 * 100.0),
            ]
        })
        .collect();
    let t = TableSpec {
        name: "Big".into(),
        col: 0,
        row: 1,
        cols: vec![
            col("날짜", "yyyy-mm-dd", None),
            col("지점", "General", None),
            col("품목", "General", None),
            col("매출", "#,##0", None),
            col("원가", "#,##0", None),
            col("이익", "#,##0", Some("Big[[#This Row],[매출]]-Big[[#This Row],[원가]]")),
        ],
        rows,
    };
    build(&[SheetSpec { name: "Data".into(), tables: vec![t] }])
}

#[test]
fn a_hundred_thousand_rows() {
    let pkg = workbook();
    let base = peak_mb();
    let mut times = vec![];
    let mut lap = |what: &'static str, t: Instant| times.push((what, t.elapsed().as_millis(), peak_mb()));

    let t = Instant::now();
    let imp = XlsxEngine.import(&pkg, &ImportOptions::default()).unwrap();
    lap("import", t);
    let lines = imp.text.lines().count();
    assert!(lines < 40, "the structure text has {lines} lines");
    assert!(imp.text.contains("<table name=\"Big\" range=\"A1:F100001\">"), "{}", imp.text);

    // The generator writes no cached values (as openpyxl): compute all 100,000.
    let t = Instant::now();
    let (rem, rc) = XlsxEngine::recalculate(&imp.remainder).unwrap();
    lap("recalculate 100,000 formulas", t);
    assert_eq!((rc.written, rc.left.len()), (ROWS, 0));
    let imp = hanji_core::Imported { remainder: rem, ..imp };

    let t = Instant::now();
    let w = XlsxEngine::window(&imp.remainder, &WindowOf::Table { name: "Big".into(), rows: Some((50_002, 50_101)) })
        .unwrap();
    lap("window of 100 rows", t);
    assert_eq!(w.lines().count(), 104, "{w}");
    assert!(
        w.contains("| 50002 | 2026-09-21 | 강남 | item 150 | 5,000,000 | 3,000,000 | 2,000,000 |"),
        "{}",
        &w[..w.len().min(600)]
    );

    let ops = r##"[{"op": "set", "range": "Data!D50002", "values": [[123400]]},
        {"op": "append_rows", "table": "Big", "rows": [{"날짜": "2026-12-31", "지점": "강남", "품목": "new", "매출": 1000, "원가": 400}]},
        {"op": "delete_rows", "table": "Big", "rows": "2:11"}]"##;
    let t = Instant::now();
    let a = XlsxEngine::apply(&imp.text, &imp.remainder, ops).unwrap();
    lap("set, append and delete rows", t);
    assert!(a.text.contains("<table name=\"Big\" range=\"A1:F99992\">"), "{}", a.text);
    let w = XlsxEngine::window(&a.remainder, &WindowOf::Table { name: "Big".into(), rows: Some((99_991, 99_992)) })
        .unwrap();
    assert!(w.contains("| 99992 | 2026-12-31 | 강남 | new | 1,000 | 400 | 600 |"), "{w}");
    let w = XlsxEngine::window(&a.remainder, &WindowOf::Table { name: "Big".into(), rows: Some((49_992, 49_992)) })
        .unwrap();
    assert!(w.contains("| 49992 | 2026-09-21 | 강남 | item 150 | 123,400 | 3,000,000 | -2,876,600 |"), "{w}");

    let t = Instant::now();
    let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
    lap("export", t);

    let t = Instant::now();
    let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
    lap("import of the export", t);
    assert_eq!(again.text, a.text);

    let peak = peak_mb();
    let line = format!(
        "large sheet: {ROWS} rows, package {} KB, text {} lines; {}; peak memory {peak} MB ({} MB over the workbook's build), export {} KB",
        pkg.len() / 1024,
        lines,
        times.iter().map(|(w, ms, mb)| format!("{w} {ms} ms (peak {mb} MB)")).collect::<Vec<_>>().join(", "),
        peak.saturating_sub(base),
        out.len() / 1024,
    );
    if report() {
        println!("{line}");
    }
    // Bounds with room for a slow CI machine and a debug build.
    for (what, ms, _) in &times {
        assert!(*ms < 120_000, "{what} took {ms} ms: {line}");
    }
    if peak > 0 {
        assert!(peak.saturating_sub(base) < 1_500, "{line}");
    }
}
