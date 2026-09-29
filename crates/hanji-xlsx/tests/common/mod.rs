//! Helpers the xlsx tests share.
#![allow(dead_code)]

pub mod fixture;

use std::path::PathBuf;

use hanji_xlsx::{package, xml};

pub fn corpus() -> Vec<(String, Vec<u8>)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus");
    let mut files = hanji_testkit::corpus(&dir, "xlsx");
    files.extend(hanji_testkit::corpus(&dir, "xlsm"));
    files.sort();
    files
}

pub fn report() -> bool {
    std::env::var("HANJI_REPORT").is_ok()
}

/// A worksheet part: the engine splits it, so GetPut compares it canonically.
pub fn is_split_part(name: &str) -> bool {
    name.starts_with("xl/worksheets/") && name.ends_with(".xml")
}

/// GetPut on two packages: the same parts in the same order, worksheets
/// canonically equal, every other part byte for byte.
pub fn getput(a: &[u8], b: &[u8]) -> Result<(), String> {
    let (a, b) = (package::read(a)?, package::read(b)?);
    let names = |v: &[hanji_core::Part]| v.iter().map(|p| p.name.clone()).collect::<Vec<_>>();
    if names(&a) != names(&b) {
        return Err(format!("parts differ: {:?} vs {:?}", names(&a), names(&b)));
    }
    for (x, y) in a.iter().zip(&b) {
        if x.data == y.data {
            continue;
        }
        if is_split_part(&x.name) && xml::canon_part(&x.data).ok() == xml::canon_part(&y.data).ok() {
            continue;
        }
        return Err(format!("{} differs", x.name));
    }
    Ok(())
}

/// LibreOffice's view of a workbook: each worksheet as CSV, values as shown
/// (`None` when soffice cannot convert it).
#[cfg(not(target_os = "wasi"))]
pub fn soffice_csv(xlsx: &[u8], dir: &std::path::Path) -> Option<Vec<(String, Vec<Vec<String>>)>> {
    use std::process::Command;
    // One LibreOffice at a time: a second one hands its conversion to the first.
    static ONE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _one = ONE.lock().unwrap_or_else(|e| e.into_inner());
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).ok()?;
    let src = dir.join("book.xlsx");
    std::fs::write(&src, xlsx).ok()?;
    let out = Command::new("soffice")
        .env("LANG", "C.UTF-8")
        .env("LC_ALL", "C.UTF-8")
        .args([
            "--headless",
            "--convert-to",
            "csv:Text - txt - csv (StarCalc):44,34,76,1,,0,false,true,true,false,false,-1",
            "--outdir",
        ])
        .arg(dir)
        .arg(&src)
        .output()
        .ok()?;
    let log = String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    let mut sheets = vec![];
    for line in log.lines() {
        // "Writing sheet 매출 -> /dir/book-매출.csv"
        let Some(rest) = line.strip_prefix("Writing sheet ") else { continue };
        let (name, path) = rest.split_once(" -> ")?;
        let text = std::fs::read_to_string(path.trim()).ok()?;
        sheets.push((name.to_string(), parse_csv(&text)));
    }
    if sheets.is_empty() {
        // One sheet: no per-sheet lines.
        let p = dir.join("book.csv");
        let text = std::fs::read_to_string(p).ok()?;
        sheets.push((String::new(), parse_csv(&text)));
    }
    Some(sheets)
}

pub fn parse_csv(text: &str) -> Vec<Vec<String>> {
    let mut rows = vec![];
    let mut row = vec![];
    let mut cur = String::new();
    let mut quoted = false;
    let mut it = text.chars().peekable();
    while let Some(c) = it.next() {
        match (quoted, c) {
            (true, '"') if it.peek() == Some(&'"') => {
                cur.push('"');
                it.next();
            }
            (true, '"') => quoted = false,
            (true, c) => cur.push(c),
            (false, '"') => quoted = true,
            (false, ',') => row.push(std::mem::take(&mut cur)),
            (false, '\n') => {
                row.push(std::mem::take(&mut cur));
                rows.push(std::mem::take(&mut row));
            }
            (false, '\r') => {}
            (false, c) => cur.push(c),
        }
    }
    if !cur.is_empty() || !row.is_empty() {
        row.push(cur);
        rows.push(row);
    }
    rows
}

/// Cells where LibreOffice shows what the engine's windows show: (same, compared).
#[cfg(not(target_os = "wasi"))]
pub fn soffice_agrees(
    rem: &hanji_core::Remainder,
    xlsx: &[u8],
    dir: &std::path::Path,
) -> Option<(usize, usize, Vec<String>)> {
    use hanji_format::sheet::{split_pipe_row, WindowOf};
    let lo = soffice_csv(xlsx, dir)?;
    let st = hanji_xlsx::XlsxEngine::structure(rem, None).ok()?;
    let (mut same, mut total, mut diffs) = (0, 0, vec![]);
    let worksheets: Vec<&hanji_format::sheet::SheetDecl> = st
        .sheets
        .iter()
        .filter(|s| {
            s.range.is_some()
                || !s.items.iter().any(|i| matches!(i, hanji_format::sheet::SheetItem::Keep(k) if k.kind == "chart"))
        })
        .collect();
    for (name, rows) in &lo {
        let Some(sh) = worksheets.iter().find(|s| &s.name == name || (name.is_empty() && worksheets.len() == 1)) else {
            continue;
        };
        let Some(range) = &sh.range else { continue };
        let r = hanji_core::cells::CellRange::parse(range)?;
        // The CSV starts at A1.
        let last_row = r.last.row.min(r.first.row + 499);
        let first = hanji_core::cells::CellRef::new(0, 1.max(r.first.row));
        let win =
            hanji_core::cells::CellRange::new(first, hanji_core::cells::CellRef::new(r.last.col.min(63), last_row));
        let w =
            hanji_xlsx::XlsxEngine::window(rem, &WindowOf::Range { sheet: sh.name.clone(), range: win.to_string() })
                .ok()?;
        for line in w.lines().skip(3).take_while(|l| !l.starts_with("</data>")) {
            let cells = split_pipe_row(line)?;
            let rn: usize = cells[0].parse().ok()?;
            for (k, ours) in cells[1..].iter().enumerate() {
                let theirs = rows.get(rn - 1).and_then(|r| r.get(k)).map(String::as_str).unwrap_or("");
                let ours = ours.replace("<br/>", "\n");
                if ours.trim().is_empty() && theirs.trim().is_empty() {
                    continue;
                }
                total += 1;
                if ours.trim() == theirs.trim() {
                    same += 1;
                } else if diffs.len() < 12 {
                    diffs.push(format!(
                        "{}!{}{rn}: ours {ours:?}, LibreOffice {theirs:?}",
                        sh.name,
                        hanji_core::cells::col_letters(k as u32)
                    ));
                }
            }
        }
    }
    Some((same, total, diffs))
}
