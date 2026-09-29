//! Applies range operations to a workbook and prints the new structure and windows:
//! `cargo run -p hanji-xlsx --example apply_ops -- file.xlsx ops.json [out.xlsx]`.

use hanji_core::{Engine, ImportOptions};
use hanji_xlsx::XlsxEngine;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let bytes = std::fs::read(&args[0]).unwrap();
    let ops = std::fs::read_to_string(&args[1]).unwrap();
    let imp = XlsxEngine.import(&bytes, &ImportOptions::default()).unwrap();
    match XlsxEngine::apply(&imp.text, &imp.remainder, &ops) {
        Ok(a) => {
            println!("{}", XlsxEngine::view(&a.remainder, 12).unwrap());
            for e in &a.report.entries {
                println!("entry {} {} {}: {} -> {:?}", e.id, e.tag, e.sheet, e.before, e.after);
            }
            for n in &a.report.notices {
                println!("notice {}: {} ({})", n.kind, n.location, n.detail);
            }
            println!("recalc {:?}", a.report.recalc);
            if let Some(out) = args.get(2) {
                std::fs::write(out, XlsxEngine.export(&a.text, &a.remainder).unwrap()).unwrap();
            }
        }
        Err(e) => println!("ERROR {e}"),
    }
}
