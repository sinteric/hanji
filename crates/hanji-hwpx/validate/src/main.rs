//! Re-opens the hwpx corpus test's exports with rhwp (DESIGN.md §9 validity):
//!
//! ```sh
//! cargo test --release -p hanji-hwpx --test corpus          # writes target/tmp/hwpx-corpus-out
//! cargo run --release --manifest-path crates/hanji-hwpx/validate/Cargo.toml [DIR]
//! ```
//!
//! For every export: rhwp parses it (`parse_hwpx`), lays it out
//! (`DocumentCore::from_bytes`, page count) and passes its own save gate
//! (`roundtrip_ir_diff`: rhwp writes the document and re-reads it with no IR
//! difference). For a GetPut export, rhwp's IR of it equals its IR of the
//! original (`diff_documents`, what `rhwp ir-diff` reports), and the page
//! count is the same. For an export with tracked changes (the engine test's),
//! it reports how many marks an rhwp save keeps.
//!
//! rhwp is a git dependency (not on crates.io; its repository is large). To
//! use a local clone: `--config "patch.'https://github.com/edwardkim/rhwp'.rhwp.path='/path/to/rhwp'"`.

use std::path::PathBuf;

use rhwp::document_core::DocumentCore;
use rhwp::parser::hwpx::parse_hwpx;
use rhwp::serializer::hwpx::roundtrip::{diff_documents, roundtrip_ir_diff};
use rhwp::serializer::hwpx::serialize_hwpx;

/// Tracked-change marks in a package's sections.
fn marks(pkg: &[u8]) -> usize {
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(pkg)).unwrap();
    let mut n = 0;
    for i in 0..z.len() {
        let mut f = z.by_index(i).unwrap();
        if f.name().starts_with("Contents/section") {
            let mut s = String::new();
            std::io::Read::read_to_string(&mut f, &mut s).unwrap();
            n += ["<hp:insertBegin", "<hp:insertEnd", "<hp:deleteBegin", "<hp:deleteEnd"]
                .iter()
                .map(|m| s.matches(m).count())
                .sum::<usize>();
        }
    }
    n
}

fn main() {
    let dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../target/tmp/hwpx-corpus-out"));
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e} (run the hwpx corpus test first)", dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    let (mut total, mut opened, mut gate, mut failures) = (0, 0, 0, vec![]);
    let mut getput = (0, 0, 0);
    let mut gate_fail_original = vec![];
    let mut tracked = vec![];
    for d in &dirs {
        let name = d.file_name().unwrap().to_string_lossy().to_string();
        let original = std::fs::read(d.join("ORIGINAL.hwpx")).unwrap();
        let orig_doc = parse_hwpx(&original).unwrap_or_else(|e| panic!("{name}: rhwp cannot open the original: {e}"));
        let orig_pages = DocumentCore::from_bytes(&original).map(|c| c.page_count()).ok();
        let orig_gate = roundtrip_ir_diff(&original).map(|d| d.is_empty()).unwrap_or(false);
        if !orig_gate {
            gate_fail_original.push(name.clone());
        }
        let mut files: Vec<PathBuf> = std::fs::read_dir(d)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "hwpx") && !p.ends_with("ORIGINAL.hwpx"))
            .collect();
        files.sort();
        for f in files {
            total += 1;
            let what = format!("{name}/{}", f.file_name().unwrap().to_string_lossy());
            let bytes = std::fs::read(&f).unwrap();
            let doc = match parse_hwpx(&bytes) {
                Ok(doc) => doc,
                Err(e) => {
                    failures.push(format!("{what}: rhwp cannot open it: {e}"));
                    continue;
                }
            };
            let pages = match DocumentCore::from_bytes(&bytes) {
                Ok(c) => c.page_count(),
                Err(e) => {
                    failures.push(format!("{what}: rhwp cannot lay it out: {e}"));
                    continue;
                }
            };
            opened += 1;
            // What a save through rhwp would do to tracked changes (§10.2: why hwpx stays on direct changes).
            let m = marks(&bytes);
            if m > 0 {
                let saved = serialize_hwpx(&doc).map(|b| marks(&b)).unwrap_or(0);
                tracked.push(format!("{what}: {m} marks in the export, {saved} after an rhwp save"));
            }
            // rhwp's own save gate; only meaningful where the original passes it.
            match roundtrip_ir_diff(&bytes) {
                Ok(diff) if diff.is_empty() => gate += 1,
                Ok(diff) if orig_gate => failures.push(format!(
                    "{what}: rhwp's save gate: {} differences, e.g. {}",
                    diff.differences.len(),
                    diff.differences[0]
                )),
                Ok(_) => {}
                Err(e) => failures.push(format!("{what}: rhwp cannot save it: {e}")),
            }
            if f.ends_with("getput.hwpx") {
                getput.0 += 1;
                let diff = diff_documents(&orig_doc, &doc);
                if diff.is_empty() {
                    getput.1 += 1;
                } else {
                    failures.push(format!("{what}: IR differs from the original: {}", diff.differences[0]));
                }
                if Some(pages) == orig_pages {
                    getput.2 += 1;
                } else {
                    failures.push(format!("{what}: {pages} pages, original {orig_pages:?}"));
                }
            }
        }
    }
    println!("rhwp opened and laid out {opened}/{total} exports of {} files", dirs.len());
    println!("rhwp save gate (roundtrip_ir_diff) clean on {gate}/{total} exports; originals failing it themselves: {gate_fail_original:?}");
    println!(
        "GetPut exports: IR equal to the original {}/{}, same page count {}/{}",
        getput.1, getput.0, getput.2, getput.0
    );
    for t in &tracked {
        println!("tracked changes: {t}");
    }
    for f in &failures {
        println!("FAIL {f}");
    }
    if !failures.is_empty() {
        std::process::exit(1);
    }
}
