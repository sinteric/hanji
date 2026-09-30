//! `word/document.xml` → resolved model blocks + remainder entries (a port
//! of the prototype's importer; entry kinds are documented in
//! `hanji_core::remainder::Kind`).

use std::collections::HashMap;

use hanji_core::{Block, Entry, KeepIds, Kind, ListItem, Meta, Para, StyleDef, StyleSet, Table};
use hanji_format::styled;
use hanji_format::{Atom, Cell, CellPara, Inline, Keep, Key, Marks, Props, Unit};

use crate::format;

use crate::numbering::{num_pr, Numbering};
use crate::ooxml::*;
use crate::xml::{is_blank, Element, Node, Scope};
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
}

/// Import-time checkpoint, so a table that turns out not to be a pipe
/// table can be kept whole instead.
struct Mark(usize, u64, u64);

impl<'a> Importer<'a> {
    pub fn new(
        scope: &'a Scope,
        styles: StyleSet,
        default_table: Option<String>,
        notes: &'a HashMap<(String, String), String>,
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
            buf: vec![],
            stats: Stats::default(),
            inherited: vec![],
            numbering,
            list: None,
            fmt,
            values: HashMap::new(),
            text_base: Props::new(),
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
        self.entries.push(Entry { id, kind, xml, fp, path: path.to_vec(), start, end, seq, meta });
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
        let keep = Keep { id, kind: kind_name, summary: self.summary(els) };
        let xml = els.iter().map(|e| self.frag(e)).collect();
        let meta = Meta { keep: Some(keep.clone()), run, ..Default::default() };
        self.entry(kind, xml, fpv, path, pos, pos.map(|p| p + 1), meta);
        keep
    }

    // ------------------------------------------------------------ summaries

    fn note(&self, part: &str, id: Option<String>) -> String {
        id.and_then(|id| self.notes.get(&(part.to_string(), id)).cloned()).unwrap_or_default()
    }

    fn summary(&self, els: &[&Element]) -> String {
        let el = els[0];
        let text = |e: &Element, n: usize| clip(&e.text_of(&["w:t", "w:delText", "w:instrText"]), n);
        let s = match el.local() {
            "drawing" | "pict" | "AlternateContent" | "object" => {
                let mut found = None;
                el.walk(&mut |e| {
                    if found.is_none() && e.local() == "docPr" {
                        found = e.get("descr").filter(|d| !d.is_empty()).or_else(|| e.get("name"));
                    }
                });
                found.unwrap_or_else(|| if el.local() == "pict" { "vml shape".into() } else { el.local().into() })
            }
            "footnoteReference" => format!("footnote: {}", self.note("footnote", el.get("w:id"))),
            "endnoteReference" => format!("endnote: {}", self.note("endnote", el.get("w:id"))),
            "commentReference" => format!("comment: {}", self.note("comment", el.get("w:id"))),
            "ins" | "del" | "moveFrom" | "moveTo" => {
                format!("{} by {}: {}", el.local(), el.get("w:author").unwrap_or_else(|| "?".into()), text(el, 40))
            }
            "sdt" => {
                let alias = el.descendants("w:alias").first().and_then(|a| a.get("w:val"));
                format!("{}{}", alias.map(|a| a + ": ").unwrap_or_default(), text(el, 40))
            }
            "fldSimple" => {
                format!("field {} → {}", clip(&el.get("w:instr").unwrap_or_default(), 30), text(el, 25))
            }
            "r" if els.len() > 1 || !el.descendants("w:fldChar").is_empty() => {
                let instr: String = els.iter().map(|e| e.text_of(&["w:instrText"])).collect();
                let result = field_result(els);
                format!("field {} → {}", clip(&instr, 30), clip(&result, 25)).trim().to_string()
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
        let kids: Vec<&Node> = body.children.iter().filter(|n| !is_blank(n)).collect();
        let last_el = kids.iter().rposition(|n| matches!(n, Node::El(_)));
        for (idx, n) in kids.iter().enumerate() {
            let bi = blocks.len();
            let el = match n {
                Node::El(e) => e,
                Node::Comment(_) | Node::Pi(_) => {
                    return Err("an XML comment or processing instruction between blocks is not supported yet".into())
                }
                _ => return Err("text directly inside <w:body>".into()),
            };
            if el.is("w:sectPr") && Some(idx) == last_el {
                pending.push(el);
                continue;
            }
            if MARKERS.contains(&el.name.as_str()) {
                pending.push(el);
                continue;
            }
            for m in std::mem::take(&mut pending) {
                self.bmarker(m, &[bi]);
            }
            let b = match el.name.as_str() {
                "w:p" => Block::Para(self.para(el, &[bi])?),
                "w:tbl" => self.table(el, bi)?,
                _ => self.block_keep(el, bi),
            };
            if !matches!(b, Block::Para(Para { item: Some(_), .. })) {
                self.list = None;
            }
            blocks.push(b);
        }
        for m in pending {
            let (x, f) = (self.frag(m), self.fp(&[Some(m)]));
            self.entry(Kind::Tail, vec![x], f, &[], None, None, Meta::default());
        }
        Ok(Split { blocks, entries: self.entries, styles: self.styles, next_id: self.next_id, stats: self.stats })
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
            if let Some(r) = structure_reason(tr, &["w:tblPrEx", "w:trPr"], "w:tc", "row") {
                return Some(r);
            }
            if let Some(trpr) = tr.child("w:trPr") {
                if trpr.child("w:gridBefore").is_some() || trpr.child("w:gridAfter").is_some() {
                    return Some("gridBefore/gridAfter".into());
                }
            }
            for tc in tr.elements().filter(|e| e.is("w:tc")) {
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
                        .filter_map(
                            |n| if let Node::El(e) = n { (!e.is("w:p")).then(|| e.local()) } else { Some("#text") },
                        )
                        .collect();
                    kinds.sort();
                    kinds.dedup();
                    return Some(if ps.is_empty() {
                        "cell without a paragraph".into()
                    } else {
                        format!("cell holds {}", kinds.join(","))
                    });
                }
                let paras: Vec<&Element> =
                    ps.iter().filter_map(|n| if let Node::El(e) = n { Some(e) } else { None }).collect();
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
                row.push(if covered { Cell::Up } else { Cell::text(Inline::default()) });
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
                self.stats.kept_reasons.push("cell text not writable in a pipe table".into());
                Ok(self.block_keep(tbl, bi))
            }
        }
    }

    fn table_inner(&mut self, tbl: &Element, bi: usize) -> Result<Option<Table>, String> {
        let shell = tbl.shell();
        let head: Vec<&Element> = tbl.elements().filter(|e| e.is("w:tblPr") || e.is("w:tblGrid")).collect();
        // The style the table is drawn with: its own, or the file's default.
        let style_id = tbl.child("w:tblPr").and_then(|p| p.child("w:tblStyle")).and_then(|s| s.get("w:val"));
        let style_id = style_id.or_else(|| self.default_table.clone());
        let mut head_fp: Vec<Element> = head.iter().map(|e| (*e).clone()).collect();
        for h in &mut head_fp {
            if h.is("w:tblPr") {
                remove_child(h, "w:tblStyle");
            }
        }
        let fpv = fp(self.scope, &std::iter::once(Some(&shell)).chain(head_fp.iter().map(Some)).collect::<Vec<_>>());
        let xml = std::iter::once(self.frag(&shell)).chain(head.iter().map(|e| self.frag(e))).collect();
        self.entry(Kind::Tbl, xml, fpv, &[bi], None, None, Meta { style: style_id.clone(), ..Default::default() });
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
            let rhead: Vec<&Element> = tr.elements().filter(|e| e.is("w:tblPrEx") || e.is("w:trPr")).collect();
            let fpv =
                fp(self.scope, &std::iter::once(Some(&trs)).chain(rhead.iter().map(|e| Some(*e))).collect::<Vec<_>>());
            let xml = std::iter::once(self.frag(&trs)).chain(rhead.iter().map(|e| self.frag(e))).collect();
            self.entry(Kind::Tr, xml, fpv, &[bi, ri], None, None, Meta::default());
            let mut row = vec![];
            let mut brow = vec![];
            for tc in tr.elements() {
                if MARKERS.contains(&tc.name.as_str()) {
                    // A marker between cells (or after the last): before grid column `row.len()`.
                    self.bmarker(tc, &[bi, ri, row.len()]);
                    continue;
                }
                if !tc.is("w:tc") {
                    continue;
                }
                let gc = row.len();
                let tcpr = tc.child("w:tcPr");
                let tcs = tc.shell();
                let xml = std::iter::once(self.frag(&tcs)).chain(tcpr.map(|t| self.frag(t))).collect();
                let mut rest = tcpr.cloned();
                if let Some(x) = &mut rest {
                    for n in format::SHOWN_TCPR {
                        remove_child(x, n);
                    }
                }
                let f = self.fp(&[Some(&tcs), rest.as_ref()]);
                self.entry(Kind::Tc, xml, f, &[bi, ri, gc], None, None, Meta::default());
                let mut paras = vec![];
                for e in tc.elements().filter(|e| !e.is("w:tcPr")) {
                    let k = paras.len();
                    if e.is("w:p") {
                        let para = self.para(e, &[bi, ri, gc, k])?;
                        let style = (para.style != self.styles.default_paragraph).then_some(para.style);
                        paras.push(CellPara { props: para.props, style, content: para.content });
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
                    brow.push(tcpr.map(format::tcpr_props).unwrap_or_default().diff(&styled::box_default()));
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
        Ok(Some(Table { style, rows, boxes }))
    }

    // ------------------------------------------------------------ paragraphs

    fn para(&mut self, p: &Element, path: &[usize]) -> Result<Para, String> {
        let saved = self.inherited.len();
        self.inherit(p);
        let ppr = p.child("w:pPr");
        let sid = ppr.and_then(|x| x.child("w:pStyle")).and_then(|s| s.get("w:val"));
        let default_id = self.styles.default_paragraph_id().to_string();
        let sid = sid.unwrap_or_else(|| default_id.clone());
        let style = match self.styles.paragraph_name(&sid) {
            Some(n) => n.to_string(),
            None => {
                // A style id missing from styles.xml: keep it by id.
                self.styles.paragraph.push(StyleDef::new(sid.clone(), sid.clone()));
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
        let xml = std::iter::once(self.frag(&shell)).chain(ppr.map(|x| self.frag(x))).collect();
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
        let props = ppr.map(|x| format::ppr_props(x, size)).unwrap_or_default().only(&styled::PARA_OWN).diff(&base);
        self.text_base = base.only(&Key::TEXT);
        self.buf.clear();
        let page_break_only = sid == default_id && is_page_break_para(p);
        self.walk(p, path, page_break_only)?;
        self.inherited.truncate(saved);
        let mut content = Inline { units: std::mem::take(&mut self.buf), spans: vec![] };
        content.normalize();
        // A paragraph without text shows none of it, and keeps it (§5.2).
        self.entries[ppr_at].meta.unshown = !styled::shows(&content) && !props.is_empty();
        let item = item.map(|i| i.0);
        self.stats.list_items += item.is_some() as usize;
        Ok(Para { style, content, item, props })
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
        Some((ListItem { ordered, level, first }, [num.to_string(), ilvl.to_string(), how.to_string()]))
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
        let mut kids: Vec<&Element> = vec![];
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
        let groups = field_groups(&kids);
        let mut i = 0;
        while i < kids.len() {
            let k = kids[i];
            if let Some(&end) = groups.get(&i) {
                let pos = self.pos();
                let keep = self.keep_entry(Kind::Keep, &kids[i..=end], path, Some(pos), None);
                self.push(Atom::Keep(keep), Marks::NONE);
                i = end + 1;
                continue;
            }
            let name = k.name.as_str();
            if name == "w:r" {
                self.run(k, path, pb)?;
            } else if WRAPPERS.contains(&name) {
                let mut shell = k.shell();
                shell.children = k
                    .children
                    .iter()
                    .filter(|n| matches!(n, Node::El(e) if e.is("w:smartTagPr") || e.is("w:customXmlPr")))
                    .cloned()
                    .collect();
                let (x, f) = (self.frag(&shell), self.fp(&[Some(&shell)]));
                let at = self.pos();
                let idx = self.entry(Kind::Wrap, vec![x], f, path, Some(at), None, Meta::default());
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
        let trivial =
            shell.attrs.is_empty() && rest.as_ref().is_none_or(|x| x.children.is_empty() && x.attrs.is_empty());
        let f = if trivial { String::new() } else { self.fp(&[Some(&shell), rest.as_ref()]) };
        let xml = std::iter::once(self.frag(&shell)).chain(rpr.map(|x| self.frag(x))).collect();
        let start = self.pos();
        let idx = self.entry(Kind::Run, xml, f, path, Some(start), None, Meta { marks, ..Default::default() });
        let text_props = rpr
            .map(|x| format::rpr_props(x, &self.fmt.theme).only(&Key::TEXT).diff(&self.text_base))
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
                "w:tab" if c.attrs.is_empty() && c.children.is_empty() => self.push(Atom::Char('\t'), marks),
                "w:br" if c.attrs.is_empty() && c.children.is_empty() => self.push(Atom::Break, marks),
                "w:br" if pb && c.children.is_empty() && is_page_br(c) => self.push(Atom::PageBreak, marks),
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
                        Meta { run: Some(run_id), tag: n.into(), ..Default::default() },
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

/// Why a `w:tbl` or `w:tr` cannot be a pipe table: anything but its
/// properties (`head`, first and in that order), its rows or cells (`item`),
/// and zero-width markers after the properties.
fn structure_reason(e: &Element, head: &[&str], item: &str, level: &str) -> Option<String> {
    let mut past_head = false;
    for ch in &e.children {
        match ch {
            Node::El(x) if head.contains(&x.name.as_str()) => {
                if past_head {
                    return Some(format!("{level}-level {} after a row, cell or marker", x.local()));
                }
            }
            Node::El(x) if x.is(item) || MARKERS.contains(&x.name.as_str()) => past_head = true,
            Node::El(x) => return Some(format!("{level}-level {}", x.local())),
            n if is_blank(n) => {}
            _ => return Some(format!("text or comment in a table{}", if level == "row" { " row" } else { "" })),
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

/// Complex fields whose begin..end lie among these siblings: start → end.
fn field_groups(kids: &[&Element]) -> HashMap<usize, usize> {
    let mut groups = HashMap::new();
    let mut i = 0;
    while i < kids.len() {
        let begins = kids[i].is("w:r")
            && kids[i].elements().any(|f| f.is("w:fldChar") && f.get("w:fldCharType").as_deref() == Some("begin"));
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
            if depth == 0
                && j < kids.len()
                && kids[i..=j].iter().all(|x| x.is("w:r") || MARKERS.contains(&x.name.as_str()))
            {
                groups.insert(i, j);
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    groups
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
