//! Direct formatting in a Presentation's text (§5.3, fluency round 6 F1):
//! the text keys `font`, `size` and `color`, written on a shape's tag or a
//! slot's marker when all of its text shares them, at the end of a
//! paragraph (`{…}`) when all of that paragraph shares them, and on a
//! stretch of text as `[text]{…}`. Every value shown is the effective one,
//! whether the run sets it or inherits it; the model holds each unit's
//! effective values ([`SpanKind::Style`] spans), and where they are written
//! is canonical form (lifting).

use crate::ast::{Atom, Inline, Span, SpanKind};

/// A run's text formatting as the text shows it. `size` is in hundredths
/// of a point (`a:rPr sz`); `color` is a canonical colour (see
/// [`parse_color`]).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct TextStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// The text keys, in canonical order.
pub const TEXT_KEYS: &[&str] = &["font", "size", "color"];

pub use crate::vocab::{quote, COLOR_FORM, THEME_COLORS};

impl TextStyle {
    pub fn is_empty(&self) -> bool {
        self.font.is_none() && self.size.is_none() && self.color.is_none()
    }

    /// `self`'s values, and `under`'s where `self` has none.
    pub fn over(&self, under: &TextStyle) -> TextStyle {
        TextStyle {
            font: self.font.clone().or_else(|| under.font.clone()),
            size: self.size.or(under.size),
            color: self.color.clone().or_else(|| under.color.clone()),
        }
    }

    /// The values of `self` that `ctx` does not already give.
    pub fn minus(&self, ctx: &TextStyle) -> TextStyle {
        TextStyle {
            font: self.font.clone().filter(|v| ctx.font.as_ref() != Some(v)),
            size: self.size.filter(|v| ctx.size != Some(*v)),
            color: self.color.clone().filter(|v| ctx.color.as_ref() != Some(v)),
        }
    }

    /// The values every one of `all` has (none when `all` is empty).
    pub fn shared<'a>(all: impl IntoIterator<Item = &'a TextStyle>) -> TextStyle {
        let mut it = all.into_iter();
        let Some(first) = it.next() else { return TextStyle::default() };
        let mut out = first.clone();
        for s in it {
            if out.font != s.font {
                out.font = None;
            }
            if out.size != s.size {
                out.size = None;
            }
            if out.color != s.color {
                out.color = None;
            }
        }
        out
    }

    /// `key=value` pairs, space-separated, in canonical order (no leading space).
    pub fn attrs(&self) -> String {
        let mut out = vec![];
        if let Some(f) = &self.font {
            out.push(format!("font={}", quote(f)));
        }
        if let Some(s) = self.size {
            out.push(format!("size={}", size_text(s)));
        }
        if let Some(c) = &self.color {
            out.push(format!("color={}", quote(c)));
        }
        out.join(" ")
    }
}

/// A size in hundredths of a point as the text shows it: `18pt`, `10.5pt`.
pub fn size_text(sz: i64) -> String {
    crate::vocab::length_text(sz)
}

/// `24pt`, `10.5pt` or a bare number of points → hundredths of a point.
pub fn parse_size(v: &str) -> Result<i64, String> {
    let sz = crate::vocab::parse_length(v, false)
        .map_err(|_| format!("size={v} is not a size; it is a length in points, such as size=24pt or size=10.5pt"))?;
    if !(100..=400_000).contains(&sz) {
        return Err(format!("size={v} is out of range: a text size is from 1pt to 4000pt"));
    }
    Ok(sz)
}

/// A colour as the text writes it → its canonical form (see
/// [`crate::vocab::Color`]).
pub fn parse_color(v: &str) -> Result<String, String> {
    crate::vocab::Color::parse(v).map(|c| c.to_string())
}

/// Whether a canonical colour carries `*`: a stored adjustment the text
/// does not show, which cannot be written anew.
pub fn is_kept_color(c: &str) -> bool {
    crate::vocab::Color::parse(c).is_ok_and(|c| !c.is_writable())
}

/// The text keys of `attrs` (name, value, column): the style they give,
/// and the other attributes. An unknown key is left to the caller.
#[allow(clippy::type_complexity)]
pub fn split_text_keys(
    attrs: &[(String, String, usize)],
) -> Result<(TextStyle, Vec<(String, String, usize)>), (usize, String)> {
    let mut st = TextStyle::default();
    let mut rest = vec![];
    for (k, v, col) in attrs {
        let twice = |s: bool| if s { Err((*col, format!("{k}= is written twice."))) } else { Ok(()) };
        match k.as_str() {
            "font" => {
                twice(st.font.is_some())?;
                if v.trim().is_empty() {
                    return Err((*col, "font= names a font, such as font=Arial or font=\"맑은 고딕\".".into()));
                }
                st.font = Some(v.clone());
            }
            "size" => {
                twice(st.size.is_some())?;
                st.size = Some(parse_size(v).map_err(|m| (*col, m))?);
            }
            "color" | "colour" => {
                twice(st.color.is_some())?;
                if k == "colour" {
                    return Err((*col, "the key is color=, as in color=#1F4E79.".into()));
                }
                st.color = Some(match v.as_str() {
                    // No colour, and the fills the text shows but cannot write (kept while left as shown).
                    "none" | "gradient" | "pattern" | "picture" => v.clone(),
                    _ => parse_color(v).map_err(|m| (*col, m))?,
                });
            }
            _ => rest.push((k.clone(), v.clone(), *col)),
        }
    }
    Ok((st, rest))
}

/// A `{…}` or `[…]{…}` brace's inside → the style it gives.
pub fn parse_braces(src: &str) -> Result<TextStyle, String> {
    let mut kv = vec![];
    for (k, v, c) in crate::vocab::attrs(src).map_err(|e| e.1)? {
        match v {
            Some(v) => kv.push((k, v, c)),
            None if TEXT_KEYS.contains(&k.as_str()) => return Err(format!("{k} needs a value, such as {k}=…")),
            None => return Err(unknown_key(&k)),
        }
    }
    let (st, rest) = split_text_keys(&kv).map_err(|e| e.1)?;
    if let Some((k, _, _)) = rest.first() {
        return Err(unknown_key(k));
    }
    if st.is_empty() {
        return Err("empty braces {}: write the formatting, such as {size=24pt}, or leave the braces out".into());
    }
    Ok(st)
}

/// The error for a formatting key the text does not have.
pub fn unknown_key(k: &str) -> String {
    match k {
        "bold" | "italic" | "underline" | "strike" => {
            format!("{k} is written in the text itself: **bold**, *italic*, <u>underline</u>, ~~strike~~, not as a key")
        }
        "style" => {
            "a slide has no styles: write the formatting itself, such as size=24pt or color=#FF7F50, not style=".into()
        }
        _ => format!(
            "\"{k}\" is not a text formatting key; the keys are font, size and color, such as size=24pt color=#FF7F50"
        ),
    }
}

// ---------------------------------------------------------------- per unit

/// Whether a unit is text that shows: its formatting is what the text states.
pub fn visible(a: &Atom) -> bool {
    match a {
        Atom::Char(c) => !c.is_whitespace(),
        Atom::Math(_) => true,
        _ => false,
    }
}

/// Each unit's formatting, from the inline's style spans.
pub fn unit_styles(inl: &Inline) -> Vec<TextStyle> {
    let mut out = vec![TextStyle::default(); inl.units.len()];
    for s in &inl.spans {
        if let SpanKind::Style(st) = &s.kind {
            for o in out.iter_mut().take(s.end.min(inl.units.len())).skip(s.start) {
                *o = st.clone();
            }
        }
    }
    out
}

/// Replace the inline's style spans with `styles` (one per unit): one span
/// per stretch of equal, non-empty formatting. Other spans stay; where one
/// is present the style spans are left out (a link or field in a slide's
/// text is refused elsewhere).
pub fn set_unit_styles(inl: &mut Inline, styles: &[TextStyle]) {
    inl.spans.retain(|s| !matches!(s.kind, SpanKind::Style(_)));
    if !inl.spans.is_empty() {
        return;
    }
    let mut k = 0;
    while k < styles.len() {
        let a = k;
        while k < styles.len() && styles[k] == styles[a] {
            k += 1;
        }
        if !styles[a].is_empty() {
            inl.spans.push(Span { start: a, end: k, kind: SpanKind::Style(styles[a].clone()) });
        }
    }
}

/// `under` beneath every unit's formatting of `paras`.
pub fn fold_under(paras: &mut [&mut Inline], under: &TextStyle) {
    if under.is_empty() {
        return;
    }
    for p in paras.iter_mut() {
        let st: Vec<TextStyle> = unit_styles(p).iter().map(|s| s.over(under)).collect();
        set_unit_styles(p, &st);
    }
}

/// The canonical formatting of the units that do not show (spaces, line
/// breaks, placeholders), which the text never states on its own (§5.3):
/// in a paragraph with text, a unit between two stretches of equal
/// formatting has theirs; any other takes the paragraph's shared
/// formatting, and for what that leaves open, the text before it (else
/// after it). In a paragraph without text they have none.
pub fn normalize(paras: &mut [&mut Inline]) {
    let all: Vec<Vec<TextStyle>> = paras.iter().map(|p| unit_styles(p)).collect();
    for (p, st) in paras.iter_mut().zip(all) {
        let vis: Vec<usize> = (0..p.units.len()).filter(|&i| visible(&p.units[i].atom)).collect();
        let mut out = st.clone();
        if vis.is_empty() {
            // Nothing shows: nothing is stated, and the file keeps what it has.
            for o in out.iter_mut() {
                *o = TextStyle::default();
            }
        } else {
            let ctx = TextStyle::shared(vis.iter().map(|&i| &st[i]));
            for (i, o) in out.iter_mut().enumerate() {
                if visible(&p.units[i].atom) {
                    continue;
                }
                let prev = vis.iter().rev().find(|&&j| j < i).map(|&j| &st[j]);
                let next = vis.iter().find(|&&j| j > i).map(|&j| &st[j]);
                *o = match (prev, next) {
                    (Some(a), Some(b)) if a == b => a.clone(),
                    _ => ctx.over(prev.or(next).unwrap()),
                };
            }
        }
        set_unit_styles(p, &out);
    }
}

/// Lifting (canonical form): what an object's tag or marker states (what
/// all its shown text shares), what each paragraph states at its end (what
/// its shown text shares beyond that), and each paragraph's inline with
/// style spans holding only what is left to state, each span starting and
/// ending on shown text.
pub fn lift(paras: &[&Inline]) -> (TextStyle, Vec<TextStyle>, Vec<Inline>) {
    let all: Vec<Vec<TextStyle>> = paras.iter().map(|p| unit_styles(p)).collect();
    let obj = TextStyle::shared(
        paras.iter().zip(&all).flat_map(|(p, s)| p.units.iter().zip(s).filter(|(u, _)| visible(&u.atom)).map(|x| x.1)),
    );
    let mut own = vec![];
    let mut out = vec![];
    for (p, st) in paras.iter().zip(&all) {
        let vis: Vec<usize> = (0..p.units.len()).filter(|&i| visible(&p.units[i].atom)).collect();
        let mut q = (*p).clone();
        q.spans.retain(|s| !matches!(s.kind, SpanKind::Style(_)));
        if vis.is_empty() {
            own.push(TextStyle::default());
            out.push(q);
            continue;
        }
        let ctx = TextStyle::shared(vis.iter().map(|&i| &st[i]));
        own.push(ctx.minus(&obj));
        let stated: Vec<TextStyle> = st.iter().map(|s| s.minus(&ctx)).collect();
        if q.spans.is_empty() {
            let mut k = 0;
            let n = stated.len();
            while k < n {
                let a = k;
                while k < n && stated[k] == stated[a] {
                    k += 1;
                }
                if stated[a].is_empty() {
                    continue;
                }
                let (mut s, mut e) = (a, k);
                while s < e && !visible(&p.units[s].atom) {
                    s += 1;
                }
                while e > s && !visible(&p.units[e - 1].atom) {
                    e -= 1;
                }
                if s < e {
                    q.spans.push(Span { start: s, end: e, kind: SpanKind::Style(stated[a].clone()) });
                }
            }
        }
        // Marks as the text can write them between these brackets: a space
        // left at a stretch's edge goes outside it.
        q.normalize();
        out.push(q);
    }
    (obj, own, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_read_and_write_in_canonical_form() {
        assert_eq!(parse_size("24pt"), Ok(2400));
        assert_eq!(parse_size("10.5"), Ok(1050));
        assert_eq!(size_text(1050), "10.5pt");
        assert_eq!(size_text(1025), "10.25pt");
        assert!(parse_size("0pt").is_err() && parse_size("12px").is_err());
        assert_eq!(parse_color("#ff7f50").as_deref(), Ok("#FF7F50"));
        assert_eq!(parse_color("accent2/50%").as_deref(), Ok("accent2/50%"));
        assert_eq!(parse_color("accent1*/100%").as_deref(), Ok("accent1*"));
        assert!(parse_color("red").is_err() && parse_color("#FF7F5").is_err() && parse_color("accent7").is_err());
    }
}
