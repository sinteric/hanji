//! An object's own look on a slide (§5.3): its fill and outline (and a
//! line's arrowheads), shown on its tag or slot marker after its box, in the
//! direct-formatting vocabulary (§5.1). A fill or outline left out is none.

use crate::vocab::{self, Border, Fill};

/// An object's fill, canonical (see [`vocab::Fill`]); `None` is no fill.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Look {
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
}

/// The keys of an object's look, in canonical order.
pub const LOOK_KEYS: &[&str] = &["fill", "border", "start", "end"];

/// Arrowheads (DrawingML's line end types), `none` for none.
pub const ARROWS: &[&str] = &["triangle", "stealth", "diamond", "oval", "arrow"];

impl Look {
    pub fn is_empty(&self) -> bool {
        self.fill.is_none() && self.border.is_none() && self.start.is_none() && self.end.is_none()
    }

    /// `fill=… border=… start=… end=…`, as a tag writes them (empty for no look).
    pub fn attrs(&self) -> String {
        let mut out = vec![];
        for (k, v) in [("fill", &self.fill), ("border", &self.border), ("start", &self.start), ("end", &self.end)] {
            if let Some(v) = v {
                out.push(format!("{k}={}", vocab::quote(v)));
            }
        }
        out.join(" ")
    }
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
            "fill" => look.fill = parse_fill(v).map_err(e)?,
            "border" => look.border = parse_border(v).map_err(e)?,
            "start" => look.start = parse_arrow(k, v).map_err(e)?,
            "end" => look.end = parse_arrow(k, v).map_err(e)?,
            "outline" | "stroke" | "line-color" => {
                return Err((
                    *col,
                    format!(
                        "{k}= is not a key here: an object's outline is border=, as in border=\"1pt solid accent1\"."
                    ),
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
}
