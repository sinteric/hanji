//! A Document in its other home format (DESIGN.md §3, §5.2): docx ↔ hwpx,
//! with a report of whatever the target could not carry across.
//!
//! What crosses is what the text says. The source revision's text is parsed
//! with the source file's names, rewritten into a text the target accepts,
//! and written into the target's blank package like any whole-file rewrite
//! (design C), so the target engine does the writing and every refusal it
//! has stays its own. The remainder does not cross: its entries are anchored
//! to the source package (rule 1 keeps them there, so the stored document is
//! never touched), and the report names each one that held something.
//!
//! Nothing is dropped without a line in the [`Conversion`]:
//!
//! - **placeholders** (`<keep/>`): each one, with its kind, summary and line;
//! - **properties** the target cannot write (a gradient fill, a table's own
//!   position in hwpx, a table style, a colour it has no form for), each with
//!   the target's reason; a theme colour is not one of them while the source
//!   theme gives its `#RRGGBB`, which is written instead and listed;
//! - **styles** the target has under its own definition (its default style,
//!   its headings, a style of the same name), with the values that differ;
//!   a style it lacks is created, as a model would create one;
//! - **what the remainder held** that the text does not show: paragraph and
//!   cell properties, bookmarks, headers and footers, the package's pictures
//!   and properties, counted by what they are.

use std::collections::{BTreeMap, HashMap};
use std::fmt;

use hanji_core::edit::names;
use hanji_core::{ImportOptions, Kind, Remainder};
use hanji_docx::format::Theme;
use hanji_format::vocab::{Border, Color, Fill};
use hanji_format::{
    self as fm, Atom, Block, BlockMapKind, Cell, CellPara, Inline, Key, Para, ParaMap, ParaStyle, Parsed, Props, Span,
    StyleLine, Unit, Value,
};
use serde::Serialize;

use crate::error::{Code, Error, Result};
use crate::formats::{DocType, Format};
use crate::storage::MemStorage;
use crate::{blank, Workspace};

/// How many places a grouped entry names; its `count` is the whole.
const PLACES: usize = 8;

/// A placeholder the target file does not have: it is not in the converted text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Placeholder {
    pub id: String,
    pub kind: String,
    pub summary: String,
    /// Where it was: a line of the source revision's text.
    pub location: String,
}

/// A property the target could not write, as the text had it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Property {
    /// `key=value`, or what the text had (`table style "Grid Table 4"`).
    pub property: String,
    /// The target's reason, with what to write instead.
    pub reason: String,
    pub count: usize,
    /// The first places it was, as lines of the source revision's text.
    pub at: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StyleFate {
    /// The target has the style (by name, or as its default or heading) and
    /// keeps its own definition.
    Replaced,
    /// The target lacked the style: it was written as a new one.
    Created,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StyleNote {
    /// The source's style.
    pub name: String,
    pub fate: StyleFate,
    /// `replaced`: the target's style the paragraphs have now.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// `replaced`: the values the source style had that the target's has
    /// differently (`size 16pt → 15pt`); empty when they are the same.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub detail: String,
}

/// A theme colour written as the `#RRGGBB` the source theme gives it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Resolved {
    pub from: String,
    pub to: String,
    pub count: usize,
}

/// Something the remainder held that the text does not show, by what it is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NotCarried {
    pub what: String,
    pub count: usize,
    /// The package parts, when it is whole parts.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<String>,
}

/// What an export in the other format did not carry across (§2 rule 1: it
/// never drops silently).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Conversion {
    pub from: Format,
    pub to: Format,
    /// How many placeholders were dropped, by kind.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub placeholder_kinds: BTreeMap<String, usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub placeholders: Vec<Placeholder>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<Property>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub styles: Vec<StyleNote>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub resolved_colours: Vec<Resolved>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub not_carried: Vec<NotCarried>,
}

impl Conversion {
    fn new(from: Format, to: Format) -> Conversion {
        Conversion {
            from,
            to,
            placeholder_kinds: BTreeMap::new(),
            placeholders: vec![],
            properties: vec![],
            styles: vec![],
            resolved_colours: vec![],
            not_carried: vec![],
        }
    }

    /// Nothing was dropped or replaced (a theme colour written as RGB, and a
    /// style created, keep what they were).
    pub fn is_lossless(&self) -> bool {
        self.placeholders.is_empty()
            && self.properties.is_empty()
            && self.not_carried.is_empty()
            && self.styles.iter().all(|s| s.fate == StyleFate::Created)
    }
}

impl fmt::Display for Conversion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (from, to) = (self.from.name(), self.to.name());
        if self.is_lossless() {
            write!(
                f,
                "converted {from} → {to}: the text, its formatting and its styles crossed; nothing was dropped."
            )?;
        } else {
            write!(f, "converted {from} → {to}. Not carried across:")?;
        }
        if !self.placeholders.is_empty() {
            let kinds: Vec<String> = self.placeholder_kinds.iter().map(|(k, n)| format!("{k} ×{n}")).collect();
            write!(f, "\n- placeholders dropped, {} ({}):", self.placeholders.len(), kinds.join(", "))?;
            for kind in self.placeholder_kinds.keys() {
                let of_kind: Vec<&Placeholder> = self.placeholders.iter().filter(|p| &p.kind == kind).collect();
                for p in of_kind.iter().take(3) {
                    write!(f, "\n    {} {} ({}): {}", p.kind, p.id, p.location, p.summary)?;
                }
                if of_kind.len() > 3 {
                    write!(f, "\n    … {} more {kind}", of_kind.len() - 3)?;
                }
            }
        }
        if !self.properties.is_empty() {
            write!(f, "\n- properties {to} cannot write, dropped:")?;
            for p in &self.properties {
                write!(f, "\n    {} ×{} ({}): {}", p.property, p.count, p.at.join(", "), p.reason)?;
            }
        }
        let replaced: Vec<&StyleNote> =
            self.styles.iter().filter(|s| s.fate == StyleFate::Replaced && !s.detail.is_empty()).collect();
        if !replaced.is_empty() {
            write!(f, "\n- styles {to} has under its own definition (its values differ):")?;
            for s in replaced {
                let target = s.target.as_deref().unwrap_or(&s.name);
                write!(f, "\n    {} → {target}: {}", s.name, s.detail)?;
            }
        }
        let created: Vec<&str> =
            self.styles.iter().filter(|s| s.fate == StyleFate::Created).map(|s| s.name.as_str()).collect();
        if !created.is_empty() {
            write!(f, "\n- styles written as new ones: {}", created.join(", "))?;
        }
        if !self.resolved_colours.is_empty() {
            let n: usize = self.resolved_colours.iter().map(|r| r.count).sum();
            let shown: Vec<String> =
                self.resolved_colours.iter().take(5).map(|r| format!("{} → {}", r.from, r.to)).collect();
            let more = if self.resolved_colours.len() > 5 { ", …" } else { "" };
            write!(f, "\n- theme colours written as #RRGGBB, {n} places ({}{more})", shown.join(", "))?;
        }
        if !self.not_carried.is_empty() {
            write!(f, "\n- what the file held beyond the text, left behind:")?;
            for n in &self.not_carried {
                if n.parts.is_empty() {
                    write!(f, "\n    {} ×{}", n.what, n.count)?;
                } else {
                    write!(f, "\n    {} ({})", n.what, n.parts.join(", "))?;
                }
            }
        }
        Ok(())
    }
}

/// Where a style's name goes in the target.
#[derive(Clone, Debug)]
enum Mapped {
    /// The target's default paragraph style.
    Default,
    /// A style the target has, by this exact name.
    Target(String),
    /// A style the target lacks: written anew, under its name.
    Created,
}

struct Convert<'a> {
    to: Format,
    caps: hanji_core::Capabilities,
    /// Where each line of the source revision's text ends (its `\n`), for the
    /// line of each thing dropped.
    ends: Vec<usize>,
    src: &'a Remainder,
    dst: &'a Remainder,
    parsed: &'a Parsed,
    theme: Option<Theme>,
    /// The text has list items (they take the file's list style where they name none).
    has_items: bool,
    src_default: String,
    dst_default: String,
    /// The target's paragraph styles, by lower-case name.
    dst_names: HashMap<String, String>,
    mapped: HashMap<String, Mapped>,
    /// Source styles the converted text uses and the target lacks, in order of first use.
    created: Vec<String>,
    out: Conversion,
    properties: Vec<(String, Property)>,
    resolved: BTreeMap<(String, String), usize>,
}

impl<'a> Convert<'a> {
    /// `line N` of the source revision's text, of the byte at `offset`.
    fn line(&self, offset: usize) -> String {
        format!("line {}", self.ends.partition_point(|&e| e < offset) + 1)
    }

    // ------------------------------------------------------------ styles

    /// The target's style for the source's paragraph style `name`.
    fn style(&mut self, name: &str) -> Mapped {
        if let Some(m) = self.mapped.get(name) {
            return m.clone();
        }
        let m = if name == self.src_default {
            Mapped::Default
        } else if let Some(t) = self.dst_names.get(&name.to_lowercase()) {
            if *t == self.dst_default {
                Mapped::Default
            } else {
                Mapped::Target(t.clone())
            }
        } else if let Some(h) =
            self.src.styles.heading_level(name).and_then(|k| self.dst.styles.headings[usize::from(k) - 1].clone())
        {
            Mapped::Target(h)
        } else if self.src.styles.default_item.as_deref() == Some(name) {
            // The style a list item has when the text names none: the target's own.
            let t = self.dst.styles.item_style().to_string();
            if t == self.dst_default {
                Mapped::Default
            } else {
                Mapped::Target(t)
            }
        } else {
            Mapped::Created
        };
        self.mapped.insert(name.to_string(), m.clone());
        m
    }

    /// A named paragraph style (a `<div style>`, an item's or a cell paragraph's) in the target.
    fn named(&mut self, name: &str) -> Option<String> {
        match self.style(name) {
            Mapped::Default => None,
            Mapped::Target(t) => Some(t),
            Mapped::Created => {
                if !self.created.iter().any(|c| c == name) {
                    self.created.push(name.to_string());
                }
                Some(name.to_string())
            }
        }
    }

    /// The values the source style `name` has that the target's `target` has
    /// differently, as `size 16pt → 15pt`: every value of the default style,
    /// and of another style the ones its own line sets (the rest is the
    /// default's, noted with it).
    fn differing(&self, name: &str, target: &str) -> String {
        let (a, b) = (self.src.styles.values(name), self.dst.styles.values(target));
        let own: Option<Vec<Key>> = (name != self.src_default)
            .then(|| self.parsed.doc.styles.iter().find(|l| l.name == name).map(|l| l.props.keys().collect()))
            .flatten();
        let mut out = vec![];
        for k in Key::ALL.into_iter().filter(|k| own.as_ref().is_none_or(|o| o.contains(k))) {
            let (x, y) = (a.get(k), b.get(k));
            if x != y {
                let show = |v: Option<&Value>| v.map_or("unset".to_string(), |v| v.to_string());
                out.push(format!("{k} {} → {}", show(x), show(y)));
            }
        }
        out.join(", ")
    }

    /// The notes on the source styles the target has under its own definition.
    fn note_styles(&mut self) {
        let mut notes = vec![];
        let mut seen = vec![];
        let lines = &self.parsed.doc.styles;
        let names: Vec<String> = lines
            .iter()
            .map(|l| l.name.clone())
            .chain(self.src.styles.default_item.clone().filter(|_| self.has_items))
            .collect();
        for name in names {
            if seen.contains(&name) {
                continue;
            }
            seen.push(name.clone());
            let (target, fate) = match self.style(&name) {
                Mapped::Default => (self.dst_default.clone(), StyleFate::Replaced),
                Mapped::Target(t) => (t, StyleFate::Replaced),
                Mapped::Created => continue,
            };
            let detail = self.differing(&name, &target);
            if !detail.is_empty() {
                notes.push(StyleNote { name, fate, target: Some(target), detail });
            }
        }
        self.out.styles.extend(notes);
    }

    // ------------------------------------------------------------ properties

    fn dropped(&mut self, property: String, reason: String, at: String) {
        let key = format!("{property}\n{reason}");
        let slot = match self.properties.iter().position(|(k, _)| *k == key) {
            Some(i) => i,
            None => {
                self.properties.push((key, Property { property, reason, count: 0, at: vec![] }));
                self.properties.len() - 1
            }
        };
        let p = &mut self.properties[slot].1;
        p.count += 1;
        if p.at.len() < PLACES && !p.at.contains(&at) {
            p.at.push(at);
        }
    }

    /// `v` with its theme colour resolved to RGB, and the colour as it was and
    /// became; `None` where `v` has no theme colour the source theme can give.
    fn resolve(&self, v: &Value) -> Option<(Value, (String, String))> {
        let theme = self.theme.as_ref()?;
        let said = |from: &Color, to: &Color| (from.to_string(), to.to_string());
        match v {
            Value::Color(c) => theme.resolve(c).map(|r| (Value::Color(r.clone()), said(c, &r))),
            Value::Fill(Fill::Color(c)) => theme.resolve(c).map(|r| (Value::Fill(Fill::Color(r.clone())), said(c, &r))),
            Value::Border(Border::Line { width, style, color }) => theme.resolve(color).map(|r| {
                (Value::Border(Border::Line { width: *width, style: style.clone(), color: r.clone() }), said(color, &r))
            }),
            _ => None,
        }
    }

    /// `p` as the target can write it; what it cannot is dropped, with a line in the report.
    fn props(&mut self, p: &Props, at: &str) -> Props {
        let mut out = Props::new();
        for (k, v) in p.iter() {
            let (w, via) = match self.resolve(v) {
                Some((w, via)) => (w, Some(via)),
                None => (v.clone(), None),
            };
            match self.to.writes(k, &w) {
                Ok(()) => {
                    out.set(k, w);
                    if let Some(via) = via {
                        *self.resolved.entry(via).or_default() += 1;
                    }
                }
                Err(reason) => self.dropped(format!("{k}={v}"), reason, at.to_string()),
            }
        }
        out
    }

    // ------------------------------------------------------------ text

    /// `content` without its placeholders (each reported), its properties
    /// as the target can write them. `map` gives the offset of each unit.
    fn inline(&mut self, content: &Inline, map: Option<&ParaMap>, at: usize) -> Inline {
        let off = |k: usize| map.and_then(|m| m.units.get(k).copied()).unwrap_or(at);
        let mut units: Vec<Unit> = Vec::with_capacity(content.units.len());
        // Where each source unit went: the count of kept units before it.
        let mut index = Vec::with_capacity(content.units.len() + 1);
        for (k, u) in content.units.iter().enumerate() {
            index.push(units.len());
            match &u.atom {
                Atom::Keep(keep) => {
                    let location = self.line(off(k));
                    self.placeholder(keep, location);
                }
                _ if u.props.is_empty() => units.push(u.clone()),
                _ => {
                    let place = self.line(off(k));
                    let props = self.props(&u.props, &place);
                    units.push(Unit { atom: u.atom.clone(), marks: u.marks, props });
                }
            }
        }
        index.push(units.len());
        let spans = content
            .spans
            .iter()
            .filter_map(|s| {
                let (start, end) = (index[s.start], index[s.end.min(content.units.len())]);
                (start < end).then(|| Span { start, end, kind: s.kind.clone() })
            })
            .collect();
        Inline { units, spans }
    }

    fn placeholder(&mut self, keep: &fm::Keep, location: String) {
        *self.out.placeholder_kinds.entry(keep.kind.clone()).or_default() += 1;
        self.out.placeholders.push(Placeholder {
            id: keep.id.clone(),
            kind: keep.kind.clone(),
            summary: keep.summary.clone(),
            location,
        });
    }

    fn block(&mut self, b: &Block, map: &fm::BlockMap, out: &mut Vec<Block>) {
        let at = map.start;
        match b {
            Block::Keep(k) => {
                let location = self.line(at);
                self.placeholder(k, location);
            }
            Block::PageBreak => out.push(Block::PageBreak),
            Block::FootnoteDef(d) => out.push(Block::FootnoteDef(d.clone())),
            Block::Para(p) => {
                let pm = match &map.kind {
                    BlockMapKind::Para(m) => Some(m),
                    _ => None,
                };
                let place = self.line(at);
                let style = match &p.style {
                    ParaStyle::Plain => ParaStyle::Plain,
                    ParaStyle::Heading(k) if self.dst.styles.headings[usize::from(*k) - 1].is_some() => {
                        ParaStyle::Heading(*k)
                    }
                    ParaStyle::Heading(k) => {
                        let reason = format!(
                            "{} has no Heading {k} style: the paragraph is written as a plain one",
                            self.to.name()
                        );
                        self.dropped(format!("heading level {k}"), reason, place.clone());
                        ParaStyle::Plain
                    }
                    ParaStyle::Named(n) => self.named(n).map_or(ParaStyle::Plain, ParaStyle::Named),
                };
                let content = self.inline(&p.content, pm, at);
                let props = self.props(&p.props, &place);
                out.push(Block::Para(Para { style, content, props }));
            }
            Block::List(items) => {
                let maps = match &map.kind {
                    BlockMapKind::List(m) => Some(m),
                    _ => None,
                };
                let mut new = Vec::with_capacity(items.len());
                for (k, it) in items.iter().enumerate() {
                    let (start, pm) = match maps.and_then(|m| m.get(k)) {
                        Some((s, _, pm)) => (*s, Some(pm)),
                        None => (at, None),
                    };
                    let place = self.line(start);
                    let style = it.style.as_deref().and_then(|n| self.named(n));
                    new.push(fm::Item {
                        ordered: it.ordered,
                        level: it.level,
                        content: self.inline(&it.content, pm, start),
                        style,
                        props: self.props(&it.props, &place),
                    });
                }
                out.push(Block::List(new));
            }
            Block::Table(t) => out.push(Block::Table(self.table(t, map))),
        }
    }

    fn table(&mut self, t: &fm::Table, map: &fm::BlockMap) -> fm::Table {
        let at = map.start;
        let place = self.line(at);
        let style = match &t.style {
            Some(s) if self.dst.styles.table_names().iter().any(|n| n == s) => Some(s.clone()),
            Some(s) => {
                let reason = format!(
                    "{} has no table style \"{s}\" (its tables are drawn by their cells' own borders and fills): the table keeps the cells' own formatting",
                    self.to.name()
                );
                self.dropped(format!("table style \"{s}\""), reason, place.clone());
                None
            }
            None => None,
        };
        let mut place_of_table = t.place.clone();
        if !self.caps.table_place && !t.place.is_empty() {
            let reason = format!(
                "{} has no table position (table-align, table-indent): the table sits where {} puts it",
                self.to.name(),
                self.to.name()
            );
            self.dropped(t.place.write(), reason, place.clone());
            place_of_table = fm::TablePlace::default();
        }
        let cells = match &map.kind {
            BlockMapKind::Table(c) => Some(c),
            _ => None,
        };
        let cell_at = |r: usize, c: usize| {
            cells
                .and_then(|m| m.get(r))
                .and_then(|row| row.get(c))
                .and_then(|x| x.as_ref())
                .and_then(|ps| ps.first())
                .map_or(at, |pm| pm.mark)
        };
        let mut rows = Vec::with_capacity(t.rows.len());
        for (r, row) in t.rows.iter().enumerate() {
            let mut new = Vec::with_capacity(row.len());
            for (c, cell) in row.iter().enumerate() {
                new.push(match cell {
                    Cell::Up => Cell::Up,
                    Cell::Left => Cell::Left,
                    Cell::Text(paras) => {
                        let pms = cells.and_then(|m| m.get(r)).and_then(|row| row.get(c)).and_then(|x| x.as_ref());
                        let mut out = Vec::with_capacity(paras.len());
                        for (k, p) in paras.iter().enumerate() {
                            let pm = pms.and_then(|ps| ps.get(k));
                            let start = cell_at(r, c);
                            let place = self.line(start);
                            let style = p.style.as_deref().and_then(|n| self.named(n));
                            out.push(CellPara {
                                style,
                                content: self.inline(&p.content, pm, start),
                                props: self.props(&p.props, &place),
                            });
                        }
                        Cell::Text(out)
                    }
                });
            }
            rows.push(new);
        }
        let boxes = t
            .boxes
            .iter()
            .enumerate()
            .map(|(r, row)| {
                row.iter()
                    .enumerate()
                    .map(|(c, p)| {
                        let place = self.line(cell_at(r, c));
                        self.props(p, &place)
                    })
                    .collect()
            })
            .collect();
        fm::Table { style, rows, boxes, place: place_of_table }
    }
}

/// The text of `rem`'s revision `text`, written for `to`: the converted
/// text (valid for the target's blank package) and what it left behind.
pub fn prepare(text: &str, rem: &Remainder, from: Format, to: Format) -> Result<(String, Conversion)> {
    let parsed = fm::parse_with(text, &names(rem)).map_err(|d| Error::invalid(&d))?;
    let imp = to.engine().import(&blank::package(to), &ImportOptions::default())?;
    let dst_doc = fm::parse(&imp.text).map_err(|d| Error::invalid(&d))?;
    let dst_default = imp.remainder.styles.default_paragraph.clone();
    let mut c = Convert {
        to,
        caps: to.caps(),
        ends: text.match_indices('\n').map(|(k, _)| k).collect(),
        src: rem,
        dst: &imp.remainder,
        parsed: &parsed,
        theme: (from == Format::Docx).then(|| Theme::read(&rem.parts)),
        has_items: parsed.doc.blocks.iter().any(|b| matches!(b, Block::List(_))),
        src_default: rem.styles.default_paragraph.clone(),
        dst_default,
        dst_names: imp.remainder.styles.paragraph.iter().map(|s| (s.name.to_lowercase(), s.name.clone())).collect(),
        mapped: HashMap::new(),
        created: vec![],
        out: Conversion::new(from, to),
        properties: vec![],
        resolved: BTreeMap::new(),
    };
    let mut doc = parsed.doc.clone();
    doc.front.format = to.name().to_string();
    if let Some(t) = doc.front.template.take() {
        c.out.not_carried.push(NotCarried {
            what: format!("template \"{t}\" (the converted file starts from {}'s blank package)", to.name()),
            count: 1,
            parts: vec![],
        });
    }
    let mut blocks = Vec::with_capacity(doc.blocks.len());
    for (b, m) in parsed.doc.blocks.iter().zip(&parsed.map.blocks) {
        c.block(b, m, &mut blocks);
    }
    doc.blocks = blocks;
    c.note_styles();
    // The target's default style line, then a line for each style it lacks.
    let mut styles: Vec<StyleLine> = dst_doc.styles.iter().take(1).cloned().collect();
    for name in std::mem::take(&mut c.created) {
        let line = parsed.doc.styles.iter().find(|l| l.name == name);
        let props = line.map_or_else(Props::new, |l| l.props.clone());
        let props = c.props(&props, &format!("style \"{name}\""));
        styles.push(StyleLine { name: name.clone(), props });
        c.out.styles.push(StyleNote { name, fate: StyleFate::Created, target: None, detail: String::new() });
    }
    doc.styles = styles;
    for (_, p) in std::mem::take(&mut c.properties) {
        c.out.properties.push(p);
    }
    for ((from, to), count) in std::mem::take(&mut c.resolved) {
        c.out.resolved_colours.push(Resolved { from, to, count });
    }
    c.out.not_carried.extend(not_carried(rem, from, c.has_items));
    let out = c.out;
    Ok((fm::serialize(&doc), out))
}

/// The source revision as the package of format `to`, and the report.
pub fn export(text: &str, rem: &Remainder, from: Format, to: Format) -> Result<(Vec<u8>, Conversion)> {
    let (candidate, conversion) = prepare(text, rem, from, to)?;
    let mut ws = Workspace::new(MemStorage::new());
    let opened = ws.create(DocType::Document, Some(to), None)?;
    let failed = |mut e: Error| {
        e.message = format!("the text cannot be written as {}: {}", to.name(), e.message);
        e
    };
    ws.write(&opened.doc_id, opened.revision, &candidate).map_err(failed)?;
    let doc = ws.doc(&opened.doc_id)?;
    let (written, rem) = ws.revision(&doc, doc.head)?;
    let bytes = to.engine().export(&written, &rem).map_err(|e| failed(e.into()))?;
    Ok((bytes, conversion))
}

/// Across types nothing converts (§3): a Presentation is not a Document.
pub fn refuse_across(from: Format, to: Format) -> Result<()> {
    if from.doc_type() == to.doc_type() {
        return Ok(());
    }
    let same: Vec<&str> = Format::ALL.iter().filter(|f| f.doc_type() == from.doc_type()).map(|f| f.name()).collect();
    Err(Error::new(
        Code::Unsupported,
        format!(
            "a {} is exported as {}, not as {}: hanji converts within a type only (DESIGN.md §3). To make a {} from it, write a new one using its text as the source.",
            from.doc_type().name(),
            same.join(" or "),
            to.name(),
            to.doc_type().name()
        ),
    ))
}

// ---------------------------------------------------------------- the remainder

const SECTIONS: &str =
    "section properties: page size, orientation and margins, headers and footers (the target's own blank page is used)";
const BREAKS: &str = "section breaks (page setup that changes inside the document)";

/// What the source's remainder held beyond the text, which a conversion
/// leaves behind: entries that held something unmodelled, and the package
/// parts that are content, not structure. Placeholders are listed one by one.
fn not_carried(rem: &Remainder, from: Format, has_items: bool) -> Vec<NotCarried> {
    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for e in &rem.entries {
        if e.is_trivial() {
            continue;
        }
        let tag = e.meta.tag.rsplit(':').next().unwrap_or("");
        let what = match e.kind {
            Kind::Keep | Kind::Bkeep => continue,
            Kind::Ppr if e.xml.iter().any(|x| x.contains("sectPr")) => BREAKS,
            Kind::Ppr => "paragraph properties the text does not show (keep-with-next, tabs, list numbering, …)",
            Kind::Run => "run formatting the text does not show",
            Kind::Tbl | Kind::Tr | Kind::Tc => "table, row and cell properties the text does not show",
            Kind::Tail => SECTIONS,
            Kind::Marker | Kind::Rmarker | Kind::Wrap | Kind::Bmarker => match tag {
                "bookmarkStart" | "bookmarkEnd" => "bookmarks",
                "commentRangeStart" | "commentRangeEnd" | "commentReference" => "comment ranges",
                "moveFromRangeStart" | "moveFromRangeEnd" | "moveToRangeStart" | "moveToRangeEnd" => {
                    "tracked-move ranges"
                }
                "hyperlink" => "hyperlinks (the text stays, the address does not)",
                "fieldBegin" | "fieldEnd" => "field ranges",
                // A layout hint Word writes for the page it last rendered.
                "lastRenderedPageBreak" => continue,
                "" => "wrapped text (hyperlinks, smart tags) and block markers",
                _ => "other zero-width markers",
            },
            Kind::Slide | Kind::Shape | Kind::Range => continue,
        };
        *counts.entry(what).or_default() += 1;
    }
    if from == Format::Hwpx {
        // Page size, orientation, margins and the headers and footers are in each section's own element.
        let sections = rem
            .shell
            .first()
            .and_then(|s| serde_json::from_str::<hanji_hwpx::PackageShell>(s).ok())
            .map_or(0, |s| s.sections.iter().filter(|x| x.start_run.is_some()).count());
        if sections > 0 {
            counts.insert(SECTIONS, sections);
        }
    }
    let mut out: Vec<NotCarried> =
        counts.into_iter().map(|(what, count)| NotCarried { what: what.to_string(), count, parts: vec![] }).collect();
    let mut parts: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    for p in &rem.parts {
        if let Some(what) = part_content(&p.name, from, has_items) {
            parts.entry(what).or_default().push(p.name.clone());
        }
    }
    out.extend(parts.into_iter().map(|(what, parts)| NotCarried { what: what.to_string(), count: parts.len(), parts }));
    out
}

/// What a package part holds that is content (not structure, and not a
/// placeholder's, which is reported as one); `None` for the rest.
fn part_content(name: &str, from: Format, has_items: bool) -> Option<&'static str> {
    let any = |ps: &[&str]| ps.iter().any(|p| name.starts_with(p));
    match from {
        Format::Docx => {
            if any(&["word/header", "word/footer"]) {
                Some("headers and footers")
            } else if any(&["word/media/", "word/embeddings/", "word/charts/", "word/diagrams/", "word/drawings/"]) {
                Some("pictures, charts and embedded objects")
            } else if name.starts_with("docProps/") {
                Some("document properties (author, dates, statistics)")
            } else if name.starts_with("customXml/") {
                Some("custom XML data")
            } else if name.starts_with("word/glossary/") {
                Some("building blocks (glossary)")
            } else if name == "word/numbering.xml" && has_items {
                Some("list definitions (number formats, bullet characters): the lists use the target's own")
            } else {
                None
            }
        }
        Format::Hwpx => {
            if name.starts_with("BinData/") {
                Some("pictures and embedded objects")
            } else if name.starts_with("Preview/") {
                Some("preview image and text")
            } else if name.starts_with("Scripts/") {
                Some("scripts")
            } else {
                None
            }
        }
        Format::Pptx | Format::Xlsx => None,
    }
}
