//! Package plumbing shared by the XML engines (docx, hwpx): zip parts copied
//! through byte for byte, a lossless XML tree over quick-xml with a canonical
//! form for GetPut and fingerprints, and small helpers. No I/O.

pub mod package;
pub mod xml;

/// Whitespace runs collapsed to one space, ends trimmed.
pub fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// [`squash`]ed and cut to `n` characters (for placeholder summaries and notices).
pub fn clip(s: &str, n: usize) -> String {
    let s = squash(s);
    if s.chars().count() > n {
        s.chars().take(n).collect::<String>() + "…"
    } else {
        s
    }
}
