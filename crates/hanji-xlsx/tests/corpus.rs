//! §9 on the xlsx corpus (`corpus/`, see its SOURCES.md): GetPut, the
//! remainder through JSON, well-formed exports, and §8 on import. Set
//! HANJI_REPORT=1 to print per-file numbers, HANJI_SOFFICE=1 to also convert
//! every export with LibreOffice and compare its values with the windows'.

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

/// Files where LibreOffice shows cells otherwise than the windows (and than
/// Excel's cached values), in the original as in the export.
#[cfg(not(target_os = "wasi"))]
const LIBREOFFICE_DIFFERS: &[(&str, &str)] = &[
    ("ExcelPivotTableSample.xlsx", "LibreOffice rebuilds pivot tables with its own labels"),
    (
        "GeneralFormatTests.xlsx",
        "General shows 10 digits in Excel (its cached TEXT() results agree), 11 in LibreOffice",
    ),
    ("StructuredReferences.xlsx", "LibreOffice cannot compute these structured references"),
    ("xlookup.xlsx", "this LibreOffice has no XLOOKUP"),
    ("rx_dynamic_array01.xlsx", "the writer cached 0; LibreOffice recalculates"),
];

/// With HANJI_SOFFICE=1: LibreOffice converts every original and every
/// export (neutralised and recalculated, as a model's round trip gives it) to
/// CSV, values as shown. Every cell of the export must show as the original's
/// does, and the export's cells are compared with what the engine's windows show.
#[cfg(not(target_os = "wasi"))]
#[test]
fn libreoffice_reads_every_export() {
    if std::env::var("HANJI_SOFFICE").is_err() {
        return;
    }
    let (mut files, mut same, mut total, mut kept, mut shown) = (0, 0, 0, 0, 0);
    let (mut failures, mut changed) = (vec![], vec![]);
    for (name, bytes) in corpus() {
        let imp = XlsxEngine.import(&bytes, &ImportOptions::default()).unwrap();
        let (rem, _) = XlsxEngine::recalculate(&imp.remainder).unwrap();
        let out = XlsxEngine.export(&imp.text, &rem).unwrap();
        let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("xlsx-corpus-soffice").join(&name);
        let before = soffice_csv(&bytes, &dir.join("original"));
        let after = soffice_csv(&out, &dir.join("export"));
        match (before, after) {
            (Some(b), Some(a)) => {
                let (mut n, mut moved) = (0, vec![]);
                for ((sb, rb), (sa, ra)) in b.iter().zip(&a) {
                    for r in 0..rb.len().max(ra.len()) {
                        let (xb, xa) = (rb.get(r).cloned().unwrap_or_default(), ra.get(r).cloned().unwrap_or_default());
                        for c in 0..xb.len().max(xa.len()) {
                            let (vb, va) = (xb.get(c).map_or("", String::as_str), xa.get(c).map_or("", String::as_str));
                            n += usize::from(!vb.is_empty() || !va.is_empty());
                            if vb != va {
                                moved.push(format!(
                                    "{sa}!{}{}: {vb:?} became {va:?}",
                                    hanji_core::cells::col_letters(c as u32),
                                    r + 1
                                ));
                            }
                        }
                    }
                    if sb != sa {
                        moved.push(format!("sheet {sb} became {sa}"));
                    }
                }
                if b.len() != a.len() {
                    moved.push(format!("{} sheets became {}", b.len(), a.len()));
                }
                kept += n - moved.len().min(n);
                shown += n;
                if !moved.is_empty() {
                    changed.push(format!("{name}: {}", moved.join("; ")));
                }
            }
            _ => failures.push(format!("{name}: LibreOffice could not convert the original or the export")),
        }
        match soffice_agrees(&rem, &out, &dir.join("windows")) {
            Some((s, t, diffs)) => {
                files += 1;
                same += s;
                total += t;
                if s != t && !LIBREOFFICE_DIFFERS.iter().any(|(f, _)| *f == name) {
                    failures.push(format!("{name}: LibreOffice shows {} cells otherwise: {}", t - s, diffs.join("; ")));
                }
                if report() {
                    println!(
                        "{name}: {s}/{t} cells as LibreOffice shows them{}",
                        if diffs.is_empty() { String::new() } else { format!("; {}", diffs.join("; ")) }
                    );
                }
            }
            None => failures.push(format!("{name}: LibreOffice could not convert the export")),
        }
    }
    println!("LibreOffice: {files} exports converted; {kept}/{shown} cells show as in the original; {same}/{total} cells as the windows show them");
    failures.extend(changed.iter().map(|c| format!("LibreOffice shows the export otherwise than the original: {c}")));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
