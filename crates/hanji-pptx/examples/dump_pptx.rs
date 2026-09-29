//! Prints a pptx file's model text: `cargo run -p hanji-pptx --example dump_pptx -- file.pptx`.

use hanji_core::{Engine, ImportOptions};
use hanji_pptx::PptxEngine;

fn main() {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        match PptxEngine.import(&bytes, &ImportOptions::default()) {
            Ok(imp) => {
                println!("{}", imp.text);
                for n in imp.report.neutralised.iter().chain(&imp.report.surface) {
                    eprintln!("{}: {} ({})", n.kind, n.location, n.detail);
                }
            }
            Err(e) => eprintln!("{path}: {e}"),
        }
    }
}
