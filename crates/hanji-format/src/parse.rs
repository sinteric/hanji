//! Parser: text → [`Document`] plus a [`SourceMap`], with validator errors
//! written for the model (line, column, expected form, allowed names).

use crate::ast::*;
use crate::diag::{quoted, Diagnostic};
use crate::names::Names;
use crate::SCHEMA_VERSION;

/// Byte positions of every block and unit in the source text; used to
/// re-anchor the remainder from an exact edit span.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceMap {
    pub blocks: Vec<BlockMap>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockMap {
    /// Byte range of the block's lines, line terminator included.
    pub start: usize,
    pub end: usize,
    pub kind: BlockMapKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockMapKind {
    Para(ParaMap),
    /// Per row and column, one map per cell paragraph; `None` for a `||` cell.
    Table(Vec<Vec<Option<Vec<ParaMap>>>>),
    /// Per list item: its line's byte range and its map.
    List(Vec<(usize, usize, ParaMap)>),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParaMap {
    /// Source offset of each unit (for a `Block::Keep` or `PageBreak`: its tag).
    pub units: Vec<usize>,
    /// Offset of the paragraph mark: the line end, the `<p/>` that starts the
    /// next paragraph of a cell, or the pipe closing the cell.
    pub mark: usize,
}

#[derive(Clone, Debug)]
pub struct Parsed {
    pub doc: Document,
    pub map: SourceMap,
}

/// Parse with structural checks only.
pub fn parse(text: &str) -> Result<Document, Vec<Diagnostic>> {
    parse_with(text, &Names::default()).map(|p| p.doc)
}

/// Parse and check names (styles, fields, placeholders) against `names`.
pub fn parse_with(text: &str, names: &Names) -> Result<Parsed, Vec<Diagnostic>> {
    let mut p = Parser {
        names,
        errors: vec![],
        lines: split_lines(text),
        keeps: vec![],
        refs: vec![],
        defs: vec![],
        in_cell: false,
    };
    let (front, first) = p.front_matter();
    let mut blocks = vec![];
    let mut map = SourceMap::default();
    let mut i = first;
    while i < p.lines.len() {
        let before = p.errors.len();
        match p.block(i) {
            Some((b, bm, next)) => {
                if p.errors.len() == before {
                    blocks.push(b);
                    map.blocks.push(bm);
                }
                i = next;
            }
            None => i += 1,
        }
    }
    p.check_names_after();
    if !p.errors.is_empty() {
        return Err(p.errors);
    }
    let mut doc = Document { front: front.unwrap(), blocks };
    doc.normalize();
    Ok(Parsed { doc, map })
}

struct Line<'a> {
    text: &'a str,
    /// Byte offset of the line start.
    at: usize,
    /// Byte offset of the line terminator (or end of text).
    end: usize,
    /// Byte offset just past the terminator.
    next: usize,
}

impl Line<'_> {
    /// Bytes of leading whitespace.
    fn indent(&self) -> usize {
        self.text.len() - self.text.trim_start().len()
    }
}

fn split_lines(text: &str) -> Vec<Line<'_>> {
    let mut out = vec![];
    let mut at = 0;
    while at < text.len() {
        let nl = text[at..].find('\n').map(|k| at + k);
        let (end, next) = match nl {
            Some(k) => (k, k + 1),
            None => (text.len(), text.len()),
        };
        let body = text[at..end].strip_suffix('\r').unwrap_or(&text[at..end]);
        out.push(Line { text: body, at, end: at + body.len(), next });
        at = next;
    }
    out
}

/// One source character: absolute byte offset, 1-based column, char.
type Src = (usize, usize, char);

fn chars_of(line: &Line<'_>, from_byte: usize) -> Vec<Src> {
    let mut col = line.text[..from_byte].chars().count();
    line.text[from_byte..]
        .char_indices()
        .map(|(k, c)| {
            col += 1;
            (line.at + from_byte + k, col, c)
        })
        .collect()
}

struct Parser<'a> {
    names: &'a Names,
    errors: Vec<Diagnostic>,
    lines: Vec<Line<'a>>,
    /// (line, col, keep) of every placeholder written.
    keeps: Vec<(usize, usize, Keep)>,
    refs: Vec<(usize, usize, String)>,
    defs: Vec<(usize, usize, String)>,
    /// Parsing a table cell: `<p/>` starts another paragraph.
    in_cell: bool,
}

const FM_FORM: &str = "a line ---, the lines type: document, format: …, template: …, schema: 1, and a closing line ---";
const P_FORM: &str =
    "<p/> and <p style=\"Name\"/> are single tags that start a paragraph: no </p>, and no attribute but style.";
const INLINE_TAGS: &str =
    "<u>…</u>, <br/>, <math>…</math>, <field name=\"…\">…</field>, <keep id=\"…\" kind=\"…\" summary=\"…\"/>, and <p/> in a table cell";

impl<'a> Parser<'a> {
    fn err(&mut self, line: usize, col: usize, msg: impl Into<String>) {
        self.errors.push(Diagnostic { line: line + 1, col, message: msg.into() });
    }

    // ------------------------------------------------------------ front matter

    fn front_matter(&mut self) -> (Option<FrontMatter>, usize) {
        if self.lines.first().map(|l| l.text.trim()) != Some("---") {
            self.err(0, 1, format!("the file must begin with its front matter: {FM_FORM}."));
            return (None, 0);
        }
        let (mut ty, mut format, mut template, mut schema) = (None, None, None, None);
        for i in 1..self.lines.len() {
            let s = self.lines[i].text.trim();
            if s == "---" {
                for (key, v) in [("type", &ty), ("format", &format), ("schema", &schema)] {
                    if v.is_none() {
                        self.err(i, 1, format!("the front matter has no \"{key}:\" line. It is {FM_FORM}."));
                    }
                }
                let schema_n = match schema.as_deref().map(str::parse::<u32>) {
                    Some(Ok(n)) if n == SCHEMA_VERSION => n,
                    Some(_) => {
                        self.err(
                            i,
                            1,
                            format!(
                                "schema: {} is not supported; this toolchain reads schema: {SCHEMA_VERSION}.",
                                schema.clone().unwrap()
                            ),
                        );
                        SCHEMA_VERSION
                    }
                    None => SCHEMA_VERSION,
                };
                if let Some(t) = &ty {
                    if t != "document" {
                        self.err(1, 1, format!("type: {t} is not a Document; this parser reads type: document."));
                    }
                }
                if let (Some(f), Some(allowed)) = (&format, &self.names.formats) {
                    if !allowed.contains(f) {
                        self.err(
                            1,
                            1,
                            format!("format: {f} is not a home format of this file. Allowed: {}.", quoted(allowed)),
                        );
                    }
                }
                let fm = FrontMatter {
                    doc_type: ty.unwrap_or_default(),
                    format: format.unwrap_or_default(),
                    template,
                    schema: schema_n,
                };
                return (Some(fm), i + 1);
            }
            let Some((k, v)) = s.split_once(':') else {
                if !s.is_empty() {
                    self.err(i, 1, format!("front matter lines are \"key: value\". It is {FM_FORM}."));
                }
                continue;
            };
            let v = strip_comment(v.trim()).to_string();
            let slot = match k.trim() {
                "type" => &mut ty,
                "format" => &mut format,
                "template" => &mut template,
                "schema" => &mut schema,
                other => {
                    self.err(
                        i,
                        1,
                        format!("\"{other}\" is not a front matter key. The keys are type, format, template, schema."),
                    );
                    continue;
                }
            };
            if slot.is_some() {
                self.err(i, 1, format!("\"{}\" appears twice in the front matter.", k.trim()));
            }
            *slot = Some(v);
        }
        self.err(0, 1, "the front matter is not closed by a line ---.");
        (None, self.lines.len())
    }

    // ------------------------------------------------------------ blocks

    /// Parse the block starting at line `i`: (block, map, next line). `None`
    /// for a blank line.
    fn block(&mut self, i: usize) -> Option<(Block, BlockMap, usize)> {
        let line = &self.lines[i];
        let text = line.text;
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        let (at, next) = (line.at, line.next);
        if trimmed.starts_with('{') {
            return self.styled_table(i);
        }
        if trimmed.starts_with('|') {
            let (t, cells, j) = self.table(i, None)?;
            let end = self.lines[j - 1].next;
            return Some((Block::Table(t), BlockMap { start: at, end, kind: BlockMapKind::Table(cells) }, j));
        }
        let mark = line.end;
        let para = |b: Block, units: Vec<usize>| {
            Some((b, BlockMap { start: at, end: next, kind: BlockMapKind::Para(ParaMap { units, mark }) }, i + 1))
        };
        if trimmed == "<pagebreak/>" || trimmed == "<pagebreak />" {
            return para(Block::PageBreak, vec![at + line.indent()]);
        }
        if trimmed.starts_with("<p") && trimmed[2..].starts_with(['/', ' ', '>']) || trimmed.starts_with("</p") {
            return self.empty_para(i).and_then(|p| para(Block::Para(p), vec![]));
        }
        if trimmed.starts_with("<table") || trimmed.starts_with("</table") {
            self.err(i, 1, "a table style is a line {style=\"Name\"} directly before the header row of the pipe table; there is no <table> tag.");
            return Some((Block::PageBreak, dummy(at), i + 1));
        }
        if text.starts_with("<div") {
            return self.div(i).and_then(|(p, units)| para(Block::Para(p), units));
        }
        if let Some(level) = heading_level(text) {
            let from = (level as usize + 1).min(text.len());
            let src = chars_of(&self.lines[i], from);
            let (content, units) = self.inline(i, &src, false)?.0;
            return para(Block::Para(Para { style: ParaStyle::Heading(level), content }), units);
        }
        if text.starts_with("[^") {
            if let Some(close) = text.find("]:") {
                let label = &text[2..close];
                if valid_label(label) {
                    let mut from = close + 2;
                    if text[from..].starts_with(' ') {
                        from += 1;
                    }
                    self.defs.push((i, 1, label.to_string()));
                    let src = chars_of(&self.lines[i], from);
                    let (content, units) = self.inline(i, &src, false)?.0;
                    return para(Block::FootnoteDef(FootnoteDef { label: label.into(), content }), units);
                }
            }
        }
        if list_marker(text).is_some() {
            return self.list(i);
        }
        if let Some(msg) = unsupported_block(text) {
            self.err(i, 1, msg);
            return Some((Block::PageBreak, dummy(at), i + 1));
        }
        let src = chars_of(&self.lines[i], 0);
        let (content, units) = self.inline(i, &src, false)?.0;
        para(Block::Para(Para { style: ParaStyle::Plain, content }), units)
    }

    /// Consecutive list item lines from line `i`. An item nests under the one
    /// before by indenting to its content column (2 spaces under `- `, 3 under
    /// `1. `).
    fn list(&mut self, i: usize) -> Option<(Block, BlockMap, usize)> {
        let (mut items, mut maps) = (vec![], vec![]);
        // Content column widths of the open ancestors, by level.
        let mut widths: Vec<usize> = vec![];
        let mut j = i;
        let before = self.errors.len();
        while let Some(m) = self.lines.get(j).and_then(|l| list_marker(l.text)) {
            let line = &self.lines[j];
            if line.text[..m.indent].contains('\t') {
                self.err(j, 1, "list items are indented with spaces, not tabs: 2 spaces under - and 3 under 1.");
                j += 1;
                continue;
            }
            let columns: Vec<usize> = (0..=widths.len()).map(|k| widths[..k].iter().sum()).collect();
            let Some(level) = columns.iter().position(|&c| c == m.indent) else {
                let cols = columns.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(", ");
                let msg = if widths.is_empty() {
                    format!("a list starts at the left margin; this item is indented {} spaces.", m.indent)
                } else {
                    format!("this item is indented {} spaces. A nested item is indented to its parent's text: 2 spaces under - and 3 under 1. Here an item can be indented {cols} spaces.", m.indent)
                };
                self.err(j, 1, msg);
                j += 1;
                continue;
            };
            widths.truncate(level);
            widths.push(m.width);
            let (line_start, line_next, mark) = (line.at, line.next, line.end);
            let src = chars_of(&self.lines[j], m.content);
            if let Some(((content, units), _)) = self.inline(j, &src, false) {
                items.push(Item { ordered: m.ordered, level, content });
                maps.push((line_start, line_next, ParaMap { units, mark }));
            }
            j += 1;
        }
        let (start, end) = (self.lines[i].at, self.lines[j - 1].next);
        if self.errors.len() > before {
            return Some((Block::PageBreak, dummy(start), j));
        }
        Some((Block::List(items), BlockMap { start, end, kind: BlockMapKind::List(maps) }, j))
    }

    /// A line holding only `<p/>` or `<p style="Name"/>`: an empty paragraph.
    fn empty_para(&mut self, i: usize) -> Option<Para> {
        let line = &self.lines[i];
        let lead = line.indent();
        let src = chars_of(line, lead);
        let tag = parse_tag(&src, 0).filter(|t| src[t.1..].iter().all(|c| c.2.is_whitespace()));
        let tag = match tag {
            Some((t, _)) if t.name == "p" && t.self_closing && !t.closing => t,
            Some((t, _)) if t.name == "p" => {
                self.err(i, lead + 1, P_FORM);
                return None;
            }
            _ => {
                self.err(
                    i,
                    lead + 1,
                    format!("an empty paragraph is a line holding only <p/> or <p style=\"Name\"/>. {P_FORM}"),
                );
                return None;
            }
        };
        if tag.attrs.is_empty() {
            return Some(Para { style: ParaStyle::Plain, content: Inline::default() });
        }
        let style = self.style_attr(i, &tag, "p", "paragraph")?;
        Some(Para { style: ParaStyle::Named(style), content: Inline::default() })
    }

    fn div(&mut self, i: usize) -> Option<(Para, Vec<usize>)> {
        let src = chars_of(&self.lines[i], 0);
        let form = "a styled paragraph is <div style=\"Name\">text</div> on one line";
        let Some((tag, after)) = parse_tag(&src, 0) else {
            self.err(i, 1, format!("expected {form}."));
            return None;
        };
        if tag.name != "div" || tag.closing || tag.self_closing {
            self.err(i, 1, format!("expected {form}."));
            return None;
        }
        let style = self.style_attr(i, &tag, "div", "paragraph")?;
        let ((content, units), end) = self.inline(i, &src[after..], true)?;
        let Some(end) = end else {
            self.err(
                i,
                src.last().map_or(1, |c| c.1),
                format!("<div> is not closed by </div> on the same line: {form}."),
            );
            return None;
        };
        if let Some(extra) = src[after + end..].iter().find(|c| !c.2.is_whitespace()) {
            self.err(i, extra.1, format!("nothing may follow </div> on its line: {form}."));
            return None;
        }
        if content.is_empty() {
            self.err(i, 1, format!("an empty paragraph is <p style=\"{style}\"/>, not an empty <div>."));
            return None;
        }
        Some((Para { style: ParaStyle::Named(style), content }, units))
    }

    /// Checks the single `style="Name"` attribute of a div or table style line.
    fn style_attr(&mut self, i: usize, tag: &Tag, elem: &str, kind: &str) -> Option<String> {
        let form = match elem {
            "div" => "<div style=\"Name\">text</div>",
            "p" => "<p style=\"Name\"/>",
            _ => "{style=\"Name\"}",
        };
        for (k, _, col) in &tag.attrs {
            if k != "style" {
                let msg = format!("{elem} has no attribute \"{k}\". Its only attribute is style=\"Name\", one {kind} style of this file: {form}.");
                self.err(i, *col, msg);
            }
        }
        let Some((_, v, col)) = tag.attrs.iter().find(|a| a.0 == "style").cloned() else {
            self.err(i, tag.col, format!("{elem} needs style=\"Name\" with one {kind} style of this file: {form}."));
            return None;
        };
        let allowed = if kind == "paragraph" { &self.names.paragraph_styles } else { &self.names.table_styles };
        if let Some(allowed) = allowed {
            let other = if kind == "paragraph" { &self.names.table_styles } else { &self.names.paragraph_styles };
            if let Some(msg) = style_problem(&v, kind, allowed, other.as_deref()) {
                self.err(i, col, msg);
            }
        } else if looks_like_css(&v) {
            self.err(i, col, format!("style=\"{v}\" is direct formatting. Formatting is by style name only: style holds exactly one {kind} style name."));
        }
        tag.attrs.iter().all(|a| a.0 == "style").then_some(v)
    }

    fn styled_table(&mut self, i: usize) -> Option<(Block, BlockMap, usize)> {
        let line = &self.lines[i];
        let (at, trimmed) = (line.at, line.text.trim());
        let form = "A table style line is exactly {style=\"Name\"}, directly before the header row of the table.";
        let lead = line.indent();
        if !trimmed.ends_with('}') {
            self.err(i, lead + 1, format!("{form} The braces hold only style=\"Name\"."));
            return Some((Block::PageBreak, dummy(at), i + 1));
        }
        // Reuse the tag attribute parser on `{…}` by treating the braces as a tag.
        let src = chars_of(&Line { text: line.text.trim_end(), ..*line }, lead);
        let inner: Vec<Src> = src[1..src.len() - 1].to_vec();
        let attrs = parse_attrs(&inner);
        let style = match attrs {
            Some(attrs) if !attrs.is_empty() => {
                let tag = Tag { name: "table".into(), closing: false, self_closing: false, attrs, col: lead + 1 };
                self.style_attr(i, &tag, "a table style line", "table")
            }
            _ => {
                self.err(i, lead + 1, format!("the braces hold only style=\"Name\". {form}"));
                None
            }
        };
        let next_is_table = self.lines.get(i + 1).is_some_and(|l| l.text.trim_start().starts_with('|'));
        if !next_is_table {
            self.err(i, lead + 1, "a {style=\"Name\"} line must be directly followed by the header row of its table, with no blank line or other text between.");
            return Some((Block::PageBreak, dummy(at), i + 1));
        }
        let (t, cells, j) = self.table(i + 1, style)?;
        let end = self.lines[j - 1].next;
        Some((Block::Table(t), BlockMap { start: at, end, kind: BlockMapKind::Table(cells) }, j))
    }

    #[allow(clippy::type_complexity)]
    fn table(&mut self, i: usize, style: Option<String>) -> Option<(Table, Vec<Vec<Option<Vec<ParaMap>>>>, usize)> {
        let mut j = i;
        while j < self.lines.len() && self.lines[j].text.trim_start().starts_with('|') {
            j += 1;
        }
        let before = self.errors.len();
        let mut raw_rows: Vec<(usize, Vec<RawCell>)> = vec![];
        for k in i..j {
            if let Some(cells) = self.split_row(k) {
                raw_rows.push((k, cells));
            }
        }
        if self.errors.len() > before {
            return Some((Table { style, rows: vec![] }, vec![], j));
        }
        if raw_rows.len() < 2 {
            self.err(i, 1, "a pipe table needs a header row, then a delimiter row such as |---|---|.");
            return Some((Table { style, rows: vec![] }, vec![], j));
        }
        let width = raw_rows[0].1.len();
        let (dline, delim) = &raw_rows[1];
        for c in delim {
            let t: String = c.chars.iter().map(|s| s.2).collect();
            let t = t.trim();
            if t.contains(':') && t.trim_matches(':').chars().all(|c| c == '-') && !t.is_empty() {
                self.err(*dline, c.col, "column alignment is direct formatting and cannot be written; the delimiter row is one --- per column, such as |---|---|.");
                return Some((Table { style, rows: vec![] }, vec![], j));
            }
            if t.is_empty() || !t.chars().all(|c| c == '-') {
                self.err(
                    *dline,
                    c.col,
                    "the second line of a table is the delimiter row, with one --- per column, such as |---|---|---|.",
                );
                return Some((Table { style, rows: vec![] }, vec![], j));
            }
        }
        if delim.len() != width {
            self.err(
                *dline,
                1,
                format!(
                    "the delimiter row has {} cells but the header row has {width}. Write one --- per column.",
                    delim.len()
                ),
            );
        }
        let body: Vec<&(usize, Vec<RawCell>)> = std::iter::once(&raw_rows[0]).chain(raw_rows[2..].iter()).collect();
        for (ln, cells) in &body {
            if cells.len() != width {
                self.err(*ln, 1, format!("this row has {} cells but the header row has {width}. Every row has one cell per column: a cell merged into the one above is written ^^, and a column covered by the cell to its left is written || (nothing between the pipes; three columns are |||).", cells.len()));
            }
        }
        if self.errors.len() > before {
            return Some((Table { style, rows: vec![] }, vec![], j));
        }
        let mut rows = vec![];
        let mut maps = vec![];
        for (r, (ln, cells)) in body.iter().enumerate() {
            let mut row = vec![];
            let mut mrow = vec![];
            for (c, cell) in cells.iter().enumerate() {
                let text: String = cell.chars.iter().map(|s| s.2).collect();
                if cell.chars.is_empty() {
                    if c == 0 {
                        self.err(*ln, cell.col, "|| at the start of a row: there is no cell to the left to extend. An empty cell is written with a space: |  |.");
                    }
                    row.push(Cell::Left);
                    mrow.push(None);
                } else if text.trim() == "^^" {
                    if r == 0 {
                        self.err(*ln, cell.col, "^^ in the header row: there is no cell above to merge into.");
                    }
                    let at = cell.chars.iter().find(|s| s.2 == '^').map_or(cell.close, |s| s.0);
                    row.push(Cell::Up);
                    mrow.push(Some(vec![ParaMap { units: vec![], mark: at }]));
                } else {
                    let mut src: &[Src] = &cell.chars;
                    if src.first().is_some_and(|s| s.2 == ' ') {
                        src = &src[1..];
                    }
                    if src.last().is_some_and(|s| s.2 == ' ') {
                        src = &src[..src.len() - 1];
                    }
                    self.in_cell = true;
                    let out = self.inline_full(*ln, src, false);
                    self.in_cell = false;
                    let (paras, pmaps) = out.map(|o| o.cell_paras(cell.close)).unwrap_or_default();
                    row.push(Cell::Text(paras));
                    mrow.push(Some(pmaps));
                }
            }
            rows.push(row);
            maps.push(mrow);
        }
        if self.errors.len() == before {
            if let Some((r, c)) = merge_problem(&rows) {
                let text = match &rows[r][c] {
                    Cell::Text(ps) => ps.first().map(|p| p.content.text()).unwrap_or_default(),
                    _ => String::new(),
                };
                self.err(body[r].0, 1, format!("the merged area of the cell \"{text}\" (row {}, column {}) is not a rectangle. Every cell it covers is ^^ (the cell above belongs to it) or || (the cell to the left belongs to it); a 2×2 merge is the text and || in the first row, then ^^ || in the row below.", r + 1, c + 1));
            } else {
                // Canonical markers: inside a merged area only its first column is ^^.
                for r in 1..rows.len() {
                    for c in 1..rows[r].len() {
                        if rows[r][c] == Cell::Up
                            && matches!(rows[r][c - 1], Cell::Up | Cell::Left)
                            && covers_left(&rows, r, c)
                        {
                            rows[r][c] = Cell::Left;
                            maps[r][c] = None;
                        }
                    }
                }
            }
        }
        Some((Table { style, rows }, maps, j))
    }

    fn split_row(&mut self, k: usize) -> Option<Vec<RawCell>> {
        let line = &self.lines[k];
        let lead = line.indent();
        let body = line.text.trim_end();
        let src = chars_of(&Line { text: body, at: line.at, end: line.end, next: line.next }, lead);
        let n = src.len();
        let escaped_end = n >= 2 && src[n - 2].2 == '\\' && src[n - 1].2 == '|';
        if n < 2 || src[n - 1].2 != '|' || escaped_end {
            self.err(k, src.last().map_or(1, |s| s.1 + 1), "a table row starts and ends with |.");
            return None;
        }
        let mut cells = vec![];
        let mut cur: Vec<Src> = vec![];
        let mut col = src[0].1 + 1;
        let mut x = 1;
        while x < n {
            let ch = src[x];
            // `\|` is a pipe inside a cell wherever it appears (GFM); other
            // backslashes are left for the inline parser.
            if ch.2 == '\\' && x + 1 < n && src[x + 1].2 == '|' {
                cur.push((ch.0, ch.1, '|'));
                x += 2;
                continue;
            }
            if ch.2 == '|' {
                cells.push(RawCell { chars: std::mem::take(&mut cur), col, close: ch.0 });
                col = ch.1 + 1;
                x += 1;
                continue;
            }
            cur.push(ch);
            x += 1;
        }
        Some(cells)
    }

    // ------------------------------------------------------------ inline

    /// Parse inline content. With `stop_at_div`, stops at `</div>` and
    /// returns its index in `src`.
    #[allow(clippy::type_complexity)]
    fn inline(&mut self, line: usize, src: &[Src], stop_at_div: bool) -> Option<((Inline, Vec<usize>), Option<usize>)> {
        self.inline_full(line, src, stop_at_div).map(|o| ((o.inline, o.offsets), o.stop))
    }

    fn inline_full(&mut self, line: usize, src: &[Src], stop_at_div: bool) -> Option<InlineOut> {
        let mut st = InlineState::default();
        let n = src.len();
        let mut x = 0;
        let col_of = |x: usize| src.get(x).map_or_else(|| src.last().map_or(1, |s| s.1 + 1), |s| s.1);
        let mut stop = None;
        macro_rules! fail {
            ($x:expr, $($m:tt)*) => {{
                self.err(line, col_of($x), format!($($m)*));
                return None;
            }};
        }
        while x < n {
            if st.link_close == Some(x) {
                let (start, url, resume, _) = st.link.take().unwrap();
                if st.units.len() == start {
                    fail!(x, "link text is empty: a link is [text](url).");
                }
                st.spans.push(Span { start, end: st.units.len(), kind: SpanKind::Link(url) });
                st.link_close = None;
                x = resume;
                continue;
            }
            let (off, _, c) = src[x];
            match c {
                '\\' => {
                    if let Some(&(_, _, nx)) = src.get(x + 1).filter(|s| s.2.is_ascii_punctuation()) {
                        st.push(off, Atom::Char(nx));
                        x += 2;
                    } else {
                        st.push(off, Atom::Char('\\'));
                        x += 1;
                    }
                }
                '*' | '~' => {
                    let mut y = x;
                    while y < n && src[y].2 == c {
                        y += 1;
                    }
                    let run = y - x;
                    let prev = x.checked_sub(1).map(|k| src[k].2);
                    let next = src.get(y).map(|s| s.2);
                    let can_open = next.is_some_and(|c| !c.is_whitespace());
                    let can_close = prev.is_some_and(|c| !c.is_whitespace());
                    let marks: &[Marks] = match (c, run) {
                        ('~', 1) => {
                            st.push(off, Atom::Char('~'));
                            x += 1;
                            continue;
                        }
                        ('~', 2) => &[Marks::STRIKE],
                        ('*', 1) => &[Marks::ITALIC],
                        ('*', 2) => &[Marks::BOLD],
                        ('*', 3) => &[Marks::BOLD, Marks::ITALIC],
                        ('~', _) => fail!(x, "strikethrough is ~~text~~; write \\~ for a literal ~."),
                        _ => fail!(x, "{run} * in a row: bold is **text**, italic *text*, both ***text***; write \\* for a literal *."),
                    };
                    if !can_open && !can_close {
                        if let Some(&m) = marks.iter().find(|&&m| st.marks.has(m)) {
                            let d = delim(m);
                            fail!(x, "{d} closes {m} only directly after text, not after a space: write {d}text{d}, with the space outside.");
                        }
                        for s in &src[x..y] {
                            st.push(s.0, Atom::Char(c));
                        }
                        x = y;
                        continue;
                    }
                    for &m in marks {
                        let d = delim(m);
                        if st.marks.has(m) {
                            if !can_close {
                                fail!(x, "{d} closes {m} only directly after text, not after a space: write {d}text{d}, with the space outside.");
                            }
                            st.marks = st.marks.with(m, false);
                        } else {
                            if !can_open && can_close {
                                fail!(x, "{d} closes {m} that was never opened. A {m} text is {d}text{d}; the opening {d} is directly followed by text, not a space.");
                            }
                            st.marks = st.marks.with(m, true);
                            st.opened[mark_index(m)] = col_of(x);
                        }
                    }
                    x = y;
                }
                '$' => {
                    let next = src.get(x + 1).map(|s| s.2);
                    if next == Some('$') {
                        fail!(x, "display math $$…$$ is not supported; inline math is $…$. Write \\$ for a literal $.");
                    }
                    let close = next.filter(|c| !c.is_whitespace()).and_then(|_| math_close(src, x + 1));
                    match close {
                        Some(y) => {
                            let body: String = src[x + 1..y].iter().map(|s| s.2).collect();
                            st.push(off, Atom::Math(body));
                            x = y + 1;
                        }
                        None => {
                            st.push(off, Atom::Char('$'));
                            x += 1;
                        }
                    }
                }
                '[' => {
                    if src.get(x + 1).map(|s| s.2) == Some('^') {
                        let label: String = src[x + 2..].iter().map(|s| s.2).take_while(|&c| c != ']').collect();
                        let end = x + 2 + label.chars().count();
                        if end >= n || !valid_label(&label) {
                            fail!(x, "a footnote reference is [^label], with a label of letters, digits, - or _; write \\[ for a literal [.");
                        }
                        self.refs.push((line, col_of(x), label.clone()));
                        st.push(off, Atom::NoteRef(label));
                        x = end + 1;
                        continue;
                    }
                    match link_at(src, x) {
                        Some((close, url, resume)) if st.link.is_none() && st.field.is_none() => {
                            st.link = Some((st.units.len(), url, resume, col_of(x)));
                            st.link_close = Some(close);
                            x += 1;
                        }
                        Some(_) => fail!(x, "a link cannot be inside another link or a field."),
                        None => {
                            st.push(off, Atom::Char('['));
                            x += 1;
                        }
                    }
                }
                '<' if src.get(x + 1).is_some_and(|s| s.2.is_ascii_alphabetic() || matches!(s.2, '/' | '!' | '?')) => {
                    let Some((tag, after)) = parse_tag(src, x) else {
                        fail!(x, "a tag is not closed by >. Inline tags are {INLINE_TAGS}; write \\< for a literal <.");
                    };
                    match (tag.name.as_str(), tag.closing) {
                        ("math", false) if !tag.self_closing => {
                            if !tag.attrs.is_empty() {
                                fail!(x, "<math> takes no attributes: <math>…</math>.");
                            }
                            let Some((body_end, resume)) = math_tag_end(src, after) else {
                                fail!(x, "<math> is not closed by </math> on the same line.");
                            };
                            if body_end == after {
                                fail!(x, "<math></math> is empty; math holds its formula: <math>x^2</math> or $x^2$.");
                            }
                            let body: String = src[after..body_end].iter().map(|s| s.2).collect();
                            st.push(off, Atom::Math(body));
                            x = resume;
                            continue;
                        }
                        ("math", _) => fail!(x, "math is <math>…</math> (or $…$); </math> closes an open <math>."),
                        ("br", false) => {
                            if !tag.attrs.is_empty() {
                                fail!(x, "<br/> takes no attributes.");
                            }
                            st.push(off, Atom::Break);
                        }
                        ("u", false) if !tag.self_closing => {
                            if st.marks.has(Marks::UNDERLINE) {
                                fail!(x, "<u> inside <u>: underline is already open here.");
                            }
                            st.marks = st.marks.with(Marks::UNDERLINE, true);
                            st.opened[3] = col_of(x);
                        }
                        ("u", true) => {
                            if !st.marks.has(Marks::UNDERLINE) {
                                fail!(x, "</u> without an opening <u>.");
                            }
                            st.marks = st.marks.with(Marks::UNDERLINE, false);
                        }
                        ("keep", false) => {
                            if !tag.self_closing {
                                fail!(x, "a placeholder is written <keep id=\"…\" kind=\"…\" summary=\"…\"/>, closed by />.");
                            }
                            let k = self.keep_tag(line, &tag)?;
                            self.keeps.push((line, col_of(x), k.clone()));
                            st.push(off, Atom::Keep(k));
                        }
                        ("field", false) if !tag.self_closing => {
                            if st.field.is_some() || st.link.is_some() {
                                fail!(x, "a field cannot be inside a link or another field.");
                            }
                            let name = self.field_name(line, &tag)?;
                            st.field = Some((st.units.len(), name, col_of(x)));
                        }
                        ("field", true) => {
                            let Some((start, name, _)) = st.field.take() else {
                                fail!(x, "</field> without an opening <field name=\"…\">.");
                            };
                            st.spans.push(Span { start, end: st.units.len(), kind: SpanKind::Field(name) });
                        }
                        ("p", false) if tag.self_closing && self.in_cell => {
                            if st.link.is_some() || st.field.is_some() {
                                fail!(x, "a <p/> cannot be inside a link or a field.");
                            }
                            if let Some(m) = Marks::ALL.into_iter().find(|&m| st.marks.has(m)) {
                                let d = closer(m);
                                fail!(x, "close {m} with {d} before <p/>: each paragraph of a cell carries its own emphasis.");
                            }
                            let style = if tag.attrs.is_empty() { None } else { Some(self.style_attr(line, &tag, "p", "paragraph")?) };
                            st.splits.push((st.units.len(), style, off));
                        }
                        ("p", false) if tag.self_closing => fail!(x, "<p/> inside a line starts a paragraph only in a table cell; elsewhere a paragraph is its own line, and an empty paragraph is a line holding only <p/>."),
                        ("p", _) => fail!(x, "{P_FORM}"),
                        ("div", true) if stop_at_div => {
                            stop = Some(after);
                            break;
                        }
                        ("div", _) => fail!(x, "a <div> is a whole line, <div style=\"Name\">text</div>, and cannot contain another <div>."),
                        ("pagebreak", _) => fail!(x, "a page break is its own line: write <pagebreak/> alone on a line."),
                        ("table", _) => fail!(x, "a table style is a line {{style=\"Name\"}} directly before the header row; there is no <table> tag."),
                        (name, _) => fail!(x, "<{name}> is not a tag of this format. Inline tags are {INLINE_TAGS}; write \\< for a literal <."),
                    }
                    x = after;
                }
                _ => {
                    st.push(off, Atom::Char(c));
                    x += 1;
                }
            }
        }
        if let Some((_, _, _, col)) = st.link {
            self.err(line, col, "this link is not closed: a link is [text](url).");
            return None;
        }
        if let Some((_, name, col)) = st.field {
            self.err(line, col, format!("<field name=\"{name}\"> is not closed by </field> on the same line."));
            return None;
        }
        for m in Marks::ALL {
            if st.marks.has(m) {
                let close = closer(m);
                self.err(line, st.opened[mark_index(m)], format!("{m} opened here is never closed. Close it with {close} before the end of the line; a closing {close} follows text directly, not a space."));
                return None;
            }
        }
        st.spans.sort_by_key(|s| s.start);
        Some(InlineOut {
            inline: Inline { units: st.units, spans: st.spans },
            offsets: st.offsets,
            stop,
            splits: st.splits,
        })
    }

    fn keep_tag(&mut self, line: usize, tag: &Tag) -> Option<Keep> {
        let form = "<keep id=\"…\" kind=\"…\" summary=\"…\"/>";
        let (mut id, mut kind, mut summary) = (None, None, None);
        for (k, v, col) in &tag.attrs {
            match k.as_str() {
                "id" => id = Some(v.clone()),
                "kind" => kind = Some(v.clone()),
                "summary" => summary = Some(v.clone()),
                other => {
                    self.err(
                        line,
                        *col,
                        format!(
                            "<keep> has no attribute \"{other}\"; it is {form}, kept exactly as it is in the file."
                        ),
                    );
                    return None;
                }
            }
        }
        match (id, kind, summary) {
            (Some(id), Some(kind), Some(summary)) => Some(Keep { id, kind, summary }),
            _ => {
                self.err(
                    line,
                    tag.col,
                    format!("a placeholder keeps its id, kind and summary as they are in the file: {form}."),
                );
                None
            }
        }
    }

    fn field_name(&mut self, line: usize, tag: &Tag) -> Option<String> {
        let mut name = None;
        for (k, v, col) in &tag.attrs {
            if k == "name" {
                name = Some((v.clone(), *col));
            } else {
                self.err(
                    line,
                    *col,
                    format!("<field> has no attribute \"{k}\"; it is <field name=\"…\">value</field>."),
                );
                return None;
            }
        }
        let Some((name, col)) = name else {
            self.err(line, tag.col, "a field is <field name=\"…\">value</field>.");
            return None;
        };
        if let Some(allowed) = &self.names.fields {
            if !allowed.contains(&name) {
                self.err(line, col, format!("\"{name}\" is not a field of this file. Fields: {}.", quoted(allowed)));
                return None;
            }
        }
        Some(name)
    }

    // ------------------------------------------------------------ cross-checks

    fn check_names_after(&mut self) {
        let mut errs = vec![];
        if let Some(known) = &self.names.keeps {
            let mut seen: Vec<&str> = vec![];
            for (line, col, k) in &self.keeps {
                match known.iter().find(|x| x.id == k.id) {
                    None => errs.push((*line, *col, format!("placeholder id=\"{}\" is not in this file. Placeholders come from the file: keep, move or delete them, but never create one.", k.id))),
                    Some(orig) if orig != k => errs.push((*line, *col, format!(
                        "placeholder id=\"{}\" was altered; keep it exactly as <keep id=\"{}\" kind=\"{}\" summary=\"{}\"/>.",
                        k.id, orig.id, orig.kind, orig.summary
                    ))),
                    Some(_) => {}
                }
                if seen.contains(&k.id.as_str()) {
                    errs.push((*line, *col, format!("placeholder id=\"{}\" appears twice. A placeholder stands for one object: move it, never copy it.", k.id)));
                }
                seen.push(&k.id);
            }
        }
        for (line, col, label) in &self.refs {
            if !self.defs.iter().any(|d| &d.2 == label) {
                errs.push((
                    *line,
                    *col,
                    format!("footnote [^{label}] has no definition. Add a line [^{label}]: text."),
                ));
            }
        }
        for (k, (line, col, label)) in self.defs.iter().enumerate() {
            if self.defs[..k].iter().any(|d| &d.2 == label) {
                errs.push((*line, *col, format!("footnote [^{label}] is defined twice.")));
            } else if !self.refs.iter().any(|r| &r.2 == label) {
                errs.push((*line, *col, format!("footnote [^{label}] is defined but never referenced; reference it with [^{label}] or delete the definition.")));
            }
        }
        for (l, c, m) in errs {
            self.errors.push(Diagnostic { line: l + 1, col: c, message: m });
        }
        self.errors.sort_by_key(|d| (d.line, d.col));
    }
}

struct RawCell {
    /// Content between the pipes, `\|` already unescaped.
    chars: Vec<Src>,
    col: usize,
    /// Offset of the closing pipe.
    close: usize,
}

#[derive(Default)]
struct InlineState {
    units: Vec<Unit>,
    offsets: Vec<usize>,
    spans: Vec<Span>,
    marks: Marks,
    opened: [usize; 4],
    /// (start unit, url, resume index, col)
    link: Option<(usize, String, usize, usize)>,
    link_close: Option<usize>,
    field: Option<(usize, String, usize)>,
    /// Cell paragraph starts: (unit index, style, source offset).
    splits: Vec<(usize, Option<String>, usize)>,
}

struct InlineOut {
    inline: Inline,
    offsets: Vec<usize>,
    stop: Option<usize>,
    splits: Vec<(usize, Option<String>, usize)>,
}

impl InlineOut {
    /// A cell's paragraphs: the text starts the first, each `<p/>` another;
    /// a tag written first starts the first paragraph itself.
    fn cell_paras(self, close: usize) -> (Vec<CellPara>, Vec<ParaMap>) {
        let (mut paras, mut maps) = (vec![], vec![]);
        let (mut style, mut from) = (None, 0);
        for (k, (at, st, off)) in self.splits.into_iter().enumerate() {
            if k == 0 && at == 0 {
                style = st;
                continue;
            }
            let content = Inline {
                units: self.inline.units[from..at].to_vec(),
                spans: slice_spans(&self.inline.spans, from, at),
            };
            paras.push(CellPara { style: std::mem::replace(&mut style, st), content });
            maps.push(ParaMap { units: self.offsets[from..at].to_vec(), mark: off });
            from = at;
        }
        let n = self.inline.units.len();
        let content =
            Inline { units: self.inline.units[from..].to_vec(), spans: slice_spans(&self.inline.spans, from, n) };
        paras.push(CellPara { style, content });
        maps.push(ParaMap { units: self.offsets[from..].to_vec(), mark: close });
        (paras, maps)
    }
}

fn slice_spans(spans: &[Span], a: usize, b: usize) -> Vec<Span> {
    spans
        .iter()
        .filter(|s| s.start >= a && s.end <= b)
        .map(|s| Span { start: s.start - a, end: s.end - a, kind: s.kind.clone() })
        .collect()
}

impl InlineState {
    fn push(&mut self, off: usize, atom: Atom) {
        self.units.push(Unit { atom, marks: self.marks });
        self.offsets.push(off);
    }
}

fn dummy(at: usize) -> BlockMap {
    BlockMap { start: at, end: at, kind: BlockMapKind::Para(ParaMap::default()) }
}

/// The text that closes mark `m`.
pub(crate) fn closer(m: Marks) -> &'static str {
    if m == Marks::UNDERLINE {
        "</u>"
    } else {
        delim(m)
    }
}

pub(crate) fn delim(m: Marks) -> &'static str {
    match m {
        Marks::BOLD => "**",
        Marks::ITALIC => "*",
        Marks::STRIKE => "~~",
        _ => "<u>",
    }
}

fn mark_index(m: Marks) -> usize {
    Marks::ALL.iter().position(|&x| x == m).unwrap()
}

fn strip_comment(v: &str) -> &str {
    match v.find(" #").or_else(|| v.find("\t#")) {
        Some(k) => v[..k].trim_end(),
        None => v,
    }
}

struct ListMarker {
    /// Leading spaces.
    indent: usize,
    ordered: bool,
    /// Marker plus its space: the content column nested items indent to.
    width: usize,
    /// Byte offset of the item's text.
    content: usize,
}

/// `- text` or `1. text` (any digits), after leading spaces: a list item line.
fn list_marker(text: &str) -> Option<ListMarker> {
    let rest = text.trim_start_matches([' ', '\t']);
    let indent = text.len() - rest.len();
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    let (ordered, len) = match rest.as_bytes() {
        [b'-', ..] => (false, 1),
        _ if (1..=9).contains(&digits) && rest[digits..].starts_with('.') => (true, digits + 1),
        _ => return None,
    };
    match rest.as_bytes().get(len) {
        None => Some(ListMarker { indent, ordered, width: len + 1, content: indent + len }),
        Some(b' ') => Some(ListMarker { indent, ordered, width: len + 1, content: indent + len + 1 }),
        _ => None,
    }
}

fn heading_level(text: &str) -> Option<u8> {
    let n = text.bytes().take_while(|&b| b == b'#').count();
    let rest = &text[n..];
    ((1..=6).contains(&n) && (rest.is_empty() || rest.starts_with(' '))).then_some(n as u8)
}

pub(crate) fn valid_label(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

/// Markdown block syntax this format does not have; an error names the
/// form to write instead.
fn unsupported_block(text: &str) -> Option<&'static str> {
    let t = text.trim_end();
    let first = t.chars().next()?;
    let second = t.chars().nth(1);
    if (first == '-' || first == '*' || first == '_') && t.len() >= 3 && t.chars().all(|c| c == first || c == ' ') {
        return Some("a line of --- is not allowed in a document body; a page break is <pagebreak/>. Write \\- for a literal dash.");
    }
    if matches!(first, '+' | '*') && second == Some(' ') {
        return Some("a bullet item is written - text (a dash and a space); write \\* or \\+ for a literal character.");
    }
    let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 && t.chars().nth(digits) == Some(')') && matches!(t.chars().nth(digits + 1), Some(' ') | None) {
        return Some("a numbered item is written 1. text (canonically always 1.); write 1\\) for a literal number.");
    }
    if first == '>' {
        return Some("block quotes are not supported; use a quote style, <div style=\"Name\">text</div>, or write \\> for a literal >.");
    }
    None
}

/// Closing `$` of math opened at `from - 1`: preceded by a non-space, not
/// followed by a digit. `\$` inside is content.
fn math_close(src: &[Src], from: usize) -> Option<usize> {
    let mut y = from;
    while y < src.len() {
        let c = src[y].2;
        if p_tag_at(src, y) {
            return None;
        }
        if c == '\\' {
            y += 2;
            continue;
        }
        if c == '$' {
            let prev_ok = !src[y - 1].2.is_whitespace();
            let next_digit = src.get(y + 1).is_some_and(|s| s.2.is_ascii_digit());
            if prev_ok && !next_digit && y > from {
                return Some(y);
            }
        }
        y += 1;
    }
    None
}

/// The `</math>` closing a `<math>` whose body starts at `from`: (body end,
/// index after `</math>`). The body is raw text.
fn math_tag_end(src: &[Src], from: usize) -> Option<(usize, usize)> {
    let close: Vec<char> = "</math>".chars().collect();
    (from..src.len())
        .find(|&y| {
            src.len() - y >= close.len() && src[y..y + close.len()].iter().map(|s| s.2).eq(close.iter().copied())
        })
        .map(|y| (y, y + close.len()))
}

/// A cell paragraph tag starts here: math and link text end at it.
fn p_tag_at(src: &[Src], y: usize) -> bool {
    src[y].2 == '<'
        && src.get(y + 1).is_some_and(|s| s.2 == 'p')
        && src.get(y + 2).is_some_and(|s| matches!(s.2, '/' | ' ' | '>'))
}

/// A link starting at `[`: (index of `]`, url, index after `)`). The
/// link text runs to the first unescaped `]`; a `[` before it makes this
/// `[` literal (the later one may open the link).
fn link_at(src: &[Src], x: usize) -> Option<(usize, String, usize)> {
    let mut y = x + 1;
    while y < src.len() {
        // Skip what the inline parser reads as one token.
        match src[y].2 {
            '\\' => y += 2,
            '[' if src.get(y + 1).map(|s| s.2) == Some('^') => {
                y = (y..src.len()).find(|&k| src[k].2 == ']').map_or(src.len(), |k| k + 1);
            }
            '[' => return None,
            ']' => break,
            '<' if p_tag_at(src, y) => return None,
            '<' if parse_tag(src, y).is_some_and(|t| t.0.name == "math" && !t.0.closing) => {
                let after = parse_tag(src, y).unwrap().1;
                y = math_tag_end(src, after).map_or(after, |m| m.1);
            }
            '<' => y = parse_tag(src, y).map_or(y + 1, |t| t.1),
            '$' => {
                let opens = src.get(y + 1).is_some_and(|s| !s.2.is_whitespace() && s.2 != '$');
                y = if opens { math_close(src, y + 1).map_or(y + 1, |k| k + 1) } else { y + 1 };
            }
            _ => y += 1,
        }
    }
    if y >= src.len() || src.get(y + 1).map(|s| s.2) != Some('(') {
        return None;
    }
    let mut url = String::new();
    let mut z = y + 2;
    while z < src.len() {
        match src[z].2 {
            ')' => return (!url.is_empty()).then_some((y, url, z + 1)),
            '\\' if src.get(z + 1).is_some_and(|s| s.2.is_ascii_punctuation()) => {
                url.push(src[z + 1].2);
                z += 2;
            }
            c if c.is_whitespace() => return None,
            c => {
                url.push(c);
                z += 1;
            }
        }
    }
    None
}

pub(crate) struct Tag {
    pub name: String,
    pub closing: bool,
    pub self_closing: bool,
    /// (name, decoded value, column)
    pub attrs: Vec<(String, String, usize)>,
    pub col: usize,
}

/// `<name attr="v" …>`, `</name>` or `<name …/>` at `src[x]`: the tag and
/// the index after `>`.
pub(crate) fn parse_tag(src: &[Src], x: usize) -> Option<(Tag, usize)> {
    if src.get(x)?.2 != '<' {
        return None;
    }
    let mut y = x + 1;
    let closing = src.get(y)?.2 == '/';
    if closing {
        y += 1;
    }
    let name_start = y;
    while y < src.len() && (src[y].2.is_ascii_alphanumeric() || src[y].2 == '-') {
        y += 1;
    }
    let name: String = src[name_start..y].iter().map(|s| s.2).collect();
    if name.is_empty() {
        return None;
    }
    // Find the closing `>` outside quotes.
    let mut z = y;
    let mut quote = None;
    while z < src.len() {
        let c = src[z].2;
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == '>' => break,
            None => {}
        }
        z += 1;
    }
    if z >= src.len() {
        return None;
    }
    let mut inner = &src[y..z];
    let self_closing = inner.last().is_some_and(|s| s.2 == '/');
    if self_closing {
        inner = &inner[..inner.len() - 1];
    }
    let attrs = parse_attrs(inner)?;
    Some((Tag { name: name.to_ascii_lowercase(), closing, self_closing, attrs, col: src[x].1 }, z + 1))
}

/// `a="v" b='w'` → decoded attributes; `None` if malformed.
pub(crate) fn parse_attrs(src: &[Src]) -> Option<Vec<(String, String, usize)>> {
    let mut out = vec![];
    let mut y = 0;
    loop {
        while y < src.len() && src[y].2.is_whitespace() {
            y += 1;
        }
        if y >= src.len() {
            return Some(out);
        }
        let start = y;
        while y < src.len() && (src[y].2.is_alphanumeric() || matches!(src[y].2, '-' | '_' | ':')) {
            y += 1;
        }
        if y == start {
            return None;
        }
        let name: String = src[start..y].iter().map(|s| s.2).collect();
        while y < src.len() && src[y].2.is_whitespace() {
            y += 1;
        }
        if src.get(y).map(|s| s.2) != Some('=') {
            return None;
        }
        y += 1;
        while y < src.len() && src[y].2.is_whitespace() {
            y += 1;
        }
        let q = src.get(y)?.2;
        if q != '"' && q != '\'' {
            return None;
        }
        let vstart = y + 1;
        let vend = (vstart..src.len()).find(|&k| src[k].2 == q)?;
        let raw: String = src[vstart..vend].iter().map(|s| s.2).collect();
        out.push((name.to_ascii_lowercase(), decode_attr(&raw), src[start].1));
        y = vend + 1;
    }
}

pub(crate) fn decode_attr(s: &str) -> String {
    s.replace("&quot;", "\"").replace("&apos;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

pub(crate) fn looks_like_css(v: &str) -> bool {
    v.contains(':') || v.contains(';') || v.contains('#') || v.contains('{')
}

/// Why `value` is not an allowed style name, written for the model.
fn style_problem(value: &str, kind: &str, allowed: &[String], other: Option<&[String]>) -> Option<String> {
    if allowed.iter().any(|a| a == value) {
        return None;
    }
    let tail =
        format!(" The value is exactly one style name, spaces included. Allowed {kind} styles: {}.", quoted(allowed));
    let other_kind = if kind == "paragraph" { "table" } else { "paragraph" };
    if other.is_some_and(|o| o.iter().any(|a| a == value)) {
        return Some(format!("style=\"{value}\" is a {other_kind} style; a {kind} takes a {kind} style.{tail}"));
    }
    if let Some(c) = allowed.iter().find(|a| a.to_lowercase() == value.to_lowercase()) {
        return Some(format!(
            "style=\"{value}\" is not a {kind} style of this file; names are case-sensitive: \"{c}\".{tail}"
        ));
    }
    let squash = |s: &str| {
        s.chars().filter(|c| !c.is_whitespace() && !matches!(c, '_' | '-' | '.')).collect::<String>().to_lowercase()
    };
    if let Some(c) = allowed.iter().find(|a| squash(a) == squash(value)) {
        return Some(format!("style=\"{value}\" is not a {kind} style of this file; did you mean \"{c}\"?{tail}"));
    }
    if looks_like_css(value) {
        return Some(format!("style=\"{value}\" is direct formatting. Formatting is by style name only; colours, fonts, sizes and borders cannot be written. Use a {kind} style of this file.{tail}"));
    }
    Some(format!("style=\"{value}\" is not a {kind} style of this file.{tail}"))
}

/// Whether the `^^` at (r, c) belongs to the same merged area as its left
/// neighbour (a 2×2 merge written `^^ ^^`).
fn covers_left(rows: &[Vec<Cell>], r: usize, c: usize) -> bool {
    let origin = |mut r: usize, mut c: usize| loop {
        match rows[r][c] {
            Cell::Left if c > 0 => c -= 1,
            Cell::Up if r > 0 => r -= 1,
            _ => return (r, c),
        }
    };
    origin(r, c) == origin(r, c - 1)
}

/// First cell whose merged area is not a rectangle, as the fluency validator
/// defines it: covered cells point at their origin through ^^ and ||.
pub fn merge_problem(rows: &[Vec<Cell>]) -> Option<(usize, usize)> {
    let h = rows.len();
    let w = rows.first().map_or(0, Vec::len);
    let mut origin = vec![vec![(0, 0); w]; h];
    for r in 0..h {
        for c in 0..w {
            origin[r][c] = match rows[r][c] {
                Cell::Left if c > 0 => origin[r][c - 1],
                Cell::Up if r > 0 => origin[r - 1][c],
                _ => (r, c),
            };
        }
    }
    let mut areas: std::collections::BTreeMap<(usize, usize), Vec<(usize, usize)>> = Default::default();
    for (r, row) in origin.iter().enumerate() {
        for (c, o) in row.iter().enumerate() {
            areas.entry(*o).or_default().push((r, c));
        }
    }
    for (o, cells) in areas {
        let (r0, r1) = (cells.iter().map(|p| p.0).min()?, cells.iter().map(|p| p.0).max()?);
        let (c0, c1) = (cells.iter().map(|p| p.1).min()?, cells.iter().map(|p| p.1).max()?);
        if cells.len() != (r1 - r0 + 1) * (c1 - c0 + 1) || (r0, c0) != o {
            return Some(o);
        }
    }
    None
}
