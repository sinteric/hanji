//! `Contents/section*.xml` → resolved model blocks + remainder entries.
//! Entry kinds are those of `hanji_core::remainder::Kind`; what each OWPML
//! element becomes is listed on [`Importer`].

use std::collections::BTreeSet;

use hanji_core::{Block, Entry, KeepIds, Kind, ListDefs, ListItem, Meta, Para, StyleDef, StyleSet, Table};
use hanji_format::{Atom, Cell, CellPara, Inline, Keep, Marks, Unit};
use hanji_package::clip;
use hanji_package::xml::{canon, fp, is_blank, unescape, Element, Node, Scope};

use crate::header::{list_num, Header, Style};
use crate::owpml::*;

pub struct Split {
    pub blocks: Vec<Block>,
    pub entries: Vec<Entry>,
    pub styles: StyleSet,
    pub next_id: u64,
    pub stats: Stats,
    /// Track-change ids the sections refer to, with what each change is.
    pub tracked: Vec<(String, String)>,
}

#[derive(Default, Debug, Clone)]
pub struct Stats {
    pub tables_modelled: usize,
    pub tables_kept: usize,
    pub kept_reasons: Vec<String>,
    /// Paragraphs imported as list items.
    pub list_items: usize,
    /// Paragraphs holding two or more tables, imported as separate table blocks.
    pub side_by_side: usize,
    /// Paragraphs kept whole because they hold or lie inside a tracked change.
    pub tracked_paragraphs: usize,
}

/// Section content → blocks and entries:
///
/// - `hp:p`: a paragraph (`Ppr`: its shell and `hp:linesegarray`). A
///   `pageBreak="1"` paragraph is `<pagebreak/>` then the paragraph.
/// - `hp:run`: `Run` (its shell; bold, italic, underline and strikeout come
///   from its `hh:charPr`). Its `hp:t` text is the paragraph's text.
/// - `hp:ctrl` for columns, header, footer, page numbers, bookmarks and
///   field begin/end: a durable `Marker` in its run.
/// - Objects in a run (tables in text, pictures, shapes, equations, form
///   controls, footnotes): inline `Keep`.
/// - A paragraph holding only tables (side by side or one): one table block
///   per table; the paragraph itself is a `Tbl` entry of the first.
/// - A paragraph with a tracked change, or inside one: a block `Bkeep`.
/// - The run holding a section's `hp:secPr` goes with the section, so it
///   stays at the start of the section whatever happens to its paragraph.
pub struct Importer<'a> {
    scope: Scope,
    header: &'a Header,
    styles: StyleSet,
    entries: Vec<Entry>,
    next_id: u64,
    next_seq: u64,
    keep_ids: KeepIds,
    buf: Vec<Unit>,
    stats: Stats,
    /// Levels of the open text list (a stack of `hh:heading` levels).
    list: Option<Vec<u32>>,
    /// Tracked regions open at the end of the last paragraph (mark `Id`s).
    open_tracked: BTreeSet<String>,
    /// Group of the tracked paragraphs of the open region.
    tracked_group: u64,
    tracked_ids: BTreeSet<String>,
    next_group: u64,
}

/// Import-time checkpoint, so tables that turn out not to be pipe tables
/// can be kept whole instead.
struct Mark(usize, u64, u64);

impl<'a> Importer<'a> {
    pub fn new(header: &'a Header) -> Self {
        Importer {
            scope: Scope::new(),
            header,
            styles: header.style_set(),
            entries: vec![],
            next_id: 1,
            next_seq: 1000,
            keep_ids: KeepIds::default(),
            buf: vec![],
            stats: Stats::default(),
            list: None,
            open_tracked: BTreeSet::new(),
            tracked_group: 0,
            tracked_ids: BTreeSet::new(),
            next_group: 1,
        }
    }

    pub fn finish(mut self, blocks: Vec<Block>) -> Split {
        // Each tracked paragraph also records how many paragraphs its change has.
        let is_tracked =
            |e: &Entry| e.kind == Kind::Bkeep && e.meta.keep.as_ref().is_some_and(|k| k.kind == "tracked-change");
        let mut sizes: std::collections::HashMap<String, usize> = Default::default();
        for e in self.entries.iter().filter(|e| is_tracked(e)) {
            *sizes.entry(e.meta.aux[0].clone()).or_default() += 1;
        }
        for e in self.entries.iter_mut().filter(|e| is_tracked(e)) {
            let n = sizes[&e.meta.aux[0]];
            e.meta.aux.push(n.to_string());
        }
        let tracked = self
            .tracked_ids
            .iter()
            .map(|id| {
                (id.clone(), self.header.tracked.get(id).cloned().unwrap_or_else(|| format!("tracked change {id}")))
            })
            .collect();
        Split { blocks, entries: self.entries, styles: self.styles, next_id: self.next_id, stats: self.stats, tracked }
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
    }

    fn seq(&mut self) -> u64 {
        let s = self.next_seq;
        self.next_seq += 1000;
        s
    }

    fn fp(&self, els: &[Option<&Element>]) -> String {
        fp(&self.scope, els)
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
        self.entries.push(Entry { id, kind, xml, fp, path: path.to_vec(), start, end, seq, meta });
        self.entries.len() - 1
    }

    fn keep_entry(
        &mut self,
        kind: Kind,
        el: &Element,
        path: &[usize],
        pos: Option<usize>,
        run: Option<u64>,
        kind_name: &str,
    ) -> Keep {
        let fpv = self.fp(&[Some(el)]);
        let id = self.keep_ids.next(&fpv);
        let keep = Keep { id, kind: kind_name.to_string(), summary: self.summary(el) };
        let meta = Meta { keep: Some(keep.clone()), run, ..Default::default() };
        self.entry(kind, vec![el.to_xml()], fpv, path, pos, pos.map(|p| p + 1), meta);
        keep
    }

    fn summary(&self, el: &Element) -> String {
        let text = |e: &Element, n: usize| clip(&e.text_of(&["hp:t"]), n);
        let described = |e: &Element| {
            let mut found = None;
            e.walk(&mut |x| {
                if found.is_none() && x.is("hp:shapeComment") {
                    found = Some(clip(&x.text_of(&["hp:shapeComment"]), 40)).filter(|s| !s.is_empty());
                }
            });
            found
        };
        let s = match el.local() {
            "tbl" => {
                let (r, c) = (el.get("rowCnt").unwrap_or_default(), el.get("colCnt").unwrap_or_default());
                format!("table {r}×{c}: {}", text(el, 40))
            }
            "pic" => described(el).unwrap_or_else(|| "picture".into()),
            "equation" => format!("equation: {}", clip(&el.text_of(&["hp:script"]), 40)),
            "ctrl" => match ctrl_kind(el) {
                Some(k) if k.is("hp:footNote") => format!("footnote: {}", text(k, 40)),
                Some(k) if k.is("hp:endNote") => format!("endnote: {}", text(k, 40)),
                Some(k) => k.local().to_string(),
                None => "control".into(),
            },
            "p" => {
                let mut first = None;
                tracked_ids(el, &mut |id| {
                    first.get_or_insert(id);
                });
                let what = first.and_then(|id| self.header.tracked.get(&id).cloned());
                let what = what.unwrap_or_else(|| {
                    if has_tracked_change(el) {
                        "tracked change".into()
                    } else {
                        "paragraph".into()
                    }
                });
                format!("{what}: {}", text(el, 40))
            }
            local => {
                let t = described(el).unwrap_or_else(|| text(el, 40));
                if t.is_empty() {
                    local.to_string()
                } else {
                    format!("{local}: {t}")
                }
            }
        };
        clip(&s, 80)
    }

    // ------------------------------------------------------------ sections

    /// One section's paragraphs into `blocks`. Returns the run holding its
    /// `hp:secPr`, taken out of its first paragraph, and whether it was the
    /// start of a longer run.
    pub fn section(
        &mut self,
        k: usize,
        sec: &Element,
        blocks: &mut Vec<Block>,
    ) -> Result<Option<(Element, bool)>, String> {
        self.scope = hanji_package::xml::scope_of(sec);
        if k > 0 {
            // The section starts before its first block, wherever that goes.
            let meta = Meta { tag: "hwpx:section".into(), aux: vec![k.to_string()], ..Default::default() };
            let f = format!("section {k}|{}", self.fp(&[Some(&sec.shell())]));
            self.entry(Kind::Bmarker, vec![], f, &[blocks.len()], None, None, meta);
        }
        let mut start = None;
        let mut first = true;
        for n in &sec.children {
            let p = match n {
                Node::El(p) if p.is("hp:p") => p,
                Node::El(p) => return Err(format!("<{}> directly inside <hs:sec> is not supported yet", p.name)),
                n if is_blank(n) => continue,
                _ => {
                    return Err(
                        "text, a comment or a processing instruction inside <hs:sec> is not supported yet".into()
                    )
                }
            };
            check_namespaces(p)?;
            if std::mem::take(&mut first) {
                if let Some((run, merged, rest)) = split_section_start(p) {
                    start = Some((run, merged));
                    self.top_para(&rest, blocks)?;
                    continue;
                }
            }
            self.top_para(p, blocks)?;
        }
        if first {
            return Err(format!("section {k} has no paragraph"));
        }
        Ok(start)
    }

    fn top_para(&mut self, p: &Element, blocks: &mut Vec<Block>) -> Result<(), String> {
        if p.get("pageBreak").as_deref() == Some("1") {
            let content = Inline { units: vec![Unit { atom: Atom::PageBreak, marks: Marks::NONE }], spans: vec![] };
            blocks.push(Block::Para(Para { style: self.styles.default_paragraph.clone(), content, item: None }));
            self.list = None;
        }
        let bi = blocks.len();
        let tracked = has_tracked_change(p) || !self.open_tracked.is_empty();
        let b = if tracked {
            self.tracked(p, bi)
        } else if !p.descendants("hp:secPr").is_empty() {
            // Section settings that are not the section's first run: the paragraph stays whole.
            let k = self.keep_entry(Kind::Bkeep, p, &[bi], None, None, "section");
            Block::Keep(k.id)
        } else if let Some(tables) = self.tables_only(p, bi)? {
            blocks.extend(tables);
            self.list = None;
            return Ok(());
        } else {
            Block::Para(self.para(p, &[bi])?)
        };
        if !matches!(b, Block::Para(Para { item: Some(_), .. })) {
            self.list = None;
        }
        blocks.push(b);
        Ok(())
    }

    /// A paragraph that holds a tracked change or lies inside one: a block
    /// placeholder, grouped with the other paragraphs of a change that spans several.
    fn tracked(&mut self, p: &Element, bi: usize) -> Block {
        if self.open_tracked.is_empty() {
            self.tracked_group = self.next_group;
            self.next_group += 1;
        }
        p.walk(&mut |x| {
            let id = x.get("Id").unwrap_or_default();
            match x.name.as_str() {
                "hp:insertBegin" | "hp:deleteBegin" => {
                    self.open_tracked.insert(id);
                }
                "hp:insertEnd" | "hp:deleteEnd" => {
                    self.open_tracked.remove(&id);
                }
                _ => {}
            }
        });
        tracked_ids(p, &mut |t| {
            self.tracked_ids.insert(t);
        });
        self.stats.tracked_paragraphs += 1;
        let k = self.keep_entry(Kind::Bkeep, p, &[bi], None, None, "tracked-change");
        let e = self.entries.last_mut().unwrap();
        e.meta.aux = vec![self.tracked_group.to_string()];
        if !has_tracked_change(p) {
            // A paragraph that lies inside a change holds none of its marks.
            let keep = e.meta.keep.as_mut().unwrap();
            keep.summary = clip(&format!("inside a tracked change: {}", clip(&p.text_of(&["hp:t"]), 40)), 80);
        }
        Block::Keep(k.id)
    }

    // ------------------------------------------------------------ tables

    /// A paragraph that holds only tables (and zero-width controls): one
    /// table block per table, if every one is a pipe table.
    fn tables_only(&mut self, p: &Element, bi: usize) -> Result<Option<Vec<Block>>, String> {
        let mut tbls: Vec<&Element> = vec![];
        for n in &p.children {
            match n {
                Node::El(r) if r.is("hp:run") => {
                    for c in &r.children {
                        match c {
                            Node::El(t) if t.is("hp:tbl") => tbls.push(t),
                            // Spaces after a table lay the line out; they stay in the paragraph.
                            Node::El(t) if t.is("hp:t") && t.attrs.is_empty() && spaces_only(t) => {}
                            Node::El(c)
                                if c.is("hp:ctrl")
                                    && ctrl_kind(c).is_some_and(|k| MARKER_CTRLS.contains(&k.name.as_str())) => {}
                            n if is_blank(n) => {}
                            _ => return Ok(None),
                        }
                    }
                }
                Node::El(l) if l.is("hp:linesegarray") => {}
                n if is_blank(n) => {}
                _ => return Ok(None),
            }
        }
        if tbls.is_empty() || tbls.iter().any(|t| table_reason(t).is_some()) {
            return Ok(None);
        }
        let mark = self.checkpoint();
        let group = self.next_group;
        let mut out = vec![];
        for (k, t) in tbls.iter().enumerate() {
            let at = bi + k;
            match self.table(t, at)? {
                Some(table) => out.push(Block::Table(table)),
                None => {
                    self.rollback(mark);
                    return Ok(None);
                }
            }
            let (tag, xml, f) = if k == 0 {
                // The paragraph, its tables left as empty `hp:tbl` slots.
                let mut anchor = p.clone();
                for r in anchor.elements_mut().filter(|r| r.is("hp:run")) {
                    for c in r.elements_mut().filter(|c| c.is("hp:tbl")) {
                        *c = Element::new("hp:tbl");
                    }
                }
                let mut shown = anchor.clone();
                shown.remove_attr("pageBreak");
                let f = self.fp(&[Some(&shown)]);
                ("hwpx:anchor", vec![anchor.to_xml()], f)
            } else {
                ("hwpx:join", vec![], "hwpx:join".into())
            };
            let meta = Meta { tag: tag.into(), aux: vec![group.to_string(), k.to_string()], ..Default::default() };
            self.entry(Kind::Tbl, xml, f, &[at], None, None, meta);
        }
        self.next_group += 1;
        self.stats.tables_modelled += out.len();
        self.stats.side_by_side += (out.len() > 1) as usize;
        Ok(Some(out))
    }

    fn table(&mut self, tbl: &Element, bi: usize) -> Result<Option<Table>, String> {
        let shell = tbl.shell();
        let head: Vec<&Element> = tbl.elements().filter(|e| !e.is("hp:tr")).collect();
        let mut fshell = shell.clone();
        fshell.remove_attr("rowCnt");
        fshell.remove_attr("colCnt");
        let f = self.fp(&std::iter::once(Some(&fshell)).chain(head.iter().map(|e| Some(*e))).collect::<Vec<_>>());
        let xml = std::iter::once(shell.to_xml()).chain(head.iter().map(|e| e.to_xml())).collect();
        let (rows_n, cols_n) = dims(tbl);
        let meta = Meta { aux: vec![rows_n.to_string(), cols_n.to_string()], ..Default::default() };
        self.entry(Kind::Tbl, xml, f, &[bi], None, None, meta);
        let mut rows: Vec<Vec<Option<Cell>>> = vec![vec![None; cols_n]; rows_n];
        for (ri, tr) in tbl.elements().filter(|e| e.is("hp:tr")).enumerate() {
            let trs = tr.shell();
            let f = self.fp(&[Some(&trs)]);
            self.entry(Kind::Tr, vec![trs.to_xml()], f, &[bi, ri], None, None, Meta::default());
            for tc in tr.elements().filter(|e| e.is("hp:tc")) {
                let (c, r, cs, rs) = cell_place(tc).expect("checked by table_reason");
                let sub = tc.child("hp:subList").expect("checked by table_reason");
                let rest: Vec<&Element> = tc.elements().filter(|e| !e.is("hp:subList")).collect();
                let sub_shell = sub.shell();
                let tcs = tc.shell();
                let xml =
                    [tcs.to_xml(), sub_shell.to_xml()].into_iter().chain(rest.iter().map(|e| e.to_xml())).collect();
                let kept: Vec<Option<&Element>> = [Some(&tcs), Some(&sub_shell)]
                    .into_iter()
                    .chain(rest.iter().filter(|e| !e.is("hp:cellAddr") && !e.is("hp:cellSpan")).map(|e| Some(*e)))
                    .collect();
                let f = self.fp(&kept);
                self.entry(Kind::Tc, xml, f, &[bi, r, c], None, None, Meta::default());
                let mut paras = vec![];
                for p in sub.elements() {
                    let k = paras.len();
                    let para = self.para(p, &[bi, r, c, k])?;
                    let style = (para.style != self.styles.default_paragraph).then_some(para.style);
                    paras.push(CellPara { style, content: para.content });
                }
                rows[r][c] = Some(Cell::Text(paras));
                for (y, row) in rows.iter_mut().enumerate().skip(r).take(rs) {
                    for (x, slot) in row.iter_mut().enumerate().skip(c).take(cs) {
                        if (y, x) != (r, c) {
                            *slot = Some(if x == c { Cell::Up } else { Cell::Left });
                        }
                    }
                }
            }
        }
        let rows: Vec<Vec<Cell>> =
            rows.into_iter().map(|r| r.into_iter().map(|c| c.expect("covered")).collect()).collect();
        Ok(Some(Table { style: None, rows }))
    }

    // ------------------------------------------------------------ paragraphs

    fn para(&mut self, p: &Element, path: &[usize]) -> Result<Para, String> {
        let n = |k: &str| p.get(k).and_then(|v| v.parse::<u32>().ok());
        let sid = n("styleIDRef").unwrap_or(0);
        let style = match self.header.style(sid) {
            Some(s) => s.clone(),
            None => {
                // A style id missing from header.xml: keep it by id.
                if self.styles.paragraph_name(&sid.to_string()).is_none() {
                    self.styles.paragraph.push(StyleDef { id: sid.to_string(), name: sid.to_string() });
                }
                Style { id: sid, name: sid.to_string(), para_pr: n("paraPrIDRef").unwrap_or(0), char_pr: 0 }
            }
        };
        let ppr = n("paraPrIDRef").unwrap_or(style.para_pr);
        let shell = p.shell();
        let lines = p.child("hp:linesegarray");
        let top = path.len() == 1;
        // What the text does not show: the paragraph shape when it is not the
        // style's, and attributes other than the ones the text states.
        let mut rest = shell.clone();
        for a in ["id", "styleIDRef", "paraPrIDRef"] {
            rest.remove_attr(a);
        }
        if top {
            rest.remove_attr("pageBreak");
        }
        rest.attrs.retain(|a| a.1 != "0");
        let f = if ppr == style.para_pr && rest.attrs.is_empty() {
            String::new()
        } else {
            format!("{}|{}", canon(&rest, &self.scope), self.header.para_rest(ppr))
        };
        let xml = std::iter::once(shell.to_xml()).chain(lines.map(|x| x.to_xml())).collect();
        // A list paragraph at the top level is a list item; a heading keeps
        // its numbering in the remainder, and so does a cell paragraph.
        let is_heading = self.styles.heading_level(&style.name).is_some();
        let item = (top && !is_heading).then(|| self.list_item(ppr)).flatten();
        let aux = match item {
            Some((_, num, lvl)) => vec![num.to_string(), lvl.to_string()],
            None => vec![String::new(), String::new()],
        };
        let meta = Meta { style: Some(sid.to_string()), item: item.map(|i| i.0), aux, ..Default::default() };
        let pidx = self.entry(Kind::Ppr, xml, f, path, None, None, meta);
        self.buf.clear();
        for c in &p.children {
            match c {
                Node::El(e) if e.is("hp:run") => self.run(e, path, &style)?,
                Node::El(e) if e.is("hp:linesegarray") => {}
                Node::El(e) => {
                    let pos = self.buf.len();
                    let keep = self.keep_entry(Kind::Keep, e, path, Some(pos), None, &keep_kind(&e.name));
                    self.buf.push(Unit { atom: Atom::Keep(keep), marks: Marks::NONE });
                }
                n if is_blank(n) => {}
                _ => return Err("text or a comment directly inside <hp:p> is not supported yet".into()),
            }
        }
        let mut content = Inline { units: std::mem::take(&mut self.buf), spans: vec![] };
        content.normalize();
        // The layout cache, and where it sits among the paragraph's children.
        let lines_at = p.elements().position(|e| e.is("hp:linesegarray")).unwrap_or(0);
        self.entries[pidx].meta.aux.extend([layout_key(sid, ppr, &content), lines_at.to_string()]);
        self.stats.list_items += item.is_some() as usize;
        let item = item.map(|i| i.0);
        // A list item's style is not in the text; it stays in the remainder.
        let style = if item.is_some() { String::new() } else { style.name };
        Ok(Para { style, content, item })
    }

    /// The list item a paragraph shape makes: the item, its list and level.
    fn list_item(&mut self, ppr: u32) -> Option<(ListItem, u32, u32)> {
        let (num, lvl) = list_num(self.header.heading(ppr)?)?;
        let ordered = self.header.ordered(num, lvl)?;
        let (first, level) = match &mut self.list {
            Some(stack) => {
                while stack.last().is_some_and(|&t| t >= lvl) {
                    stack.pop();
                }
                stack.push(lvl);
                (false, stack.len() - 1)
            }
            None => {
                self.list = Some(vec![lvl]);
                (true, 0)
            }
        };
        Some((ListItem { ordered, level, first }, num, lvl))
    }

    fn push(&mut self, atom: Atom, marks: Marks) {
        self.buf.push(Unit { atom, marks });
    }

    fn run(&mut self, r: &Element, path: &[usize], style: &Style) -> Result<(), String> {
        let cp = r.get("charPrIDRef").and_then(|v| v.parse::<u32>().ok()).unwrap_or(style.char_pr);
        let marks = Marks(self.header.flags(cp).0 & !self.header.flags(style.char_pr).0);
        let shell = r.shell();
        let mut rest = shell.clone();
        rest.remove_attr("charPrIDRef");
        let trivial = rest.attrs.is_empty() && self.header.char_rest(cp) == self.header.char_rest(style.char_pr);
        let f = if trivial {
            String::new()
        } else {
            format!("{}|{}", canon(&rest, &self.scope), self.header.char_rest(cp))
        };
        let start = self.buf.len();
        let run_xml = shell.to_xml();
        let idx = self.entry(
            Kind::Run,
            vec![run_xml.clone()],
            f,
            path,
            Some(start),
            None,
            Meta { marks, ..Default::default() },
        );
        let run_id = self.entries[idx].id;
        let mut tabs = vec![];
        let mut prev_t = false;
        for n in &r.children {
            let c = match n {
                Node::El(c) => c,
                n if is_blank(n) => continue,
                _ => return Err("text or a comment directly inside <hp:run> is not supported yet".into()),
            };
            let is_t = c.is("hp:t") && t_modellable(c);
            if is_t {
                // A new `hp:t` where the export would not start one by itself.
                if prev_t || !c.attrs.is_empty() || c.children.is_empty() {
                    let ts = c.shell();
                    let f = if c.attrs.is_empty() { String::new() } else { self.fp(&[Some(&ts)]) };
                    let meta = Meta { run: Some(run_id), tag: "hp:t".into(), ..Default::default() };
                    let at = self.buf.len();
                    self.entry(Kind::Rmarker, vec![ts.to_xml()], f, path, Some(at), Some(at), meta);
                }
                for n in &c.children {
                    match n {
                        Node::Text(t) => {
                            for ch in unescape(t).chars() {
                                self.push(Atom::Char(ch), marks);
                            }
                        }
                        Node::El(x) if x.is("hp:lineBreak") => self.push(Atom::Break, marks),
                        Node::El(x) => match t_char(&x.name) {
                            Some(ch) => {
                                if ch == '\t' {
                                    tabs.push(x.to_xml());
                                }
                                self.push(Atom::Char(ch), marks);
                            }
                            None => {
                                let meta = Meta {
                                    run: Some(run_id),
                                    tag: x.name.clone(),
                                    aux: vec!["in-t".into()],
                                    ..Default::default()
                                };
                                let (at, f) = (self.buf.len(), self.fp(&[Some(x)]));
                                self.entry(Kind::Rmarker, vec![x.to_xml()], f, path, Some(at), Some(at), meta);
                            }
                        },
                        _ => unreachable!("t_modellable"),
                    }
                }
            } else if let Some(k) =
                c.is("hp:ctrl").then(|| ctrl_kind(c)).flatten().filter(|k| MARKER_CTRLS.contains(&k.name.as_str()))
            {
                let meta = Meta {
                    run: Some(run_id),
                    tag: k.name.clone(),
                    durable: true,
                    opens: k.is("hp:fieldBegin"),
                    ..Default::default()
                };
                let (at, f) = (self.buf.len(), self.fp(&[Some(c)]));
                self.entry(Kind::Marker, vec![c.to_xml(), run_xml.clone()], f, path, Some(at), Some(at), meta);
            } else {
                let kind = match c.name.as_str() {
                    "hp:ctrl" => ctrl_kind(c).map_or("control".into(), |k| keep_kind(&k.name)),
                    n => keep_kind(n),
                };
                if c.is("hp:tbl") {
                    self.stats.tables_kept += 1;
                    let why = table_reason(c).unwrap_or_else(|| "shares its paragraph with text or objects".into());
                    self.stats.kept_reasons.push(why);
                }
                let pos = self.buf.len();
                let keep = self.keep_entry(Kind::Keep, c, path, Some(pos), Some(run_id), &kind);
                self.push(Atom::Keep(keep), marks);
            }
            prev_t = is_t;
        }
        let end = self.buf.len();
        let e = &mut self.entries[idx];
        e.end = Some(end);
        e.meta.aux = tabs;
        Ok(())
    }
}

/// Section content cut into fragments must not declare namespaces of its
/// own: fragments are resolved against the section root's declarations.
fn check_namespaces(p: &Element) -> Result<(), String> {
    let mut bad = None;
    p.walk(&mut |e| {
        if bad.is_none() && e.attrs.iter().any(|a| hanji_package::xml::ns_prefix(&a.0).is_some()) {
            bad = Some(e.name.clone());
        }
    });
    match bad {
        Some(n) => Err(format!("a namespace declaration on <{n}> inside a section is not supported yet")),
        None => Ok(()),
    }
}

/// The part of a section's first run that holds its `hp:secPr` (and the
/// controls around it): a run of those, whether it was only the start of a
/// run that goes on with other content, and the paragraph without it.
fn split_section_start(p: &Element) -> Option<(Element, bool, Element)> {
    let k = p.children.iter().position(|n| matches!(n, Node::El(_)))?;
    let Node::El(run) = &p.children[k] else { return None };
    if !run.is("hp:run") {
        return None;
    }
    let settings = |n: &Node| matches!(n, Node::El(e) if e.is("hp:secPr") || e.is("hp:ctrl")) || is_blank(n);
    let n = run.children.iter().take_while(|n| settings(n)).count();
    if !run.children[..n].iter().any(|n| matches!(n, Node::El(e) if e.is("hp:secPr"))) {
        return None;
    }
    let rest_empty = run.children[n..]
        .iter()
        .all(|n| is_blank(n) || matches!(n, Node::El(e) if e.is("hp:t") && e.children.is_empty()));
    let mut rest = p.clone();
    if rest_empty {
        rest.children.remove(k);
        return Some((run.clone(), false, rest));
    }
    let mut start = run.shell();
    start.children = run.children[..n].to_vec();
    if let Node::El(r) = &mut rest.children[k] {
        r.children.drain(..n);
    }
    Some((start, true, rest))
}

/// An `hp:t` whose content the model can hold: text without control
/// characters or the characters its child elements stand for, line breaks,
/// tabs, fixed and non-breaking spaces, and zero-width marks.
fn t_modellable(t: &Element) -> bool {
    t.children.iter().all(|n| match n {
        Node::Text(s) => unescape(s).chars().all(|c| c as u32 >= 0x20 && char_element(c).is_none()),
        Node::El(x) if x.is("hp:lineBreak") => x.attrs.is_empty() && x.children.is_empty(),
        Node::El(x) if t_char(&x.name).is_some() => x.children.is_empty() && (x.is("hp:tab") || x.attrs.is_empty()),
        Node::El(x) => T_MARKERS.contains(&x.name.as_str()) && x.children.is_empty(),
        _ => false,
    })
}

/// An `hp:t` of spaces, or empty.
fn spaces_only(t: &Element) -> bool {
    t.children.iter().all(|n| matches!(n, Node::Text(s) if unescape(s).chars().all(|c| c == ' ')))
}

/// `rowCnt` × `colCnt` of a table.
pub fn dims(tbl: &Element) -> (usize, usize) {
    let n = |k: &str| tbl.get(k).and_then(|v| v.parse().ok()).unwrap_or(0);
    (n("rowCnt"), n("colCnt"))
}

/// A cell's column, row, column span and row span.
pub fn cell_place(tc: &Element) -> Option<(usize, usize, usize, usize)> {
    let n = |e: &Element, k: &str| e.get(k).and_then(|v| v.parse::<usize>().ok());
    let addr = tc.child("hp:cellAddr")?;
    let span = tc.child("hp:cellSpan");
    Some((
        n(addr, "colAddr")?,
        n(addr, "rowAddr")?,
        span.and_then(|s| n(s, "colSpan")).unwrap_or(1).max(1),
        span.and_then(|s| n(s, "rowSpan")).unwrap_or(1).max(1),
    ))
}

/// Why an `hp:tbl` cannot be a pipe table.
pub fn table_reason(tbl: &Element) -> Option<String> {
    let (rows_n, cols_n) = dims(tbl);
    if rows_n == 0 || cols_n == 0 {
        return Some("empty table".into());
    }
    let mut past_rows = false;
    for n in &tbl.children {
        match n {
            Node::El(e) if e.is("hp:tr") => past_rows = true,
            Node::El(e) if past_rows => return Some(format!("table-level {} after a row", e.local())),
            Node::El(_) => {}
            n if is_blank(n) => {}
            _ => return Some("text or comment in a table".into()),
        }
    }
    let trs: Vec<&Element> = tbl.elements().filter(|e| e.is("hp:tr")).collect();
    if trs.len() != rows_n {
        return Some("row count differs from rowCnt".into());
    }
    let mut grid = vec![vec![false; cols_n]; rows_n];
    for (ri, tr) in trs.iter().enumerate() {
        for n in &tr.children {
            match n {
                Node::El(tc) if tc.is("hp:tc") => {
                    let Some((c, r, cs, rs)) = cell_place(tc) else { return Some("cell without an address".into()) };
                    if r != ri {
                        return Some("cell address outside its row".into());
                    }
                    if r + rs > rows_n || c + cs > cols_n {
                        return Some("cell outside the grid".into());
                    }
                    for row in grid.iter_mut().skip(r).take(rs) {
                        for slot in row.iter_mut().skip(c).take(cs) {
                            if std::mem::replace(slot, true) {
                                return Some("cells overlap".into());
                            }
                        }
                    }
                    let subs: Vec<&Element> = tc.elements().filter(|e| e.is("hp:subList")).collect();
                    if subs.len() != 1 {
                        return Some("cell without one subList".into());
                    }
                    if subs[0].children.iter().any(|n| !matches!(n, Node::El(p) if p.is("hp:p")) && !is_blank(n)) {
                        return Some("cell holds something other than paragraphs".into());
                    }
                    if subs[0].elements().next().is_none() {
                        return Some("cell without a paragraph".into());
                    }
                    if !subs[0].descendants("hp:tbl").is_empty() {
                        return Some("cell holds tbl".into());
                    }
                }
                n if is_blank(n) => {}
                _ => return Some("row holds something other than cells".into()),
            }
        }
    }
    if grid.iter().flatten().any(|x| !x) {
        return Some("grid slot without a cell".into());
    }
    None
}

/// What the layout cache (`hp:linesegarray`) of a paragraph was computed
/// for: its style, paragraph shape, text and marks. Export drops the cache
/// when any of them changed.
pub fn layout_key(style: u32, para_pr: u32, content: &Inline) -> String {
    let units = content.units.iter().flat_map(|u| [hanji_core::model::key(&u.atom) as u64, u.marks.0 as u64]);
    let values = [style as u64, para_pr as u64].into_iter().chain(units);
    format!("{:016x}", hanji_core::remainder::fnv1a(values.flat_map(u64::to_le_bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hanji_package::xml::fragment;

    #[test]
    fn section_start_run_is_split_off() {
        let p = fragment("<hp:p><hp:run charPrIDRef=\"0\"><hp:secPr/><hp:ctrl><hp:colPr/></hp:ctrl></hp:run><hp:run charPrIDRef=\"1\"><hp:t>a</hp:t></hp:run></hp:p>");
        let (run, merged, rest) = split_section_start(&p).unwrap();
        assert!(run.child("hp:secPr").is_some() && !merged);
        assert_eq!(rest.to_xml(), "<hp:p><hp:run charPrIDRef=\"1\"><hp:t>a</hp:t></hp:run></hp:p>");
        let p = fragment("<hp:p><hp:run><hp:secPr/><hp:ctrl><hp:colPr/></hp:ctrl><hp:tbl/><hp:t/></hp:run></hp:p>");
        let (run, merged, rest) = split_section_start(&p).unwrap();
        assert_eq!(run.to_xml(), "<hp:run><hp:secPr/><hp:ctrl><hp:colPr/></hp:ctrl></hp:run>");
        assert!(merged);
        assert_eq!(rest.to_xml(), "<hp:p><hp:run><hp:tbl/><hp:t/></hp:run></hp:p>");
        let p = fragment("<hp:p><hp:run><hp:t>a</hp:t><hp:secPr/></hp:run></hp:p>");
        assert!(split_section_start(&p).is_none());
    }

    #[test]
    fn t_with_colliding_characters_is_not_modellable() {
        assert!(t_modellable(&fragment("<hp:t>a<hp:tab width=\"4000\"/>b<hp:lineBreak/><hp:nbSpace/></hp:t>")));
        assert!(!t_modellable(&fragment("<hp:t>a\u{a0}b</hp:t>")));
        assert!(!t_modellable(&fragment("<hp:t>a<hp:insertBegin Id=\"1\"/></hp:t>")));
    }
}
