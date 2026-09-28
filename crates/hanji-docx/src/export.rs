//! Resolved model + placed entries → `word/document.xml` (a port of the
//! prototype's exporter).

use std::collections::{BTreeMap, HashMap};

use hanji_core::{Block, Entry, Kind, StyleSet};
use hanji_format::{Atom, Cell, Inline, Marks};

use crate::ooxml::*;
use crate::xml::{self, fragment, Element, Node};

pub struct Exporter<'a> {
    styles: &'a StyleSet,
    by: HashMap<(Vec<usize>, Kind), Vec<&'a Entry>>,
    keep: HashMap<&'a str, &'a Entry>,
    tail: Vec<&'a Entry>,
}

fn el(name: &str) -> Element {
    Element::new(name)
}

fn node(e: Element) -> Node {
    Node::El(e)
}

/// An element stored as its shell followed by its leading children.
fn assemble(xml: &[String]) -> Element {
    let mut e = fragment(&xml[0]);
    e.children.extend(xml[1..].iter().map(|x| node(fragment(x))));
    e
}

impl<'a> Exporter<'a> {
    pub fn new(styles: &'a StyleSet, entries: &'a [Entry]) -> Self {
        let mut ex = Exporter { styles, by: HashMap::new(), keep: HashMap::new(), tail: vec![] };
        for e in entries {
            match e.kind {
                Kind::Tail => ex.tail.push(e),
                Kind::Keep | Kind::Bkeep => {
                    ex.keep.insert(e.meta.keep.as_ref().unwrap().id.as_str(), e);
                }
                _ => ex.by.entry((e.path.clone(), e.kind)).or_default().push(e),
            }
        }
        ex
    }

    fn at(&self, path: &[usize], kind: Kind) -> Vec<&'a Entry> {
        self.by.get(&(path.to_vec(), kind)).cloned().unwrap_or_default()
    }

    fn keep_entry(&self, id: &str) -> Result<&'a Entry, String> {
        self.keep.get(id).copied().ok_or_else(|| format!("placeholder {id} has no remainder entry"))
    }

    /// The body's children.
    pub fn body(&self, blocks: &[Block]) -> Result<Vec<Node>, String> {
        let mut out = vec![];
        for (bi, b) in blocks.iter().enumerate() {
            let mut holder = el("x");
            self.bmarkers(&mut holder, &[bi]);
            out.extend(holder.children);
            match b {
                Block::Para(p) => out.push(node(self.para(&p.content, Some(&p.style), &[bi])?)),
                Block::Table(t) => out.push(node(self.table(t, bi)?)),
                Block::Keep(id) => {
                    for x in &self.keep_entry(id)?.xml {
                        out.push(node(fragment(x)));
                    }
                }
            }
        }
        let mut tail = self.tail.clone();
        tail.sort_by_key(|e| e.seq);
        for e in tail {
            out.push(node(fragment(&e.xml[0])));
        }
        Ok(out)
    }

    // ------------------------------------------------------------ paragraph

    /// `w:p` with its properties. `style: None` (a table cell) keeps the
    /// style the remainder has.
    fn ppr(&self, style: Option<&str>, path: &[usize]) -> Result<Element, String> {
        let pp = self.at(path, Kind::Ppr);
        let want_id = match style {
            Some(s) => {
                Some(self.styles.paragraph_id(s).ok_or_else(|| format!("unknown paragraph style {s:?}"))?.to_string())
            }
            None => None,
        };
        let default_id = self.styles.default_paragraph_id().to_string();
        let (mut pel, mut ppr) = match pp.first() {
            Some(e) => {
                let mut ppr = e.xml.get(1).map(|x| fragment(x));
                let old = e.meta.style.clone().unwrap_or_else(|| default_id.clone());
                if let Some(want) = want_id.filter(|w| *w != old) {
                    let mut x = ppr.take().unwrap_or_else(|| el("w:pPr"));
                    remove_child(&mut x, "w:pStyle");
                    if want != default_id {
                        x.children.insert(0, node(el("w:pStyle").with_attr("w:val", &want)));
                    }
                    ppr = Some(x);
                }
                (fragment(&e.xml[0]), ppr)
            }
            None => {
                let ppr = want_id.filter(|w| *w != default_id).map(|w| {
                    let mut x = el("w:pPr");
                    x.children.push(node(el("w:pStyle").with_attr("w:val", &w)));
                    x
                });
                (el("w:p"), ppr)
            }
        };
        if let Some(x) = ppr.take() {
            pel.children.push(node(x));
        }
        Ok(pel)
    }

    fn para(&self, p: &Inline, style: Option<&str>, path: &[usize]) -> Result<Element, String> {
        let mut pel = self.ppr(style, path)?;
        let n = p.units.len();
        let runs = self.at(path, Kind::Run);
        let mut owner: Vec<Option<&Entry>> = vec![None; n];
        let mut zruns = vec![];
        for r in &runs {
            let (s, e) = (r.start.unwrap(), r.end.unwrap());
            if s == e {
                zruns.push(*r);
            }
            for slot in owner.iter_mut().take(e.min(n)).skip(s) {
                *slot = Some(r);
            }
        }
        // Marks as exported: a unit the text cannot state a mark on (a space
        // at a mark's edge, a page break) keeps what its run had.
        let eff: Vec<Marks> = (0..n)
            .map(|c| {
                let mut m = p.units[c].marks;
                for x in Marks::ALL {
                    if !m.has(x) && !p.mark_expressible(c, x) {
                        m = m.with(x, owner[c].is_some_and(|o| o.meta.marks.has(x)));
                    }
                }
                m
            })
            .collect();
        let mut pkeep: BTreeMap<usize, &Entry> = BTreeMap::new();
        for (c, u) in p.units.iter().enumerate() {
            if let Atom::Keep(k) = &u.atom {
                let ke = self.keep_entry(&k.id)?;
                if ke.meta.run.is_none() {
                    pkeep.insert(c, ke);
                }
            }
        }
        let markers = self.at(path, Kind::Marker);
        let wraps = self.at(path, Kind::Wrap);
        let mut bounds: std::collections::HashSet<usize> = markers.iter().map(|m| m.start.unwrap()).collect();
        bounds.extend(zruns.iter().map(|z| z.start.unwrap()));
        for w in &wraps {
            bounds.insert(w.start.unwrap());
            bounds.insert(w.end.unwrap());
        }
        let key = |c: usize| (owner[c].map(|o| o.id), eff[c]);
        let mut segs: Vec<(usize, usize, Option<&Entry>)> = vec![];
        let mut c = 0;
        while c < n {
            if pkeep.contains_key(&c) {
                c += 1;
                continue;
            }
            let a = c;
            c += 1;
            while c < n && !pkeep.contains_key(&c) && !bounds.contains(&c) && key(c) == key(a) {
                c += 1;
            }
            segs.push((a, c, owner[a]));
        }
        // Run-internal markers go into the segment of their own run when it still covers them.
        let mut rm_at: HashMap<Target, Vec<&Entry>> = HashMap::new();
        for m in self.at(path, Kind::Rmarker) {
            let s = m.start.unwrap();
            let own = |seg: &(usize, usize, Option<&Entry>)| seg.2.map(|o| o.id) == m.meta.run;
            let t = segs
                .iter()
                .position(|g| own(g) && g.0 <= s && s <= g.1)
                .or_else(|| segs.iter().position(|g| g.0 <= s && s < g.1))
                .or_else(|| (segs.last().is_some_and(|g| g.1 == s)).then(|| segs.len() - 1))
                .map(Target::Seg)
                .or_else(|| zruns.iter().find(|z| Some(z.id) == m.meta.run).map(|z| Target::Zrun(z.id)));
            match t {
                Some(t) => rm_at.entry(t).or_default().push(m),
                None => return Err(format!("a run-internal marker at {path:?}:{s} has no run to live in")),
            }
        }
        #[derive(Clone, Copy)]
        enum It<'b> {
            Open(&'b Entry),
            Close(&'b Entry),
            Marker(&'b Entry),
            Zrun(&'b Entry),
            Pkeep(&'b Entry),
            Seg(usize),
        }
        let mut items: Vec<(usize, u8, u64, It)> = vec![];
        for w in &wraps {
            items.push((w.start.unwrap(), 0, w.seq, It::Open(w)));
            items.push((w.end.unwrap(), 0, w.meta.seq_close, It::Close(w)));
        }
        for m in &markers {
            items.push((m.start.unwrap(), 0, m.seq, It::Marker(m)));
        }
        for z in &zruns {
            items.push((z.start.unwrap(), 0, z.seq, It::Zrun(z)));
        }
        for (c, k) in &pkeep {
            items.push((*c, 1, k.seq, It::Pkeep(k)));
        }
        for (si, s) in segs.iter().enumerate() {
            items.push((s.0, 1, s.2.map_or(u64::MAX, |o| o.seq), It::Seg(si)));
        }
        items.sort_by_key(|t| (t.0, t.1, t.2));
        let mut stack: Vec<(Option<u64>, Element)> = vec![(None, std::mem::take(&mut pel))];
        for (_, _, _, it) in items {
            match it {
                It::Open(w) => stack.push((Some(w.id), fragment(&w.xml[0]))),
                It::Close(w) => {
                    if !stack.iter().any(|s| s.0 == Some(w.id)) {
                        return Err("a wrapper closes before it opens".into());
                    }
                    let (id, e) = stack.pop().unwrap();
                    if id != Some(w.id) {
                        return Err("wrappers are mis-nested".into());
                    }
                    stack.last_mut().unwrap().1.children.push(node(e));
                }
                It::Marker(m) => stack.last_mut().unwrap().1.children.push(node(fragment(&m.xml[0]))),
                It::Pkeep(k) => {
                    for x in &k.xml {
                        stack.last_mut().unwrap().1.children.push(node(fragment(x)));
                    }
                }
                It::Zrun(z) => {
                    let mut r = assemble(&z.xml);
                    for m in rm_at.get(&Target::Zrun(z.id)).into_iter().flatten() {
                        r.children.push(node(fragment(&m.xml[0])));
                    }
                    // Its own empty `w:t`s (a run with text is never zero-width).
                    for t in z.meta.aux.iter().map(|x| fragment(x)) {
                        if t.text_of(&["w:t"]).is_empty() {
                            r.children.push(node(t));
                        }
                    }
                    stack.last_mut().unwrap().1.children.push(node(r));
                }
                It::Seg(si) => {
                    let r =
                        self.run_el(p, &eff, segs[si], rm_at.get(&Target::Seg(si)).map(Vec::as_slice).unwrap_or(&[]))?;
                    stack.last_mut().unwrap().1.children.push(node(r));
                }
            }
        }
        if stack.len() != 1 {
            return Err("a wrapper is not closed".into());
        }
        Ok(stack.pop().unwrap().1)
    }

    fn run_el(
        &self,
        p: &Inline,
        eff: &[Marks],
        seg: (usize, usize, Option<&Entry>),
        rmarkers: &[&Entry],
    ) -> Result<Element, String> {
        let (a, b, own) = seg;
        let (mut r, mut rpr, t_attrs): (Element, Option<Element>, Vec<Element>) = match own {
            Some(o) => (
                fragment(&o.xml[0]),
                o.xml.get(1).map(|x| fragment(x)),
                o.meta.aux.iter().map(|x| fragment(x)).collect(),
            ),
            None => (el("w:r"), None, vec![]),
        };
        let want = eff[a];
        if rpr.is_none() && !want.is_empty() {
            rpr = Some(el("w:rPr"));
        }
        if let Some(x) = &mut rpr {
            set_flag(x, "w:b", want.has(Marks::BOLD));
            set_flag(x, "w:i", want.has(Marks::ITALIC));
            set_flag(x, "w:strike", want.has(Marks::STRIKE));
            set_underline(x, want.has(Marks::UNDERLINE));
            r.children.push(node(rpr.take().unwrap()));
        }
        let mut rm: BTreeMap<usize, Vec<&Entry>> = BTreeMap::new();
        for m in rmarkers {
            rm.entry(m.start.unwrap()).or_default().push(m);
        }
        for v in rm.values_mut() {
            v.sort_by_key(|e| e.seq);
        }
        let mut buf = String::new();
        let mut tcount = 0;
        let flush = |buf: &mut String, r: &mut Element, tcount: &mut usize| {
            if buf.is_empty() {
                return;
            }
            let (mut t, orig) = match t_attrs.get(*tcount).or(t_attrs.last()) {
                Some(x) => (x.shell(), (*tcount < t_attrs.len()).then(|| x.text_of(&["w:t"]))),
                None => (el("w:t"), None),
            };
            // A w:t keeps its attributes as they were; changed text that needs it gets preserve.
            let needs = buf.trim() != buf || buf.contains("  ");
            if orig.as_deref() != Some(buf.as_str()) && needs && t.attr("xml:space").is_none() {
                t.set("xml:space", "preserve");
            }
            t.children = vec![Node::Text(xml::escape_text(buf))];
            r.children.push(node(t));
            *tcount += 1;
            buf.clear();
        };
        for c in a..b {
            for m in rm.get(&c).into_iter().flatten() {
                flush(&mut buf, &mut r, &mut tcount);
                r.children.push(node(fragment(&m.xml[0])));
            }
            match &p.units[c].atom {
                Atom::Char('\t') => {
                    flush(&mut buf, &mut r, &mut tcount);
                    r.children.push(node(el("w:tab")));
                }
                Atom::Char(ch) => buf.push(*ch),
                Atom::Break => {
                    flush(&mut buf, &mut r, &mut tcount);
                    r.children.push(node(el("w:br")));
                }
                Atom::PageBreak => {
                    flush(&mut buf, &mut r, &mut tcount);
                    r.children.push(node(el("w:br").with_attr("w:type", "page")));
                }
                Atom::Keep(k) => {
                    flush(&mut buf, &mut r, &mut tcount);
                    for x in &self.keep_entry(&k.id)?.xml {
                        r.children.push(node(fragment(x)));
                    }
                }
                Atom::NoteRef(_) | Atom::Math(_) => {
                    return Err("footnotes and math are not supported by the docx engine yet".into())
                }
            }
        }
        flush(&mut buf, &mut r, &mut tcount);
        for m in rm.get(&b).into_iter().flatten() {
            r.children.push(node(fragment(&m.xml[0])));
        }
        Ok(r)
    }

    // ------------------------------------------------------------ table

    fn table(&self, t: &hanji_core::Table, bi: usize) -> Result<Element, String> {
        let te = self.at(&[bi], Kind::Tbl);
        let mut tbl = match te.first() {
            Some(e) => {
                let mut tbl = assemble(&e.xml);
                let old = e.meta.style.as_deref().and_then(|id| self.styles.table_style_name(id));
                if old != t.style {
                    self.set_table_style(&mut tbl, t.style.as_deref())?;
                }
                tbl
            }
            None => {
                let mut tbl = el("w:tbl");
                let mut pr = el("w:tblPr");
                pr.children.push(node(el("w:tblW").with_attr("w:w", "0").with_attr("w:type", "auto")));
                tbl.children.push(node(pr));
                let mut grid = el("w:tblGrid");
                for _ in &t.rows[0] {
                    grid.children.push(node(el("w:gridCol").with_attr("w:w", "2000")));
                }
                tbl.children.push(node(grid));
                self.set_table_style(&mut tbl, t.style.as_deref())?;
                tbl
            }
        };
        for (ri, row) in t.rows.iter().enumerate() {
            self.bmarkers(&mut tbl, &[bi, ri]);
            let mut tr = self.at(&[bi, ri], Kind::Tr).first().map_or_else(|| el("w:tr"), |e| assemble(&e.xml));
            let mut ci = 0;
            while ci < row.len() {
                let cell = &row[ci];
                if *cell == Cell::Left {
                    return Err(format!("row {}: || with no cell to its left", ri + 1));
                }
                let mut span = 1;
                while ci + span < row.len() && row[ci + span] == Cell::Left {
                    span += 1;
                }
                let path = [bi, ri, ci];
                self.bmarkers(&mut tr, &path);
                let (mut tc, tcpr) = match self.at(&path, Kind::Tc).first() {
                    Some(e) => (fragment(&e.xml[0]), e.xml.get(1).map(|x| fragment(x))),
                    None => (el("w:tc"), None),
                };
                let below_up = t.rows.get(ri + 1).and_then(|r| r.get(ci)) == Some(&Cell::Up);
                if let Some(x) = merge_props(tcpr, span, *cell == Cell::Up, below_up) {
                    tc.children.push(node(x));
                }
                let n_paras = match cell {
                    Cell::Text(ps) => {
                        for (k, p) in ps.iter().enumerate() {
                            self.bmarkers(&mut tc, &[bi, ri, ci, k]);
                            let style = p.style.as_deref().unwrap_or(&self.styles.default_paragraph);
                            tc.children.push(node(self.para(&p.content, Some(style), &[bi, ri, ci, k])?));
                        }
                        ps.len()
                    }
                    _ => {
                        // A covered cell keeps the paragraphs the remainder has.
                        let n = self
                            .by
                            .keys()
                            .filter(|(p, k)| *k == Kind::Ppr && p.len() == 4 && p[..3] == path)
                            .map(|(p, _)| p[3] + 1)
                            .max()
                            .unwrap_or(1);
                        for k in 0..n {
                            self.bmarkers(&mut tc, &[bi, ri, ci, k]);
                            tc.children.push(node(self.para(&Inline::default(), None, &[bi, ri, ci, k])?));
                        }
                        n
                    }
                };
                self.bmarkers(&mut tc, &[bi, ri, ci, n_paras]);
                tr.children.push(node(tc));
                ci += span;
            }
            self.bmarkers(&mut tr, &[bi, ri, row.len()]);
            tbl.children.push(node(tr));
        }
        self.bmarkers(&mut tbl, &[bi, t.rows.len()]);
        Ok(tbl)
    }

    /// Markers stored before the block, row, cell or cell paragraph at `path`.
    fn bmarkers(&self, parent: &mut Element, path: &[usize]) {
        let mut ms = self.at(path, Kind::Bmarker);
        ms.sort_by_key(|e| e.seq);
        for m in ms {
            parent.children.push(node(fragment(&m.xml[0])));
        }
    }

    fn set_table_style(&self, tbl: &mut Element, style: Option<&str>) -> Result<(), String> {
        let id = match style {
            Some(s) => Some(self.styles.table_id(s).ok_or_else(|| format!("unknown table style {s:?}"))?.to_string()),
            None => None,
        };
        if tbl.child("w:tblPr").is_none() {
            tbl.children.insert(0, node(el("w:tblPr")));
        }
        let pr = tbl.child_mut("w:tblPr").unwrap();
        remove_child(pr, "w:tblStyle");
        if let Some(id) = id {
            pr.children.insert(0, node(el("w:tblStyle").with_attr("w:val", &id)));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Target {
    Seg(usize),
    Zrun(u64),
}

fn set_flag(rpr: &mut Element, name: &str, want: bool) {
    let cur = rpr.child(name);
    if on(cur) == want {
        return;
    }
    if cur.is_some() {
        remove_child(rpr, name);
    }
    if want {
        insert_ordered(rpr, el(name), RPR_ORDER);
    }
}

fn set_underline(rpr: &mut Element, want: bool) {
    let cur = rpr.child("w:u");
    if underline_on(cur) == want {
        return;
    }
    if cur.is_some() {
        remove_child(rpr, "w:u");
    }
    if want {
        insert_ordered(rpr, el("w:u").with_attr("w:val", "single"), RPR_ORDER);
    }
}

/// `gridSpan` and `vMerge` regenerated from the `||` / `^^` markers.
fn merge_props(tcpr: Option<Element>, span: usize, covered: bool, top: bool) -> Option<Element> {
    let cur_span = grid_span(tcpr.as_ref());
    let has_vm = tcpr.as_ref().is_some_and(|t| t.child("w:vMerge").is_some());
    let cur_v = has_vm.then(|| if v_merged(tcpr.as_ref()) { "continue" } else { "restart" });
    let want_v = if covered {
        Some("continue")
    } else if top {
        Some("restart")
    } else {
        None
    };
    if cur_span == span && cur_v == want_v {
        return tcpr;
    }
    let mut x = tcpr.unwrap_or_else(|| el("w:tcPr"));
    if cur_span != span {
        remove_child(&mut x, "w:gridSpan");
        if span > 1 {
            insert_ordered(&mut x, el("w:gridSpan").with_attr("w:val", &span.to_string()), TCPR_ORDER);
        }
    }
    if cur_v != want_v {
        remove_child(&mut x, "w:vMerge");
        match want_v {
            Some("continue") => insert_ordered(&mut x, el("w:vMerge"), TCPR_ORDER),
            Some(_) => insert_ordered(&mut x, el("w:vMerge").with_attr("w:val", "restart"), TCPR_ORDER),
            None => {}
        }
    }
    Some(x)
}
