//! The resolved model: the text's blocks with style names resolved against
//! the file's style set. Remainder anchors refer to paths in it.

use hanji_format::{self as fmt, Atom, Cell, Diagnostic, Inline, Props};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Block {
    Para(Para),
    Table(Table),
    /// A block placeholder, by id.
    Keep(String),
    /// A structural line of a Presentation (§5.3) that holds the blocks after
    /// it: a slide, or a slot or shape of one.
    Head(Head),
}

/// A slide (`level` 0, `key` `slide`, `label` its layout), or a slot
/// (`level` 1, `key` `slot:title`) or shape (`level` 1, `key` `shape:s4`,
/// `label` its name) of the slide before it. The blocks after it, up to the
/// next head of its level or above, belong to it. Heads are aligned by what
/// they hold, and a slide's layout is its `label`: a changed layout is a
/// restyled slide, not another one.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Head {
    pub level: u8,
    pub key: String,
    pub label: String,
    /// Where a slot, shape, object, line or group is on its slide (§5.3).
    /// Like the label, not what the head is: a moved shape is the same shape.
    pub place: Option<Place>,
    /// A slot's, shape's or line's fill, outline and arrowheads (§5.3), likewise.
    pub look: Box<fmt::look::Look>,
}

/// A head's geometry: a box, a line's ends, or a group with its objects.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Place {
    Box(fmt::Geom),
    Line(fmt::Ends),
    Group(fmt::GroupItem),
    /// A picture: its box, image, crop, mask and alternative text.
    Picture(fmt::PictureItem),
}

impl Block {
    pub fn head(&self) -> Option<&Head> {
        match self {
            Block::Head(h) => Some(h),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Para {
    /// Paragraph style name; empty for a list item where the file's
    /// formatting is not shown (its style stays in the remainder).
    pub style: String,
    pub content: Inline,
    /// A list item (`- ` / `1. `).
    pub item: Option<ListItem>,
    /// What the paragraph sets beyond its style (§5.2): its layout, fill
    /// and borders. Its text keys are on the units.
    pub props: Props,
}

impl Para {
    pub fn new(style: String, content: Inline, item: Option<ListItem>) -> Para {
        Para { style, content, item, props: Props::default() }
    }
}

/// A paragraph's place in a list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ListItem {
    pub ordered: bool,
    /// Nesting level, 0 at the margin.
    pub level: usize,
    /// First item of a list in the text: a blank line or another block
    /// before it, or an item of the other kind before it at the margin.
    pub first: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Table {
    /// Table style name; `None` is the file's default table style.
    pub style: Option<String>,
    /// Cell paragraph styles: `None` is the default paragraph style.
    pub rows: Vec<Vec<Cell>>,
    /// Each cell's box properties, as [`fmt::Table::boxes`].
    pub boxes: Vec<Vec<Props>>,
    /// The table's own position, as [`fmt::Table::place`].
    pub place: fmt::TablePlace,
}

/// Anchor path: `[block]`, `[block, row]`, `[block, row, col]` or
/// `[block, row, col, paragraph]`; empty for the document itself.
pub type Path = Vec<usize>;

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StyleDef {
    /// The file's id; empty for a style the text created, which the
    /// engine writes at export.
    pub id: String,
    pub name: String,
    /// The style's line (§5.2): complete for the default style (less what
    /// it leaves out), else what differs from the default style.
    #[serde(default, skip_serializing_if = "Props::is_empty")]
    pub props: Props,
    /// Whether the revision's text shows the style's line (it uses the
    /// style, or wrote its line).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub shown: bool,
}

impl StyleDef {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> StyleDef {
        StyleDef { id: id.into(), name: name.into(), ..Default::default() }
    }
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
    /// Name of the style of a table the text gives no `{style}` line: the
    /// one a new table gets (the engine chooses it).
    pub default_table: Option<String>,
    /// The text shows formatting (§5.2): a style section, and paragraphs',
    /// runs' and cells' own properties.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub formatting: bool,
    /// Name of the style a list item has when the text names none (docx:
    /// List Paragraph, as Word gives a new item); `None`: the default style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_item: Option<String>,
}

impl StyleSet {
    /// The style of a list item that names none.
    pub fn item_style(&self) -> &str {
        self.default_item.as_deref().unwrap_or(&self.default_paragraph)
    }
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

    pub fn paragraph_def(&self, name: &str) -> Option<&StyleDef> {
        self.paragraph.iter().find(|s| s.name == name)
    }

    /// The style section's values as lines, the default style's first.
    pub fn lines(&self) -> Vec<fmt::StyleLine> {
        let d = self.paragraph_def(&self.default_paragraph);
        d.into_iter()
            .chain(self.paragraph.iter().filter(|s| s.name != self.default_paragraph))
            .map(|s| fmt::StyleLine { name: s.name.clone(), props: s.props.clone() })
            .collect()
    }

    /// The values of style `name` (the default style's when it has no line).
    pub fn values(&self, name: &str) -> Props {
        fmt::styled::StyleTable { default: Some(self.default_paragraph.clone()), lines: self.lines() }
            .values(Some(name))
    }
}

/// What an engine can export; the rest of the format is refused with a reason.
#[derive(Clone, Copy, Debug, Default)]
pub struct Capabilities {
    pub links: bool,
    pub fields: bool,
    pub footnotes: bool,
    pub math: bool,
    /// Direct formatting and the style section (§5.2).
    pub formatting: bool,
    /// A table's own position: `table-align`, `table-indent` (§5.2).
    pub table_place: bool,
}

/// An empty paragraph's content.
pub(crate) static EMPTY: Inline = Inline { units: Vec::new(), spans: Vec::new() };

/// Content of one unmarked atom.
fn single(atom: Atom) -> Inline {
    Inline { units: vec![fmt::Unit::new(atom, fmt::Marks::NONE)], spans: vec![] }
}

/// Where a resolved block is in the text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockSrc {
    /// Byte range of its lines.
    pub start: usize,
    pub end: usize,
    pub kind: SrcKind,
}

/// A block's paragraph map: for a paragraph (a head: its mark only), or per
/// table cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SrcKind {
    Para(fmt::ParaMap),
    Table(Vec<Vec<Option<Vec<fmt::ParaMap>>>>),
}

/// The source of each block [`resolve`] returns, in the same order:
/// footnote definitions are not blocks of the model, and each list item is one.
pub fn block_maps(parsed: &fmt::Parsed) -> Vec<BlockSrc> {
    let mut out = vec![];
    for (b, m) in parsed.doc.blocks.iter().zip(&parsed.map.blocks) {
        if !matches!(b, fmt::Block::FootnoteDef(_)) {
            push_block_map(&mut out, m);
        }
    }
    out
}

/// The sources of one parsed block: one per list item, else one.
pub(crate) fn push_block_map(out: &mut Vec<BlockSrc>, m: &fmt::BlockMap) {
    match &m.kind {
        fmt::BlockMapKind::List(items) => {
            out.extend(items.iter().map(|(s, e, pm)| BlockSrc { start: *s, end: *e, kind: SrcKind::Para(pm.clone()) }))
        }
        fmt::BlockMapKind::Para(pm) => {
            out.push(BlockSrc { start: m.start, end: m.end, kind: SrcKind::Para(pm.clone()) })
        }
        fmt::BlockMapKind::Table(t) => {
            out.push(BlockSrc { start: m.start, end: m.end, kind: SrcKind::Table(t.clone()) })
        }
    }
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
    check_styles(parsed, text, styles, caps, &mut errs);
    // Line of each block, counted incrementally (blocks are in text order).
    let (mut line, mut counted) = (1, 0);
    for (b, m) in parsed.doc.blocks.iter().zip(&parsed.map.blocks) {
        line += text[counted..m.start].matches('\n').count();
        counted = m.start;
        let mut err = |msg: String| errs.push(Diagnostic { line, col: 1, message: msg });
        let para = |style: String, content: Inline| Block::Para(Para::new(style, content, None));
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
                check_props(&p.props, &p.content, caps, &mut err);
                out.push(Block::Para(Para { style, content: p.content.clone(), item: None, props: p.props.clone() }));
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
                if !caps.formatting && t.boxes.iter().flatten().any(|b| !b.is_empty()) {
                    err(NO_FORMATTING.into());
                }
                for c in rows.iter_mut().flatten() {
                    if let Cell::Text(ps) = c {
                        for p in ps {
                            check_inline(&p.content, caps, is_block_keep, &mut err);
                            check_props(&p.props, &p.content, caps, &mut err);
                            if p.style.as_deref() == Some(styles.default_paragraph.as_str()) {
                                p.style = None;
                            }
                        }
                    }
                }
                // The default table style named is no style line (canonical form).
                let style = t.style.clone().filter(|s| Some(s) != styles.default_table.as_ref());
                if !caps.table_place && !t.place.is_empty() {
                    err(NO_TABLE_PLACE.into());
                }
                out.push(Block::Table(Table { style, rows, boxes: t.boxes.clone(), place: t.place.clone() }));
            }
            fmt::Block::List(items) => {
                let mut margin = None;
                for it in items {
                    check_inline(&it.content, caps, is_block_keep, &mut err);
                    check_props(&it.props, &it.content, caps, &mut err);
                    if it.style.is_some() && !caps.formatting {
                        err(NO_FORMATTING.into());
                    }
                    // An item at the margin of the other kind starts a new list,
                    // as in GFM (§5.2); canonical form puts a blank line before it.
                    let first = it.level == 0 && margin.replace(it.ordered) != Some(it.ordered);
                    let item = ListItem { ordered: it.ordered, level: it.level, first };
                    // A list item names its style where the file shows formatting;
                    // elsewhere its style stays in the remainder.
                    let style = match &it.style {
                        Some(s) => s.clone(),
                        None if styles.formatting => styles.item_style().to_string(),
                        None => String::new(),
                    };
                    out.push(Block::Para(Para {
                        style,
                        content: it.content.clone(),
                        item: Some(item),
                        props: it.props.clone(),
                    }));
                }
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

const NO_TABLE_PLACE: &str = "a table's own position (table-align, table-indent) cannot be written to this file format; leave them out of the table line, and the table keeps the position the file gives it.";

const NO_FORMATTING: &str = "formatting ({…}, [text]{…}, style lines) cannot be written to this file format yet; write the text and style names only.";

/// Formatting is written only where the engine writes it.
fn check_props(p: &Props, content: &Inline, caps: Capabilities, err: &mut dyn FnMut(String)) {
    if !caps.formatting && (!p.is_empty() || content.units.iter().any(|u| !u.props.is_empty())) {
        err(NO_FORMATTING.into());
    }
}

/// The style section against the file's styles (§5.2): a line for a style
/// the file has changes it; a line for a name no style has creates one; a
/// name already taken (in another case, or a table style's) is refused,
/// and so is a line for a style the text did not show, unless it repeats
/// that style's values.
fn check_styles(parsed: &fmt::Parsed, text: &str, styles: &StyleSet, caps: Capabilities, errs: &mut Vec<Diagnostic>) {
    let lines = &parsed.doc.styles;
    if lines.is_empty() {
        return;
    }
    let at = |name: &str| {
        let needle = format!("name=\"{}\"", fmt::serialize::attr_value(name));
        text.lines().position(|l| l.trim_start().starts_with("<style") && l.contains(&needle)).map_or(1, |k| k + 1)
    };
    let mut err = |name: &str, message: String| errs.push(Diagnostic { line: at(name), col: 1, message });
    if !caps.formatting {
        err(&lines[0].name, NO_FORMATTING.into());
        return;
    }
    // A line is compared in canonical form: what it says beyond the default.
    let table = fmt::styled::StyleTable { default: Some(styles.default_paragraph.clone()), lines: styles.lines() };
    let same = |d: &StyleDef, l: &fmt::StyleLine| {
        table.canonical(&fmt::StyleLine { name: d.name.clone(), props: d.props.clone() }) == table.canonical(l)
    };
    for l in lines {
        match styles.paragraph_def(&l.name) {
            Some(d) if !d.shown && !same(d, l) => err(&l.name, format!("\"{}\" is already a style of this file (the text uses none of its paragraphs, so its line was not shown): a new style needs a name no style has. To change \"{}\", give a paragraph that style first; its line then shows its values.", l.name, l.name)),
            Some(_) => {}
            None => {
                let lower = l.name.to_lowercase();
                let taken = styles.paragraph.iter().chain(&styles.table).find(|s| s.name.to_lowercase() == lower);
                if let Some(t) = taken {
                    err(&l.name, format!("\"{}\" is taken: the file has the style \"{}\" (style names are compared without case). A new style needs a name no style has.", l.name, t.name));
                } else if l.name.trim().is_empty() || l.name.trim() != l.name {
                    err(&l.name, "a style's name is its words, without spaces at its ends.".into());
                }
            }
        }
    }
}

/// After an edit: the style section of `parsed` (the new revision's text)
/// in the style set, lines the text wrote over the file's values, a new
/// line as a new style; a style is shown when the text uses it (its
/// canonical form has its line).
pub fn restyle(styles: &mut StyleSet, parsed: &fmt::Parsed, blocks: &[Block]) {
    if !styles.formatting {
        return;
    }
    for l in &parsed.doc.styles {
        match styles.paragraph.iter_mut().find(|s| s.name == l.name) {
            Some(s) => s.props = l.props.clone(),
            None => styles.paragraph.push(StyleDef {
                id: String::new(),
                name: l.name.clone(),
                props: l.props.clone(),
                shown: true,
            }),
        }
    }
    mark_shown(styles, blocks);
}

/// The paragraph styles `blocks` use, in order of first use, the default first.
pub fn used_styles(blocks: &[Block], styles: &StyleSet) -> Vec<String> {
    let mut out = vec![styles.default_paragraph.clone()];
    let mut add = |s: &str| {
        if !s.is_empty() && !out.iter().any(|x| x == s) {
            out.push(s.to_string());
        }
    };
    for b in blocks {
        match b {
            Block::Para(p) => add(&p.style),
            Block::Table(t) => {
                for c in t.rows.iter().flatten() {
                    if let Cell::Text(ps) = c {
                        for p in ps {
                            add(p.style.as_deref().unwrap_or(&styles.default_paragraph));
                        }
                    }
                }
            }
            Block::Keep(_) | Block::Head(_) => {}
        }
    }
    out
}

/// Marks the styles `blocks` use as shown (an import's).
pub fn mark_shown(styles: &mut StyleSet, blocks: &[Block]) {
    let used = used_styles(blocks, styles);
    for s in &mut styles.paragraph {
        s.shown = used.contains(&s.name);
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
    // What differs from a paragraph's style, in canonical form (an edit that
    // restyles a paragraph may leave it values its new style has).
    let table = fmt::styled::StyleTable { default: Some(styles.default_paragraph.clone()), lines: styles.lines() };
    let mut values: std::collections::HashMap<String, Props> = Default::default();
    let mut canon = |style: &str, props: &Props, content: &Inline| -> (Props, Inline) {
        if !styles.formatting {
            return (props.clone(), content.clone());
        }
        let style = if style.is_empty() { styles.default_paragraph.as_str() } else { style };
        let v = values.entry(style.to_string()).or_insert_with(|| table.values(Some(style)));
        let text = v.only(&fmt::Key::TEXT);
        let mut c = content.clone();
        for u in &mut c.units {
            if !u.props.is_empty() {
                u.props = u.props.diff(&text);
            }
        }
        (props.diff(v), c)
    };
    let blocks: Vec<Block> = blocks
        .iter()
        .map(|b| match b {
            Block::Para(p) => {
                let (props, content) = canon(&p.style, &p.props, &p.content);
                Block::Para(Para { props, content, ..p.clone() })
            }
            Block::Table(t) => {
                let mut t = t.clone();
                for c in t.rows.iter_mut().flatten() {
                    if let Cell::Text(ps) = c {
                        for p in ps {
                            let (props, content) =
                                canon(p.style.as_deref().unwrap_or(&styles.default_paragraph), &p.props, &p.content);
                            (p.props, p.content) = (props, content);
                        }
                    }
                }
                Block::Table(t)
            }
            other => other.clone(),
        })
        .collect();
    let blocks = &blocks[..];
    for b in blocks {
        if let Block::Para(Para { content, item: Some(it), style, props }) = b {
            let named = (!style.is_empty() && style != styles.item_style()).then(|| style.clone());
            let fi = fmt::Item {
                ordered: it.ordered,
                level: it.level,
                content: content.clone(),
                style: named.filter(|_| styles.formatting),
                props: props.clone(),
            };
            match out.last_mut() {
                Some(fmt::Block::List(items)) if !it.first => items.push(fi),
                _ => out.push(fmt::Block::List(vec![fi])),
            }
            continue;
        }
        out.push(match b {
            // A Document has no heads.
            Block::Head(_) => continue,
            Block::Keep(id) => fmt::Block::Keep(keep(id)),
            Block::Table(t) => fmt::Block::Table(fmt::Table {
                style: t.style.clone(),
                rows: t.rows.clone(),
                boxes: t.boxes.clone(),
                place: t.place.clone(),
            }),
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
                fmt::Block::Para(fmt::Para { style, content: p.content.clone(), props: p.props.clone() })
            }
        });
    }
    // The style section: the default style, then every style the text
    // uses, in order of first use (a style no paragraph uses has no line;
    // the file keeps it).
    let lines = if styles.formatting {
        used_styles(blocks, styles)
            .iter()
            .filter_map(|n| styles.paragraph_def(n))
            .map(|s| fmt::StyleLine { name: s.name.clone(), props: s.props.clone() })
            .collect()
    } else {
        vec![]
    };
    let mut d = fmt::Document { front, styles: lines, blocks: out };
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
            Block::Keep(_) | Block::Head(_) => out.push((vec![j], None)),
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
