//! Cell formatting (DESIGN.md §5.4, round 6 part D): a cell style's
//! (`cellXfs`) font, fill, border and alignment as the vocabulary's values,
//! and the cell styles a `format` operation needs, each a copy of the
//! cell's own with the written values changed, so what the vocabulary does
//! not show (a diagonal border, wrap, rotation, protection, a font's
//! family) stays. A font, fill, border or cell style the file already has
//! is reused.
//!
//! Theme colours stay names, the theme index as Excel numbers it (0 `bg1`,
//! 1 `tx1`, 2 `bg2`, 3 `tx2`, then the accents), a `tint` as `+N%`/`-N%`.
//! Excel border styles show as widths (hair 0.25pt dotted, thin 0.75pt,
//! medium 1.5pt, thick 2.25pt, double 2.25pt); a written width snaps to
//! the nearest style of its kind.

use hanji_format::cellfmt::CellFormat;
use hanji_format::vocab::{Border, Color, ColorBase, Fill, Tint};
use hanji_package::xml::{self, Element, Node};

use crate::styles::Styles;

/// The theme colours by Excel's `theme` index.
const THEME: [&str; 12] =
    ["bg1", "tx1", "bg2", "tx2", "accent1", "accent2", "accent3", "accent4", "accent5", "accent6", "hlink", "folHlink"];

/// Excel's border styles as widths (hundredths of a point) and vocabulary styles.
const XL_BORDERS: [(&str, i64, &str); 13] = [
    ("thin", 75, "solid"),
    ("medium", 150, "solid"),
    ("thick", 225, "solid"),
    ("double", 225, "double"),
    ("hair", 25, "dotted"),
    ("dotted", 75, "dotted"),
    ("dashed", 75, "dashed"),
    ("mediumDashed", 150, "dashed"),
    ("dashDot", 75, "dash-dot"),
    ("mediumDashDot", 150, "dash-dot"),
    ("dashDotDot", 75, "dash-dot-dot"),
    ("mediumDashDotDot", 150, "dash-dot-dot"),
    ("slantDashDot", 150, "dash-dot"),
];

/// The legacy indexed palette (`indexed` 0–63).
const INDEXED: [u32; 64] = [
    0x000000, 0xFFFFFF, 0xFF0000, 0x00FF00, 0x0000FF, 0xFFFF00, 0xFF00FF, 0x00FFFF, 0x000000, 0xFFFFFF, 0xFF0000,
    0x00FF00, 0x0000FF, 0xFFFF00, 0xFF00FF, 0x00FFFF, 0x800000, 0x008000, 0x000080, 0x808000, 0x800080, 0x008080,
    0xC0C0C0, 0x808080, 0x9999FF, 0x993366, 0xFFFFCC, 0xCCFFFF, 0x660066, 0xFF8080, 0x0066CC, 0xCCCCFF, 0x000080,
    0xFF00FF, 0xFFFF00, 0x00FFFF, 0x800080, 0x800000, 0x008080, 0x0000FF, 0x00CCFF, 0xCCFFFF, 0xCCFFCC, 0xFFFF99,
    0x99CCFF, 0xFF99CC, 0xCC99FF, 0xFFCC99, 0x3366FF, 0x33CCCC, 0x99CC00, 0xFFCC00, 0xFF9900, 0xFF6600, 0x666699,
    0x969696, 0x003366, 0x339966, 0x003300, 0x333300, 0x993300, 0x993366, 0x333399, 0x333333,
];

fn black() -> Color {
    Color::rgb(0, 0, 0)
}

fn kid<'a>(e: &'a Element, local: &str) -> Option<&'a Element> {
    e.elements().find(|c| c.local() == local)
}

fn kid_mut<'a>(e: &'a mut Element, local: &str) -> Option<&'a mut Element> {
    e.elements_mut().find(|c| c.local() == local)
}

/// An Excel colour element → the vocabulary's (`None`: automatic or absent).
pub fn color_of(e: Option<&Element>) -> Option<Color> {
    let e = e?;
    if let Some(rgb) = e.get("rgb") {
        let hex = rgb.get(rgb.len().saturating_sub(6)..)?;
        let v = u32::from_str_radix(hex, 16).ok()?;
        return Some(Color::rgb((v >> 16) as u8, (v >> 8) as u8, v as u8));
    }
    if let Some(t) = e.get("theme").and_then(|t| t.parse::<usize>().ok()) {
        let mut c = Color::theme(THEME.get(t).copied().unwrap_or("tx1"));
        let tint: f64 = e.get("tint").and_then(|t| t.parse().ok()).unwrap_or(0.0);
        if tint != 0.0 {
            let pct = tint.abs() * 100.0;
            let n = pct.round();
            c.tint = if (pct - n).abs() < 0.05 && (1.0..=99.0).contains(&n) {
                if tint > 0.0 {
                    Tint::Lighter(n as u8)
                } else {
                    Tint::Darker(n as u8)
                }
            } else {
                Tint::Other
            };
        }
        return Some(c);
    }
    if let Some(i) = e.get("indexed").and_then(|t| t.parse::<usize>().ok()) {
        let v = *INDEXED.get(i)?;
        return Some(Color::rgb((v >> 16) as u8, (v >> 8) as u8, v as u8));
    }
    None
}

/// The vocabulary's colour as an Excel colour element named `name`.
fn color_el(name: &str, c: &Color) -> Result<Element, String> {
    if c.alpha.is_some() {
        return Err(format!("{c}: a workbook has no transparent colours; write the colour without /NN%"));
    }
    let mut e = Element::new(name);
    match &c.base {
        ColorBase::Rgb([r, g, b]) => e.set("rgb", &format!("FF{r:02X}{g:02X}{b:02X}")),
        ColorBase::Theme(t) => {
            let alias = match t.as_str() {
                "lt1" => "bg1",
                "dk1" => "tx1",
                "lt2" => "bg2",
                "dk2" => "tx2",
                other => other,
            };
            let k = THEME.iter().position(|x| *x == alias).ok_or_else(|| format!("{t} is not a theme colour"))?;
            e.set("theme", &k.to_string());
            match c.tint {
                Tint::None => {}
                Tint::Lighter(p) => e.set("tint", &format!("{}", p as f64 / 100.0)),
                Tint::Darker(p) => e.set("tint", &format!("{}", -(p as f64) / 100.0)),
                Tint::Other => {
                    return Err(format!(
                        "{c}: a theme colour with another transform is kept as written, not written anew"
                    ))
                }
            }
        }
    }
    Ok(e)
}

/// A flag element's value (`<b/>`, `<b val="0"/>`).
fn flag(e: Option<&Element>) -> bool {
    e.is_some_and(|e| !matches!(e.get("val").as_deref(), Some("0" | "false" | "none")))
}

/// Everything a font element shows.
fn font_values(f: Option<&Element>, out: &mut CellFormat) {
    out.bold = Some(flag(f.and_then(|f| kid(f, "b"))));
    out.italic = Some(flag(f.and_then(|f| kid(f, "i"))));
    out.strike = Some(flag(f.and_then(|f| kid(f, "strike"))));
    out.underline = Some(flag(f.and_then(|f| kid(f, "u"))));
    out.size = Some(
        f.and_then(|f| kid(f, "sz"))
            .and_then(|s| s.get("val"))
            .and_then(|v| v.parse::<f64>().ok())
            .map_or(1100, |v| (v * 100.0).round() as i64),
    );
    out.color = Some(color_of(f.and_then(|f| kid(f, "color"))).unwrap_or_else(black));
    // A font without a name shows none (the application's own).
    out.font = f.and_then(|f| kid(f, "name")).and_then(|n| n.get("val")).filter(|n| !n.is_empty());
}

fn fill_value(f: Option<&Element>) -> Fill {
    let Some(f) = f else { return Fill::None };
    if kid(f, "gradientFill").is_some() {
        return Fill::Gradient;
    }
    let Some(p) = kid(f, "patternFill") else { return Fill::None };
    match p.get("patternType").as_deref() {
        None | Some("none") => Fill::None,
        Some("solid") => Fill::Color(color_of(kid(p, "fgColor")).unwrap_or_else(black)),
        Some(_) => Fill::Pattern,
    }
}

/// A border element's sides: top, right, bottom, left.
fn border_values(b: Option<&Element>) -> [Border; 4] {
    let side = |names: &[&str]| -> Border {
        let Some(e) = b.and_then(|b| names.iter().find_map(|n| kid(b, n))) else { return Border::None };
        let Some(st) = e.get("style").filter(|s| s != "none") else { return Border::None };
        let (w, style) = XL_BORDERS.iter().find(|x| x.0 == st).map_or((75, "solid"), |x| (x.1, x.2));
        Border::Line { width: w, style: style.into(), color: color_of(kid(e, "color")).unwrap_or_else(black) }
    };
    [side(&["top"]), side(&["right", "end"]), side(&["bottom"]), side(&["left", "start"])]
}

fn list<'a>(root: &'a Element, name: &str) -> Vec<&'a Element> {
    kid(root, name).map(|l| l.elements().collect()).unwrap_or_default()
}

/// A cell style (`xf` element) with its font, fill and border → every key.
fn xf_values(root: &Element, xf: Option<&Element>) -> CellFormat {
    let id = |k: &str| xf.and_then(|x| x.get(k)).and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
    let fonts = list(root, "fonts");
    let fills = list(root, "fills");
    let borders = list(root, "borders");
    let mut out = CellFormat::default();
    font_values(fonts.get(id("fontId")).copied(), &mut out);
    out.fill = Some(fill_value(fills.get(id("fillId")).copied()));
    out.borders = border_values(borders.get(id("borderId")).copied()).map(Some);
    let al = xf.and_then(|x| kid(x, "alignment"));
    let h = al.and_then(|a| a.get("horizontal"));
    out.align = Some(
        match h.as_deref() {
            Some("left") => "left",
            Some("center" | "centerContinuous") => "center",
            Some("right") => "right",
            Some("justify") => "justify",
            Some("distributed") => "distribute",
            _ => "general",
        }
        .into(),
    );
    out.valign = Some(
        match al.and_then(|a| a.get("vertical")).as_deref() {
            Some("top") => "top",
            Some("center") => "middle",
            _ => "bottom",
        }
        .into(),
    );
    out.indent = Some(al.and_then(|a| a.get("indent")).and_then(|v| v.parse().ok()).unwrap_or(0));
    out
}

impl Styles {
    fn root(&self) -> Option<&Element> {
        self.doc.as_ref().map(|d| &d.root)
    }

    /// Every key of cell style `s` (`cellXfs` index).
    pub fn cell_format(&self, s: u32) -> CellFormat {
        match self.root() {
            Some(r) => xf_values(r, list(r, "cellXfs").get(s as usize).copied()),
            None => xf_values(&Element::new("styleSheet"), None),
        }
    }

    /// The Normal style's keys: its `cellStyleXfs` entry.
    pub fn normal_format(&self) -> CellFormat {
        let Some(r) = self.root() else { return self.cell_format(0) };
        let normal = list(r, "cellStyles")
            .into_iter()
            .find(|c| c.get("builtinId").as_deref() == Some("0"))
            .and_then(|c| c.get("xfId"))
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);
        match list(r, "cellStyleXfs").get(normal) {
            Some(x) => xf_values(r, Some(x)),
            None => self.cell_format(0),
        }
    }

    /// The name of the cell style `s` is in, when it is not Normal.
    pub fn style_name(&self, s: u32) -> Option<String> {
        let r = self.root()?;
        let xf_id = list(r, "cellXfs").get(s as usize)?.get("xfId").unwrap_or_else(|| "0".into());
        let st = list(r, "cellStyles").into_iter().find(|c| c.get("xfId").as_deref() == Some(xf_id.as_str()))?;
        (st.get("builtinId").as_deref() != Some("0")).then(|| st.get("name")).flatten()
    }

    /// A cell style like `s` with the values `set` writes (borders by side).
    /// The same `s` when nothing changes; an existing style when one has
    /// exactly the new values.
    pub fn with_cell_format(&mut self, s: u32, set: &CellFormat) -> Result<u32, String> {
        let have = self.cell_format(s);
        if have.overlay(set) == have {
            return Ok(s);
        }
        let doc = self.doc.as_mut().ok_or("the workbook has no styles part")?;
        let root = &mut doc.root;
        let p = root.name.strip_suffix("styleSheet").unwrap_or("").to_string();
        let xfs: Vec<Element> = list(root, "cellXfs").into_iter().cloned().collect();
        let mut xf = xfs.get(s as usize).or(xfs.first()).cloned().unwrap_or_else(|| Element::new(&format!("{p}xf")));
        let id = |x: &Element, k: &str| x.get(k).and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
        let text_keys = set.font.is_some()
            || set.size.is_some()
            || set.color.is_some()
            || set.bold.is_some()
            || set.italic.is_some()
            || set.underline.is_some()
            || set.strike.is_some();
        if text_keys {
            let base = list(root, "fonts").get(id(&xf, "fontId")).map(|e| (*e).clone());
            let font = new_font(base, set, &p)?;
            let k = reuse(root, "fonts", font, &p);
            xf.set("fontId", &k.to_string());
            xf.set("applyFont", "1");
        }
        if let Some(f) = &set.fill {
            let fill = new_fill(f, &p)?;
            let k = reuse(root, "fills", fill, &p);
            xf.set("fillId", &k.to_string());
            xf.set("applyFill", "1");
        }
        if set.borders.iter().any(Option::is_some) {
            let base = list(root, "borders").get(id(&xf, "borderId")).map(|e| (*e).clone());
            let border = new_border(base, &set.borders, &p)?;
            let k = reuse(root, "borders", border, &p);
            xf.set("borderId", &k.to_string());
            xf.set("applyBorder", "1");
        }
        if set.align.is_some() || set.valign.is_some() || set.indent.is_some() {
            set_alignment(&mut xf, set, &have, &p);
            xf.set("applyAlignment", "1");
        }
        let canon = |e: &Element| xml::canon(e, &xml::Scope::new());
        if let Some(k) = xfs.iter().position(|x| canon(x) == canon(&xf)) {
            return Ok(k as u32);
        }
        let list = kid_mut(root, "cellXfs").ok_or("styles.xml has no cellXfs")?;
        list.children.push(Node::El(xf));
        let n = list.elements().count();
        list.set("count", &n.to_string());
        let fmt = self.xf_fmt.get(s as usize).copied().unwrap_or(0);
        self.xf_fmt.push(fmt);
        self.changed = true;
        Ok(n as u32 - 1)
    }
}

/// The index of `el` in list `name` (`fonts`, `fills`, `borders`), added when
/// the list has no equal one.
fn reuse(root: &mut Element, name: &str, el: Element, p: &str) -> usize {
    let canon = |e: &Element| xml::canon(e, &xml::Scope::new());
    let want = canon(&el);
    if let Some(k) = list(root, name).iter().position(|e| canon(e) == want) {
        return k;
    }
    if kid(root, name).is_none() {
        let order = ["numFmts", "fonts", "fills", "borders", "cellStyleXfs", "cellXfs"];
        xml::insert_ordered(root, Element::new(&format!("{p}{name}")).with_attr("count", "0"), &order);
    }
    let l = kid_mut(root, name).unwrap();
    l.children.push(Node::El(el));
    let n = l.elements().count();
    l.set("count", &n.to_string());
    n - 1
}

/// The order Excel writes a font's children in.
const FONT_ORDER: [&str; 15] = [
    "b",
    "i",
    "strike",
    "condense",
    "extend",
    "outline",
    "shadow",
    "u",
    "vertAlign",
    "sz",
    "color",
    "name",
    "family",
    "charset",
    "scheme",
];

fn new_font(base: Option<Element>, set: &CellFormat, p: &str) -> Result<Element, String> {
    let mut f = base.unwrap_or_else(|| Element::new(&format!("{p}font")));
    let put = |f: &mut Element, local: &str, el: Option<Element>| {
        f.children.retain(|n| !matches!(n, Node::El(c) if c.local() == local));
        if let Some(el) = el {
            xml::insert_ordered(f, el, &FONT_ORDER);
        }
    };
    for (local, v) in [("b", set.bold), ("i", set.italic), ("strike", set.strike)] {
        if let Some(on) = v {
            put(&mut f, local, on.then(|| Element::new(&format!("{p}{local}"))));
        }
    }
    if let Some(on) = set.underline {
        put(&mut f, "u", on.then(|| Element::new(&format!("{p}u"))));
    }
    if let Some(sz) = set.size {
        let v = format!("{}", sz as f64 / 100.0);
        put(&mut f, "sz", Some(Element::new(&format!("{p}sz")).with_attr("val", &v)));
    }
    if let Some(c) = &set.color {
        put(&mut f, "color", Some(color_el(&format!("{p}color"), c)?));
    }
    if let Some(name) = &set.font {
        put(&mut f, "name", Some(Element::new(&format!("{p}name")).with_attr("val", name)));
        // A theme font's scheme would win over the name.
        put(&mut f, "scheme", None);
    }
    Ok(f)
}

fn new_fill(f: &Fill, p: &str) -> Result<Element, String> {
    let mut pf = Element::new(&format!("{p}patternFill"));
    match f {
        Fill::None => pf.set("patternType", "none"),
        Fill::Color(c) => {
            pf.set("patternType", "solid");
            pf.children.push(Node::El(color_el(&format!("{p}fgColor"), c)?));
            pf.children.push(Node::El(Element::new(&format!("{p}bgColor")).with_attr("indexed", "64")));
        }
        other => return Err(format!("fill={other} is shown but cannot be written; write a colour or none")),
    }
    let mut fill = Element::new(&format!("{p}fill"));
    fill.children.push(Node::El(pf));
    Ok(fill)
}

/// The Excel style for a written border: the nearest width of its kind.
pub fn snap(b: &Border) -> Result<Option<(&'static str, &Color)>, String> {
    match b {
        Border::None => Ok(None),
        Border::Line { width, style, color } => {
            let best = XL_BORDERS
                .iter()
                .filter(|x| x.2 == style && x.0 != "slantDashDot")
                .min_by_key(|x| (x.1 - width).abs())
                .ok_or_else(|| format!("{b}: a workbook has no {style} border; write solid, dashed, dotted, double, dash-dot or dash-dot-dot"))?;
            Ok(Some((best.0, color)))
        }
    }
}

const BORDER_ORDER: [&str; 9] =
    ["start", "left", "end", "right", "top", "bottom", "diagonal", "vertical", "horizontal"];

fn new_border(base: Option<Element>, sides: &[Option<Border>; 4], p: &str) -> Result<Element, String> {
    let mut b = base.unwrap_or_else(|| Element::new(&format!("{p}border")));
    for (k, want) in sides.iter().enumerate() {
        let Some(want) = want else { continue };
        let names: &[&str] = match k {
            0 => &["top"],
            1 => &["right", "end"],
            2 => &["bottom"],
            _ => &["left", "start"],
        };
        let local = names.iter().find(|n| kid(&b, n).is_some()).copied().unwrap_or(names[0]);
        let mut side = kid(&b, local).cloned().unwrap_or_else(|| Element::new(&format!("{p}{local}")));
        side.children.retain(|n| !matches!(n, Node::El(c) if c.local() == "color"));
        match snap(want)? {
            None => {
                side.remove_attr("style");
            }
            Some((st, c)) => {
                side.set("style", st);
                side.children.push(Node::El(color_el(&format!("{p}color"), c)?));
            }
        }
        b.children.retain(|n| !matches!(n, Node::El(c) if c.local() == local));
        xml::insert_ordered(&mut b, side, &BORDER_ORDER);
    }
    Ok(b)
}

fn set_alignment(xf: &mut Element, set: &CellFormat, have: &CellFormat, p: &str) {
    let mut al = kid(xf, "alignment").cloned().unwrap_or_else(|| Element::new(&format!("{p}alignment")));
    if let Some(a) = &set.align {
        match a.as_str() {
            "general" => {
                al.remove_attr("horizontal");
            }
            "distribute" => al.set("horizontal", "distributed"),
            other => al.set("horizontal", other),
        }
    }
    if let Some(v) = &set.valign {
        match v.as_str() {
            "bottom" => {
                al.remove_attr("vertical");
            }
            "middle" => al.set("vertical", "center"),
            other => al.set("vertical", other),
        }
    }
    if let Some(n) = set.indent {
        if n == 0 {
            al.remove_attr("indent");
        } else {
            al.set("indent", &n.to_string());
            // Excel indents only left-, right- or distributed-aligned text.
            let h = set.align.as_ref().or(have.align.as_ref()).map(String::as_str);
            if matches!(h, None | Some("general" | "center" | "justify")) {
                al.set("horizontal", "left");
            }
        }
    }
    xf.children.retain(|n| !matches!(n, Node::El(c) if c.local() == "alignment"));
    if !al.attrs.is_empty() || al.has_elements() {
        // alignment comes first in an xf, before protection and extLst.
        xf.children.insert(0, Node::El(al));
    }
}
