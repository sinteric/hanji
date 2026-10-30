//! `word/document.xml` → resolved model blocks + remainder entries (a port
//! of the prototype's importer; entry kinds are documented in
//! `hanji_core::remainder::Kind`).

use std::collections::HashMap;

use hanji_core::{Block, Entry, KeepIds, Kind, ListItem, Meta, Para, StyleDef, StyleSet, Table};
use hanji_format::{
    styled, Atom, Cell, CellPara, Inline, Keep, Key, Marks, Note, NoteKind, Props, Unit,
};

use crate::format;

use crate::numbering::{num_pr, Numbering};
use crate::ooxml::*;
use crate::xml::{fragment, is_blank, Element, Node, Scope};
use hanji_package::clip;

pub struct Split {
    pub blocks: Vec<Block>,
    pub entries: Vec<Entry>,
    pub styles: StyleSet,
    pub next_id: u64,
    pub stats: Stats,
}

#[derive(Default, Debug, Clone)]
pub struct Stats {
    pub tables_modelled: usize,
    pub tables_kept: usize,
    pub kept_reasons: Vec<String>,
    /// Paragraphs imported as list items.
    pub list_items: usize,
}

pub struct Importer<'a> {
    scope: &'a Scope,
    styles: StyleSet,
    /// The id of the table style a table without `w:tblStyle` is drawn with.
    default_table: Option<String>,
    entries: Vec<Entry>,
    next_id: u64,
    next_seq: u64,
    keep_ids: KeepIds,
    notes: &'a HashMap<(String, String), String>,
    /// Comment ids whose body the text shows, mapped to that body.
    shown: &'a HashMap<String, String>,
    /// Footnote and endnote ids whose body the text shows, mapped to that body.
    shown_notes: &'a HashMap<(String, String), String>,
    /// Next footnote / endnote label (`1`, `2`, … / `e1`, `e2`, …).
    fn_n: u32,
    en_n: u32,
    buf: Vec<Unit>,
    stats: Stats,
    /// Namespace declarations on ancestors below the root.
    inherited: Vec<(String, String)>,
    numbering: &'a Numbering,
    /// The list the previous block belonged to: the numId of its items at
    /// the margin, whether they are numbered, and the ilvl of each open level.
    list: Option<(u32, bool, Vec<u32>)>,
    fmt: &'a format::Styles,
    /// Each paragraph style's values, by id.
    values: HashMap<String, Props>,
    /// The text properties of the paragraph being read's style.
    text_base: Props,
    /// A spanning field wrap opened in an earlier paragraph, waiting for
    /// its `end` run (DESIGN.md §5.1).
    open_span: Option<usize>,
}

/// Import-time checkpoint, so a table that turns out not to be a pipe
/// table can be kept whole instead.
struct Mark(usize, u64, u64);

impl<'a> Importer<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        scope: &'a Scope,
        styles: StyleSet,
        default_table: Option<String>,
        notes: &'a HashMap<(String, String), String>,
        shown: &'a HashMap<String, String>,
        shown_notes: &'a HashMap<(String, String), String>,
        numbering: &'a Numbering,
        fmt: &'a format::Styles,
    ) -> Self {
        Importer {
            scope,
            styles,
            default_table,
            entries: vec![],
            next_id: 1,
            next_seq: 1000,
            keep_ids: KeepIds::default(),
            notes,
            shown,
            shown_notes,
            fn_n: 0,
            en_n: 0,
            buf: vec![],
            stats: Stats::default(),
            inherited: vec![],
            numbering,
            list: None,
            fmt,
            values: HashMap::new(),
            text_base: Props::new(),
            open_span: None,
        }
    }

    /// The values of paragraph style `id` (§5.2).
    fn style_values(&mut self, id: &str) -> Props {
        if let Some(v) = self.values.get(id) {
            return v.clone();
        }
        let v = self.fmt.values(id);
        self.values.insert(id.to_string(), v.clone());
        v
    }

    fn checkpoint(&self) -> Mark {
        Mark(self.entries.len(), self.next_id, self.next_seq)
    }

    fn rollback(&mut self, m: Mark) {
        for e in self.entries.drain(m.0..) {
            if let Some(k) = e.meta.keep {
                self.keep_ids.release(&k.id);
            }
        }
        (self.next_id, self.next_seq) = (m.1, m.2);
        if self.open_span.is_some_and(|i| i >= m.0) {
            self.open_span = None;
        }
    }

    fn seq(&mut self) -> u64 {
        let s = self.next_seq;
        self.next_seq += 1000;
        s
    }

    /// A fragment, self-contained against the document's namespace map.
    fn frag(&self, e: &Element) -> String {
        if self.inherited.is_empty() {
            return e.to_xml();
        }
        let mut x = e.clone();
        for (k, v) in &self.inherited {
            if x.attr(k).is_none() {
                x.attrs.push((k.clone(), v.clone()));
            }
        }
        x.to_xml()
    }

    fn fp(&self, els: &[Option<&Element>]) -> String {
        fp(self.scope, els)
    }

    #[allow(clippy::too_many_arguments)]
    fn entry(
        &mut self,
        kind: Kind,
        xml: Vec<String>,
        fp: String,
        path: &[usize],
        start: Option<usize>,
        end: Option<usize>,
        meta: Meta,
    ) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        let seq = self.seq();
        self.entries.push(Entry {
            id,
            kind,
            xml,
            fp,
            path: path.to_vec(),
            start,
            end,
            seq,
            meta,
        });
        self.entries.len() - 1
    }

    fn keep_entry(
        &mut self,
        kind: Kind,
        els: &[&Element],
        path: &[usize],
        pos: Option<usize>,
        run: Option<u64>,
    ) -> Keep {
        let fpv = self.fp(&els.iter().map(|e| Some(*e)).collect::<Vec<_>>());
        let id = self.keep_ids.next(&fpv);
        let kind_name = if kind == Kind::Bkeep && els[0].is("w:tbl") {
            "table".to_string()
        } else if els.len() > 1 || (els[0].is("w:r") && kind == Kind::Keep) {
            "field".to_string()
        } else {
            keep_kind(&els[0].name)
        };
        let keep = Keep {
            id,
            kind: kind_name,
            summary: self.summary(els),
        };
        let xml = els.iter().map(|e| self.frag(e)).collect();
        let meta = Meta {
            keep: Some(keep.clone()),
            run,
            ..Default::default()
        };
        self.entry(kind, xml, fpv, path, pos, pos.map(|p| p + 1), meta);
        keep
    }

    // ------------------------------------------------------------ summaries

    fn note(&self, part: &str, id: Option<String>) -> String {
        id.and_then(|id| self.notes.get(&(part.to_string(), id)).cloned())
            .unwrap_or_default()
    }

    fn summary(&self, els: &[&Element]) -> String {
        let el = els[0];
        let text =
            |e: &Element, n: usize| clip(&e.text_of(&["w:t", "w:delText", "w:instrText"]), n);
        let s = match el.local() {
            "drawing" | "pict" | "AlternateContent" | "object" => {
                let mut found = None;
                el.walk(&mut |e| {
                    if found.is_none() && e.local() == "docPr" {
                        found = e
                            .get("descr")
                            .filter(|d| !d.is_empty())
                            .or_else(|| e.get("name"));
                    }
                });
                found.unwrap_or_else(|| {
                    if el.local() == "pict" {
                        "vml shape".into()
                    } else {
                        el.local().into()
                    }
                })
            }
            "footnoteReference" => format!("footnote: {}", self.note("footnote", el.get("w:id"))),
            "endnoteReference" => format!("endnote: {}", self.note("endnote", el.get("w:id"))),
            "commentReference" => format!("comment: {}", self.note("comment", el.get("w:id"))),
            "ins" | "del" | "moveFrom" | "moveTo" => {
                format!(
                    "{} by {}: {}",
                    el.local(),
                    el.get("w:author").unwrap_or_else(|| "?".into()),
                    text(el, 40)
                )
            }
            "sdt" => {
                let alias = el
                    .descendants("w:alias")
                    .first()
                    .and_then(|a| a.get("w:val"));
                format!(
                    "{}{}",
                    alias.map(|a| a + ": ").unwrap_or_default(),
                    text(el, 40)
                )
            }
            "fldSimple" => {
                format!(
                    "field {} → {}",
                    clip(&el.get("w:instr").unwrap_or_default(), 30),
                    text(el, 25)
                )
            }
            "r" if els.len() > 1 || !el.descendants("w:fldChar").is_empty() => {
                let instr: String = els.iter().map(|e| e.text_of(&["w:instrText"])).collect();
                let result = field_result(els);
                format!("field {} → {}", clip(&instr, 30), clip(&result, 25))
                    .trim()
                    .to_string()
            }
            "tbl" => format!("table: {}", text(el, 40)),
            _ => {
                let t = text(el, 40);
                if t.is_empty() {
                    el.local().to_string()
                } else {
                    format!("{}: {t}", el.local())
                }
            }
        };
        clip(&s, 80)
    }

    // ------------------------------------------------------------ document

    pub fn body(mut self, body: &Element) -> Result<Split, String> {
        let mut blocks = vec![];
        let mut pending: Vec<&Element> = vec![];
        let items = expand_items(&body.children)?;
        let p_els: Vec<&Element> = items
            .iter()
            .filter_map(|it| match it {
                WalkItem::El(e) if e.is("w:p") => Some(*e),
                _ => None,
            })
            .collect();
        let span_of = span_roles(&p_els);
        let mut p_i = 0usize;
        let mut sdt_stack: Vec<OpenSdt<'_>> = vec![];
        for (idx, item) in items.iter().enumerate() {
            match item {
                WalkItem::OpenSdt(sdt) => {
                    sdt_stack.push(OpenSdt {
                        el: sdt,
                        first: None,
                        leading: vec![],
                    });
                }
                WalkItem::CloseSdt => {
                    let mut open = sdt_stack.pop().ok_or_else(|| {
                        "a content control closes with no matching open".to_string()
                    })?;
                    let trailing = if open.first.is_some() {
                        std::mem::take(&mut pending)
                    } else {
                        vec![]
                    };
                    self.finish_block_sdt(&mut blocks, &mut open, trailing);
                }
                WalkItem::El(el) => {
                    let last_el = items[idx + 1..]
                        .iter()
                        .all(|it| !matches!(it, WalkItem::El(_)));
                    if el.is("w:sectPr") && last_el {
                        pending.push(el);
                        continue;
                    }
                    if MARKERS.contains(&el.name.as_str()) {
                        if let Some(top) = sdt_stack.last_mut() {
                            if top.first.is_none() {
                                top.leading.push(el);
                                continue;
                            }
                        }
                        pending.push(el);
                        continue;
                    }
                    let bi = blocks.len();
                    for open in &mut sdt_stack {
                        if open.first.is_none() {
                            open.first = Some(bi);
                        }
                    }
                    for m in std::mem::take(&mut pending) {
                        self.bmarker(m, &[bi]);
                    }
                    let b = match el.name.as_str() {
                        "w:p" => {
                            let span = span_of.get(&p_i).copied();
                            p_i += 1;
                            Block::Para(self.para(el, &[bi], span)?)
                        }
                        "w:tbl" => self.table(el, bi)?,
                        _ => self.block_keep(el, bi),
                    };
                    if !matches!(b, Block::Para(Para { item: Some(_), .. })) {
                        self.list = None;
                    }
                    blocks.push(b);
                }
            }
        }
        if !sdt_stack.is_empty() {
            return Err("a content control is not closed".into());
        }
        for m in pending {
            let (x, f) = (self.frag(m), self.fp(&[Some(m)]));
            self.entry(Kind::Tail, vec![x], f, &[], None, None, Meta::default());
        }
        Ok(Split {
            blocks,
            entries: self.entries,
            styles: self.styles,
            next_id: self.next_id,
            stats: self.stats,
        })
    }

    /// Close a shown block content control: inner blocks are already in
    /// `blocks`; the wrap holds `sdtPr` / `sdtEndPr` and any leading or
    /// trailing markers that sat in `sdtContent` around them.
    fn finish_block_sdt(
        &mut self,
        blocks: &mut Vec<Block>,
        open: &mut OpenSdt<'_>,
        trailing: Vec<&Element>,
    ) {
        let Some(first) = open.first else {
            let bi = blocks.len();
            blocks.push(self.block_keep(open.el, bi));
            return;
        };
        let last = blocks.len() - 1;
        let end = match &blocks[last] {
            Block::Para(p) => p.content.units.len(),
            _ => 0,
        };
        let shell = sdt_block_shell(open.el, &open.leading);
        let pr = open.el.child("w:sdtPr");
        let end_pr = open.el.child("w:sdtEndPr");
        let f = self.fp(&[pr, end_pr, Some(&shell)]);
        let meta = Meta {
            tag: "w:sdt/block".into(),
            aux: trailing.iter().map(|e| self.frag(e)).collect(),
            end_path: Some(vec![last]),
            ..Default::default()
        };
        let xml = vec![self.frag(&shell)];
        let idx = self.entry(Kind::Wrap, xml, f, &[first], Some(0), Some(end), meta);
        let close = self.seq();
        self.entries[idx].meta.seq_close = close;
    }

    /// A zero-width element between blocks, rows, cells or cell paragraphs:
    /// before the one at `path`.
    fn bmarker(&mut self, m: &Element, path: &[usize]) {
        let (x, f) = (self.frag(m), self.fp(&[Some(m)]));
        self.entry(Kind::Bmarker, vec![x], f, path, None, None, Meta::default());
    }

    fn block_keep(&mut self, el: &Element, bi: usize) -> Block {
        let k = self.keep_entry(Kind::Bkeep, &[el], &[bi], None, None);
        Block::Keep(k.id)
    }

    // ------------------------------------------------------------ tables

    fn table_reason(&self, tbl: &Element) -> Option<String> {
        if let Some(r) = structure_reason(tbl, &["w:tblPr", "w:tblGrid"], "w:tr", "table") {
            return Some(r);
        }
        let mut grid: Vec<Vec<Cell>> = vec![];
        for (ri, tr) in tbl.elements().filter(|e| e.is("w:tr")).enumerate() {
            let mut row = vec![];
            if let Some(r) = row_structure_reason(tr) {
                return Some(r);
            }
            if let Some(trpr) = tr.child("w:trPr") {
                if trpr.child("w:gridBefore").is_some() || trpr.child("w:gridAfter").is_some() {
                    return Some("gridBefore/gridAfter".into());
                }
            }
            for (tc, _) in row_cells(tr) {
                let ps: Vec<&Node> = tc
                    .children
                    .iter()
                    .filter(|n| {
                        !is_blank(n)
                            && !matches!(n, Node::El(e) if e.is("w:tcPr") || MARKERS.contains(&e.name.as_str()))
                    })
                    .collect();
                let all_p = ps.iter().all(|n| matches!(n, Node::El(e) if e.is("w:p")));
                if ps.is_empty() || !all_p {
                    let mut kinds: Vec<&str> = ps
                        .iter()
                        .filter_map(|n| {
                            if let Node::El(e) = n {
                                (!e.is("w:p")).then(|| e.local())
                            } else {
                                Some("#text")
                            }
                        })
                        .collect();
                    kinds.sort();
                    kinds.dedup();
                    return Some(if ps.is_empty() {
                        "cell without a paragraph".into()
                    } else {
                        format!("cell holds {}", kinds.join(","))
                    });
                }
                let paras: Vec<&Element> = ps
                    .iter()
                    .filter_map(|n| if let Node::El(e) = n { Some(e) } else { None })
                    .collect();
                let tcpr = tc.child("w:tcPr");
                let (span, covered) = (grid_span(tcpr), v_merged(tcpr));
                if covered {
                    if ri == 0 {
                        return Some("vMerge continue in first row".into());
                    }
                    if !paras.iter().all(|p| para_is_blank(p)) {
                        return Some("content in a covered cell".into());
                    }
                }
                if tcpr.is_some_and(|t| t.child("w:hMerge").is_some()) {
                    return Some("hMerge".into());
                }
                row.push(if covered {
                    Cell::Up
                } else {
                    Cell::text(Inline::default())
                });
                row.extend((1..span).map(|_| Cell::Left));
            }
            grid.push(row);
        }
        if grid.is_empty() || grid[0].is_empty() {
            return Some("empty table".into());
        }
        if grid.iter().any(|r| r.len() != grid[0].len()) {
            return Some("ragged rows".into());
        }
        if hanji_format::merge_problem(&grid).is_some() {
            return Some("merged area is not a rectangle".into());
        }
        None
    }

    fn table(&mut self, tbl: &Element, bi: usize) -> Result<Block, String> {
        if let Some(reason) = self.table_reason(tbl) {
            self.stats.tables_kept += 1;
            self.stats.kept_reasons.push(reason);
            return Ok(self.block_keep(tbl, bi));
        }
        let mark = self.checkpoint();
        match self.table_inner(tbl, bi)? {
            Some(t) => {
                self.stats.tables_modelled += 1;
                Ok(Block::Table(t))
            }
            None => {
                self.rollback(mark);
                self.stats.tables_kept += 1;
                self.stats
                    .kept_reasons
                    .push("cell text not writable in a pipe table".into());
                Ok(self.block_keep(tbl, bi))
            }
        }
    }

    fn table_inner(&mut self, tbl: &Element, bi: usize) -> Result<Option<Table>, String> {
        let shell = tbl.shell();
        let head: Vec<&Element> = tbl
            .elements()
            .filter(|e| e.is("w:tblPr") || e.is("w:tblGrid"))
            .collect();
        // The style the table is drawn with: its own, or the file's default.
        let style_id = tbl
            .child("w:tblPr")
            .and_then(|p| p.child("w:tblStyle"))
            .and_then(|s| s.get("w:val"));
        let style_id = style_id.or_else(|| self.default_table.clone());
        // The table's own position (§5.2); the fingerprint leaves out what the text shows.
        let place = tbl
            .child("w:tblPr")
            .map(format::tblpr_place)
            .unwrap_or_default();
        let mut head_fp: Vec<Element> = head.iter().map(|e| (*e).clone()).collect();
        for h in &mut head_fp {
            if h.is("w:tblPr") {
                for n in std::iter::once("w:tblStyle").chain(format::shown_tblpr(h)) {
                    remove_child(h, n);
                }
            }
        }
        let fpv = fp(
            self.scope,
            &std::iter::once(Some(&shell))
                .chain(head_fp.iter().map(Some))
                .collect::<Vec<_>>(),
        );
        let xml = std::iter::once(self.frag(&shell))
            .chain(head.iter().map(|e| self.frag(e)))
            .collect();
        self.entry(
            Kind::Tbl,
            xml,
            fpv,
            &[bi],
            None,
            None,
            Meta {
                style: style_id.clone(),
                ..Default::default()
            },
        );
        let style = style_id.and_then(|id| self.styles.table_style_name(&id));
        let mut rows = vec![];
        let mut boxes: Vec<Vec<Props>> = vec![];
        for el in tbl.elements() {
            if MARKERS.contains(&el.name.as_str()) {
                // A marker between rows (or after the last): before row `rows.len()`.
                self.bmarker(el, &[bi, rows.len()]);
                continue;
            }
            if !el.is("w:tr") {
                continue;
            }
            let (ri, tr) = (rows.len(), el);
            let trs = tr.shell();
            let rhead: Vec<&Element> = tr
                .elements()
                .filter(|e| e.is("w:tblPrEx") || e.is("w:trPr"))
                .collect();
            let fpv = fp(
                self.scope,
                &std::iter::once(Some(&trs))
                    .chain(rhead.iter().map(|e| Some(*e)))
                    .collect::<Vec<_>>(),
            );
            let xml = std::iter::once(self.frag(&trs))
                .chain(rhead.iter().map(|e| self.frag(e)))
                .collect();
            self.entry(Kind::Tr, xml, fpv, &[bi, ri], None, None, Meta::default());
            let mut row = vec![];
            let mut brow = vec![];
            for el in tr.elements() {
                if MARKERS.contains(&el.name.as_str()) {
                    // A marker between cells (or after the last): before grid column `row.len()`.
                    self.bmarker(el, &[bi, ri, row.len()]);
                    continue;
                }
                let (tc, wrap) = if el.is("w:tc") {
                    (el, None)
                } else if let Some(inner) = cell_sdt(el) {
                    (inner, Some(el))
                } else {
                    continue;
                };
                let gc = row.len();
                let tcpr = tc.child("w:tcPr");
                let tcs = tc.shell();
                let mut xml: Vec<String> = std::iter::once(self.frag(&tcs))
                    .chain(tcpr.map(|t| self.frag(t)))
                    .collect();
                let mut rest = tcpr.cloned();
                if let Some(x) = &mut rest {
                    for n in format::SHOWN_TCPR {
                        remove_child(x, n);
                    }
                }
                let mut fparts: Vec<Option<&Element>> = vec![Some(&tcs), rest.as_ref()];
                let sdt_shell = wrap.map(sdt_cell_shell);
                if let Some(s) = &sdt_shell {
                    xml.push(self.frag(s));
                    fparts.push(Some(s));
                }
                let f = self.fp(&fparts);
                let meta = Meta {
                    tag: if wrap.is_some() {
                        "w:sdt/cell".into()
                    } else {
                        String::new()
                    },
                    ..Default::default()
                };
                self.entry(Kind::Tc, xml, f, &[bi, ri, gc], None, None, meta);
                let mut paras = vec![];
                let cell_ps: Vec<&Element> = tc.elements().filter(|e| e.is("w:p")).collect();
                let cell_span = span_roles(&cell_ps);
                let mut p_i = 0usize;
                for e in tc.elements().filter(|e| !e.is("w:tcPr")) {
                    let k = paras.len();
                    if e.is("w:p") {
                        let span = cell_span.get(&p_i).copied();
                        p_i += 1;
                        let para = self.para(e, &[bi, ri, gc, k], span)?;
                        let style =
                            (para.style != self.styles.default_paragraph).then_some(para.style);
                        paras.push(CellPara {
                            props: para.props,
                            style,
                            content: para.content,
                        });
                    } else {
                        // A marker between a cell's paragraphs: before paragraph k.
                        self.bmarker(e, &[bi, ri, gc, k]);
                    }
                }
                let span = grid_span(tcpr);
                if v_merged(tcpr) {
                    row.push(Cell::Up);
                    brow.push(Props::new());
                } else {
                    if !paras.iter().all(|p| cell_writable(&p.content)) {
                        return Ok(None);
                    }
                    row.push(Cell::Text(paras));
                    brow.push(
                        tcpr.map(format::tcpr_props)
                            .unwrap_or_default()
                            .diff(&styled::box_default()),
                    );
                }
                row.extend((1..span).map(|_| Cell::Left));
                brow.extend((1..span).map(|_| Props::new()));
            }
            rows.push(row);
            boxes.push(brow);
        }
        if boxes.iter().flatten().all(Props::is_empty) {
            boxes.clear();
        }
        Ok(Some(Table {
            style,
            rows,
            boxes,
            place,
        }))
    }

    // ------------------------------------------------------------ paragraphs

    fn para(
        &mut self,
        p: &Element,
        path: &[usize],
        span: Option<ParaSpan>,
    ) -> Result<Para, String> {
        let saved = self.inherited.len();
        self.inherit(p);
        let ppr = p.child("w:pPr");
        let sid = ppr
            .and_then(|x| x.child("w:pStyle"))
            .and_then(|s| s.get("w:val"));
        let default_id = self.styles.default_paragraph_id().to_string();
        let sid = sid.unwrap_or_else(|| default_id.clone());
        let style = match self.styles.paragraph_name(&sid) {
            Some(n) => n.to_string(),
            None => {
                // A style id missing from styles.xml: keep it by id.
                self.styles
                    .paragraph
                    .push(StyleDef::new(sid.clone(), sid.clone()));
                sid.clone()
            }
        };
        let shell = p.shell();
        // The fingerprint leaves out what the text shows (§5.2); an empty
        // paragraph's properties it cannot show are marked `unshown`.
        let mut rest = ppr.cloned();
        if let Some(r) = &mut rest {
            for n in std::iter::once("w:pStyle").chain(format::SHOWN_PPR) {
                remove_child(r, n);
            }
        }
        let rest = rest.filter(|r| !r.children.is_empty() || !r.attrs.is_empty());
        let f = if shell.attrs.is_empty() && rest.is_none() {
            String::new()
        } else {
            self.fp(&[Some(&shell), rest.as_ref()])
        };
        let xml = std::iter::once(self.frag(&shell))
            .chain(ppr.map(|x| self.frag(x)))
            .collect();
        // A numbered top-level paragraph is a list item; a heading keeps its
        // numbering in the remainder, and so does a cell paragraph.
        let item = (path.len() == 1 && self.styles.heading_level(&style).is_none())
            .then(|| self.list_item(ppr, &sid))
            .flatten();
        let meta = Meta {
            style: Some(sid.clone()),
            item: item.as_ref().map(|i| i.0),
            aux: item.as_ref().map(|i| i.1.to_vec()).unwrap_or_default(),
            ..Default::default()
        };
        let ppr_at = self.entry(Kind::Ppr, xml, f, path, None, None, meta);
        // What the paragraph sets beyond its style (§5.2).
        let base = self.style_values(&sid);
        let size = base.get(Key::Size).and_then(|v| v.length()).unwrap_or(1000);
        let props = ppr
            .map(|x| format::ppr_props(x, size))
            .unwrap_or_default()
            .only(&styled::PARA_OWN)
            .diff(&base);
        self.text_base = base.only(&Key::TEXT);
        self.buf.clear();
        let page_break_only = sid == default_id && is_page_break_para(p);
        self.walk_para(p, path, page_break_only, span)?;
        self.inherited.truncate(saved);
        let mut content = Inline {
            units: std::mem::take(&mut self.buf),
            spans: vec![],
        };
        content.normalize();
        // A paragraph without text shows none of it, and keeps it (§5.2).
        self.entries[ppr_at].meta.unshown = !styled::shows(&content) && !props.is_empty();
        let item = item.map(|i| i.0);
        self.stats.list_items += item.is_some() as usize;
        Ok(Para {
            style,
            content,
            item,
            props,
        })
    }

    /// The list item a paragraph is, from its own `w:numPr` or its style's,
    /// with the numbering to keep: `[numId, ilvl, "direct" | "style"]`.
    fn list_item(&mut self, ppr: Option<&Element>, sid: &str) -> Option<(ListItem, [String; 3])> {
        let (num, lvl) = num_pr(ppr);
        let style = self.numbering.of_style(sid);
        let num = num.or(style.map(|s| s.0)).filter(|&n| n != 0)?;
        let direct = ppr.and_then(|p| p.child("w:numPr")).is_some();
        let ilvl = lvl.or(style.filter(|_| !direct).map(|s| s.1)).unwrap_or(0);
        let ordered = self.numbering.ordered(num, ilvl)?;
        // A list is one numId and one kind at the margin: an item back at the
        // margin with another numId, or of the other kind, starts a new list,
        // as it does in the text (§5.2). A nested item may use another numId
        // (numbers under a bullet). Levels nest by ilvl; the text level counts
        // the open levels, so a list starting at ilvl 2 is still at the margin.
        let nested = match &mut self.list {
            Some((n, margin, stack)) => {
                while stack.last().is_some_and(|&t| t >= ilvl) {
                    stack.pop();
                }
                (!stack.is_empty() || (*n == num && *margin == ordered)).then(|| {
                    stack.push(ilvl);
                    stack.len() - 1
                })
            }
            _ => None,
        };
        let (first, level) = match nested {
            Some(level) => (false, level),
            None => {
                self.list = Some((num, ordered, vec![ilvl]));
                (true, 0)
            }
        };
        let how = if direct { "direct" } else { "style" };
        Some((
            ListItem {
                ordered,
                level,
                first,
            },
            [num.to_string(), ilvl.to_string(), how.to_string()],
        ))
    }

    fn inherit(&mut self, e: &Element) {
        for (k, v) in &e.attrs {
            if crate::xml::ns_prefix(k).is_some() {
                self.inherited.push((k.clone(), v.clone()));
            }
        }
    }

    fn push(&mut self, atom: Atom, marks: Marks) {
        self.buf.push(Unit::new(atom, marks));
    }

    fn pos(&self) -> usize {
        self.buf.len()
    }

    fn walk(&mut self, container: &Element, path: &[usize], pb: bool) -> Result<(), String> {
        self.walk_kids(&content_kids(container)?, path, pb)
    }

    /// A paragraph: its children, with a spanning field's begin and/or end
    /// taken out of the walk and stored as the wrap.
    fn walk_para(
        &mut self,
        p: &Element,
        path: &[usize],
        pb: bool,
        span: Option<ParaSpan>,
    ) -> Result<(), String> {
        let kids = content_kids(p)?;
        let mut i = 0;
        if let Some(sp) = span {
            if let (Some(end), Some((begin, _))) = (sp.close, sp.open) {
                if begin < end {
                    return self.walk_kids(&kids, path, pb);
                }
            }
            if let Some(end) = sp.close {
                self.walk_kids(&kids[i..end], path, pb)?;
                i = self.close_spanning(&kids, end, path)? + 1;
            }
            if let Some((begin, sep)) = sp.open {
                self.walk_kids(&kids[i..begin], path, pb)?;
                self.open_spanning(&kids, begin, sep, path)?;
                i = sep + 1;
            }
        }
        self.walk_kids(&kids[i..], path, pb)
    }

    /// [`Importer::walk`] over a paragraph's or wrapper's children.
    fn walk_kids(&mut self, kids: &[&Element], path: &[usize], pb: bool) -> Result<(), String> {
        let groups = field_groups(kids);
        let mut i = 0;
        while i < kids.len() {
            let k = kids[i];
            if let Some(&end) = groups.get(&i) {
                if let Some(sep) = shown_field(kids, i, end) {
                    let close = field_close(kids, i, end);
                    self.field(kids, (i, sep, end), close, path, pb)?;
                    i = close + 1;
                    continue;
                }
                let pos = self.pos();
                let keep = self.keep_entry(Kind::Keep, &kids[i..=end], path, Some(pos), None);
                self.push(Atom::Keep(keep), Marks::NONE);
                i = end + 1;
                continue;
            }
            let name = k.name.as_str();
            if crate::comment::shown_run(k, self.shown).is_some() {
                self.comment_ref(k, path)?;
                i += 1;
                continue;
            }
            if crate::note::shown_run(k, self.shown_notes).is_some() {
                self.note_ref(k, path)?;
                i += 1;
                continue;
            }
            if crate::revision::shown(k).is_some() {
                self.revision(k, path)?;
                i += 1;
                continue;
            }
            if name == "w:r" {
                self.run(k, path, pb)?;
            } else if name == "w:sdt" && sdt_shown(k) && !sdt_content_is_block(k) {
                let shell = sdt_inline_shell(k);
                let pr = k.child("w:sdtPr");
                let end_pr = k.child("w:sdtEndPr");
                let (x, f) = (self.frag(&shell), self.fp(&[pr, end_pr, Some(&shell)]));
                let at = self.pos();
                let meta = Meta {
                    tag: "w:sdt".into(),
                    ..Default::default()
                };
                let idx = self.entry(Kind::Wrap, vec![x], f, path, Some(at), None, meta);
                let saved = self.inherited.len();
                self.inherit(k);
                if let Some(c) = k.child("w:sdtContent") {
                    self.walk(c, path, pb)?;
                }
                self.inherited.truncate(saved);
                let end = self.pos();
                let close = self.seq();
                let e = &mut self.entries[idx];
                e.end = Some(end);
                e.meta.seq_close = close;
            } else if WRAPPERS.contains(&name) || (name == "w:fldSimple" && shown_simple_field(k)) {
                let mut shell = k.shell();
                shell.children = k
                    .children
                    .iter()
                    .filter(|n| matches!(n, Node::El(e) if e.is("w:smartTagPr") || e.is("w:customXmlPr")))
                    .cloned()
                    .collect();
                let (x, f) = (self.frag(&shell), self.fp(&[Some(&shell)]));
                let at = self.pos();
                // A simple field's tag tells conversion reports what it was.
                let meta = Meta {
                    tag: if name == "w:fldSimple" {
                        name.into()
                    } else {
                        String::new()
                    },
                    ..Default::default()
                };
                let idx = self.entry(Kind::Wrap, vec![x], f, path, Some(at), None, meta);
                let saved = self.inherited.len();
                self.inherit(k);
                self.walk(k, path, pb)?;
                self.inherited.truncate(saved);
                let end = self.pos();
                let close = self.seq();
                let e = &mut self.entries[idx];
                e.end = Some(end);
                e.meta.seq_close = close;
            } else if MARKERS.contains(&name) {
                let (x, f) = (self.frag(k), self.fp(&[Some(k)]));
                let meta = Meta {
                    tag: name.into(),
                    durable: DURABLE_MARKERS.contains(&name),
                    opens: name.ends_with("Start"),
                    ..Default::default()
                };
                let at = self.pos();
                self.entry(Kind::Marker, vec![x], f, path, Some(at), Some(at), meta);
            } else {
                let pos = self.pos();
                let keep = self.keep_entry(Kind::Keep, &[k], path, Some(pos), None);
                self.push(Atom::Keep(keep), Marks::NONE);
            }
            i += 1;
        }
        Ok(())
    }

    /// An inline `w:ins` / `w:del` whose text the model shows: a length-1
    /// wrap over [`Atom::Ins`] / [`Atom::Del`], holding the original element
    /// (DESIGN.md §5.1).
    fn revision(&mut self, el: &Element, path: &[usize]) -> Result<(), String> {
        let (ins, body) = crate::revision::shown(el)
            .ok_or_else(|| "a tracked change has no shown body".to_string())?;
        let f = self.fp(&[Some(el)]);
        let meta = Meta {
            tag: if ins { "w:ins" } else { "w:del" }.into(),
            ..Default::default()
        };
        let at = self.pos();
        let idx = self.entry(
            Kind::Wrap,
            vec![self.frag(el)],
            f,
            path,
            Some(at),
            None,
            meta,
        );
        self.push(
            if ins {
                Atom::Ins(body)
            } else {
                Atom::Del(body)
            },
            Marks::NONE,
        );
        let end = self.pos();
        let close = self.seq();
        let e = &mut self.entries[idx];
        e.end = Some(end);
        e.meta.seq_close = close;
        Ok(())
    }

    /// A comment-only run whose body the text shows: a length-1 wrap over
    /// a [`Atom::Comment`], holding the original run (DESIGN.md §5.1).
    fn comment_ref(&mut self, r: &Element, path: &[usize]) -> Result<(), String> {
        let cref = crate::comment::comment_only_run(r)
            .ok_or_else(|| "a comment run has no w:commentReference".to_string())?;
        let id = cref
            .get("w:id")
            .ok_or_else(|| "a commentReference has no w:id".to_string())?;
        let body = self
            .shown
            .get(&id)
            .cloned()
            .ok_or_else(|| format!("comment {id} has no shown body"))?;
        let f = self.fp(&[Some(r)]);
        let meta = Meta {
            tag: "w:commentReference".into(),
            ..Default::default()
        };
        let at = self.pos();
        let idx = self.entry(
            Kind::Wrap,
            vec![self.frag(r)],
            f,
            path,
            Some(at),
            None,
            meta,
        );
        self.push(Atom::Comment(body), Marks::NONE);
        let end = self.pos();
        let close = self.seq();
        let e = &mut self.entries[idx];
        e.end = Some(end);
        e.meta.seq_close = close;
        Ok(())
    }

    /// A note-only run whose body the text shows: a length-1 wrap over
    /// a [`Atom::NoteRef`], holding the original run (DESIGN.md §5.1).
    fn note_ref(&mut self, r: &Element, path: &[usize]) -> Result<(), String> {
        let (_, kind, body) = crate::note::shown_run(r, self.shown_notes)
            .ok_or_else(|| "a note run has no shown body".to_string())?;
        let label = match kind {
            NoteKind::Footnote => {
                self.fn_n += 1;
                self.fn_n.to_string()
            }
            NoteKind::Endnote => {
                self.en_n += 1;
                format!("e{}", self.en_n)
            }
        };
        let f = self.fp(&[Some(r)]);
        let tag = match kind {
            NoteKind::Footnote => "w:footnoteReference",
            NoteKind::Endnote => "w:endnoteReference",
        };
        let meta = Meta {
            tag: tag.into(),
            ..Default::default()
        };
        let at = self.pos();
        let idx = self.entry(
            Kind::Wrap,
            vec![self.frag(r)],
            f,
            path,
            Some(at),
            None,
            meta,
        );
        self.push(Atom::NoteRef(Note { label, body, kind }), Marks::NONE);
        let end = self.pos();
        let close = self.seq();
        let e = &mut self.entries[idx];
        e.end = Some(end);
        e.meta.seq_close = close;
        Ok(())
    }

    /// A complex field whose cached result shows as text (DESIGN.md §5.1):
    /// a [`Kind::Wrap`] over the result, holding the runs from `begin` to
    /// `separate` (`xml`) and the `end` run (`meta.aux`), which export writes
    /// around the result's runs as they were. A matching `bookmarkEnd` after
    /// `end` (Word's form-field bookmark) is stored with the `end` run so it
    /// leaves with the field.
    fn field(
        &mut self,
        kids: &[&Element],
        (begin, sep, end): (usize, usize, usize),
        close_at: usize,
        path: &[usize],
        pb: bool,
    ) -> Result<(), String> {
        let open: Vec<&Element> = kids[begin..=sep].to_vec();
        let tail: Vec<&Element> = if close_at > end {
            vec![kids[end], kids[close_at]]
        } else {
            vec![kids[end]]
        };
        let parts: Vec<Option<&Element>> = open
            .iter()
            .copied()
            .chain(tail.iter().copied())
            .map(Some)
            .collect();
        let f = self.fp(&parts);
        let meta = Meta {
            tag: "w:fldChar".into(),
            aux: tail.iter().map(|e| self.frag(e)).collect(),
            ..Default::default()
        };
        let xml = open.iter().map(|e| self.frag(e)).collect();
        let at = self.pos();
        let idx = self.entry(Kind::Wrap, xml, f, path, Some(at), None, meta);
        self.walk_kids(&kids[sep + 1..end], path, pb)?;
        let end = self.pos();
        let close = self.seq();
        let e = &mut self.entries[idx];
        e.end = Some(end);
        e.meta.seq_close = close;
        Ok(())
    }

    /// Open a spanning field wrap at the result's start in this paragraph.
    /// The matching [`Importer::close_spanning`] stores the `end` run.
    fn open_spanning(
        &mut self,
        kids: &[&Element],
        begin: usize,
        sep: usize,
        path: &[usize],
    ) -> Result<(), String> {
        if self.open_span.is_some() {
            return Err("a field begins inside another field that spans paragraphs".into());
        }
        let open: Vec<&Element> = kids[begin..=sep].to_vec();
        let f = self.fp(&open.iter().copied().map(Some).collect::<Vec<_>>());
        let meta = Meta {
            tag: "w:fldChar".into(),
            ..Default::default()
        };
        let xml = open.iter().map(|e| self.frag(e)).collect();
        let at = self.pos();
        let idx = self.entry(Kind::Wrap, xml, f, path, Some(at), None, meta);
        self.open_span = Some(idx);
        Ok(())
    }

    fn close_spanning(
        &mut self,
        kids: &[&Element],
        end: usize,
        path: &[usize],
    ) -> Result<usize, String> {
        let idx = self
            .open_span
            .take()
            .ok_or_else(|| "a field ends with no begin".to_string())?;
        let close_at = field_close(kids, end, end);
        let tail: Vec<&Element> = if close_at > end {
            vec![kids[end], kids[close_at]]
        } else {
            vec![kids[end]]
        };
        let xml = self.entries[idx].xml.clone();
        let opens: Vec<Element> = xml.iter().map(|x| fragment(x)).collect();
        let parts: Vec<Option<&Element>> = opens
            .iter()
            .map(Some)
            .chain(tail.iter().copied().map(Some))
            .collect();
        let f = self.fp(&parts);
        let aux: Vec<String> = tail.iter().map(|el| self.frag(el)).collect();
        let at = self.pos();
        let seq_close = self.seq();
        let e = &mut self.entries[idx];
        e.end = Some(at);
        e.fp = f;
        e.meta.seq_close = seq_close;
        e.meta.aux = aux;
        e.meta.end_path = Some(path.to_vec());
        Ok(close_at)
    }

    fn run(&mut self, r: &Element, path: &[usize], pb: bool) -> Result<(), String> {
        let rpr = r.child("w:rPr");
        let flag = |n: &str| rpr.and_then(|x| x.child(n));
        let marks = Marks::NONE
            .with(Marks::BOLD, on(flag("w:b")))
            .with(Marks::ITALIC, on(flag("w:i")))
            .with(Marks::STRIKE, on(flag("w:strike")))
            .with(Marks::UNDERLINE, underline_on(flag("w:u")));
        let shell = r.shell();
        let mut rest = rpr.cloned();
        if let Some(x) = &mut rest {
            for n in format::SHOWN_RPR {
                remove_child(x, n);
            }
        }
        let trivial = shell.attrs.is_empty()
            && rest
                .as_ref()
                .is_none_or(|x| x.children.is_empty() && x.attrs.is_empty());
        let f = if trivial {
            String::new()
        } else {
            self.fp(&[Some(&shell), rest.as_ref()])
        };
        let xml = std::iter::once(self.frag(&shell))
            .chain(rpr.map(|x| self.frag(x)))
            .collect();
        let start = self.pos();
        let idx = self.entry(
            Kind::Run,
            xml,
            f,
            path,
            Some(start),
            None,
            Meta {
                marks,
                ..Default::default()
            },
        );
        let text_props = rpr
            .map(|x| {
                format::rpr_props(x, &self.fmt.theme)
                    .only(&Key::TEXT)
                    .diff(&self.text_base)
            })
            .unwrap_or_default();
        let run_id = self.entries[idx].id;
        let mut aux = vec![];
        for n in &r.children {
            let c = match n {
                Node::El(c) => c,
                n if is_blank(n) => continue,
                _ => return Err("text or a comment directly inside <w:r>".into()),
            };
            match c.name.as_str() {
                "w:rPr" => {}
                "w:t" if text_modellable(c) => {
                    aux.push(c.to_xml());
                    for ch in c.text_of(&["w:t"]).chars() {
                        self.push(Atom::Char(ch), marks);
                    }
                }
                "w:tab" if c.attrs.is_empty() && c.children.is_empty() => {
                    self.push(Atom::Char('\t'), marks)
                }
                "w:br" if c.attrs.is_empty() && c.children.is_empty() => {
                    self.push(Atom::Break, marks)
                }
                "w:br" if pb && c.children.is_empty() && is_page_br(c) => {
                    self.push(Atom::PageBreak, marks)
                }
                n if RUN_MARKERS.contains(&n) => {
                    let (x, f) = (self.frag(c), self.fp(&[Some(c)]));
                    let at = self.pos();
                    self.entry(
                        Kind::Rmarker,
                        vec![x],
                        f,
                        path,
                        Some(at),
                        Some(at),
                        Meta {
                            run: Some(run_id),
                            tag: n.into(),
                            ..Default::default()
                        },
                    );
                }
                _ => {
                    let pos = self.pos();
                    let keep = self.keep_entry(Kind::Keep, &[c], path, Some(pos), Some(run_id));
                    self.push(Atom::Keep(keep), marks);
                }
            }
        }
        let end = self.pos();
        if !text_props.is_empty() {
            for u in &mut self.buf[start..end] {
                if u.takes_props() {
                    u.props = text_props.clone();
                }
            }
        }
        let e = &mut self.entries[idx];
        e.end = Some(end);
        e.meta.aux = aux;
        Ok(())
    }
}

/// A shown cell-level content control: `tr > sdt > sdtContent > tc` (one
/// cell). Several cells, or inner content the table model cannot hold,
/// stay a table placeholder (DESIGN.md §5.1).
fn cell_sdt(sdt: &Element) -> Option<&Element> {
    if !sdt.is("w:sdt") || !sdt_shown(sdt) {
        return None;
    }
    let c = sdt.child("w:sdtContent")?;
    let mut tc = None;
    for n in &c.children {
        match n {
            Node::El(e) if e.is("w:tc") => {
                if tc.is_some() {
                    return None;
                }
                tc = Some(e);
            }
            Node::El(e) if MARKERS.contains(&e.name.as_str()) => {}
            n if is_blank(n) => {}
            _ => return None,
        }
    }
    tc
}

/// Direct cells of a row, each with the content control that wraps it.
fn row_cells(tr: &Element) -> Vec<(&Element, Option<&Element>)> {
    let mut out = vec![];
    for e in tr.elements() {
        if e.is("w:tc") {
            out.push((e, None));
        } else if let Some(tc) = cell_sdt(e) {
            out.push((tc, Some(e)));
        }
    }
    out
}

/// Why a `w:tr` cannot be a pipe-table row: anything but its properties,
/// its cells (a shown cell-level `w:sdt` counts as its inner cell), and
/// zero-width markers after the properties.
fn row_structure_reason(tr: &Element) -> Option<String> {
    let mut past_head = false;
    for ch in &tr.children {
        match ch {
            Node::El(x) if x.is("w:tblPrEx") || x.is("w:trPr") => {
                if past_head {
                    return Some(format!(
                        "row-level {} after a row, cell or marker",
                        x.local()
                    ));
                }
            }
            Node::El(x)
                if x.is("w:tc") || MARKERS.contains(&x.name.as_str()) || cell_sdt(x).is_some() =>
            {
                past_head = true;
            }
            Node::El(x) => return Some(format!("row-level {}", x.local())),
            n if is_blank(n) => {}
            _ => return Some("text or comment in a table row".into()),
        }
    }
    None
}

/// Why a `w:tbl` or `w:tr` cannot be a pipe table: anything but its
/// properties (`head`, first and in that order), its rows or cells (`item`),
/// and zero-width markers after the properties.
fn structure_reason(e: &Element, head: &[&str], item: &str, level: &str) -> Option<String> {
    let mut past_head = false;
    for ch in &e.children {
        match ch {
            Node::El(x) if head.contains(&x.name.as_str()) => {
                if past_head {
                    return Some(format!(
                        "{level}-level {} after a row, cell or marker",
                        x.local()
                    ));
                }
            }
            Node::El(x) if x.is(item) || MARKERS.contains(&x.name.as_str()) => past_head = true,
            Node::El(x) => return Some(format!("{level}-level {}", x.local())),
            n if is_blank(n) => {}
            _ => {
                return Some(format!(
                    "text or comment in a table{}",
                    if level == "row" { " row" } else { "" }
                ))
            }
        }
    }
    None
}

/// A `w:t` whose text the model can hold: no control characters (a tab or
/// newline inside `w:t` would come back as `w:tab` / `w:br`).
fn text_modellable(t: &Element) -> bool {
    !t.text_of(&["w:t"]).chars().any(|c| (c as u32) < 0x20) && t.elements().next().is_none()
}

fn is_page_br(c: &Element) -> bool {
    c.attrs.len() == 1 && c.get("w:type").as_deref() == Some("page")
}

/// A default-style paragraph holding only a page break: `<pagebreak/>`.
fn is_page_break_para(p: &Element) -> bool {
    let mut brs = 0;
    for n in &p.children {
        let e = match n {
            Node::El(e) => e,
            n if is_blank(n) => continue,
            _ => return false,
        };
        if e.is("w:pPr") || MARKERS.contains(&e.name.as_str()) {
            continue;
        }
        if !e.is("w:r") {
            return false;
        }
        for c in e.elements() {
            match c.name.as_str() {
                "w:rPr" => {}
                "w:br" if is_page_br(c) && c.children.is_empty() => brs += 1,
                n if RUN_MARKERS.contains(&n) => {}
                _ => return false,
            }
        }
    }
    brs == 1
}

/// A covered cell's paragraph: no text and nothing shown as a placeholder.
fn para_is_blank(p: &Element) -> bool {
    let mut ok = true;
    for e in p.elements() {
        if e.is("w:pPr") || MARKERS.contains(&e.name.as_str()) {
            continue;
        }
        if !e.is("w:r") {
            ok = false;
            continue;
        }
        for c in e.elements() {
            let fine = c.is("w:rPr")
                || RUN_MARKERS.contains(&c.name.as_str())
                || (c.is("w:t") && c.text_of(&["w:t"]).is_empty() && text_modellable(c));
            ok &= fine;
        }
    }
    ok
}

/// Text a pipe table cell can hold as it is.
fn cell_writable(i: &Inline) -> bool {
    !i.units.iter().any(|u| matches!(u.atom, Atom::PageBreak))
}

/// A spanning field's begin and/or end in one paragraph of the range.
#[derive(Clone, Copy, Debug)]
struct ParaSpan {
    close: Option<usize>,
    open: Option<(usize, usize)>,
}

fn content_kids(container: &Element) -> Result<Vec<&Element>, String> {
    let mut kids = vec![];
    for n in &container.children {
        match n {
            Node::El(e) if e.is("w:pPr") || e.is("w:smartTagPr") || e.is("w:customXmlPr") => {}
            Node::El(e) => kids.push(e),
            n if is_blank(n) => {}
            Node::Comment(_) | Node::Pi(_) => {
                return Err("an XML comment inside a paragraph is not supported yet".into())
            }
            _ => return Err(format!("text directly inside <{}>", container.name)),
        }
    }
    Ok(kids)
}

fn fld_type(e: &Element, t: &str) -> bool {
    e.is("w:fldChar") && e.get("w:fldCharType").as_deref() == Some(t)
}

fn run_content(r: &Element) -> Vec<&Element> {
    r.elements().filter(|c| !c.is("w:rPr")).collect()
}

fn is_begin_run(k: &Element) -> bool {
    k.is("w:r") && run_content(k).last().is_some_and(|c| fld_type(c, "begin"))
}

fn is_sep_run(k: &Element) -> bool {
    k.is("w:r")
        && run_content(k)
            .last()
            .is_some_and(|c| fld_type(c, "separate"))
}

fn is_end_run(k: &Element) -> bool {
    let c = run_content(k);
    k.is("w:r") && c.len() == 1 && fld_type(c[0], "end")
}

/// Complex fields whose `begin` and `end` sit in different paragraphs of
/// `paras`, and whose cached result can show as text: begin and `separate`
/// are sibling runs of the first paragraph, `end` is a sibling run of the
/// last, and the result (including nested single-paragraph fields) is not
/// blank. Nested fields in the result are walked as they are today.
fn spanning_fields(paras: &[&Element]) -> Vec<(usize, usize, usize, usize, usize)> {
    let mut out = vec![];
    let mut depth = 0i32;
    let mut open: Option<(usize, usize, Option<usize>)> = None;
    for (pi, p) in paras.iter().enumerate() {
        let Ok(kids) = content_kids(p) else {
            continue;
        };
        for (ki, k) in kids.iter().enumerate() {
            for f in k.descendants("w:fldChar") {
                match f.get("w:fldCharType").as_deref() {
                    Some("begin") => {
                        if depth == 0 {
                            open = Some((pi, ki, None));
                        }
                        depth += 1;
                    }
                    Some("separate") if depth == 1 => {
                        if let Some(o) = &mut open {
                            o.2 = Some(ki);
                        }
                    }
                    Some("end") => {
                        depth -= 1;
                        if depth == 0 {
                            if let Some((first, begin, Some(sep))) = open.take() {
                                if first != pi && shown_spanning(paras, first, pi, begin, sep, ki) {
                                    out.push((first, pi, begin, sep, ki));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    out
}

fn shown_spanning(
    paras: &[&Element],
    first: usize,
    last: usize,
    begin: usize,
    sep: usize,
    end: usize,
) -> bool {
    let Ok(first_kids) = content_kids(paras[first]) else {
        return false;
    };
    let Ok(last_kids) = content_kids(paras[last]) else {
        return false;
    };
    if begin >= first_kids.len() || sep >= first_kids.len() || begin > sep || end >= last_kids.len()
    {
        return false;
    }
    if !is_begin_run(first_kids[begin])
        || !is_sep_run(first_kids[sep])
        || !is_end_run(last_kids[end])
    {
        return false;
    }
    if !first_kids[begin..=sep]
        .iter()
        .all(|k| k.is("w:r") || MARKERS.contains(&k.name.as_str()))
    {
        return false;
    }
    let mut text = String::new();
    for k in &first_kids[sep + 1..] {
        text.push_str(&k.text_of(&["w:t"]));
    }
    for p in &paras[first + 1..last] {
        text.push_str(&p.text_of(&["w:t"]));
    }
    for k in &last_kids[..end] {
        text.push_str(&k.text_of(&["w:t"]));
    }
    text.chars().any(|c| !c.is_whitespace())
}

fn span_roles(paras: &[&Element]) -> HashMap<usize, ParaSpan> {
    let mut m = HashMap::new();
    for (first, last, begin, sep, end) in spanning_fields(paras) {
        m.entry(first)
            .or_insert(ParaSpan {
                close: None,
                open: None,
            })
            .open = Some((begin, sep));
        m.entry(last)
            .or_insert(ParaSpan {
                close: None,
                open: None,
            })
            .close = Some(end);
    }
    m
}

/// A sibling that can sit in a complex field's `begin`..`end`: a run, a
/// bookmark, a hyperlink (or other wrapper), or a simple field. A TOC
/// entry is often a `w:hyperlink` wrapping a nested `PAGEREF`.
fn field_sibling(k: &Element) -> bool {
    k.is("w:r")
        || MARKERS.contains(&k.name.as_str())
        || WRAPPERS.contains(&k.name.as_str())
        || k.is("w:fldSimple")
}

/// Complex fields whose begin..end lie among these siblings: start → end.
fn field_groups(kids: &[&Element]) -> HashMap<usize, usize> {
    let mut groups = HashMap::new();
    let mut i = 0;
    while i < kids.len() {
        let begins = kids[i].is("w:r")
            && kids[i]
                .elements()
                .any(|f| f.is("w:fldChar") && f.get("w:fldCharType").as_deref() == Some("begin"));
        if begins {
            let mut depth: i32 = 0;
            let mut j = i;
            while j < kids.len() {
                for f in kids[j].descendants("w:fldChar") {
                    match f.get("w:fldCharType").as_deref() {
                        Some("begin") => depth += 1,
                        Some("end") => depth -= 1,
                        _ => {}
                    }
                }
                if depth <= 0 {
                    break;
                }
                j += 1;
            }
            if depth == 0 && j < kids.len() && kids[i..=j].iter().copied().all(field_sibling) {
                groups.insert(i, j);
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    groups
}

/// The `separate` of the complex field `kids[begin..=end]` (a group of
/// [`field_groups`]) when its cached result can show as text: the index of
/// the run it ends. A legacy form field (`w:ffData`) is shown the same way
/// when that result is not blank: `ffData` and the instruction stay in the
/// wrap. Nested single-paragraph fields and hyperlinks in the result (a
/// one-entry TOC's `PAGEREF`) are walked as they are in a spanning field.
/// Otherwise the field stays a placeholder: no result or a blank one
/// (an index entry; an empty FORMTEXT whose value is padding spaces that
/// would hide the field; a FORMCHECKBOX, which has no `separate`), or
/// parts that share a run with the result. Bookmarks Word puts between
/// the field's parts are allowed (they stay in the wrap).
fn shown_field(kids: &[&Element], begin: usize, end: usize) -> Option<usize> {
    let fld =
        |e: &Element, t: &str| e.is("w:fldChar") && e.get("w:fldCharType").as_deref() == Some(t);
    fn content(r: &Element) -> Vec<&Element> {
        r.elements().filter(|c| !c.is("w:rPr")).collect()
    }
    let mut depth = 0;
    let mut sep = None;
    for (j, k) in kids.iter().enumerate().take(end).skip(begin) {
        if MARKERS.contains(&k.name.as_str()) {
            continue;
        }
        if !k.is("w:r") {
            return None;
        }
        for f in k.descendants("w:fldChar") {
            match f.get("w:fldCharType").as_deref() {
                Some("begin") => depth += 1,
                Some("end") => depth -= 1,
                Some("separate") if depth == 1 => sep = Some(j),
                _ => {}
            }
        }
        if sep.is_some() {
            break;
        }
    }
    let sep = sep?;
    if !content(kids[sep])
        .last()
        .is_some_and(|c| fld(c, "separate"))
    {
        return None;
    }
    let close = content(kids[end]);
    if close.len() != 1 || !fld(close[0], "end") {
        return None;
    }
    let result = &kids[sep + 1..end];
    if !result.iter().copied().all(field_sibling) {
        return None;
    }
    let text: String = result.iter().map(|k| k.text_of(&["w:t"])).collect();
    text.chars().any(|c| !c.is_whitespace()).then_some(sep)
}

/// The index of a `w:bookmarkEnd` immediately after the field's `end` run
/// when it pairs with a `w:bookmarkStart` among the field's parts (Word's
/// form-field bookmark). Otherwise `end`.
fn field_close(kids: &[&Element], begin: usize, end: usize) -> usize {
    let Some(after) = kids.get(end + 1) else {
        return end;
    };
    if !after.is("w:bookmarkEnd") {
        return end;
    }
    let Some(id) = after.get("w:id") else {
        return end;
    };
    let paired = kids[begin..=end]
        .iter()
        .any(|k| k.is("w:bookmarkStart") && k.get("w:id").as_deref() == Some(id.as_str()));
    if paired {
        end + 1
    } else {
        end
    }
}

/// A `w:fldSimple` whose result shows as text: runs and markers only, with
/// text that is not blank (as [`shown_field`]).
fn shown_simple_field(f: &Element) -> bool {
    let kids: Vec<&Element> = f.elements().collect();
    kids.iter().all(|k| {
        (k.is("w:r") || MARKERS.contains(&k.name.as_str())) && k.descendants("w:fldChar").is_empty()
    }) && f.text_of(&["w:t"]).chars().any(|c| !c.is_whitespace())
}

/// The shown result of a complex field: text between `separate` and `end`.
pub fn field_result(els: &[&Element]) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for e in els {
        e.walk(&mut |x| {
            if x.is("w:fldChar") {
                match x.get("w:fldCharType").as_deref() {
                    Some("separate") => depth = 1,
                    Some("begin") | Some("end") => depth = 0,
                    _ => {}
                }
            } else if x.is("w:t") && depth > 0 {
                out.push_str(&x.text_of(&["w:t"]));
            }
        });
    }
    out
}

/// An item of the body's expanded walk: shown block content controls are
/// opened and closed around their `sdtContent` children, so inner
/// paragraphs participate in spanning-field detection.
enum WalkItem<'a> {
    OpenSdt(&'a Element),
    CloseSdt,
    El(&'a Element),
}

struct OpenSdt<'a> {
    el: &'a Element,
    first: Option<usize>,
    leading: Vec<&'a Element>,
}

/// A content control whose inner text the model can edit: rich or plain
/// text, a combo/dropdown's displayed value, a citation, a bibliography,
/// a `docPartObj` (TOC, cover page, …). Date, checkbox, picture, repeating
/// section, data-bound (`w:dataBinding`) and placeholder (`showingPlcHdr`)
/// controls stay `<keep/>`. A parent that contains any of those nested is
/// kept whole, so unwrapping does not explode the keep count.
fn sdt_shown(sdt: &Element) -> bool {
    sdt_pr_shown(sdt) && nested_sdts_shown(sdt)
}

fn sdt_pr_shown(sdt: &Element) -> bool {
    let Some(pr) = sdt.child("w:sdtPr") else {
        return true;
    };
    if pr.child("w:dataBinding").is_some() || pr.child("w:showingPlcHdr").is_some() {
        return false;
    }
    !pr.elements().any(|c| {
        matches!(
            c.local(),
            "date"
                | "checkbox"
                | "picture"
                | "repeatingSection"
                | "repeatingSectionItem"
                | "equation"
                | "group"
        )
    })
}

fn nested_sdts_shown(sdt: &Element) -> bool {
    let Some(content) = sdt.child("w:sdtContent") else {
        return true;
    };
    let mut ok = true;
    content.walk(&mut |e| {
        if e.is("w:sdt") && !sdt_pr_shown(e) {
            ok = false;
        }
    });
    ok
}

fn sdt_content_is_block(sdt: &Element) -> bool {
    sdt.child("w:sdtContent").is_some_and(|c| {
        c.elements()
            .any(|e| e.is("w:p") || e.is("w:tbl") || e.is("w:sdt"))
    })
}

fn expand_items<'a>(
    nodes: impl IntoIterator<Item = &'a Node>,
) -> Result<Vec<WalkItem<'a>>, String> {
    let mut out = vec![];
    expand_nodes(nodes, &mut out)?;
    Ok(out)
}

fn expand_nodes<'a>(
    nodes: impl IntoIterator<Item = &'a Node>,
    out: &mut Vec<WalkItem<'a>>,
) -> Result<(), String> {
    for n in nodes {
        match n {
            Node::El(e) if e.is("w:sdt") && sdt_shown(e) => expand_sdt(e, out)?,
            Node::El(e) => out.push(WalkItem::El(e)),
            n if is_blank(n) => {}
            Node::Comment(_) | Node::Pi(_) => {
                return Err(
                    "an XML comment or processing instruction between blocks is not supported yet"
                        .into(),
                )
            }
            _ => return Err("text directly inside a block container".into()),
        }
    }
    Ok(())
}

fn expand_sdt<'a>(sdt: &'a Element, out: &mut Vec<WalkItem<'a>>) -> Result<(), String> {
    let Some(content) = sdt.child("w:sdtContent") else {
        out.push(WalkItem::El(sdt));
        return Ok(());
    };
    out.push(WalkItem::OpenSdt(sdt));
    expand_nodes(&content.children, out)?;
    out.push(WalkItem::CloseSdt);
    Ok(())
}

/// The `w:sdt` shell of a cell-level control: properties and an empty
/// `sdtContent` (export puts the cell back).
fn sdt_cell_shell(sdt: &Element) -> Element {
    let mut shell = sdt.shell();
    let mut content = None;
    for n in &sdt.children {
        let Node::El(e) = n else { continue };
        if e.is("w:sdtPr") || e.is("w:sdtEndPr") {
            shell.children.push(Node::El(e.clone()));
        } else if e.is("w:sdtContent") {
            content = Some(e.shell());
        }
    }
    shell.children.push(Node::El(
        content.unwrap_or_else(|| Element::new("w:sdtContent")),
    ));
    shell
}

/// The `w:sdt` shell: properties and an `sdtContent` holding only leading
/// markers that sit before the first inner block.
fn sdt_block_shell(sdt: &Element, leading: &[&Element]) -> Element {
    let mut shell = sdt.shell();
    let mut content = None;
    for n in &sdt.children {
        let Node::El(e) = n else { continue };
        if e.is("w:sdtPr") || e.is("w:sdtEndPr") {
            shell.children.push(Node::El(e.clone()));
        } else if e.is("w:sdtContent") {
            content = Some(e.shell());
        }
    }
    let mut c = content.unwrap_or_else(|| Element::new("w:sdtContent"));
    for m in leading {
        c.children.push(Node::El((*m).clone()));
    }
    shell.children.push(Node::El(c));
    shell
}

fn sdt_inline_shell(sdt: &Element) -> Element {
    let mut shell = sdt.shell();
    let mut content = None;
    for n in &sdt.children {
        let Node::El(e) = n else { continue };
        if e.is("w:sdtPr") || e.is("w:sdtEndPr") {
            shell.children.push(Node::El(e.clone()));
        } else if e.is("w:sdtContent") {
            content = Some(e.shell());
        }
    }
    shell.children.push(Node::El(
        content.unwrap_or_else(|| Element::new("w:sdtContent")),
    ));
    shell
}
