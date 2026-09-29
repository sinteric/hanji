//! Helpers the xlsx tests share.
#![allow(dead_code)]

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
