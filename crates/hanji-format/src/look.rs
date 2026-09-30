//! An object's own look on a slide (§5.3): its fill, shown on its tag or
//! slot marker after its box, in the direct-formatting vocabulary (§5.1).
//! A fill left out is no fill.

use crate::vocab::{self, Fill};

/// An object's fill, canonical (see [`vocab::Fill`]); `None` is no fill.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Look {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
}

/// The keys of an object's look, in canonical order.
pub const LOOK_KEYS: &[&str] = &["fill"];

impl Look {
    pub fn is_empty(&self) -> bool {
        self.fill.is_none()
    }

    /// `fill=…`, as a tag writes it (empty for no look).
    pub fn attrs(&self) -> String {
        match &self.fill {
            Some(f) => format!("fill={}", vocab::quote(f)),
            None => String::new(),
        }
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
    let mut seen = false;
    let mut rest = vec![];
    for (k, v, col) in attrs {
        match k.as_str() {
            "fill" => {
                if seen {
                    return Err((*col, "fill= is written twice.".into()));
                }
                seen = true;
                look.fill = parse_fill(v).map_err(|m| (*col, m))?;
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
    }
}
