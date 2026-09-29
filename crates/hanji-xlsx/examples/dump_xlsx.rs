//! Prints a workbook's structure text and a window of every table or sheet:
//! `cargo run -p hanji-xlsx --example dump_xlsx -- file.xlsx [rows]`.

use hanji_core::ImportOptions;
use hanji_xlsx::XlsxEngine;

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let rows = args.last().and_then(|a| a.parse::<u32>().ok());
    if rows.is_some() {
        args.pop();
    }
    for path in args {
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        match XlsxEngine::split(&bytes, &ImportOptions::default()) {
            Ok((rem, report)) => {
                match XlsxEngine::view(&rem, rows.unwrap_or(20)) {
                    Ok(v) => println!("{v}"),
                    Err(e) => eprintln!("{path}: {e}"),
                }
                for n in report.neutralised.iter().chain(&report.surface) {
                    eprintln!("{}: {} ({})", n.kind, n.location, n.detail);
                }
            }
            Err(e) => eprintln!("{path}: {e}"),
        }
    }
}
