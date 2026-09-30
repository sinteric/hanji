//! Direct formatting (DESIGN.md §5.2, F2) in OWPML: the vocabulary read
//! from and written to the paragraph shapes (`hh:paraPr`), character
//! shapes (`hh:charPr`) and border fills (`hh:borderFill`) of
//! `header.xml`, and a cell's vertical alignment (`hp:subList vertAlign`).
//!
//! Units: HWPUNIT (1 pt = 100) for font sizes, and for margins and exact
//! line spacing in the `HwpUnitChar` branch of a paragraph shape's
//! `hp:switch`; its default branch, and a shape without the switch, hold
//! twice those lengths. Both branches are written. Border widths are
//! Hancom's fixed widths in millimetres (a written width snaps to the
//! nearest). Colours are RGB: hwpx has no theme colours. Writing changes
//! only the attribute or child a property is, on a copy of the element.

use std::collections::HashMap;

use hanji_format::vocab::{Border, Color, ColorBase, Fill, LineSpacing, Tint};
use hanji_format::{Key, Props, Value};
use hanji_package::xml::{insert_ordered, remove_child, Element, Node};

/// `hh:align horizontal` → the vocabulary's `align`.
const ALIGNS: [(&str, &str); 6] = [
    ("JUSTIFY", "justify"),
    ("LEFT", "left"),
    ("RIGHT", "right"),
    ("CENTER", "center"),
    ("DISTRIBUTE", "distribute"),
    ("DISTRIBUTE_SPACE", "distribute"),
];

/// Line types (`hh:leftBorder type`, …) → border styles.
const LINES: [(&str, &str); 17] = [
    ("SOLID", "solid"),
    ("DASH", "dashed"),
    ("LONG_DASH", "dashed"),
    ("DOT", "dotted"),
    ("CIRCLE", "dotted"),
    ("DASH_DOT", "dash-dot"),
    ("DASH_DOT_DOT", "dash-dot-dot"),
    ("DOUBLE_SLIM", "double"),
    ("SLIM_THICK", "thin-thick"),
    ("THICK_SLIM", "thick-thin"),
    ("SLIM_THICK_SLIM", "triple"),
    ("WAVE", "wave"),
    ("DOUBLE_WAVE", "wave"),
    ("THICK_3D", "3d"),
    ("THICK_3D_REVERS", "3d"),
    ("3D", "3d"),
    ("3D_REVERS", "3d"),
];

/// The line type a written border style is.
const WRITE_LINES: [(&str, &str); 6] = [
    ("solid", "SOLID"),
    ("dashed", "DASH"),
    ("dotted", "DOT"),
    ("double", "DOUBLE_SLIM"),
    ("dash-dot", "DASH_DOT"),
    ("dash-dot-dot", "DASH_DOT_DOT"),
];

/// Hancom's border widths, in millimetres.
pub const WIDTHS: [&str; 16] =
    ["0.1", "0.12", "0.15", "0.2", "0.25", "0.3", "0.4", "0.5", "0.6", "0.7", "1.0", "1.5", "2.0", "3.0", "4.0", "5.0"];

/// Millimetres in hundredths of a point.
fn mm(v: f64) -> i64 {
    (v * 7200.0 / 25.4).round() as i64
}

/// The Hancom width nearest `width` (hundredths of a point).
pub fn snap_width(width: i64) -> &'static str {
    WIDTHS.iter().min_by_key(|w| (mm(w.parse().unwrap()) - width).abs()).copied().unwrap()
}

/// A border as it is written, its width snapped to Hancom's.
pub fn snap_border(b: &Border) -> Border {
    match b {
        Border::Line { width, style, color } => {
            Border::Line { width: mm(snap_width(*width).parse().unwrap()), style: style.clone(), color: color.clone() }
        }
        other => other.clone(),
    }
}

/// A property value as hwpx writes it: border widths snapped, a percent
/// line spacing in whole percents.
pub fn snap(k: Key, v: &Value) -> Value {
    match v {
        Value::Border(b) => Value::Border(snap_border(b)),
        Value::Spacing(LineSpacing::Percent(p)) if k == Key::LineSpacing => {
            Value::Spacing(LineSpacing::Percent((*p as f64 / 100.0).round() as i64 * 100))
        }
        v => v.clone(),
    }
}

/// [`snap`] on every value of `p`.
pub fn snap_props(p: &mut Props) {
    for (k, v) in p.0.iter_mut() {
        *v = snap(*k, v);
    }
}

// ---------------------------------------------------------------- reading

/// `#RRGGBB` (or `#AARRGGBB`); `none` is no colour.
pub fn color(v: Option<String>) -> Option<Color> {
    let v = v?;
    let h = v.trim().trim_start_matches('#');
    if h.len() < 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Color::parse(&format!("#{}", &h[h.len() - 6..])).ok()
}

fn child_local<'a>(e: &'a Element, local: &str) -> Option<&'a Element> {
    e.elements().find(|x| x.local() == local)
}

/// A border side (`hh:leftBorder`, …): none, or its line.
pub fn border(e: Option<&Element>) -> Border {
    let Some(e) = e else { return Border::None };
    let t = e.get("type").unwrap_or_default();
    if t.is_empty() || t == "NONE" {
        return Border::None;
    }
    let style = LINES.iter().find(|x| x.0 == t).map_or("solid", |x| x.1);
    let w = e.get("width").and_then(|w| w.split_whitespace().next().and_then(|n| n.parse::<f64>().ok()));
    let c = color(e.get("color")).unwrap_or_else(|| Color::rgb(0, 0, 0));
    Border::Line { width: mm(w.unwrap_or(0.1)).max(1), style: style.into(), color: c }
}

/// A border fill's `hc:fillBrush` as a fill.
pub fn fill(bf: &Element) -> Fill {
    let Some(fb) = child_local(bf, "fillBrush") else { return Fill::None };
    if child_local(fb, "gradation").is_some() {
        return Fill::Gradient;
    }
    if child_local(fb, "imgBrush").is_some() {
        return Fill::Picture;
    }
    let Some(wb) = child_local(fb, "winBrush") else { return Fill::None };
    if wb.get("hatchStyle").is_some_and(|h| !matches!(h.as_str(), "NONE" | "-1" | "")) {
        return Fill::Pattern;
    }
    color(wb.get("faceColor")).map_or(Fill::None, Fill::Color)
}

/// The sides of a border fill, by key.
const SIDES: [(Key, &str); 4] = [
    (Key::BorderTop, "topBorder"),
    (Key::BorderRight, "rightBorder"),
    (Key::BorderBottom, "bottomBorder"),
    (Key::BorderLeft, "leftBorder"),
];

/// What a border fill draws: its four sides and fill (none without one).
pub fn border_fill_props(bf: Option<&Element>) -> Props {
    let mut p = Props::new();
    for (k, local) in SIDES {
        p.set(k, Value::Border(border(bf.and_then(|b| child_local(b, local)))));
    }
    p.set(Key::Fill, Value::Fill(bf.map_or(Fill::None, fill)));
    p
}

/// Where a paragraph shape keeps its margins and line spacing: each place
/// (the branches of its `hp:switch`, or the shape itself) with the scale of
/// its lengths, the one to read first.
fn layout_places(pp: &Element) -> Vec<(&Element, i64)> {
    let Some(sw) = pp.elements().find(|e| e.local() == "switch") else { return vec![(pp, 2)] };
    let mut out = vec![];
    for b in sw.elements() {
        match b.local() {
            "case" if b.attrs.iter().any(|a| a.0.ends_with("required-namespace") && a.1.ends_with("HwpUnitChar")) => {
                out.insert(0, (b, 1))
            }
            "default" => out.push((b, 2)),
            _ => {}
        }
    }
    if out.is_empty() {
        out.push((pp, 2));
    }
    out
}

/// Margin children, by key.
const MARGINS: [(Key, &str); 5] = [
    (Key::FirstLine, "intent"),
    (Key::IndentLeft, "left"),
    (Key::IndentRight, "right"),
    (Key::SpaceBefore, "prev"),
    (Key::SpaceAfter, "next"),
];

/// What a paragraph shape sets, complete: alignment, margins, line
/// spacing, and the fill and borders of its `hh:border`.
pub fn para_pr_props(pp: &Element, border_fill: &dyn Fn(u32) -> Option<Element>) -> Props {
    let mut p = Props::new();
    let al = pp.child("hh:align").and_then(|a| a.get("horizontal"));
    let al = al.and_then(|a| ALIGNS.iter().find(|x| x.0 == a)).map_or("justify", |x| x.1);
    p.set(Key::Align, Value::Choice(al.into()));
    let places = layout_places(pp);
    let (place, scale) = places[0];
    let margin = place.elements().find(|e| e.local() == "margin");
    for (k, local) in MARGINS {
        let v = margin
            .and_then(|m| child_local(m, local))
            .and_then(|e| e.get("value"))
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0);
        p.set(k, Value::Len(v / scale));
    }
    let ls = place.elements().find(|e| e.local() == "lineSpacing");
    let n = ls.and_then(|l| l.get("value")).and_then(|v| v.parse::<i64>().ok());
    let spacing = match (ls.and_then(|l| l.get("type")).as_deref(), n) {
        (_, None) => LineSpacing::Percent(16000),
        (Some("FIXED"), Some(v)) => LineSpacing::Exact(v / scale),
        (Some("AT_LEAST"), Some(v)) => LineSpacing::AtLeast(v / scale),
        (Some("BETWEEN_LINES"), Some(v)) => LineSpacing::Gap(v / scale),
        (_, Some(v)) => LineSpacing::Percent(v * 100),
    };
    p.set(Key::LineSpacing, Value::Spacing(spacing));
    let bf = pp.child("hh:border").and_then(|b| b.get("borderFillIDRef")).and_then(|v| v.parse().ok());
    let bf = bf.and_then(border_fill);
    p = p.overlay(&border_fill_props(bf.as_ref()));
    p
}

/// What a character shape sets: font (its Hangul face), size, colour and
/// the four flags. `fonts`: the Hangul font faces by id.
pub fn char_pr_props(cp: &Element, fonts: &HashMap<u32, String>) -> Props {
    let mut p = Props::new();
    let face = cp.child("hh:fontRef").and_then(|f| f.get("hangul")).and_then(|v| v.parse().ok());
    if let Some(f) = face.and_then(|id: u32| fonts.get(&id)).filter(|f| !f.trim().is_empty()) {
        p.set(Key::Font, Value::Text(f.trim().to_string()));
    }
    let size = cp.get("height").and_then(|v| v.parse::<i64>().ok()).filter(|n| *n > 0).unwrap_or(1000);
    p.set(Key::Size, Value::Len(size));
    p.set(Key::Color, Value::Color(color(cp.get("textColor")).unwrap_or_else(|| Color::rgb(0, 0, 0))));
    let underline = cp.child("hh:underline").is_some_and(|u| u.get("type").is_some_and(|t| t != "NONE"));
    let strike = cp.child("hh:strikeout").is_some_and(|u| u.get("shape").is_some_and(|t| strikes(&t)));
    p.set(Key::Bold, Value::Flag(cp.child("hh:bold").is_some()));
    p.set(Key::Italic, Value::Flag(cp.child("hh:italic").is_some()));
    p.set(Key::Underline, Value::Flag(underline));
    p.set(Key::Strike, Value::Flag(strike));
    p
}

/// Whether a strikeout `shape` strikes: not `NONE`, nor `3D` (which
/// Hancom writes for none).
pub fn strikes(shape: &str) -> bool {
    !matches!(shape, "NONE" | "3D" | "")
}

/// A cell's vertical alignment (`hp:subList vertAlign`, centred when unset).
pub fn valign(sub: &Element) -> &'static str {
    match sub.get("vertAlign").as_deref() {
        Some("TOP") => "top",
        Some("BOTTOM") => "bottom",
        _ => "middle",
    }
}

// ---------------------------------------------------------------- writing

/// A value the text may write to hwpx, or why not.
pub fn writable(k: Key, v: &Value) -> Result<(), String> {
    if !v.is_writable() {
        return Err(format!(
            "{k}={v} is shown as the file has it and cannot be written anew (a gradient, pattern or picture, a border style other than solid, dashed, dotted, double, dash-dot or dash-dot-dot): leave the value as it is, or write another"
        ));
    }
    let color = match v {
        Value::Color(c) => Some(c),
        Value::Fill(Fill::Color(c)) => Some(c),
        Value::Border(Border::Line { color, .. }) => Some(color),
        _ => None,
    };
    if let Some(c) = color {
        rgb(c).map_err(|m| format!("{k}={v}: {m}"))?;
    }
    Ok(())
}

/// `#RRGGBB` for an RGB colour.
pub fn rgb(c: &Color) -> Result<String, String> {
    if c.alpha.is_some() {
        return Err("hwpx has no opacity for this colour; write #RRGGBB without /NN%".into());
    }
    match (&c.base, c.tint) {
        (ColorBase::Rgb([r, g, b]), Tint::None) => Ok(format!("#{r:02X}{g:02X}{b:02X}")),
        _ => Err("hwpx stores colours as #RRGGBB and has no theme colours; write #RRGGBB".into()),
    }
}

/// Order of a paragraph shape's children.
const PARAPR_ORDER: &[&str] = &["align", "heading", "breakSetting", "margin", "lineSpacing", "border", "autoSpacing"];

/// Order of a margin's children.
const MARGIN_ORDER: &[&str] = &["intent", "left", "right", "prev", "next"];

/// Order of a border fill's children.
pub const BORDERFILL_ORDER: &[&str] =
    &["slash", "backSlash", "leftBorder", "rightBorder", "topBorder", "bottomBorder", "diagonal", "fillBrush"];

fn child_local_mut<'a>(e: &'a mut Element, local: &str) -> Option<&'a mut Element> {
    e.elements_mut().find(|x| x.local() == local)
}

/// The prefix of `e`'s name, with its colon (`hh:`).
fn prefix(e: &Element) -> &str {
    e.name.find(':').map_or("", |k| &e.name[..=k])
}

/// Sets a paragraph shape's layout key `k` (alignment, a margin, line
/// spacing) to `v`, in every place the shape keeps it.
pub fn set_para(pp: &mut Element, k: Key, v: &Value) -> Result<(), String> {
    writable(k, v)?;
    if k == Key::Align {
        let a = v.choice().unwrap_or("justify").to_uppercase();
        match pp.child_mut("hh:align") {
            Some(al) => al.set("horizontal", &a),
            None => insert_ordered(
                pp,
                Element::new("hh:align").with_attr("horizontal", &a).with_attr("vertical", "BASELINE"),
                PARAPR_ORDER,
            ),
        }
        return Ok(());
    }
    let has_switch = pp.elements().any(|e| e.local() == "switch");
    let edit = |place: &mut Element, scale: i64, order: &[&str]| -> Result<(), String> {
        if let Some(local) = MARGINS.iter().find(|m| m.0 == k).map(|m| m.1) {
            let n = v.length().ok_or_else(|| format!("{k} is a length"))? * scale;
            if place.elements().all(|e| e.local() != "margin") {
                insert_ordered(place, Element::new("hh:margin"), order);
            }
            let m = child_local_mut(place, "margin").unwrap();
            let cpre = m.elements().next().map_or("hc:".to_string(), |c| prefix(c).to_string());
            match child_local_mut(m, local) {
                Some(c) => c.set("value", &n.to_string()),
                None => insert_ordered(
                    m,
                    Element::new(&format!("{cpre}{local}"))
                        .with_attr("value", &n.to_string())
                        .with_attr("unit", "HWPUNIT"),
                    MARGIN_ORDER,
                ),
            }
            return Ok(());
        }
        let Value::Spacing(ls) = v else { return Err(format!("{k} cannot be written to a paragraph shape")) };
        let (ty, n) = match ls {
            LineSpacing::Percent(p) => ("PERCENT", (*p as f64 / 100.0).round() as i64),
            LineSpacing::Exact(l) => ("FIXED", l * scale),
            LineSpacing::AtLeast(l) => ("AT_LEAST", l * scale),
            LineSpacing::Gap(l) => ("BETWEEN_LINES", l * scale),
        };
        match child_local_mut(place, "lineSpacing") {
            Some(l) => {
                l.set("type", ty);
                l.set("value", &n.to_string());
            }
            None => insert_ordered(
                place,
                Element::new("hh:lineSpacing")
                    .with_attr("type", ty)
                    .with_attr("value", &n.to_string())
                    .with_attr("unit", "HWPUNIT"),
                order,
            ),
        }
        Ok(())
    };
    if has_switch {
        let sw = child_local_mut(pp, "switch").unwrap();
        for b in sw.elements_mut() {
            let scale = match b.local() {
                "case"
                    if b.attrs.iter().any(|a| a.0.ends_with("required-namespace") && a.1.ends_with("HwpUnitChar")) =>
                {
                    1
                }
                "default" => 2,
                _ => continue,
            };
            edit(b, scale, &["margin", "lineSpacing"])?;
        }
    } else {
        edit(pp, 2, PARAPR_ORDER)?;
    }
    Ok(())
}

/// Sets a border fill's side `k` to `b`, its width snapped to Hancom's.
pub fn set_side(bf: &mut Element, k: Key, b: &Border) -> Result<(), String> {
    writable(k, &Value::Border(b.clone()))?;
    let local = SIDES.iter().find(|s| s.0 == k).map(|s| s.1).ok_or("not a side")?;
    let pre = prefix(bf).to_string();
    if child_local(bf, local).is_none() {
        let e = Element::new(&format!("{pre}{local}"))
            .with_attr("type", "NONE")
            .with_attr("width", "0.1 mm")
            .with_attr("color", "#000000");
        insert_ordered(bf, e, BORDERFILL_ORDER);
    }
    let cur = border(child_local(bf, local));
    let e = child_local_mut(bf, local).unwrap();
    match b {
        Border::None => e.set("type", "NONE"),
        Border::Line { width, style, color } => {
            let same_style = matches!(&cur, Border::Line { style: s, .. } if s == style);
            if !same_style {
                let t = WRITE_LINES.iter().find(|x| x.0 == style).map(|x| x.1).unwrap_or("SOLID");
                e.set("type", t);
            }
            e.set("width", &format!("{} mm", snap_width(*width)));
            e.set("color", &rgb(color)?);
        }
    }
    Ok(())
}

/// Sets a border fill's fill: none, or a colour (a solid `hc:winBrush`).
pub fn set_fill(bf: &mut Element, f: &Fill) -> Result<(), String> {
    writable(Key::Fill, &Value::Fill(f.clone()))?;
    let face = match f {
        Fill::Color(c) => Some(rgb(c)?),
        _ => None,
    };
    let Some(face) = face else {
        // No fill: a brush of no colour (Hancom's own way), gradients and pictures gone.
        if let Some(fb) = child_local_mut(bf, "fillBrush") {
            fb.children.retain(|n| !matches!(n, Node::El(e) if e.local() == "gradation" || e.local() == "imgBrush"));
            if let Some(wb) = child_local_mut(fb, "winBrush") {
                wb.set("faceColor", "none");
                wb.remove_attr("hatchStyle");
            }
            if fb.elements().next().is_none() {
                let name = fb.name.clone();
                remove_child(bf, &name);
            }
        }
        return Ok(());
    };
    if child_local(bf, "fillBrush").is_none() {
        insert_ordered(bf, Element::new("hc:fillBrush"), BORDERFILL_ORDER);
    }
    let fb = child_local_mut(bf, "fillBrush").unwrap();
    fb.children.retain(|n| !matches!(n, Node::El(e) if e.local() == "gradation" || e.local() == "imgBrush"));
    let pre = prefix(fb).to_string();
    match child_local_mut(fb, "winBrush") {
        Some(wb) => {
            wb.set("faceColor", &face);
            wb.remove_attr("hatchStyle");
        }
        None => fb.children.insert(
            0,
            Node::El(
                Element::new(&format!("{pre}winBrush"))
                    .with_attr("faceColor", &face)
                    .with_attr("hatchColor", "#000000")
                    .with_attr("alpha", "0"),
            ),
        ),
    }
    Ok(())
}

/// Sets a cell's vertical alignment.
pub fn set_valign(sub: &mut Element, v: &str) {
    let a = match v {
        "top" => "TOP",
        "bottom" => "BOTTOM",
        _ => "CENTER",
    };
    sub.set("vertAlign", a);
}
