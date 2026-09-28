//! Print the model text of a .docx and a summary of its remainder:
//! `cargo run -p hanji-docx --example dump -- file.docx`
use hanji_core::{Engine, ImportOptions};
use hanji_docx::DocxEngine;

fn main() {
    let path = std::env::args().nth(1).expect("usage: dump FILE.docx");
    let bytes = std::fs::read(&path).expect("read");
    let imp = DocxEngine.import(&bytes, &ImportOptions::default()).unwrap_or_else(|e| panic!("{e}"));
    print!("{}", imp.text);
    let mut kinds = std::collections::BTreeMap::new();
    for e in &imp.remainder.entries {
        *kinds.entry(format!("{:?}", e.kind)).or_insert(0) += 1;
    }
    eprintln!("entries: {kinds:?}");
    for n in imp.report.neutralised.iter().chain(&imp.report.surface) {
        eprintln!("{}: {} — {}", n.kind, n.location, n.detail);
    }
}
