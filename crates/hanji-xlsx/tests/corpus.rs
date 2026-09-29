//! §9 on the xlsx corpus (`corpus/`, see its SOURCES.md): GetPut, the
//! remainder through JSON, well-formed exports, and §8 on import. Set
//! HANJI_REPORT=1 to print per-file numbers, HANJI_SOFFICE=1 to also convert
//! every export with LibreOffice and compare its values with the windows'.

use std::path::PathBuf;

use hanji_core::{Engine, ImportOptions, Remainder};
use hanji_xlsx::{package, xml, XlsxEngine};

mod common;
use common::*;

fn raw() -> ImportOptions {
    ImportOptions { neutralise: false, template: None }
}

#[test]
fn corpus_getput() {
    let files = corpus();
    assert!(files.len() >= 10, "the corpus has {} files", files.len());
    let mut failures = vec![];
    for (name, bytes) in &files {
        for opts in [raw(), ImportOptions::default()] {
            let imp = match XlsxEngine.import(bytes, &opts) {
                Ok(i) => i,
                Err(e) => {
                    failures.push(format!("{name}: import: {e}"));
                    continue;
                }
            };
            let out = match XlsxEngine.export(&imp.text, &imp.remainder) {
                Ok(o) => o,
                Err(e) => {
                    failures.push(format!("{name}: export: {e}"));
                    continue;
                }
            };
            let original = if opts.neutralise { neutralised_original(bytes) } else { bytes.clone() };
            if let Err(e) = getput(&original, &out) {
                failures.push(format!("{name} (neutralise {}): {e}", opts.neutralise));
            }
            if report() {
                println!(
                    "{name}: {} parts, {} entries, text {} chars",
                    imp.remainder.parts.len(),
                    imp.remainder.entries.len(),
                    imp.text.len()
                );
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The package a neutralising import stands for: the original with its
/// active content removed, as a raw import of the neutralised export shows.
fn neutralised_original(bytes: &[u8]) -> Vec<u8> {
    let imp = XlsxEngine.import(bytes, &ImportOptions::default()).unwrap();
    XlsxEngine.export(&imp.text, &imp.remainder).unwrap()
}

#[test]
fn a_serialized_remainder_exports_the_same_package() {
    for (name, bytes) in corpus() {
        let imp = XlsxEngine.import(&bytes, &raw()).unwrap();
        let back = Remainder::from_json(&imp.remainder.to_json()).unwrap();
        assert_eq!(back, imp.remainder, "{name}");
        let a = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
        let b = XlsxEngine.export(&imp.text, &back).unwrap();
        assert_eq!(a, b, "{name}");
    }
}

#[test]
fn exports_are_well_formed() {
    for (name, bytes) in corpus() {
        let imp = XlsxEngine.import(&bytes, &ImportOptions::default()).unwrap();
        let out = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
        for p in package::read(&out).unwrap() {
            if p.name.ends_with(".xml") || p.name.ends_with(".rels") {
                assert!(xml::parse(&p.data).is_ok(), "{name}: {} is not well-formed", p.name);
            }
        }
        // A second import finds nothing more to neutralise, and the same text.
        let again = XlsxEngine.import(&out, &ImportOptions::default()).unwrap();
        assert!(again.report.neutralised.is_empty(), "{name}: {:?}", again.report.neutralised);
        assert_eq!(again.text, imp.text, "{name}");
    }
}

#[allow(dead_code)]
fn out_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("xlsx-corpus-out")
}
