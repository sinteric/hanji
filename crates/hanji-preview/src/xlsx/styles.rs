//! Small visual style projection; unsupported style semantics are diagnosed.
use hanji_package::xml::{self, Element};
use oxml_layout::Color;

#[derive(Clone)]
pub(super) struct Edge {
    pub width: f64,
    pub color: Color,
    pub dash: Option<(f64, f64)>,
}
#[derive(Clone)]
pub(super) struct Style {
    pub family: String,
    pub size: f64,
    pub bold: bool,
    pub italic: bool,
    pub color: Color,
    pub fill: Option<Color>,
    pub edges: [Option<Edge>; 4],
    pub horizontal: String,
    pub vertical: String,
    pub wrap: bool,
    pub indent: u32,
    pub losses: Vec<String>,
}
impl Default for Style {
    fn default() -> Self {
        Self {
            family: "Calibri".into(),
            size: 11.0,
            bold: false,
            italic: false,
            color: Color::BLACK,
            fill: None,
            edges: Default::default(),
            horizontal: "general".into(),
            vertical: "bottom".into(),
            wrap: false,
            indent: 0,
            losses: vec![],
        }
    }
}
fn child<'a>(e: &'a Element, name: &str) -> Option<&'a Element> {
    e.elements().find(|e| e.local() == name)
}
fn list<'a>(e: &'a Element, name: &str) -> Vec<&'a Element> {
    child(e, name).map(|e| e.elements().collect()).unwrap_or_default()
}
fn rgb(value: &str) -> Option<Color> {
    if !matches!(value.len(), 6 | 8) || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let value = if value.len() == 8 { &value[2..] } else { value };
    Some(Color::from_hex(value))
}
fn theme_colors(data: Option<&[u8]>) -> [Option<Color>; 12] {
    let mut colors = [None; 12];
    let Some(doc) = data.and_then(|data| xml::parse(data).ok()) else { return colors };
    let Some(scheme) = child(&doc.root, "themeElements").and_then(|e| child(e, "clrScheme")) else {
        return colors;
    };
    // Spreadsheet indices use light/dark pairs, not DrawingML's XML child order.
    for (index, name) in [
        "lt1", "dk1", "lt2", "dk2", "accent1", "accent2", "accent3", "accent4", "accent5", "accent6", "hlink",
        "folHlink",
    ]
    .iter()
    .enumerate()
    {
        let Some(slot) = child(scheme, name) else { continue };
        let Some(value) = slot.elements().next() else { continue };
        // Color transforms and other theme color forms remain unsupported.
        if value.elements().next().is_some() {
            continue;
        }
        colors[index] = match value.local() {
            "srgbClr" => value.get("val").filter(|v| v.len() == 6).and_then(|v| rgb(&v)),
            "sysClr" => value.get("lastClr").filter(|v| v.len() == 6).and_then(|v| rgb(&v)),
            _ => None,
        };
    }
    colors
}
fn tinted(color: Color, tint: f64) -> Color {
    if tint == 0.0 {
        return color;
    }
    let (r, g, b) = (color.r, color.g, color.b);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let luminance = (max + min) / 2.0;
    let delta = max - min;
    let saturation = if delta == 0.0 { 0.0 } else { delta / (1.0 - (2.0 * luminance - 1.0).abs()) };
    let hue = if delta == 0.0 {
        0.0
    } else if max == r {
        ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };
    // SpreadsheetML tint changes HLS luminance, rather than each RGB channel.
    let luminance = if tint < 0.0 { luminance * (1.0 + tint) } else { luminance * (1.0 - tint) + tint };
    let chroma = (1.0 - (2.0 * luminance - 1.0).abs()) * saturation;
    let x = chroma * (1.0 - (hue.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match hue as u8 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let offset = luminance - chroma / 2.0;
    let channel = |value: f64| ((value + offset).clamp(0.0, 1.0) * 255.0).round() / 255.0;
    Color { r: channel(r), g: channel(g), b: channel(b), a: color.a }
}
fn color(e: Option<&Element>, theme: &[Option<Color>; 12], losses: &mut Vec<String>) -> Option<Color> {
    let e = e?;
    let base = if let Some(value) = e.get("rgb") {
        rgb(&value)
    } else if let Some(index) = e.get("theme") {
        index.parse::<usize>().ok().and_then(|i| theme.get(i)).copied().flatten()
    } else if e.get("auto").is_some_and(|v| v == "1" || v == "true") {
        Some(Color::BLACK)
    } else {
        None
    };
    let tint = e
        .get("tint")
        .map_or(Some(0.0), |v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite() && (-1.0..=1.0).contains(v));
    if let (Some(base), Some(tint)) = (base, tint) {
        return Some(tinted(base, tint));
    }
    losses.push("theme/indexed/tinted or invalid color uses the default color".into());
    None
}
fn enabled(e: &Element) -> bool {
    !e.get("val").is_some_and(|v| v == "0" || v == "false")
}
fn index(x: &Element, key: &str, apply: &str) -> usize {
    if x.get(apply).is_some_and(|v| v == "0" || v == "false") {
        return 0;
    }
    x.get(key).and_then(|s| s.parse().ok()).unwrap_or(0)
}
fn edge(e: Option<&Element>, theme: &[Option<Color>; 12], losses: &mut Vec<String>) -> Option<Edge> {
    let e = e?;
    let style = e.get("style")?;
    let (width, dash) = match style.as_str() {
        "thin" | "hair" => (0.5, None),
        "medium" => (1.0, None),
        "thick" => (1.5, None),
        "dashed" => (0.5, Some((3.0, 2.0))),
        "dotted" => (0.5, Some((0.5, 1.5))),
        "none" => return None,
        _ => {
            losses.push(format!("border style {style:?} approximated as a thin solid line"));
            (0.5, None)
        }
    };
    Some(Edge { width, dash, color: color(child(e, "color"), theme, losses).unwrap_or(Color::BLACK) })
}

fn font_metrics(s: &mut Style, f: &Element, max_family_bytes: usize) -> Result<(), String> {
    s.family = child(f, "name").and_then(|e| e.get("val")).unwrap_or_else(|| s.family.clone());
    if s.family.len() > max_family_bytes {
        return Err("font family exceeds the configured metadata budget".into());
    }
    if let Some(v) = child(f, "sz").and_then(|e| e.get("val")) {
        s.size = v.parse::<f64>().map_err(|_| "invalid font size")?;
        if !s.size.is_finite() || !(1.0..=409.0).contains(&s.size) {
            return Err("invalid font size".into());
        }
    }
    s.bold = child(f, "b").is_some_and(enabled);
    s.italic = child(f, "i").is_some_and(enabled);
    Ok(())
}

pub(super) fn parse(
    data: Option<&[u8]>,
    theme_data: Option<&[u8]>,
    max_family_bytes: usize,
) -> Result<(Vec<Style>, Style), String> {
    let Some(data) = data else {
        return Ok((vec![Style::default()], Style::default()));
    };
    let doc = xml::parse(data).map_err(|e| format!("visual styles: {e}"))?;
    let theme = theme_colors(theme_data);
    let fonts = list(&doc.root, "fonts");
    let fills = list(&doc.root, "fills");
    let borders = list(&doc.root, "borders");
    let mut out = vec![];
    for xf in list(&doc.root, "cellXfs") {
        let mut s = Style::default();
        if let Some(f) = fonts.get(index(xf, "fontId", "applyFont")) {
            font_metrics(&mut s, f, max_family_bytes)?;
            s.color = color(child(f, "color"), &theme, &mut s.losses).unwrap_or(Color::BLACK);
            if f.elements().any(|e| matches!(e.local(), "u" | "strike" | "vertAlign" | "outline" | "shadow")) {
                s.losses.push("font underline/strike/script/effects are not drawn".into());
            }
        } else {
            s.losses.push("font style index is absent; default font used".into());
        }
        if let Some(f) = fills.get(index(xf, "fillId", "applyFill")) {
            if let Some(p) = child(f, "patternFill") {
                match p.get("patternType").as_deref() {
                    Some("solid") => s.fill = color(child(p, "fgColor"), &theme, &mut s.losses),
                    None | Some("none") => (),
                    _ => s.losses.push("pattern fill is not drawn".into()),
                }
            } else if child(f, "gradientFill").is_some() {
                s.losses.push("gradient fill is not drawn".into());
            }
        } else {
            s.losses.push("fill style index is absent; no fill used".into());
        }
        if let Some(b) = borders.get(index(xf, "borderId", "applyBorder")) {
            for (k, name) in ["left", "right", "top", "bottom"].iter().enumerate() {
                s.edges[k] = edge(child(b, name), &theme, &mut s.losses);
            }
            if b.elements().any(|e| {
                matches!(e.local(), "diagonal" | "vertical" | "horizontal" | "start" | "end")
                    && e.get("style").is_some()
            }) {
                s.losses.push("diagonal/inside/directional borders are not drawn".into());
            }
        } else {
            s.losses.push("border style index is absent; grid lines used".into());
        }
        if !xf.get("applyAlignment").is_some_and(|v| v == "0" || v == "false") {
            if let Some(a) = child(xf, "alignment") {
                if let Some(h) = a.get("horizontal") {
                    if matches!(h.as_str(), "left" | "right" | "center" | "general") {
                        s.horizontal = h;
                    } else {
                        s.losses.push(format!("horizontal alignment {h:?} uses general alignment"));
                    }
                }
                if let Some(v) = a.get("vertical") {
                    if matches!(v.as_str(), "top" | "center" | "bottom") {
                        s.vertical = v;
                    } else {
                        s.losses.push(format!("vertical alignment {v:?} uses bottom alignment"));
                    }
                }
                s.wrap = a.get("wrapText").is_some_and(|v| v == "1" || v == "true");
                if let Some(indent) = a.get("indent") {
                    match indent.trim().parse::<u32>() {
                        Ok(0) => (),
                        Ok(n) if matches!(a.get("horizontal").as_deref(), Some("left" | "right")) => s.indent = n,
                        Ok(_) => s.losses.push("indentation is only applied for explicit left/right alignment; this alignment's indentation is not applied".into()),
                        Err(_) => s.losses.push("invalid indentation uses zero indentation".into()),
                    }
                }
                for (attr, name) in [
                    ("textRotation", "rotation"),
                    ("readingOrder", "reading order"),
                    ("shrinkToFit", "shrink-to-fit"),
                    ("relativeIndent", "relative indentation"),
                ] {
                    if a.get(attr).is_some_and(|v| v != "0" && v != "false") {
                        s.losses.push(format!("{name} is not applied"));
                    }
                }
            }
        }
        if xf.get("xfId").is_some_and(|v| v != "0") {
            s.losses.push("named base-style inheritance is not resolved".into());
        }
        out.push(s);
    }
    if out.is_empty() {
        out.push(Style::default());
    }
    // ISO/IEC 29500 §18.8.1 defines an indent unit using the Normal style's
    // font, not the cell's font. Built-in identity survives localized names.
    // Only resolve these metrics when indentation actually needs them.
    let mut normal = Style::default();
    if out.iter().any(|s| s.indent != 0) {
        let style_xfs = list(&doc.root, "cellStyleXfs");
        let font = list(&doc.root, "cellStyles")
            .into_iter()
            .find(|s| s.get("builtinId").and_then(|v| v.parse::<u32>().ok()) == Some(0))
            .and_then(|s| s.get("xfId").and_then(|v| v.parse::<usize>().ok()))
            .and_then(|i| style_xfs.get(i))
            .and_then(|xf| xf.get("fontId").map_or(Some(0), |v| v.parse::<usize>().ok()))
            .and_then(|i| fonts.get(i));
        if let Some(f) = font.or_else(|| fonts.first()) {
            font_metrics(&mut normal, f, max_family_bytes)?;
        }
        if font.is_none() {
            for s in out.iter_mut().filter(|s| s.indent != 0) {
                s.losses.push("Normal style font is unresolved; indentation uses font 0 or the default font".into());
            }
        }
    }
    Ok((out, normal))
}
