//! Formatting in a flow document (DESIGN.md §5.2, F2): the style section's
//! values, and the lifting rules of canonical form. The AST holds, per
//! element, only what differs from its style (a paragraph's own keys, a
//! unit's text keys) or, for a cell, what it sets beyond the defaults; row
//! and table lines, and a paragraph's `{…}` text keys, are a way of writing
//! them. The parser spreads them back onto the elements.

use crate::ast::*;

/// A style line's values resolved against the default style's: the
/// default's line is complete (a property it leaves out is 0pt, none, off
/// or `align=left`), any other line holds what differs from it.
#[derive(Clone, Debug, Default)]
pub struct StyleTable {
    /// The default paragraph style's name, when known.
    pub default: Option<String>,
    pub lines: Vec<StyleLine>,
}

/// What the default style's line leaves out: 0pt, none, off, left.
pub fn implicit() -> Props {
    let mut p = Props::new();
    p.set(Key::Align, Value::Choice("left".into()));
    for k in [Key::IndentLeft, Key::IndentRight, Key::FirstLine, Key::SpaceBefore, Key::SpaceAfter] {
        p.set(k, Value::Len(0));
    }
    p.set(Key::Fill, Value::Fill(crate::vocab::Fill::None));
    for k in Key::SIDES {
        p.set(k, Value::Border(crate::vocab::Border::None));
    }
    for k in Key::FLAGS {
        p.set(k, Value::Flag(false));
    }
    p
}

/// A cell's box when nothing sets it: no fill, no borders, top.
pub fn box_default() -> Props {
    let mut p = implicit().only(&Key::BOX);
    p.set(Key::Valign, Value::Choice("top".into()));
    p
}

/// The keys a style line may hold.
pub const STYLE_KEYS: [Key; 19] = [
    Key::Fill,
    Key::BorderTop,
    Key::BorderRight,
    Key::BorderBottom,
    Key::BorderLeft,
    Key::Align,
    Key::IndentLeft,
    Key::IndentRight,
    Key::FirstLine,
    Key::SpaceBefore,
    Key::SpaceAfter,
    Key::LineSpacing,
    Key::Font,
    Key::Size,
    Key::Color,
    Key::Bold,
    Key::Italic,
    Key::Underline,
    Key::Strike,
];

/// A paragraph's own keys: its layout, shading and borders.
pub const PARA_OWN: [Key; 12] = [
    Key::Fill,
    Key::BorderTop,
    Key::BorderRight,
    Key::BorderBottom,
    Key::BorderLeft,
    Key::Align,
    Key::IndentLeft,
    Key::IndentRight,
    Key::FirstLine,
    Key::SpaceBefore,
    Key::SpaceAfter,
    Key::LineSpacing,
];

/// What a table line holds for the cells' paragraphs: layout and text keys
/// (a table line's `fill` and borders are the cells').
pub const TABLE_PARA: [Key; 10] = [
    Key::Align,
    Key::IndentLeft,
    Key::IndentRight,
    Key::FirstLine,
    Key::SpaceBefore,
    Key::SpaceAfter,
    Key::LineSpacing,
    Key::Font,
    Key::Size,
    Key::Color,
];

impl StyleTable {
    pub fn line(&self, name: &str) -> Option<&StyleLine> {
        self.lines.iter().find(|l| l.name == name)
    }

    /// The default style's values, complete.
    pub fn default_values(&self) -> Props {
        let own = self.default.as_deref().and_then(|d| self.line(d)).map(|l| l.props.clone()).unwrap_or_default();
        implicit().overlay(&own)
    }

    /// The values of style `name` (`None`: the default style).
    pub fn values(&self, name: Option<&str>) -> Props {
        let base = self.default_values();
        match name.filter(|n| Some(*n) != self.default.as_deref()).and_then(|n| self.line(n)) {
            Some(l) => base.overlay(&l.props),
            None => base,
        }
    }

    /// A style line in canonical form: the default's without what it leaves
    /// out, any other's without what equals the default's.
    pub fn canonical(&self, line: &StyleLine) -> Props {
        if Some(line.name.as_str()) == self.default.as_deref() {
            line.props.diff(&implicit())
        } else {
            line.props.diff(&self.default_values())
        }
    }
}

/// The text keys every unit that takes properties has with the same value
/// (`None` for a key where one has none, or they differ; nothing when no
/// unit takes properties).
pub fn common_text(inl: &Inline) -> Props {
    let mut units = inl.units.iter().filter(|u| u.takes_props());
    let Some(first) = units.next() else { return Props::new() };
    let mut out = first.props.only(&Key::TEXT);
    for u in units {
        out.0.retain(|k, v| u.props.get(*k) == Some(v));
    }
    out
}

/// Whether a paragraph shows formatting: it has a unit that takes properties.
pub fn shows(inl: &Inline) -> bool {
    inl.units.iter().any(Unit::takes_props)
}

/// The value most of `vals` have, and how many: ties go to the first seen.
pub fn mode<T: PartialEq + Clone>(vals: &[T]) -> Option<(T, usize)> {
    let mut best: Option<(T, usize)> = None;
    for v in vals {
        let n = vals.iter().filter(|x| *x == v).count();
        if best.as_ref().is_none_or(|b| n > b.1) {
            best = Some((v.clone(), n));
        }
    }
    best
}
