//! An object's own look on a slide (§5.3): its preset shape and its
//! adjustments, its fill and outline (and a line's arrowheads), shown on its
//! tag or slot marker after its box, in the direct-formatting vocabulary
//! (§5.1). A kind left out is a rectangle (a straight line), adjustments left
//! out are the preset's own, and a fill or outline left out is none.

use crate::vocab::{self, Border, Fill};

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Look {
    /// Its preset shape, a DrawingML name (`roundRect`); `None` is a
    /// rectangle, or for a line a straight one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Its preset's adjustments, canonical (see [`parse_adj`]); `None` is
    /// the preset's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adj: Option<String>,
    /// Its fill, canonical (see [`vocab::Fill`]); `None` is no fill.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    /// Its outline, canonical (see [`vocab::Border`]); `None` is none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<String>,
    /// A line's arrowheads at its `from` and `to` ends (one of [`ARROWS`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    /// Its effects' summary, their names in the file's order (one of
    /// [`EFFECTS`] each); `None` is none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effects: Option<String>,
}

/// The keys of an object's look, in canonical order.
pub const LOOK_KEYS: &[&str] = &["kind", "adj", "fill", "border", "start", "end", "effects"];

/// The effects a summary names; `custom` is an effect graph.
pub const EFFECTS: &[&str] =
    &["shadow", "inner-shadow", "glow", "soft-edges", "reflection", "blur", "fill-overlay", "custom"];

/// Arrowheads (DrawingML's line end types), `none` for none.
pub const ARROWS: &[&str] = &["triangle", "stealth", "diamond", "oval", "arrow"];

impl Look {
    pub fn is_empty(&self) -> bool {
        self.kind.is_none()
            && self.adj.is_none()
            && self.fill.is_none()
            && self.border.is_none()
            && self.start.is_none()
            && self.end.is_none()
            && self.effects.is_none()
    }

    /// `kind=… adj=… fill=… border=… start=… end=…`, as a tag writes them
    /// (empty for no look).
    pub fn attrs(&self) -> String {
        let mut out = vec![];
        let keys = [
            ("kind", &self.kind),
            ("adj", &self.adj),
            ("fill", &self.fill),
            ("border", &self.border),
            ("start", &self.start),
            ("end", &self.end),
            ("effects", &self.effects),
        ];
        for (k, v) in keys {
            if let Some(v) = v {
                // A shape's kind and adjustments are quoted, as a picture's mask is.
                match k {
                    "kind" | "adj" | "effects" => out.push(format!("{k}=\"{v}\"")),
                    _ => out.push(format!("{k}={}", vocab::quote(v))),
                }
            }
        }
        out.join(" ")
    }
}

/// An effects summary as the text writes it → its canonical form; `None`
/// for `none`.
pub fn parse_effects(v: &str) -> Result<Option<String>, String> {
    let names: Vec<&str> = v.split_whitespace().collect();
    if names == ["none"] {
        return Ok(None);
    }
    if names.is_empty() {
        return Err("effects=\"\" names no effect; leave effects out for none.".into());
    }
    let mut out: Vec<&str> = vec![];
    for n in names {
        if !EFFECTS.contains(&n) {
            return Err(format!("effects=\"{v}\": {n} is not an effect; they are {}.", EFFECTS.join(", ")));
        }
        if !out.contains(&n) {
            out.push(n);
        }
    }
    Ok(Some(out.join(" ")))
}

/// A preset shape as the text writes it: one of DrawingML's names.
pub fn parse_kind(v: &str) -> Result<String, String> {
    let v = v.trim();
    if crate::presets::is_preset(v) {
        return Ok(v.to_string());
    }
    let near = crate::presets::near_presets(v);
    let hint = if near.is_empty() {
        " such as roundRect, ellipse, triangle, chevron or bentConnector3".to_string()
    } else {
        format!(": {}", near.join(", "))
    };
    Err(format!(
        "kind=\"{v}\" is not a preset shape; a kind is a DrawingML preset name{hint}; a rectangle leaves kind out."
    ))
}

/// A preset's adjustments as the text writes them: its guides' names and
/// values in the file's units (`adj1=10000 adj2=50000`), or one number for
/// a preset whose one guide is `adj` (`16667`).
pub fn adj_values(v: &str) -> Result<Vec<(String, i64)>, String> {
    let bad = || {
        format!("adj=\"{v}\" is not a preset's adjustments; write one number (adj=\"16667\") or name=number pairs (adj=\"adj1=10000 adj2=50000\").")
    };
    let toks: Vec<&str> = v.split_whitespace().collect();
    match toks.as_slice() {
        [] => Err(bad()),
        [n] if !n.contains('=') => Ok(vec![("adj".into(), n.parse().map_err(|_| bad())?)]),
        _ => toks
            .iter()
            .map(|t| {
                let (k, n) = t.split_once('=').ok_or_else(bad)?;
                let ok = !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric());
                if !ok {
                    return Err(bad());
                }
                Ok((k.to_string(), n.parse().map_err(|_| bad())?))
            })
            .collect(),
    }
}

/// Adjustments in canonical form: one number for a lone `adj`, else
/// `name=value` pairs in order.
pub fn adj_text(vals: &[(String, i64)]) -> String {
    match vals {
        [(k, n)] if k == "adj" => n.to_string(),
        _ => vals.iter().map(|(k, n)| format!("{k}={n}")).collect::<Vec<_>>().join(" "),
    }
}

/// Adjustments as the text writes them → their canonical form.
pub fn parse_adj(v: &str) -> Result<String, String> {
    let vals = adj_values(v)?;
    let mut names: Vec<&str> = vals.iter().map(|(k, _)| k.as_str()).collect();
    names.sort();
    if names.windows(2).any(|w| w[0] == w[1]) {
        return Err(format!("adj=\"{v}\" names a guide twice."));
    }
    Ok(adj_text(&vals))
}

/// An outline as the text writes it → its canonical form; `None` for `none`.
pub fn parse_border(v: &str) -> Result<Option<String>, String> {
    match Border::parse(v)? {
        Border::None => Ok(None),
        b => Ok(Some(b.to_string())),
    }
}

/// Whether a canonical outline is one the text shows but cannot write.
pub fn is_kept_border(b: &str) -> bool {
    Border::parse(b).is_ok_and(|b| !b.is_writable())
}

/// An arrowhead as the text writes it; `None` for `none`.
pub fn parse_arrow(k: &str, v: &str) -> Result<Option<String>, String> {
    match v.trim() {
        "none" => Ok(None),
        a if ARROWS.contains(&a) => Ok(Some(a.to_string())),
        _ => Err(format!("{k}={v} is not an arrowhead; it is none, {}", ARROWS.join(", "))),
    }
}

/// A fill as the text writes it → its canonical form; `None` for `none`.
pub fn parse_fill(v: &str) -> Result<Option<String>, String> {
    match Fill::parse(v)? {
        Fill::None => Ok(None),
        f => Ok(Some(f.to_string())),
    }
}

/// Whether a canonical fill is one the text shows but cannot write
/// (a gradient, pattern or picture, or a colour with `*`).
pub fn is_kept_fill(f: &str) -> bool {
    Fill::parse(f).is_ok_and(|f| !f.is_writable())
}

/// The look keys of `attrs` (name, value, column): the look they give,
/// and the other attributes.
#[allow(clippy::type_complexity)]
pub fn split_look_keys(
    attrs: &[(String, String, usize)],
) -> Result<(Look, Vec<(String, String, usize)>), (usize, String)> {
    let mut look = Look::default();
    let mut seen: Vec<&str> = vec![];
    let mut rest = vec![];
    for (k, v, col) in attrs {
        let key = k.as_str();
        if LOOK_KEYS.contains(&key) {
            if seen.contains(&key) {
                return Err((*col, format!("{k}= is written twice.")));
            }
            seen.push(key);
        }
        let e = |m: String| (*col, m);
        match key {
            "kind" => look.kind = Some(parse_kind(v).map_err(e)?),
            "adj" => look.adj = Some(parse_adj(v).map_err(e)?),
            "fill" => look.fill = parse_fill(v).map_err(e)?,
            "border" => look.border = parse_border(v).map_err(e)?,
            "start" => look.start = parse_arrow(k, v).map_err(e)?,
            "end" => look.end = parse_arrow(k, v).map_err(e)?,
            "effects" => look.effects = parse_effects(v).map_err(e)?,
            "shadow" | "effect" | "glow" => {
                return Err((
                    *col,
                    format!("{k}= is not a key here: an object's effects are effects=, as in effects=\"shadow\"."),
                ));
            }
            "outline" | "stroke" | "line-color" => {
                return Err((
                    *col,
                    format!(
                        "{k}= is not a key here: an object's outline is border=, as in border=\"1pt solid accent1\"."
                    ),
                ));
            }
            "shape" | "preset" | "prst" | "geometry" | "prstGeom" => {
                return Err((
                    *col,
                    format!("{k}= is not a key here: an object's preset shape is kind=, as in kind=\"roundRect\"."),
                ));
            }
            "background" | "background-color" | "bg" => {
                return Err((*col, format!("{k}= is not a key here: an object's fill is fill=, as in fill=accent1.")));
            }
            _ => rest.push((k.clone(), v.clone(), *col)),
        }
    }
    Ok((look, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_parse_and_write_canonically() {
        let a = |s: &str| vec![("fill".to_string(), s.to_string(), 1)];
        assert_eq!(split_look_keys(&a("accent1")).unwrap().0.attrs(), "fill=accent1");
        assert_eq!(split_look_keys(&a("#ff7f50")).unwrap().0.attrs(), "fill=#FF7F50");
        assert_eq!(split_look_keys(&a("accent1+40%/50%")).unwrap().0.attrs(), "fill=accent1+40%/50%");
        assert!(split_look_keys(&a("none")).unwrap().0.is_empty());
        assert_eq!(split_look_keys(&a("gradient")).unwrap().0.attrs(), "fill=gradient");
        assert!(split_look_keys(&a("red")).is_err());
        assert!(is_kept_fill("gradient") && is_kept_fill("accent1*") && !is_kept_fill("accent1"));
        let b = |s: &str| vec![("border".to_string(), s.to_string(), 1)];
        assert_eq!(split_look_keys(&b("1pt solid #000000")).unwrap().0.attrs(), "border=\"1pt solid #000000\"");
        assert!(split_look_keys(&b("none")).unwrap().0.is_empty());
        assert!(split_look_keys(&b("1pt wavy red")).is_err());
        let a = vec![("end".to_string(), "triangle".to_string(), 1), ("start".to_string(), "none".to_string(), 5)];
        assert_eq!(split_look_keys(&a).unwrap().0.attrs(), "end=triangle");
        assert!(split_look_keys(&[("end".to_string(), "big".to_string(), 1)]).is_err());
    }

    #[test]
    fn kinds_and_adjustments_parse_and_write_canonically() {
        let k = |kind: &str, adj: &str| {
            let mut a = vec![("fill".to_string(), "accent1".to_string(), 1), ("kind".to_string(), kind.to_string(), 9)];
            if !adj.is_empty() {
                a.push(("adj".to_string(), adj.to_string(), 20));
            }
            split_look_keys(&a).map(|l| l.0.attrs())
        };
        assert_eq!(k("roundRect", "").unwrap(), "kind=\"roundRect\" fill=accent1");
        assert_eq!(k("roundRect", " 16667 ").unwrap(), "kind=\"roundRect\" adj=\"16667\" fill=accent1");
        assert_eq!(k("roundRect", "adj=16667").unwrap(), "kind=\"roundRect\" adj=\"16667\" fill=accent1");
        assert_eq!(
            k("rightArrow", "adj1=50000  adj2=-5").unwrap(),
            "kind=\"rightArrow\" adj=\"adj1=50000 adj2=-5\" fill=accent1"
        );
        let e = k("roundedRectangle", "").unwrap_err().1;
        assert!(e.contains("not a preset shape") && e.contains("kind"), "{e}");
        assert!(k("hexagon", "adj=1 adj=2").unwrap_err().1.contains("twice"));
        assert!(k("hexagon", "25%").unwrap_err().1.contains("adjustments"));
        let e = split_look_keys(&[("shape".to_string(), "ellipse".to_string(), 3)]).unwrap_err().1;
        assert!(e.contains("kind="), "{e}");
    }
}
