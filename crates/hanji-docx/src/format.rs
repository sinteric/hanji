//! Direct formatting (DESIGN.md §5.2, F2) in WordprocessingML: the
//! vocabulary read from and written to `w:pPr`, `w:rPr`, `w:tcPr` and the
//! paragraph styles of `word/styles.xml`.
//!
//! Units: twips (1 pt = 20) for indents and spacing, half-points for font
//! sizes, eighths of a point for border widths (0.25–12 pt), 240ths of a
//! line for auto line spacing. The text's lengths are hundredths of a point.
//! Writing changes only the child (or attribute) a property is, in schema
//! order, and keeps every other child and attribute.

use std::collections::HashMap;

use hanji_core::{Part, StyleSet};
use hanji_format::styled::{self, StyleTable};
use hanji_format::vocab::{Border, Color, ColorBase, Fill, LineSpacing, Tint};
use hanji_format::{Key, Props, StyleLine, TablePlace, Value};
use hanji_package::package;

use crate::ooxml::{insert_ordered, remove_child, PPR_ORDER, RPR_ORDER, TBLPR_ORDER, TCPR_ORDER};
use crate::xml::{self, Element, Node};

// ---------------------------------------------------------------- theme

/// The theme's fonts and colours (`word/theme/theme1.xml`).
#[derive(Clone, Debug, Default)]
pub struct Theme {
    /// `minor` / `major` → (latin, east Asian, Hangul script font).
    fonts: HashMap<&'static str, (String, String, String)>,
    /// `tx1`, `accent1`, … → `RRGGBB`.
    colors: HashMap<String, String>,
}

impl Theme {
    pub fn read(parts: &[Part]) -> Theme {
        let mut t = Theme::default();
        let name = parts.iter().map(|p| p.name.as_str()).find(|n| n.starts_with("word/theme/") && n.ends_with(".xml"));
        let Some(root) = name.and_then(|n| package::get(parts, n)).and_then(|d| xml::parse(d).ok()).map(|d| d.root)
        else {
            return t;
        };
        root.walk(&mut |e| match e.local() {
            "clrScheme" => {
                for c in e.elements() {
                    let v = c
                        .elements()
                        .find_map(|x| if x.local() == "sysClr" { x.get("lastClr") } else { x.get("val") })
                        .unwrap_or_default();
                    let n = match c.local() {
                        "dk1" => "tx1",
                        "lt1" => "bg1",
                        "dk2" => "tx2",
                        "lt2" => "bg2",
                        other => other,
                    };
                    t.colors.insert(n.to_string(), v.to_uppercase());
                }
            }
            "minorFont" | "majorFont" => {
                let get = |local: &str| {
                    e.elements().find(|x| x.local() == local).and_then(|x| x.get("typeface")).unwrap_or_default()
                };
                let hang = e
                    .elements()
                    .find(|x| x.local() == "font" && x.get("script").as_deref() == Some("Hang"))
                    .and_then(|x| x.get("typeface"))
                    .unwrap_or_default();
                let key = if e.local() == "minorFont" { "minor" } else { "major" };
                t.fonts.insert(key, (get("latin"), get("ea"), hang));
            }
            _ => {}
        });
        t
    }

    /// A theme font reference (`minorEastAsia`, `majorHAnsi`, …) → its typeface.
    fn font(&self, r: &str) -> Option<String> {
        let (which, rest) =
            if let Some(x) = r.strip_prefix("minor") { ("minor", x) } else { ("major", r.strip_prefix("major")?) };
        let (latin, ea, hang) = self.fonts.get(which)?;
        let f = if rest == "EastAsia" {
            [ea, hang, latin].into_iter().find(|f| !f.is_empty())
        } else {
            Some(latin).filter(|f| !f.is_empty())
        };
        f.cloned()
    }

    fn rgb(&self, name: &str) -> String {
        self.colors.get(name).cloned().unwrap_or_else(|| "000000".into())
    }
}

// ---------------------------------------------------------------- reading

/// Hundredths of a point in twips.
fn tw(v: &str) -> Option<i64> {
    v.parse::<f64>().ok().map(|x| (x * 5.0).round() as i64)
}

fn attr(e: &Element, names: &[&str]) -> Option<String> {
    names.iter().find_map(|n| e.get(n))
}

const THEME_NAMES: &[(&str, &str)] = &[
    ("dark1", "tx1"),
    ("text1", "tx1"),
    ("light1", "bg1"),
    ("background1", "bg1"),
    ("dark2", "tx2"),
    ("text2", "tx2"),
    ("light2", "bg2"),
    ("background2", "bg2"),
    ("accent1", "accent1"),
    ("accent2", "accent2"),
    ("accent3", "accent3"),
    ("accent4", "accent4"),
    ("accent5", "accent5"),
    ("accent6", "accent6"),
    ("hyperlink", "hlink"),
    ("followedHyperlink", "folHlink"),
];

/// A colour from `val` (`RRGGBB` or `auto`) and theme attributes.
fn color(val: Option<String>, theme: Option<String>, tint: Option<String>, shade: Option<String>) -> Option<Color> {
    if let Some(t) = theme {
        let name = THEME_NAMES.iter().find(|x| x.0 == t).map_or("tx1", |x| x.1);
        let pct = |h: &str| u8::from_str_radix(h, 16).ok().map(|b| ((1.0 - b as f64 / 255.0) * 100.0).round() as i64);
        let tint = match (tint.as_deref().and_then(pct), shade.as_deref().and_then(pct)) {
            (Some(_), Some(_)) => Tint::Other,
            (Some(p), None) if (1..=99).contains(&p) => Tint::Lighter(p as u8),
            (None, Some(p)) if (1..=99).contains(&p) => Tint::Darker(p as u8),
            (Some(0), None) | (None, Some(0)) | (None, None) => Tint::None,
            _ => Tint::Other,
        };
        return Some(Color { base: ColorBase::Theme(name.into()), tint, alpha: None });
    }
    let v = val.filter(|v| v != "auto")?;
    Color::parse(&format!("#{v}")).ok()
}

/// A border element (`w:top`, …): none, or its line.
fn border(e: &Element) -> Border {
    let v = e.get("w:val").unwrap_or_default();
    if matches!(v.as_str(), "nil" | "none" | "") {
        return Border::None;
    }
    let sz: i64 = e.get("w:sz").and_then(|s| s.parse().ok()).unwrap_or(4).max(2);
    let style = match v.as_str() {
        "single" | "thick" => "solid",
        "dashed" | "dashSmallGap" => "dashed",
        "dotted" => "dotted",
        "double" => "double",
        "dotDash" => "dash-dot",
        "dotDotDash" => "dash-dot-dot",
        "wave" | "doubleWave" => "wave",
        "threeDEmboss" | "threeDEngrave" | "outset" | "inset" => "3d",
        "thinThickSmallGap" | "thinThickMediumGap" | "thinThickLargeGap" => "thin-thick",
        "thickThinSmallGap" | "thickThinMediumGap" | "thickThinLargeGap" => "thick-thin",
        _ => "triple",
    };
    let c = color(e.get("w:color"), e.get("w:themeColor"), e.get("w:themeTint"), e.get("w:themeShade"))
        .unwrap_or_else(|| Color::rgb(0, 0, 0));
    Border::Line { width: (sz as f64 * 12.5).round() as i64, style: style.into(), color: c }
}

/// `w:shd` as a fill.
fn shd(e: &Element) -> Fill {
    let v = e.get("w:val").unwrap_or_else(|| "clear".into());
    match v.as_str() {
        "nil" => Fill::None,
        "clear" => color(e.get("w:fill"), e.get("w:themeFill"), e.get("w:themeFillTint"), e.get("w:themeFillShade"))
            .map_or(Fill::None, Fill::Color),
        "solid" => color(e.get("w:color"), e.get("w:themeColor"), e.get("w:themeTint"), e.get("w:themeShade"))
            .map_or(Fill::None, Fill::Color),
        _ => Fill::Pattern,
    }
}

const SIDES: [(Key, &[&str]); 4] = [
    (Key::BorderTop, &["w:top"]),
    (Key::BorderRight, &["w:right", "w:end"]),
    (Key::BorderBottom, &["w:bottom"]),
    (Key::BorderLeft, &["w:left", "w:start"]),
];

fn sides(e: &Element, out: &mut Props) {
    for (k, names) in SIDES {
        if let Some(b) = names.iter().find_map(|n| e.child(n)) {
            out.set(k, Value::Border(border(b)));
        }
    }
}

/// What a `w:pPr` sets: layout, shading, borders. `size`: the paragraph's
/// font size (hundredths of a point), for indents written in characters.
pub fn ppr_props(ppr: &Element, size: i64) -> Props {
    let mut p = Props::new();
    if let Some(jc) = ppr.child("w:jc").and_then(|j| j.get("w:val")) {
        let a = match jc.as_str() {
            "center" => "center",
            "right" | "end" => "right",
            "both" | "lowKashida" | "mediumKashida" | "highKashida" => "justify",
            "distribute" | "thaiDistribute" => "distribute",
            _ => "left",
        };
        p.set(Key::Align, Value::Choice(a.into()));
    }
    if let Some(ind) = ppr.child("w:ind") {
        let chars =
            |n: &str| ind.get(n).and_then(|v| v.parse::<f64>().ok()).map(|c| (c / 100.0 * size as f64).round() as i64);
        let left = chars("w:leftChars")
            .or_else(|| chars("w:startChars"))
            .or_else(|| attr(ind, &["w:left", "w:start"]).and_then(|v| tw(&v)));
        if let Some(l) = left {
            p.set(Key::IndentLeft, Value::Len(l));
        }
        let right = chars("w:rightChars")
            .or_else(|| chars("w:endChars"))
            .or_else(|| attr(ind, &["w:right", "w:end"]).and_then(|v| tw(&v)));
        if let Some(r) = right {
            p.set(Key::IndentRight, Value::Len(r));
        }
        let hanging = chars("w:hangingChars").or_else(|| ind.get("w:hanging").and_then(|v| tw(&v)));
        let first = chars("w:firstLineChars").or_else(|| ind.get("w:firstLine").and_then(|v| tw(&v)));
        match (hanging, first) {
            (Some(h), _) if h != 0 => p.set(Key::FirstLine, Value::Len(-h)),
            (_, Some(f)) => p.set(Key::FirstLine, Value::Len(f)),
            (Some(h), None) => p.set(Key::FirstLine, Value::Len(-h)),
            _ => {}
        }
    }
    if let Some(sp) = ppr.child("w:spacing") {
        let lines =
            |n: &str| sp.get(n).and_then(|v| v.parse::<f64>().ok()).map(|l| (l / 100.0 * 12.0 * 100.0).round() as i64);
        let auto = |n: &str| matches!(sp.get(n).as_deref(), Some("1" | "true" | "on"));
        for (key, n, lines_n, auto_n) in [
            (Key::SpaceBefore, "w:before", "w:beforeLines", "w:beforeAutospacing"),
            (Key::SpaceAfter, "w:after", "w:afterLines", "w:afterAutospacing"),
        ] {
            let v = if auto(auto_n) { Some(1400) } else { lines(lines_n).or_else(|| sp.get(n).and_then(|v| tw(&v))) };
            if let Some(v) = v {
                p.set(key, Value::Len(v));
            }
        }
        if let Some(l) = sp.get("w:line").and_then(|v| v.parse::<f64>().ok()) {
            let ls = match sp.get("w:lineRule").as_deref() {
                Some("exact") => LineSpacing::Exact((l * 5.0).round() as i64),
                Some("atLeast") => LineSpacing::AtLeast((l * 5.0).round() as i64),
                _ => LineSpacing::Percent((l * 10000.0 / 240.0).round() as i64),
            };
            p.set(Key::LineSpacing, Value::Spacing(ls));
        }
    }
    if let Some(s) = ppr.child("w:shd") {
        p.set(Key::Fill, Value::Fill(shd(s)));
    }
    if let Some(b) = ppr.child("w:pBdr") {
        sides(b, &mut p);
    }
    p
}

/// What a `w:rPr` sets: font, size, colour, and (for a style) the flags.
pub fn rpr_props(rpr: &Element, theme: &Theme) -> Props {
    let mut p = Props::new();
    if let Some(f) = rpr.child("w:rFonts") {
        let face = attr(f, &["w:eastAsia"])
            .or_else(|| f.get("w:eastAsiaTheme").and_then(|t| theme.font(&t)))
            .or_else(|| attr(f, &["w:ascii"]))
            .or_else(|| f.get("w:asciiTheme").and_then(|t| theme.font(&t)))
            .or_else(|| attr(f, &["w:hAnsi"]))
            .or_else(|| f.get("w:hAnsiTheme").and_then(|t| theme.font(&t)));
        if let Some(face) = face.filter(|f| !f.trim().is_empty()) {
            p.set(Key::Font, Value::Text(face.trim().to_string()));
        }
    }
    if let Some(sz) = rpr.child("w:sz").and_then(|s| s.get("w:val")).and_then(|v| v.parse::<f64>().ok()) {
        if sz > 0.0 {
            p.set(Key::Size, Value::Len((sz * 50.0).round() as i64));
        }
    }
    if let Some(c) = rpr.child("w:color") {
        let v = color(c.get("w:val"), c.get("w:themeColor"), c.get("w:themeTint"), c.get("w:themeShade"));
        p.set(Key::Color, Value::Color(v.unwrap_or_else(|| Color::rgb(0, 0, 0))));
    }
    for (k, n) in [(Key::Bold, "w:b"), (Key::Italic, "w:i"), (Key::Strike, "w:strike")] {
        if let Some(e) = rpr.child(n) {
            p.set(k, Value::Flag(crate::ooxml::on(Some(e))));
        }
    }
    if let Some(u) = rpr.child("w:u") {
        p.set(Key::Underline, Value::Flag(crate::ooxml::underline_on(Some(u))));
    }
    p
}

/// What a `w:tcPr` sets: fill, borders, vertical alignment.
pub fn tcpr_props(tcpr: &Element) -> Props {
    let mut p = Props::new();
    if let Some(s) = tcpr.child("w:shd") {
        p.set(Key::Fill, Value::Fill(shd(s)));
    }
    if let Some(b) = tcpr.child("w:tcBorders") {
        sides(b, &mut p);
    }
    if let Some(v) = tcpr.child("w:vAlign").and_then(|v| v.get("w:val")) {
        let a = match v.as_str() {
            "center" => "middle",
            "bottom" => "bottom",
            _ => "top",
        };
        p.set(Key::Valign, Value::Choice(a.into()));
    }
    p
}

/// What a `w:tblPr` sets for the table's own position (§5.2): `w:jc`
/// (`start`/`end` as left/right) and a `w:tblInd` in twips. Another `w:jc`
/// value, and a `w:tblInd` of another type (`nil`, `auto`, `pct`), are not
/// shown and stay as the file has them.
pub fn tblpr_place(tblpr: &Element) -> TablePlace {
    let align = tblpr.child("w:jc").and_then(|j| j.get("w:val")).and_then(|v| {
        Some(match v.as_str() {
            "left" | "start" => "left",
            "center" => "center",
            "right" | "end" => "right",
            _ => return None,
        })
    });
    let indent = tblpr
        .child("w:tblInd")
        .filter(|i| i.get("w:type").is_none_or(|t| t == "dxa"))
        .and_then(|i| i.get("w:w"))
        .and_then(|v| v.parse::<i64>().ok())
        .map(|tw| tw * 5);
    TablePlace { align: align.map(String::from), indent }
}

/// The `w:tblPr` children [`tblpr_place`] shows: a fingerprint leaves them out.
pub fn shown_tblpr(tblpr: &Element) -> Vec<&'static str> {
    let p = tblpr_place(tblpr);
    [p.align.is_some().then_some("w:jc"), p.indent.is_some().then_some("w:tblInd")].into_iter().flatten().collect()
}

/// Writes the table's own position on `tblpr` where it differs from what
/// it shows: each changed child only, in schema order, keeping its other
/// attributes. `None` removes the child: the table style's, or the
/// application's, position applies.
pub fn set_tblpr_place(tblpr: &mut Element, want: &TablePlace) {
    let shown = tblpr_place(tblpr);
    if shown.align != want.align {
        match &want.align {
            Some(a) => child(tblpr, "w:jc", TBLPR_ORDER).set("w:val", a),
            None => {
                remove_child(tblpr, "w:jc");
            }
        }
    }
    if shown.indent != want.indent {
        match want.indent {
            Some(n) => {
                let ind = child(tblpr, "w:tblInd", TBLPR_ORDER);
                ind.set("w:w", &twips(n));
                ind.set("w:type", "dxa");
            }
            None => {
                remove_child(tblpr, "w:tblInd");
            }
        }
    }
}

/// The `w:pPr` children the text shows (§5.2): a fingerprint leaves them out.
pub const SHOWN_PPR: [&str; 5] = ["w:jc", "w:ind", "w:spacing", "w:shd", "w:pBdr"];
/// The `w:rPr` children the text shows: marks and text properties.
pub const SHOWN_RPR: [&str; 7] = ["w:b", "w:i", "w:strike", "w:u", "w:rFonts", "w:sz", "w:color"];
/// The `w:tcPr` children the text shows: a cell's box.
pub const SHOWN_TCPR: [&str; 3] = ["w:shd", "w:tcBorders", "w:vAlign"];

// ---------------------------------------------------------------- writing

fn el(name: &str) -> Element {
    Element::new(name)
}

/// A colour as Word writes it: its hex value and, for a theme colour, the
/// theme name and its tint or shade (attribute and hex byte).
type WordColor = (String, Option<(String, Option<(&'static str, String)>)>);

fn hex(c: &Color, theme: &Theme) -> Result<WordColor, String> {
    let pct = |p: u8| format!("{:02X}", ((1.0 - p as f64 / 100.0) * 255.0).round() as u8);
    match &c.base {
        _ if c.alpha.is_some() => Err(format!(
            "{c}: a Word document has no transparent text or shading colours; write the colour without /NN%"
        )),
        ColorBase::Rgb([r, g, b]) => Ok((format!("{r:02X}{g:02X}{b:02X}"), None)),
        ColorBase::Theme(n) => {
            let n = match n.as_str() {
                "dk1" => "tx1",
                "lt1" => "bg1",
                "dk2" => "tx2",
                "lt2" => "bg2",
                x => x,
            };
            let word = match n {
                "tx1" => "text1",
                "bg1" => "background1",
                "tx2" => "text2",
                "bg2" => "background2",
                "hlink" => "hyperlink",
                "folHlink" => "followedHyperlink",
                x => x,
            };
            let t = match c.tint {
                Tint::None => None,
                Tint::Lighter(p) => Some(("tint", pct(p))),
                Tint::Darker(p) => Some(("shade", pct(p))),
                Tint::Other => {
                    return Err(format!(
                        "{c}: a theme colour with another transform is kept as it is, and cannot be written anew"
                    ))
                }
            };
            Ok((theme.rgb(n), Some((word.to_string(), t))))
        }
    }
}

/// A colour's attributes: its value and theme attributes.
struct ColorAttrs {
    val: &'static str,
    theme: &'static str,
    tint: &'static str,
    shade: &'static str,
}

const RUN_COLOR: ColorAttrs =
    ColorAttrs { val: "w:val", theme: "w:themeColor", tint: "w:themeTint", shade: "w:themeShade" };
const LINE_COLOR: ColorAttrs =
    ColorAttrs { val: "w:color", theme: "w:themeColor", tint: "w:themeTint", shade: "w:themeShade" };
const FILL_COLOR: ColorAttrs =
    ColorAttrs { val: "w:fill", theme: "w:themeFill", tint: "w:themeFillTint", shade: "w:themeFillShade" };

/// Sets colour `c` on `e`: the RGB value, and the theme colour with its
/// tint or shade when it is one.
fn set_color(e: &mut Element, c: &Color, theme: &Theme, a: &ColorAttrs) -> Result<(), String> {
    let (rgb, th) = hex(c, theme)?;
    e.set(a.val, &rgb);
    for n in [a.theme, a.tint, a.shade] {
        e.remove_attr(n);
    }
    if let Some((word, t)) = th {
        e.set(a.theme, &word);
        match t {
            Some(("tint", v)) => e.set(a.tint, &v),
            Some((_, v)) => e.set(a.shade, &v),
            None => {}
        }
    }
    Ok(())
}

fn writable(k: Key, v: &Value) -> Result<(), String> {
    if !v.is_writable() {
        return Err(format!(
            "{k}={v} is shown as the file has it and cannot be written anew (a gradient, pattern or picture, a border style other than solid, dashed, dotted, double, dash-dot or dash-dot-dot, or a theme colour with another transform): leave the value as it is, or write another"
        ));
    }
    if let Value::Spacing(LineSpacing::Gap(_)) = v {
        return Err(format!("{k}={v}: Word has no spacing between lines only (\"gap\", Hancom's); write a percent (160%), an exact length (14pt) or \"at-least 14pt\""));
    }
    Ok(())
}

/// The child `name` of `parent`, created in `order` when missing.
fn child<'a>(parent: &'a mut Element, name: &str, order: &[&str]) -> &'a mut Element {
    if parent.child(name).is_none() {
        insert_ordered(parent, el(name), order);
    }
    parent.child_mut(name).unwrap()
}

/// Removes `name` when it has no attribute and no child left.
fn drop_empty(parent: &mut Element, name: &str) {
    if parent.child(name).is_some_and(|c| c.attrs.is_empty() && !c.has_elements()) {
        remove_child(parent, name);
    }
}

fn twips(v: i64) -> String {
    ((v as f64) / 5.0).round().to_string()
}

const PBDR_ORDER: &[&str] = &["top", "left", "start", "bottom", "right", "end", "between", "bar"];
const TCBORDERS_ORDER: &[&str] =
    &["top", "start", "left", "bottom", "end", "right", "insideH", "insideV", "tl2br", "tr2bl"];

/// A side of `w:pBdr` / `w:tcBorders`: written as `b`, or removed.
fn set_side(box_el: &mut Element, k: Key, b: Option<&Border>, theme: &Theme, order: &[&str]) -> Result<(), String> {
    let names = SIDES.iter().find(|s| s.0 == k).unwrap().1;
    let existing = names.iter().find(|n| box_el.child(n).is_some()).copied();
    let Some(b) = b else {
        if let Some(n) = existing {
            remove_child(box_el, n);
        }
        return Ok(());
    };
    let name = existing.unwrap_or(names[0]);
    let mut e = box_el.child(name).cloned().unwrap_or_else(|| el(name));
    match b {
        Border::None => {
            e.attrs.clear();
            e.set("w:val", "nil");
        }
        Border::Line { width, style, color } => {
            let val = match style.as_str() {
                "dashed" => "dashed",
                "dotted" => "dotted",
                "double" => "double",
                "dash-dot" => "dotDash",
                "dash-dot-dot" => "dotDotDash",
                _ => "single",
            };
            e.set("w:val", val);
            let eighths = ((*width as f64) * 8.0 / 100.0).round().clamp(2.0, 96.0) as i64;
            e.set("w:sz", &eighths.to_string());
            if e.get("w:space").is_none() {
                e.set("w:space", "0");
            }
            set_color(&mut e, color, theme, &LINE_COLOR)?;
        }
    }
    remove_child(box_el, name);
    insert_ordered(box_el, e, order);
    Ok(())
}

fn set_shd(parent: &mut Element, f: Option<&Fill>, theme: &Theme, order: &[&str]) -> Result<(), String> {
    let Some(f) = f else {
        remove_child(parent, "w:shd");
        return Ok(());
    };
    let mut e = parent.child("w:shd").cloned().unwrap_or_else(|| el("w:shd"));
    match f {
        Fill::None => {
            e.attrs.clear();
            e.set("w:val", "nil");
        }
        Fill::Color(c) => {
            for a in ["w:themeColor", "w:themeTint", "w:themeShade"] {
                e.remove_attr(a);
            }
            e.set("w:val", "clear");
            e.set("w:color", "auto");
            set_color(&mut e, c, theme, &FILL_COLOR)?;
        }
        _ => unreachable!("checked writable"),
    }
    remove_child(parent, "w:shd");
    insert_ordered(parent, e, order);
    Ok(())
}

/// Writes (or removes, `None`) one paragraph property on `ppr`.
pub fn set_ppr(ppr: &mut Element, k: Key, v: Option<&Value>, theme: &Theme) -> Result<(), String> {
    if let Some(v) = v {
        writable(k, v)?;
    }
    match k {
        Key::Align => match v.and_then(Value::choice) {
            Some(a) => {
                let val = match a {
                    "justify" => "both",
                    x => x,
                };
                child(ppr, "w:jc", PPR_ORDER).set("w:val", val);
            }
            None => {
                remove_child(ppr, "w:jc");
            }
        },
        Key::IndentLeft | Key::IndentRight | Key::FirstLine => {
            let ind = child(ppr, "w:ind", PPR_ORDER);
            match k {
                Key::IndentLeft | Key::IndentRight => {
                    let (plain, alt, chars) = if k == Key::IndentLeft {
                        ("w:left", "w:start", ["w:leftChars", "w:startChars"])
                    } else {
                        ("w:right", "w:end", ["w:rightChars", "w:endChars"])
                    };
                    for c in chars {
                        ind.remove_attr(c);
                    }
                    let name = if ind.get(alt).is_some() { alt } else { plain };
                    match v.and_then(Value::length) {
                        Some(n) => ind.set(name, &twips(n)),
                        None => {
                            ind.remove_attr(plain);
                            ind.remove_attr(alt);
                        }
                    }
                }
                _ => {
                    for a in ["w:firstLine", "w:hanging", "w:firstLineChars", "w:hangingChars"] {
                        ind.remove_attr(a);
                    }
                    match v.and_then(Value::length) {
                        Some(n) if n < 0 => ind.set("w:hanging", &twips(-n)),
                        Some(n) => ind.set("w:firstLine", &twips(n)),
                        None => {}
                    }
                }
            }
            drop_empty(ppr, "w:ind");
        }
        Key::SpaceBefore | Key::SpaceAfter => {
            let sp = child(ppr, "w:spacing", PPR_ORDER);
            let (n, lines, auto) = if k == Key::SpaceBefore {
                ("w:before", "w:beforeLines", "w:beforeAutospacing")
            } else {
                ("w:after", "w:afterLines", "w:afterAutospacing")
            };
            sp.remove_attr(lines);
            sp.remove_attr(auto);
            match v.and_then(Value::length) {
                Some(x) => sp.set(n, &twips(x)),
                None => {
                    sp.remove_attr(n);
                }
            }
            drop_empty(ppr, "w:spacing");
        }
        Key::LineSpacing => {
            let sp = child(ppr, "w:spacing", PPR_ORDER);
            match v {
                Some(Value::Spacing(ls)) => {
                    let (line, rule) = match ls {
                        LineSpacing::Percent(p) => ((*p as f64 * 240.0 / 10000.0).round() as i64, "auto"),
                        LineSpacing::Exact(l) => (((*l as f64) / 5.0).round() as i64, "exact"),
                        LineSpacing::AtLeast(l) => (((*l as f64) / 5.0).round() as i64, "atLeast"),
                        LineSpacing::Gap(_) => unreachable!("checked writable"),
                    };
                    sp.set("w:line", &line.to_string());
                    sp.set("w:lineRule", rule);
                }
                _ => {
                    sp.remove_attr("w:line");
                    sp.remove_attr("w:lineRule");
                }
            }
            drop_empty(ppr, "w:spacing");
        }
        Key::Fill => {
            let f = match v {
                Some(Value::Fill(f)) => Some(f),
                _ => None,
            };
            set_shd(ppr, f, theme, PPR_ORDER)?;
        }
        Key::BorderTop | Key::BorderRight | Key::BorderBottom | Key::BorderLeft => {
            let b = match v {
                Some(Value::Border(b)) => Some(b),
                _ => None,
            };
            if b.is_some() || ppr.child("w:pBdr").is_some() {
                set_side(child(ppr, "w:pBdr", PPR_ORDER), k, b, theme, PBDR_ORDER)?;
                drop_empty(ppr, "w:pBdr");
            }
        }
        _ => set_rpr(ppr, k, v, theme)?,
    }
    Ok(())
}

/// Writes (or removes) one text property on `rpr`.
pub fn set_rpr(rpr: &mut Element, k: Key, v: Option<&Value>, theme: &Theme) -> Result<(), String> {
    if let Some(v) = v {
        writable(k, v)?;
    }
    match k {
        Key::Font => match v.and_then(Value::choice) {
            Some(face) => {
                let f = child(rpr, "w:rFonts", RPR_ORDER);
                for a in ["w:asciiTheme", "w:hAnsiTheme", "w:eastAsiaTheme"] {
                    f.remove_attr(a);
                }
                for a in ["w:ascii", "w:hAnsi", "w:eastAsia"] {
                    f.set(a, face);
                }
            }
            None => {
                if let Some(f) = rpr.child_mut("w:rFonts") {
                    for a in ["w:ascii", "w:hAnsi", "w:eastAsia", "w:asciiTheme", "w:hAnsiTheme", "w:eastAsiaTheme"] {
                        f.remove_attr(a);
                    }
                }
                drop_empty(rpr, "w:rFonts");
            }
        },
        Key::Size => match v.and_then(Value::length) {
            Some(n) => {
                let half = ((n as f64) / 50.0).round().max(1.0) as i64;
                child(rpr, "w:sz", RPR_ORDER).set("w:val", &half.to_string());
            }
            None => {
                remove_child(rpr, "w:sz");
            }
        },
        Key::Color => match v {
            Some(Value::Color(c)) => {
                let e = child(rpr, "w:color", RPR_ORDER);
                set_color(e, c, theme, &RUN_COLOR)?;
            }
            _ => {
                remove_child(rpr, "w:color");
            }
        },
        Key::Bold | Key::Italic | Key::Strike | Key::Underline => {
            let name = match k {
                Key::Bold => "w:b",
                Key::Italic => "w:i",
                Key::Strike => "w:strike",
                _ => "w:u",
            };
            remove_child(rpr, name);
            if let Some(on) = v.and_then(Value::flag) {
                let mut e = el(name);
                match (k, on) {
                    (Key::Underline, true) => e.set("w:val", "single"),
                    (Key::Underline, false) => e.set("w:val", "none"),
                    (_, false) => e.set("w:val", "0"),
                    _ => {}
                }
                insert_ordered(rpr, e, RPR_ORDER);
            }
        }
        other => return Err(format!("{other} is not a property of text")),
    }
    Ok(())
}

/// Writes (or removes) one box property on a cell's `tcpr`.
pub fn set_tcpr(tcpr: &mut Element, k: Key, v: Option<&Value>, theme: &Theme) -> Result<(), String> {
    if let Some(v) = v {
        writable(k, v)?;
    }
    match k {
        Key::Fill => {
            let f = match v {
                Some(Value::Fill(f)) => Some(f),
                _ => None,
            };
            set_shd(tcpr, f, theme, TCPR_ORDER)
        }
        Key::Valign => {
            match v.and_then(Value::choice) {
                Some(a) => {
                    let val = match a {
                        "middle" => "center",
                        x => x,
                    };
                    child(tcpr, "w:vAlign", TCPR_ORDER).set("w:val", val);
                }
                None => {
                    remove_child(tcpr, "w:vAlign");
                }
            }
            Ok(())
        }
        _ => {
            let b = match v {
                Some(Value::Border(b)) => Some(b),
                _ => None,
            };
            if b.is_some() || tcpr.child("w:tcBorders").is_some() {
                set_side(child(tcpr, "w:tcBorders", TCPR_ORDER), k, b, theme, TCBORDERS_ORDER)?;
                drop_empty(tcpr, "w:tcBorders");
            }
            Ok(())
        }
    }
}

/// Makes `current` (what an element sets, as shown) into `want`, one
/// property at a time with `set`.
pub fn apply(
    e: &mut Element,
    current: &Props,
    want: &Props,
    set: fn(&mut Element, Key, Option<&Value>, &Theme) -> Result<(), String>,
    theme: &Theme,
) -> Result<(), String> {
    let keys: Vec<Key> = current.keys().chain(want.keys()).collect();
    let mut done = vec![];
    for k in keys {
        if done.contains(&k) || current.get(k) == want.get(k) {
            continue;
        }
        done.push(k);
        set(e, k, want.get(k), theme)?;
    }
    Ok(())
}

// ---------------------------------------------------------------- styles

/// Word's values where `docDefaults` and the styles set nothing.
fn word_defaults(theme: &Theme) -> Props {
    let mut p = styled::implicit();
    p.set(Key::LineSpacing, Value::Spacing(LineSpacing::Percent(10000)));
    p.set(Key::Size, Value::Len(1000));
    p.set(Key::Color, Value::Color(Color::rgb(0, 0, 0)));
    let font =
        theme.font("minorEastAsia").or_else(|| theme.font("minorHAnsi")).unwrap_or_else(|| "Times New Roman".into());
    p.set(Key::Font, Value::Text(font));
    p
}

/// `word/styles.xml` parsed, with each paragraph style's values.
pub struct Styles {
    pub doc: Option<xml::Doc>,
    pub theme: Theme,
    /// docDefaults over Word's defaults: what a style based on nothing has.
    base: Props,
}

/// The keys a paragraph style line shows.
pub fn style_keys() -> &'static [Key] {
    &styled::STYLE_KEYS
}

impl Styles {
    pub fn read(parts: &[Part]) -> Styles {
        let theme = Theme::read(parts);
        let doc = package::get(parts, "word/styles.xml").and_then(|d| xml::parse(d).ok());
        let mut base = word_defaults(&theme);
        if let Some(dd) = doc.as_ref().and_then(|d| d.root.child("w:docDefaults")) {
            if let Some(r) = dd.child("w:rPrDefault").and_then(|x| x.child("w:rPr")) {
                base = base.overlay(&rpr_props(r, &theme));
            }
            let size = base.get(Key::Size).and_then(Value::length).unwrap_or(1000);
            if let Some(p) = dd.child("w:pPrDefault").and_then(|x| x.child("w:pPr")) {
                base = base.overlay(&ppr_props(p, size));
            }
        }
        Styles { doc, theme, base }
    }

    fn styles(&self) -> impl Iterator<Item = &Element> {
        self.doc.iter().flat_map(|d| d.root.elements().filter(|e| e.is("w:style")))
    }

    fn by_id(&self, id: &str) -> Option<&Element> {
        self.styles().find(|s| s.get("w:styleId").as_deref() == Some(id))
    }

    /// What style `st` sets itself (its `w:pPr` and `w:rPr`), `size` the
    /// font size its paragraphs have.
    fn own(&self, st: &Element, size: i64) -> Props {
        let mut p = Props::new();
        if let Some(r) = st.child("w:rPr") {
            p = p.overlay(&rpr_props(r, &self.theme));
        }
        if let Some(pp) = st.child("w:pPr") {
            p = p.overlay(&ppr_props(pp, size));
        }
        p.only(style_keys())
    }

    /// The values of the paragraph style with id `id`: docDefaults, then
    /// its `basedOn` chain.
    pub fn values(&self, id: &str) -> Props {
        let mut chain = vec![];
        let mut cur = self.by_id(id);
        while let Some(s) = cur {
            if chain.len() > 32 || chain.iter().any(|x: &&Element| std::ptr::eq(*x, s)) {
                break;
            }
            chain.push(s);
            cur = s.child("w:basedOn").and_then(|b| b.get("w:val")).and_then(|b| self.by_id(&b));
        }
        let mut p = self.base.clone();
        for s in chain.iter().rev() {
            let size = p.get(Key::Size).and_then(Value::length).unwrap_or(1000);
            let own = self.own(s, size);
            p = p.overlay(&own);
        }
        p
    }

    /// Each paragraph style's line (§5.2) in `set`, from the file's values.
    pub fn lines_into(&self, set: &mut StyleSet) {
        let default_id = set.default_paragraph_id().to_string();
        let dflt = self.values(&default_id);
        for s in &mut set.paragraph {
            let v = self.values(&s.id);
            s.props = if s.id == default_id { v.diff(&styled::implicit()) } else { v.diff(&dflt) };
        }
        set.formatting = true;
    }
}

/// `word/styles.xml` with the style section of the text written (§5.2):
/// every style whose values differ from what its line (the text's, or the
/// remainder's where the text has none) and the default style's say is
/// changed in its own `w:pPr` / `w:rPr`; a line with a name no style has is
/// a new style based on the default. `None` when nothing changes. The ids of
/// new styles are set in `set`.
pub fn write_styles(parts: &[Part], set: &mut StyleSet, text_lines: &[StyleLine]) -> Result<Option<Vec<u8>>, String> {
    let st = Styles::read(parts);
    let mut lines = set.lines();
    for l in text_lines {
        match lines.iter_mut().find(|x| x.name == l.name) {
            Some(x) => x.props = l.props.clone(),
            None => lines.push(l.clone()),
        }
    }
    let table = StyleTable { default: Some(set.default_paragraph.clone()), lines: lines.clone() };
    let Some(mut doc) = st.doc.clone() else {
        // Edits may already be in the remainder, with no text_lines passed.
        // Without a styles part, only the imported implicit values can be kept.
        for l in &lines {
            let Some(def) = set.paragraph_def(&l.name).filter(|d| !d.id.is_empty()) else {
                return Err(
                    "this file has no styles part (word/styles.xml), so a new style cannot be written".into(),
                );
            };
            if table.values(Some(&l.name)).only(style_keys()) != st.values(&def.id).only(style_keys()) {
                return Err(
                    "this file has no styles part (word/styles.xml), so changed style values cannot be written"
                        .into(),
                );
            }
        }
        return Ok(None);
    };
    let default_id = set.default_paragraph_id().to_string();
    // New styles: ids from their names.
    let mut ids: Vec<String> = st.styles().filter_map(|s| s.get("w:styleId")).collect();
    for l in &lines {
        let Some(def) = set.paragraph.iter_mut().find(|d| d.name == l.name) else {
            let id = new_id(&l.name, &ids);
            ids.push(id.clone());
            set.paragraph.push(hanji_core::StyleDef { id, name: l.name.clone(), props: l.props.clone(), shown: true });
            continue;
        };
        if def.id.is_empty() {
            def.id = new_id(&l.name, &ids);
            ids.push(def.id.clone());
        }
    }
    let mut changed = false;
    for l in &lines {
        let id = set.paragraph_id(&l.name).unwrap().to_string();
        if st.by_id(&id).is_none() {
            let mut s = el("w:style")
                .with_attr("w:type", "paragraph")
                .with_attr("w:customStyle", "1")
                .with_attr("w:styleId", &id);
            s.children.push(Node::El(el("w:name").with_attr("w:val", &l.name)));
            if id != default_id {
                s.children.push(Node::El(el("w:basedOn").with_attr("w:val", &default_id)));
            }
            s.children.push(Node::El(el("w:qFormat")));
            let at = doc
                .root
                .children
                .iter()
                .rposition(|n| matches!(n, Node::El(e) if e.is("w:style")))
                .map_or(doc.root.children.len(), |k| k + 1);
            doc.root.children.insert(at, Node::El(s));
            changed = true;
        }
    }
    // Parents first, so a child's values are read over its parent's new ones.
    let depth = |d: &xml::Doc, id: &str| {
        let mut n = 0;
        let mut cur = id.to_string();
        while let Some(b) = d
            .root
            .elements()
            .find(|s| s.is("w:style") && s.get("w:styleId").as_deref() == Some(cur.as_str()))
            .and_then(|s| s.child("w:basedOn"))
            .and_then(|b| b.get("w:val"))
        {
            n += 1;
            cur = b;
            if n > 32 {
                break;
            }
        }
        n
    };
    let mut order: Vec<(usize, String, String)> = lines
        .iter()
        .map(|l| {
            (
                depth(&doc, set.paragraph_id(&l.name).unwrap()),
                set.paragraph_id(&l.name).unwrap().to_string(),
                l.name.clone(),
            )
        })
        .collect();
    order.sort();
    for (_, id, name) in order {
        let now = Styles { doc: Some(doc.clone()), theme: st.theme.clone(), base: st.base.clone() };
        let have = now.values(&id).only(style_keys());
        let want = table.values(Some(&name)).only(style_keys());
        if have == want {
            continue;
        }
        let parent = now.by_id(&id).and_then(|s| s.child("w:basedOn")).and_then(|b| b.get("w:val"));
        let parent_values = parent.map_or_else(|| now.base.clone(), |p| now.values(&p));
        let sel = doc.root.children.iter_mut().find_map(|n| match n {
            Node::El(e) if e.is("w:style") && e.get("w:styleId").as_deref() == Some(id.as_str()) => Some(e),
            _ => None,
        });
        let s = sel.unwrap();
        for k in style_keys() {
            let (h, w) = (have.get(*k), want.get(*k));
            if h == w {
                continue;
            }
            // A value its parent gives needs no own one.
            let target = if w == parent_values.get(*k) { None } else { w };
            if Key::PARA.contains(k) || *k == Key::Fill || k.is_side() {
                let ppr = child(s, "w:pPr", STYLE_ORDER);
                set_ppr(ppr, *k, target, &st.theme)?;
                drop_empty(s, "w:pPr");
            } else {
                let rpr = child(s, "w:rPr", STYLE_ORDER);
                set_rpr(rpr, *k, target, &st.theme)?;
                drop_empty(s, "w:rPr");
            }
        }
        changed = true;
    }
    for d in &mut set.paragraph {
        if let Some(l) = lines.iter().find(|l| l.name == d.name) {
            d.props = l.props.clone();
        }
    }
    Ok(changed.then(|| xml::write_doc(&doc)))
}

/// Order of a `w:style`'s children.
const STYLE_ORDER: &[&str] = &[
    "name",
    "aliases",
    "basedOn",
    "next",
    "link",
    "autoRedefine",
    "hidden",
    "uiPriority",
    "semiHidden",
    "unhideWhenUsed",
    "qFormat",
    "locked",
    "personal",
    "personalCompose",
    "personalReply",
    "rsid",
    "pPr",
    "rPr",
    "tblPr",
    "trPr",
    "tcPr",
    "tblStylePr",
];

/// A style id for `name`: its letters and digits (Word's way), made unique.
fn new_id(name: &str, taken: &[String]) -> String {
    let mut base: String = name.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    if base.is_empty() {
        base = "Style".into();
    }
    let mut id = base.clone();
    let mut n = 1;
    while taken.iter().any(|t| t.eq_ignore_ascii_case(&id)) {
        id = format!("{base}{n}");
        n += 1;
    }
    id
}
