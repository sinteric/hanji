//! Typed AST of a Document file (§5.1, §5.2).
//!
//! Inline content is flat: a sequence of [`Unit`]s, each one character or one
//! atom with its emphasis [`Marks`], plus non-overlapping [`Span`]s for links
//! and fields. Canonical form is a function of this structure alone.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Document {
    pub front: FrontMatter,
    pub blocks: Vec<Block>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FrontMatter {
    pub doc_type: String,
    pub format: String,
    pub template: Option<String>,
    pub schema: u32,
}

impl FrontMatter {
    pub fn document(format: &str, template: Option<&str>) -> Self {
        FrontMatter {
            doc_type: "document".into(),
            format: format.into(),
            template: template.map(str::to_string),
            schema: crate::SCHEMA_VERSION,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Block {
    Para(Para),
    Table(Table),
    /// A line holding only a placeholder. Whether it stands for a block
    /// entry or an inline object alone in a default-style paragraph is the
    /// engine's call (it knows the placeholder registry).
    Keep(Keep),
    PageBreak,
    FootnoteDef(FootnoteDef),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Para {
    pub style: ParaStyle,
    pub content: Inline,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ParaStyle {
    /// A plain line: the file's default paragraph style.
    Plain,
    /// `#`–`######`: the file's own Heading 1–6.
    Heading(u8),
    /// `<div style="Name">`.
    Named(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Table {
    /// `{style="Name"}` line before the header row; `None` is the default table style.
    pub style: Option<String>,
    /// Every row has one cell per column; row 0 is the header row.
    pub rows: Vec<Vec<Cell>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Cell {
    Text(Inline),
    /// `^^`: merged into the cell above.
    Up,
    /// `||`: the cell to the left extends into this column.
    Left,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FootnoteDef {
    pub label: String,
    pub content: Inline,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Inline {
    pub units: Vec<Unit>,
    /// Sorted, non-overlapping, over `units` indices `[start, end)`.
    pub spans: Vec<Span>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Unit {
    pub atom: Atom,
    pub marks: Marks,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Atom {
    Char(char),
    /// `<br/>`.
    Break,
    /// Inline `<keep/>`.
    Keep(Keep),
    /// `[^label]`.
    NoteRef(String),
    /// `$…$`, kept opaque.
    Math(String),
    /// Only as the whole content of a resolved page-break paragraph; the
    /// text form is the block line `<pagebreak/>`.
    PageBreak,
}

impl Atom {
    pub fn is_space(&self) -> bool {
        matches!(self, Atom::Char(c) if c.is_whitespace())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Keep {
    pub id: String,
    pub kind: String,
    pub summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub kind: SpanKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SpanKind {
    Link(String),
    Field(String),
}

/// Emphasis flags of one unit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Marks(pub u8);

impl Marks {
    pub const NONE: Marks = Marks(0);
    pub const BOLD: Marks = Marks(1);
    pub const ITALIC: Marks = Marks(2);
    pub const STRIKE: Marks = Marks(4);
    pub const UNDERLINE: Marks = Marks(8);
    /// Opening order when several marks start together (outermost first).
    pub const ALL: [Marks; 4] = [Marks::BOLD, Marks::ITALIC, Marks::STRIKE, Marks::UNDERLINE];

    pub fn has(self, m: Marks) -> bool {
        self.0 & m.0 == m.0 && m.0 != 0
    }
    pub fn with(self, m: Marks, on: bool) -> Marks {
        if on {
            Marks(self.0 | m.0)
        } else {
            Marks(self.0 & !m.0)
        }
    }
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl fmt::Display for Marks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names = ["bold", "italic", "strike", "underline"];
        let on: Vec<&str> = Marks::ALL.iter().zip(names).filter(|(m, _)| self.has(**m)).map(|(_, n)| n).collect();
        write!(f, "{}", if on.is_empty() { "plain".into() } else { on.join("+") })
    }
}

impl Inline {
    pub fn plain(text: &str) -> Inline {
        Inline {
            units: text
                .chars()
                .map(|c| Unit { atom: if c == '\n' { Atom::Break } else { Atom::Char(c) }, marks: Marks::NONE })
                .collect(),
            spans: vec![],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.units.is_empty() && self.spans.is_empty()
    }

    /// The text with atoms shown as `\u{FFFC}` (for summaries and messages).
    pub fn text(&self) -> String {
        self.units
            .iter()
            .map(|u| match &u.atom {
                Atom::Char(c) => *c,
                Atom::Break => '\n',
                _ => '\u{FFFC}',
            })
            .collect()
    }

    /// Segment boundaries: span edges split a paragraph into independently
    /// marked stretches.
    pub(crate) fn segments(&self) -> Vec<(usize, usize, Option<&Span>)> {
        let mut out = vec![];
        let mut at = 0;
        for s in &self.spans {
            if s.start > at {
                out.push((at, s.start, None));
            }
            out.push((s.start, s.end, Some(s)));
            at = s.end;
        }
        if at < self.units.len() || out.is_empty() {
            out.push((at, self.units.len(), None));
        }
        out
    }

    /// Canonical marks: every stretch of a mark starts and ends on a
    /// non-space unit inside its segment. Text cannot express a mark that
    /// starts or ends on a space (`**fox **` is not emphasis), so boundary
    /// spaces go outside.
    pub fn normalize(&mut self) {
        let segs: Vec<(usize, usize)> = self.segments().iter().map(|s| (s.0, s.1)).collect();
        for (a, b) in segs {
            for m in Marks::ALL {
                let mut i = a;
                while i < b {
                    if !self.units[i].marks.has(m) {
                        i += 1;
                        continue;
                    }
                    let mut e = i;
                    while e < b && self.units[e].marks.has(m) {
                        e += 1;
                    }
                    let (mut s, mut t) = (i, e);
                    while s < t && self.units[s].atom.is_space() {
                        self.units[s].marks = self.units[s].marks.with(m, false);
                        s += 1;
                    }
                    while t > s && self.units[t - 1].atom.is_space() {
                        self.units[t - 1].marks = self.units[t - 1].marks.with(m, false);
                        t -= 1;
                    }
                    i = e;
                }
            }
        }
    }

    /// Whether the text can state mark `m` on unit `i` (a space at the edge
    /// of a mark cannot carry it; see [`Inline::normalize`]).
    pub fn mark_expressible(&self, i: usize, m: Marks) -> bool {
        let u = &self.units[i];
        if !u.atom.is_space() {
            return true;
        }
        let (a, b) =
            self.segments().iter().find(|s| s.0 <= i && i < s.1).map(|s| (s.0, s.1)).unwrap_or((0, self.units.len()));
        let left = (a..i).rev().find(|&k| !self.units[k].atom.is_space());
        let right = (i + 1..b).find(|&k| !self.units[k].atom.is_space());
        matches!((left, right), (Some(l), Some(r)) if self.units[l].marks.has(m) && self.units[r].marks.has(m))
    }
}

impl Document {
    /// Canonical block shapes: a plain paragraph holding only an unmarked
    /// placeholder is a keep line, one holding only a page break is `<pagebreak/>`,
    /// and every inline is normalized.
    pub fn normalize(&mut self) {
        for b in &mut self.blocks {
            match b {
                Block::Para(p) => {
                    p.content.normalize();
                    if p.style == ParaStyle::Plain && p.content.spans.is_empty() && p.content.units.len() == 1 {
                        let u = &p.content.units[0];
                        match &u.atom {
                            Atom::Keep(k) if u.marks.is_empty() => *b = Block::Keep(k.clone()),
                            Atom::PageBreak => *b = Block::PageBreak,
                            _ => {}
                        }
                    }
                }
                Block::Table(t) => {
                    for row in &mut t.rows {
                        for c in row {
                            if let Cell::Text(i) = c {
                                i.normalize();
                            }
                        }
                    }
                }
                Block::FootnoteDef(f) => f.content.normalize(),
                Block::Keep(_) | Block::PageBreak => {}
            }
        }
    }
}
