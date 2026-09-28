//! The resolved model: the text's blocks with style names resolved against
//! the file's style set. Remainder anchors refer to paths in it.

use hanji_format::{self as fmt, Atom, Cell, Diagnostic, Inline};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Block {
    Para(Para),
    Table(Table),
    /// A block placeholder, by id.
    Keep(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Para {
    /// Paragraph style name.
    pub style: String,
    pub content: Inline,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Table {
    /// Table style name; `None` is the file's default table style.
    pub style: Option<String>,
    /// Cell paragraph styles: `None` is the default paragraph style.
    pub rows: Vec<Vec<Cell>>,
}

/// Anchor path: `[block]`, `[block, row]`, `[block, row, col]` or
/// `[block, row, col, paragraph]`; empty for the document itself.
pub type Path = Vec<usize>;

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StyleDef {
    pub id: String,
    pub name: String,
}

/// A file's paragraph and table styles.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StyleSet {
    pub paragraph: Vec<StyleDef>,
    /// Name of the default paragraph style.
    pub default_paragraph: String,
    /// Names of Heading 1–6, where the file has them.
    pub headings: [Option<String>; 6],
    pub table: Vec<StyleDef>,
    pub default_table: Option<String>,
}

impl StyleSet {
    pub fn heading_level(&self, name: &str) -> Option<u8> {
        self.headings.iter().position(|h| h.as_deref() == Some(name)).map(|k| k as u8 + 1)
    }
    pub fn paragraph_names(&self) -> Vec<String> {
        self.paragraph.iter().map(|s| s.name.clone()).collect()
    }
    pub fn table_names(&self) -> Vec<String> {
        self.table.iter().map(|s| s.name.clone()).collect()
    }
    pub fn paragraph_id(&self, name: &str) -> Option<&str> {
        self.paragraph.iter().find(|s| s.name == name).map(|s| s.id.as_str())
    }
    pub fn paragraph_name(&self, id: &str) -> Option<&str> {
        self.paragraph.iter().find(|s| s.id == id).map(|s| s.name.as_str())
    }
    pub fn table_id(&self, name: &str) -> Option<&str> {
        self.table.iter().find(|s| s.name == name).map(|s| s.id.as_str())
    }
    pub fn table_name(&self, id: &str) -> Option<&str> {
        self.table.iter().find(|s| s.id == id).map(|s| s.name.as_str())
    }
    /// The id of the default paragraph style (`Normal` when the file names none).
    pub fn default_paragraph_id(&self) -> &str {
        self.paragraph_id(&self.default_paragraph).unwrap_or("Normal")
    }
    /// The text's name for table style `id`: `None` for the default table
    /// style; an id missing from the style set stands for itself.
    pub fn table_style_name(&self, id: &str) -> Option<String> {
        let name = self.table_name(id).unwrap_or(id);
        (Some(name) != self.default_table.as_deref()).then(|| name.to_string())
    }
}

/// What an engine can export; the rest of the format is refused with a reason.
#[derive(Clone, Copy, Debug, Default)]
pub struct Capabilities {
    pub links: bool,
    pub fields: bool,
    pub footnotes: bool,
    pub math: bool,
}

/// An empty paragraph's content.
pub(crate) static EMPTY: Inline = Inline { units: Vec::new(), spans: Vec::new() };

/// Content of one unmarked atom.
fn single(atom: Atom) -> Inline {
    Inline { units: vec![fmt::Unit { atom, marks: fmt::Marks::NONE }], spans: vec![] }
}

/// The source map of each block [`resolve`] returns, in the same order:
/// footnote definitions are not blocks of the model.
pub fn block_maps(parsed: &fmt::Parsed) -> Vec<&fmt::BlockMap> {
    let blocks = parsed.doc.blocks.iter().zip(&parsed.map.blocks);
    blocks.filter(|(b, _)| !matches!(b, fmt::Block::FootnoteDef(_))).map(|(_, m)| m).collect()
}

/// Model text → resolved blocks. `is_block_keep(id)` says whether a
/// placeholder stands for a block (`Some(true)`), an inline object
/// (`Some(false)`) or is unknown (`None`).
pub fn resolve(
    parsed: &fmt::Parsed,
    text: &str,
    styles: &StyleSet,
    caps: Capabilities,
    is_block_keep: &dyn Fn(&str) -> Option<bool>,
) -> Result<Vec<Block>, Vec<Diagnostic>> {
    let mut out = vec![];
    let mut errs = vec![];
    // Line of each block, counted incrementally (blocks are in text order).
    let (mut line, mut counted) = (1, 0);
    for (b, m) in parsed.doc.blocks.iter().zip(&parsed.map.blocks) {
        line += text[counted..m.start].matches('\n').count();
        counted = m.start;
        let mut err = |msg: String| errs.push(Diagnostic { line, col: 1, message: msg });
        let para = |style: String, content: Inline| Block::Para(Para { style, content });
        match b {
            fmt::Block::Para(p) => {
                let style = match &p.style {
                    fmt::ParaStyle::Plain => styles.default_paragraph.clone(),
                    fmt::ParaStyle::Heading(n) => match &styles.headings[*n as usize - 1] {
                        Some(s) => s.clone(),
                        None => {
                            let have: Vec<String> =
                                (1..=6).filter(|k| styles.headings[k - 1].is_some()).map(|k| "#".repeat(k)).collect();
                            err(format!("this file has no Heading {n} style; its heading levels are: {}. Use one of them or a <div style=\"Name\">.", if have.is_empty() { "none".into() } else { have.join(", ") }));
                            continue;
                        }
                    },
                    fmt::ParaStyle::Named(s) => s.clone(),
                };
                check_inline(&p.content, caps, is_block_keep, &mut err);
                out.push(para(style, p.content.clone()));
            }
            fmt::Block::Keep(k) => match is_block_keep(&k.id) {
                Some(true) => out.push(Block::Keep(k.id.clone())),
                Some(false) => out.push(para(styles.default_paragraph.clone(), single(Atom::Keep(k.clone())))),
                None => {
                    err(format!("placeholder id=\"{}\" is not in this file; placeholders are never created.", k.id))
                }
            },
            fmt::Block::PageBreak => out.push(para(styles.default_paragraph.clone(), single(Atom::PageBreak))),
            fmt::Block::Table(t) => {
                let mut rows = t.rows.clone();
                for c in rows.iter_mut().flatten() {
                    if let Cell::Text(ps) = c {
                        for p in ps {
                            check_inline(&p.content, caps, is_block_keep, &mut err);
                            if p.style.as_deref() == Some(styles.default_paragraph.as_str()) {
                                p.style = None;
                            }
                        }
                    }
                }
                out.push(Block::Table(Table { style: t.style.clone(), rows }));
            }
            fmt::Block::FootnoteDef(f) => {
                if !caps.footnotes {
                    err(format!(
                        "footnotes cannot be written to this file format yet; remove [^{}] and its definition.",
                        f.label
                    ));
                }
            }
        }
    }
    if errs.is_empty() {
        Ok(out)
    } else {
        Err(errs)
    }
}

fn check_inline(
    i: &Inline,
    caps: Capabilities,
    is_block_keep: &dyn Fn(&str) -> Option<bool>,
    err: &mut dyn FnMut(String),
) {
    for s in &i.spans {
        match &s.kind {
            fmt::SpanKind::Link(_) if !caps.links => {
                err("links cannot be written to this file format yet; write the link text only.".into())
            }
            fmt::SpanKind::Field(n) if !caps.fields => {
                err(format!("<field name=\"{n}\"> cannot be written to this file format yet."))
            }
            _ => {}
        }
    }
    for u in &i.units {
        match &u.atom {
            Atom::Math(_) if !caps.math => err("math cannot be written to this file format yet.".into()),
            Atom::NoteRef(l) if !caps.footnotes => {
                err(format!("footnote [^{l}] cannot be written to this file format yet."))
            }
            Atom::Keep(k) if is_block_keep(&k.id) == Some(true) => {
                err(format!("placeholder id=\"{}\" stands for a whole block; keep it alone on its own line.", k.id))
            }
            _ => {}
        }
    }
}

/// Resolved blocks → the format AST, in canonical shape.
pub fn unresolve(
    blocks: &[Block],
    styles: &StyleSet,
    front: fmt::FrontMatter,
    keep: &dyn Fn(&str) -> fmt::Keep,
) -> fmt::Document {
    let mut out = vec![];
    for b in blocks {
        out.push(match b {
            Block::Keep(id) => fmt::Block::Keep(keep(id)),
            Block::Table(t) => fmt::Block::Table(fmt::Table { style: t.style.clone(), rows: t.rows.clone() }),
            Block::Para(p) => {
                // Empty: `<p/>` / `<p style="Name"/>`. Spaces only: a `<div>`.
                // Spaces only: a `<div>` naming the style, so the spaces stay text.
                let empty = p.content.is_empty();
                let blank = p.content.units.iter().all(|u| u.atom.is_space());
                let style = match styles.heading_level(&p.style) {
                    Some(n) if !blank => fmt::ParaStyle::Heading(n),
                    _ if p.style == styles.default_paragraph && (empty || !blank) => fmt::ParaStyle::Plain,
                    _ => fmt::ParaStyle::Named(p.style.clone()),
                };
                fmt::Block::Para(fmt::Para { style, content: p.content.clone() })
            }
        });
    }
    let mut d = fmt::Document { front, blocks: out };
    d.normalize();
    d
}

/// Units of every paragraph position in document order: `(path, Some(content))`
/// for paragraphs and cell paragraphs (`[block, row, col, paragraph]`; a `^^`
/// cell has one empty one), `(path, None)` for block placeholders. `||`
/// cells have no paragraph.
pub fn paras(blocks: &[Block]) -> Vec<(Path, Option<&Inline>)> {
    let mut out = vec![];
    for (j, b) in blocks.iter().enumerate() {
        match b {
            Block::Para(p) => out.push((vec![j], Some(&p.content))),
            Block::Keep(_) => out.push((vec![j], None)),
            Block::Table(t) => {
                for (r, row) in t.rows.iter().enumerate() {
                    for (c, cell) in row.iter().enumerate() {
                        match cell {
                            Cell::Text(ps) => {
                                out.extend(ps.iter().enumerate().map(|(k, p)| (vec![j, r, c, k], Some(&p.content))))
                            }
                            Cell::Up => out.push((vec![j, r, c, 0], Some(&EMPTY))),
                            Cell::Left => {}
                        }
                    }
                }
            }
        }
    }
    out
}

/// The paragraph content at `path`, if any.
pub fn content_at<'a>(blocks: &'a [Block], path: &[usize]) -> Option<&'a Inline> {
    match (blocks.get(*path.first()?)?, path.len()) {
        (Block::Para(p), 1) => Some(&p.content),
        (Block::Table(t), 4) => match t.rows.get(path[1])?.get(path[2])? {
            Cell::Text(ps) => ps.get(path[3]).map(|p| &p.content),
            _ => None,
        },
        _ => None,
    }
}

/// A comparable key per unit: the character, or a private value for an atom.
pub fn key(a: &Atom) -> u32 {
    fn h(tag: u8, s: &str) -> u32 {
        let mut x: u32 = 0x811c_9dc5 ^ tag as u32;
        for b in s.bytes() {
            x = (x ^ b as u32).wrapping_mul(0x0100_0193);
        }
        0x0011_0000 + x % (u32::MAX - 0x0011_0000)
    }
    match a {
        Atom::Char(c) => *c as u32,
        Atom::Break => '\n' as u32,
        Atom::Keep(k) => h(1, &k.id),
        Atom::NoteRef(l) => h(2, l),
        Atom::Math(m) => h(3, m),
        Atom::PageBreak => h(4, ""),
    }
}

pub fn keys(i: &Inline) -> Vec<u32> {
    i.units.iter().map(|u| key(&u.atom)).collect()
}
