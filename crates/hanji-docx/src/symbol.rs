//! DOCX symbols (`w:sym`) whose character the model can edit: the stored
//! code point is that character; font, hex and the original element stay as
//! a `Wrap`. Untouched content writes the original `w:sym` (GetPut). Editing
//! the character to something else writes ordinary `w:t`; deleting it
//! removes the symbol. A missing or unreadable `w:char` (not hex, a C0
//! control, a surrogate) stays `<keep kind="symbol"/>`.
//!
//! The text uses the code Word stored (`w:char="F094"` → U+F094), not an
//! inferred lookalike. Adobe Symbol encoding has no 0x94, and PREVIEW.md
//! does not implement general symbol-font encoding.

use hanji_core::{Entry, Kind};
use hanji_package::xml::{self, Element};

/// A wrap whose XML is a `w:sym`.
pub(crate) fn is_symbol(e: &Entry) -> bool {
    e.kind == Kind::Wrap && e.meta.tag == "w:sym"
}

/// The character a `w:sym` stores, when that is one ordinary model character.
pub fn shown(el: &Element) -> Option<char> {
    if !el.is("w:sym") || el.has_elements() {
        return None;
    }
    let hex = el.get("w:char")?;
    let n = u32::from_str_radix(hex.trim(), 16).ok()?;
    let c = char::from_u32(n)?;
    // A tab or newline inside a run would come back as `w:tab` / `w:br`.
    ((c as u32) >= 0x20).then_some(c)
}

/// `xml` of a stored `w:sym` when `body` is still that character. Any other
/// body is ordinary text (the caller writes `w:t`).
pub fn with_body(xml: &str, body: &str) -> Option<String> {
    let e = xml::fragment(xml);
    let orig = shown(&e)?;
    let mut chars = body.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c == orig => Some(xml.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hanji_package::xml::fragment;

    #[test]
    fn a_symbol_code_is_the_stored_character() {
        let e = fragment(r#"<w:sym w:font="Symbol" w:char="F094"/>"#);
        assert_eq!(shown(&e), Some('\u{F094}'));
    }

    #[test]
    fn an_ascii_code_is_that_character() {
        let e = fragment(r#"<w:sym w:font="Symbol" w:char="002A"/>"#);
        assert_eq!(shown(&e), Some('*'));
    }

    #[test]
    fn a_tab_code_stays_unshown() {
        let e = fragment(r#"<w:sym w:font="Symbol" w:char="0009"/>"#);
        assert!(shown(&e).is_none());
    }

    #[test]
    fn a_missing_char_stays_unshown() {
        let e = fragment(r#"<w:sym w:font="Symbol"/>"#);
        assert!(shown(&e).is_none());
    }

    #[test]
    fn with_body_keeps_the_fragment_when_unchanged() {
        let xml = r#"<w:sym w:font="Symbol" w:char="F094"/>"#;
        assert_eq!(with_body(xml, "\u{F094}").as_deref(), Some(xml));
    }

    #[test]
    fn with_body_is_none_when_the_character_changed() {
        let xml = r#"<w:sym w:font="Symbol" w:char="F094"/>"#;
        assert!(with_body(xml, "X").is_none());
        assert!(with_body(xml, "ab").is_none());
        assert!(with_body(xml, "").is_none());
    }
}
