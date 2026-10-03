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
fn color(e: Option<&Element>, losses: &mut Vec<String>) -> Option<Color> {
    let e = e?;
    if let Some(rgb) = e.get("rgb") {
        let rgb = if rgb.len() == 8 && rgb.bytes().all(|b| b.is_ascii_hexdigit()) { &rgb[2..] } else { &rgb };
        if rgb.len() == 6 && rgb.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Some(Color::from_hex(rgb));
        }
    }
    if e.get("auto").is_some_and(|v| v == "1" || v == "true") {
        return Some(Color::BLACK);
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
fn edge(e: Option<&Element>, losses: &mut Vec<String>) -> Option<Edge> {
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
    Some(Edge { width, dash, color: color(child(e, "color"), losses).unwrap_or(Color::BLACK) })
}

pub(super) fn parse(data: Option<&[u8]>, max_family_bytes: usize) -> Result<Vec<Style>, String> {
    let Some(data) = data else {
        return Ok(vec![Style::default()]);
    };
    let doc = xml::parse(data).map_err(|e| format!("visual styles: {e}"))?;
    let fonts = list(&doc.root, "fonts");
    let fills = list(&doc.root, "fills");
    let borders = list(&doc.root, "borders");
    let mut out = vec![];
    for xf in list(&doc.root, "cellXfs") {
        let mut s = Style::default();
        if let Some(f) = fonts.get(index(xf, "fontId", "applyFont")) {
            s.family = child(f, "name").and_then(|e| e.get("val")).unwrap_or(s.family);
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
            s.color = color(child(f, "color"), &mut s.losses).unwrap_or(Color::BLACK);
            if f.elements().any(|e| matches!(e.local(), "u" | "strike" | "vertAlign" | "outline" | "shadow")) {
                s.losses.push("font underline/strike/script/effects are not drawn".into());
            }
        } else {
            s.losses.push("font style index is absent; default font used".into());
        }
        if let Some(f) = fills.get(index(xf, "fillId", "applyFill")) {
            if let Some(p) = child(f, "patternFill") {
                match p.get("patternType").as_deref() {
                    Some("solid") => s.fill = color(child(p, "fgColor"), &mut s.losses),
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
                s.edges[k] = edge(child(b, name), &mut s.losses);
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
                if a.attrs.iter().any(|(k, v)| {
                    matches!(k.as_str(), "textRotation" | "readingOrder" | "shrinkToFit" | "indent" | "relativeIndent")
                        && v != "0"
                        && v != "false"
                }) {
                    s.losses.push("rotation, reading order, shrink-to-fit and indentation are not applied".into());
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
    Ok(out)
}
