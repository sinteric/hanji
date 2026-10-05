//! A theme colour as the `#RRGGBB` the document's theme gives it (the
//! conversion to another format needs one where the target has no themes).
//! The expected values are Word's own palette for the Office 2013 theme: the
//! colour picker's "Lighter 40%", "Darker 25%", … of each theme colour.

use hanji_core::Part;
use hanji_docx::format::Theme;
use hanji_format::vocab::{Color, ColorBase, Tint};

const THEME: &str = r#"<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:themeElements><a:clrScheme name="Office"><a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1><a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="44546A"/></a:dk2><a:lt2><a:srgbClr val="E7E6E6"/></a:lt2><a:accent1><a:srgbClr val="4472C4"/></a:accent1><a:accent2><a:srgbClr val="ED7D31"/></a:accent2><a:hlink><a:srgbClr val="0563C1"/></a:hlink><a:folHlink><a:srgbClr val="954F72"/></a:folHlink></a:clrScheme></a:themeElements></a:theme>"#;

fn part(name: &str, xml: &str) -> Part {
    Part { name: name.into(), data: xml.as_bytes().to_vec(), dos_time: 0, external_attr: 0, deflate: true }
}

fn theme(settings: Option<&str>) -> Theme {
    let mut parts = vec![part("word/theme/theme1.xml", THEME)];
    parts.extend(settings.map(|s| part("word/settings.xml", s)));
    Theme::read(&parts)
}

fn color(name: &str, tint: Tint) -> Color {
    Color { base: ColorBase::Theme(name.into()), tint, alpha: None }
}

/// The resolved colour as `RRGGBB`.
fn hex(c: Option<Color>) -> String {
    match c.expect("resolved").base {
        ColorBase::Rgb([r, g, b]) => format!("{r:02X}{g:02X}{b:02X}"),
        other => panic!("not RGB: {other:?}"),
    }
}

/// Within one step of a channel: Word rounds a half either way.
fn close(got: &str, want: &str) -> bool {
    (0..3).all(|k| {
        let ch = |s: &str| i32::from(u8::from_str_radix(&s[2 * k..2 * k + 2], 16).unwrap());
        (ch(got) - ch(want)).abs() <= 1
    })
}

#[test]
fn a_theme_colour_is_its_theme_value() {
    let t = theme(None);
    for (name, want) in [
        ("tx1", "000000"),
        ("bg1", "FFFFFF"),
        ("tx2", "44546A"),
        ("bg2", "E7E6E6"),
        ("accent1", "4472C4"),
        ("accent2", "ED7D31"),
        ("hlink", "0563C1"),
        ("folHlink", "954F72"),
        // The scheme's own names are the same colours.
        ("dk1", "000000"),
        ("lt1", "FFFFFF"),
        ("dk2", "44546A"),
        ("lt2", "E7E6E6"),
    ] {
        assert_eq!(hex(t.resolve(&color(name, Tint::None))), want, "{name}");
    }
}

#[test]
fn lighter_and_darker_are_offices_own_values() {
    let t = theme(None);
    for (name, tint, want) in [
        ("accent1", Tint::Lighter(80), "D9E2F3"),
        ("accent1", Tint::Lighter(60), "B4C7E7"),
        ("accent1", Tint::Lighter(40), "8FAADC"),
        ("accent1", Tint::Darker(25), "2F5597"),
        ("accent1", Tint::Darker(50), "203864"),
        ("accent2", Tint::Lighter(40), "F4B183"),
        ("accent2", Tint::Darker(25), "C55A11"),
        ("tx2", Tint::Lighter(80), "D6DCE5"),
        ("tx2", Tint::Lighter(40), "8497B0"),
        ("tx2", Tint::Darker(50), "222A35"),
        ("bg1", Tint::Darker(5), "F2F2F2"),
        ("bg1", Tint::Darker(15), "D9D9D9"),
        ("bg1", Tint::Darker(25), "BFBFBF"),
        ("bg1", Tint::Darker(50), "7F7F7F"),
        ("bg2", Tint::Darker(10), "D0CECE"),
        ("bg2", Tint::Darker(25), "AEAAAA"),
        ("bg2", Tint::Darker(75), "3A3838"),
        ("tx1", Tint::Lighter(50), "7F7F7F"),
    ] {
        let got = hex(t.resolve(&color(name, tint)));
        assert!(close(&got, want), "{name} {tint:?}: {got} is not {want}");
    }
}

#[test]
fn opacity_is_kept_and_what_a_theme_cannot_say_is_not_resolved() {
    let t = theme(None);
    let faded = Color { alpha: Some(50), ..color("accent1", Tint::None) };
    let r = t.resolve(&faded).unwrap();
    assert_eq!((r.base, r.alpha), (ColorBase::Rgb([0x44, 0x72, 0xC4]), Some(50)));
    // Not a theme colour, not in this theme, another transform, or no theme at all.
    assert!(t.resolve(&Color::rgb(1, 2, 3)).is_none());
    assert!(t.resolve(&color("accent5", Tint::None)).is_none());
    assert!(t.resolve(&color("accent1", Tint::Other)).is_none());
    assert!(Theme::read(&[]).resolve(&color("accent1", Tint::None)).is_none());
}

#[test]
fn a_dark_theme_mapping_swaps_text_and_background() {
    let settings = r#"<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:clrSchemeMapping w:bg1="dark1" w:t1="light1" w:bg2="dark2" w:t2="light2" w:accent1="accent1"/></w:settings>"#;
    let t = theme(Some(settings));
    assert_eq!(hex(t.resolve(&color("tx1", Tint::None))), "FFFFFF");
    assert_eq!(hex(t.resolve(&color("bg1", Tint::None))), "000000");
    assert_eq!(hex(t.resolve(&color("tx2", Tint::None))), "E7E6E6");
    assert_eq!(hex(t.resolve(&color("bg2", Tint::None))), "44546A");
    assert_eq!(hex(t.resolve(&color("accent1", Tint::None))), "4472C4");
}
