//! Resolved model + placed entries → `Contents/section*.xml` (and
//! `header.xml` when the text needs a character or paragraph shape the file
//! does not have yet).

use std::collections::{BTreeMap, HashMap};

use hanji_core::{Block, Entry, Kind, ListPlan, StyleSet};
use hanji_format::{Atom, Cell, Inline, Marks};
use hanji_package::xml::{assemble, escape_text, fragment, Element, Node};

use crate::header::{list_heading, Header, Style};
use crate::import::{dims, layout_key};
use crate::owpml::char_element;
use crate::{PackageShell, SectionShell};

pub struct Exporter<'a> {
    styles: &'a StyleSet,
    pub header: Header,
    shell: &'a PackageShell,
    by: HashMap<(Vec<usize>, Kind), Vec<&'a Entry>>,
    keep: HashMap<&'a str, &'a Entry>,
    tail: Vec<&'a Entry>,
    lists: HashMap<usize, ListPlan>,
    /// Anchor paragraph of each table group.
    anchors: HashMap<&'a str, &'a Entry>,
    /// The first table of the file, for a new table's properties.
    table_template: Option<&'a Entry>,
    cell_template: Option<&'a Entry>,
}

fn node(e: Element) -> Node {
    Node::El(e)
}

/// An empty run in character shape `char_pr`.
fn empty_run(char_pr: u32) -> Element {
    Element::new("hp:run").with_attr("charPrIDRef", &char_pr.to_string())
}

fn num_attr(e: &Element, k: &str) -> Option<u32> {
    e.get(k).and_then(|v| v.parse().ok())
}

/// One emitted section: which stored section it is, and its paragraphs.
struct Section {
    shell: usize,
    paras: Vec<Element>,
}

/// The body being written.
struct Out {
    sections: Vec<Section>,
    /// The section's `hp:secPr` run for its first paragraph, and whether
    /// it heads that paragraph's first run.
    start: Option<(Element, bool)>,
    /// Durable markers whose paragraph went, for the next paragraph.
    runs: Vec<Element>,
    page_break: bool,
}

/// Where a zero-width item goes: into a text run segment, a zero-width run,
/// or a new run at its position.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Target {
    Seg(usize),
    Zrun(u64),
    New(usize),
}

impl<'a> Exporter<'a> {
    pub fn new(
        styles: &'a StyleSet,
        header: Header,
        shell: &'a PackageShell,
        entries: &'a [Entry],
        lists: HashMap<usize, ListPlan>,
    ) -> Self {
        let mut ex = Exporter {
            styles,
            header,
            shell,
            by: HashMap::new(),
            keep: HashMap::new(),
            tail: vec![],
            lists,
            anchors: HashMap::new(),
            table_template: None,
            cell_template: None,
        };
        for e in entries {
            match e.kind {
                Kind::Tail => ex.tail.push(e),
                Kind::Keep | Kind::Bkeep => {
                    ex.keep.insert(e.meta.keep.as_ref().unwrap().id.as_str(), e);
                }
                _ => ex.by.entry((e.path.clone(), e.kind)).or_default().push(e),
            }
            if e.kind == Kind::Tbl && e.meta.tag == "hwpx:anchor" {
                ex.anchors.insert(e.meta.aux[0].as_str(), e);
            }
            if e.kind == Kind::Tbl && e.meta.tag.is_empty() && ex.table_template.is_none() {
                ex.table_template = Some(e);
            }
            if e.kind == Kind::Tc && ex.cell_template.is_none() {
                ex.cell_template = Some(e);
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

    /// A style by its name in the text; a style missing from header.xml is
    /// named by its id (see [`Header::style_or_missing`]).
    fn style_named(&self, name: &str) -> Result<Style, String> {
        if let Some(s) = self.header.style_named(name) {
            return Ok(s.clone());
        }
        let missing = self.styles.paragraph.iter().find(|d| d.name == name).and_then(|d| d.id.parse().ok());
        missing.map(|id| self.header.style_or_missing(id)).ok_or_else(|| format!("unknown paragraph style {name:?}"))
    }

    fn default_style(&self) -> Result<Style, String> {
        self.style_named(&self.styles.default_paragraph)
    }

    // ------------------------------------------------------------ body

    /// The sections: `(stored section, paragraphs)` in text order.
    pub fn body(&mut self, blocks: &[Block]) -> Result<Vec<(&'a SectionShell, Vec<Element>)>, String> {
        let shell = self.shell;
        let first = shell.sections.first().ok_or("the remainder has no section")?;
        let mut out = Out {
            sections: vec![Section { shell: 0, paras: vec![] }],
            start: first.start_run.as_deref().map(|r| (fragment(r), first.start_merged)),
            runs: vec![],
            page_break: false,
        };
        let mut bi = 0;
        while bi < blocks.len() {
            let mut ms = self.at(&[bi], Kind::Bmarker);
            ms.sort_by_key(|e| e.seq);
            for m in ms {
                if m.meta.tag == "hwpx:section" {
                    self.start_section(&mut out, m)?;
                } else {
                    out.runs.push(wrap_run(m));
                }
            }
            let step = match &blocks[bi] {
                Block::Para(p) if is_page_break(&p.content) => {
                    if std::mem::replace(&mut out.page_break, true) {
                        return Err("two page breaks in a row cannot be written to hwpx: a page break there is a property of the paragraph after it. Put a paragraph between them.".into());
                    }
                    1
                }
                Block::Para(p) => {
                    // A list item keeps the paragraph style the remainder has.
                    let style = p.item.is_none().then_some(p.style.as_str());
                    let pel = self.para(&p.content, style, &[bi], Some((bi, blocks)))?;
                    self.emit(&mut out, pel);
                    1
                }
                Block::Table(_) => {
                    let (pel, n) = self.table_group(blocks, bi)?;
                    self.emit(&mut out, pel);
                    n
                }
                Block::Keep(id) => {
                    let e = self.keep_entry(id)?;
                    let x = fragment(&e.xml[0]);
                    if !x.is("hp:p") {
                        return Err(format!("placeholder {id} is not a paragraph"));
                    }
                    self.emit(&mut out, x);
                    1
                }
                Block::Head(_) => return Err("a Document has no slides, slots or shapes".into()),
            };
            bi += step;
        }
        if out.page_break {
            return Err("a page break at the end cannot be written to hwpx: a page break there is a property of the paragraph after it. Add a paragraph after it.".into());
        }
        // What follows the last block: markers go into the last paragraph; a
        // section whose blocks all went starts after it, with an empty paragraph.
        let mut tail = self.tail.clone();
        tail.sort_by_key(|e| e.seq);
        for e in tail {
            if e.meta.tag == "hwpx:section" {
                self.start_section(&mut out, e)?;
                continue;
            }
            self.close_section(&mut out)?;
            let last = out.sections.last_mut().unwrap().paras.last_mut().unwrap();
            let at = last.children.iter().position(|n| matches!(n, Node::El(e) if e.is("hp:linesegarray")));
            last.children.insert(at.unwrap_or(last.children.len()), node(wrap_run(e)));
        }
        self.close_section(&mut out)?;
        let mut used: Vec<usize> = out.sections.iter().map(|s| s.shell).collect();
        used.sort();
        if let Some(k) = (0..shell.sections.len()).find(|k| used.binary_search(k).is_err()) {
            return Err(format!(
                "section {} has no place in the text any more (its start was refused by the edit)",
                k + 1
            ));
        }
        if used.windows(2).any(|w| w[0] == w[1]) {
            return Err("a section starts twice".into());
        }
        for (i, s) in out.sections.iter().enumerate() {
            if s.paras.first().is_none_or(|p| p.descendants("hp:secPr").is_empty()) {
                return Err(format!(
                    "section {} lost its section settings (hp:secPr) with the paragraph that held them",
                    i + 1
                ));
            }
        }
        Ok(out.sections.into_iter().map(|s| (&shell.sections[s.shell], s.paras)).collect())
    }

    /// Section `m` starts here: the one before it is closed.
    fn start_section(&self, out: &mut Out, m: &Entry) -> Result<(), String> {
        let k: usize = m.meta.aux[0].parse().map_err(|_| "a section marker without its section")?;
        self.close_section(out)?;
        let s = self.shell.sections.get(k).ok_or_else(|| format!("section {k} is not in the remainder"))?;
        out.sections.push(Section { shell: k, paras: vec![] });
        out.start = s.start_run.as_deref().map(|r| (fragment(r), s.start_merged));
        Ok(())
    }

    /// A section with no paragraph left gets an empty one for its settings.
    fn close_section(&self, out: &mut Out) -> Result<(), String> {
        if out.sections.last().is_some_and(|s| s.paras.is_empty()) {
            let p = self.new_para(&self.default_style()?);
            self.emit(out, p);
        }
        Ok(())
    }

    /// A top-level paragraph: its page break, the section start and moved markers.
    fn emit(&self, out: &mut Out, mut p: Element) {
        if std::mem::take(&mut out.page_break) {
            p.set("pageBreak", "1");
        } else if p.get("pageBreak").as_deref() == Some("1") {
            p.set("pageBreak", "0");
        }
        let mut start = out.start.take();
        let first_run = p.children.iter_mut().find_map(|n| match n {
            Node::El(r) if r.is("hp:run") => Some(r),
            _ => None,
        });
        if let (Some((run, true)), Some(r)) = (&start, first_run) {
            if r.attrs == run.attrs {
                r.children.splice(0..0, run.children.iter().cloned());
                start = None;
            }
        }
        let lead: Vec<Node> = start.map(|s| s.0).into_iter().chain(out.runs.drain(..)).map(node).collect();
        if !lead.is_empty() {
            let at = p.children.iter().position(|n| matches!(n, Node::El(_))).unwrap_or(p.children.len());
            p.children.splice(at..at, lead);
        }
        out.sections.last_mut().unwrap().paras.push(p);
    }

    fn new_para(&self, style: &Style) -> Element {
        let mut p = Element::new("hp:p")
            .with_attr("id", "0")
            .with_attr("paraPrIDRef", &style.para_pr.to_string())
            .with_attr("styleIDRef", &style.id.to_string())
            .with_attr("pageBreak", "0")
            .with_attr("columnBreak", "0")
            .with_attr("merged", "0");
        p.children.push(node(empty_run(style.char_pr)));
        p
    }

    // ------------------------------------------------------------ paragraph

    /// The paragraph shape a new list item starts from: that of the nearest
    /// item of its text list that has one.
    fn list_template(&self, blocks: &[Block], bi: usize) -> Option<&'a Entry> {
        let item = |k: usize| match &blocks[k] {
            Block::Para(p) => p.item,
            _ => None,
        };
        let lo = (0..=bi).rev().take_while(|&k| item(k).is_some()).last().unwrap_or(bi);
        let hi = (bi..blocks.len()).take_while(|&k| item(k).is_some()).last().unwrap_or(bi);
        let mut near: Vec<usize> = (lo..=hi).filter(|&k| k != bi).collect();
        near.sort_by_key(|&k| (k.abs_diff(bi), k));
        near.into_iter().find_map(|k| self.at(&[k], Kind::Ppr).first().copied())
    }

    /// `hp:p` with its runs. `style: None` keeps the style the remainder has.
    /// `list`: the block index and blocks of a top-level paragraph.
    fn para(
        &mut self,
        content: &Inline,
        style: Option<&str>,
        path: &[usize],
        list: Option<(usize, &[Block])>,
    ) -> Result<Element, String> {
        let pp = self.at(path, Kind::Ppr).first().copied();
        let template = match (pp, list) {
            (None, Some((bi, blocks))) if self.lists.contains_key(&bi) => self.list_template(blocks, bi),
            _ => None,
        };
        let default = self.default_style()?;
        let (mut p, old_style, lines, layout) = match pp.or(template) {
            Some(e) => {
                let p = fragment(&e.xml[0]);
                let old =
                    e.meta.style.as_deref().and_then(|s| s.parse().ok()).map(|id| self.header.style_or_missing(id));
                let own = pp.is_some();
                let lines = e.xml.get(1).filter(|_| own).map(|x| fragment(x));
                let layout = own.then(|| (e.meta.aux[2].clone(), e.meta.aux[3].parse::<usize>().unwrap_or(usize::MAX)));
                (p, old, lines, layout)
            }
            None => {
                let mut p = self.new_para(&default);
                p.children.clear();
                (p, None, None, None)
            }
        };
        let now = match style {
            Some(name) => self.style_named(name)?,
            None => old_style.clone().unwrap_or_else(|| default.clone()),
        };
        let mut ppr = num_attr(&p, "paraPrIDRef").unwrap_or(now.para_pr);
        if old_style.as_ref().map(|s| s.id) != Some(now.id) {
            p.set("styleIDRef", &now.id.to_string());
            if old_style.as_ref().is_none_or(|s| s.para_pr == ppr) {
                ppr = now.para_pr;
            }
        }
        if let Some((bi, _)) = list {
            match self.lists.get(&bi) {
                Some(ListPlan::Set { num, ilvl }) => {
                    ppr = self.header.para_pr_with(ppr, Some(list_heading(*num, *ilvl)))?
                }
                Some(ListPlan::Strip) => ppr = self.header.para_pr_with(ppr, None)?,
                Some(ListPlan::Keep) | None => {}
            }
        }
        if p.attr("paraPrIDRef").is_some() || ppr != now.para_pr {
            p.set("paraPrIDRef", &ppr.to_string());
        }
        self.runs(&mut p, content, path, &now, old_style.as_ref())?;
        if pp.is_none() && p.child("hp:run").is_none() {
            // A paragraph holds at least one run.
            p.children.push(node(empty_run(now.char_pr)));
        }
        if let (Some(l), Some((k, at))) = (lines, layout) {
            if layout_key(now.id, ppr, content) == k {
                p.children.insert(at.min(p.children.len()), node(l));
            }
        }
        Ok(p)
    }

    fn runs(
        &mut self,
        pel: &mut Element,
        p: &Inline,
        path: &[usize],
        style: &Style,
        old_style: Option<&Style>,
    ) -> Result<(), String> {
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
        // at a mark's edge) keeps what its run had.
        let eff = p.written_marks(&|c| owner[c].map_or(Marks::NONE, |o| o.meta.marks));
        let mut pkeep: BTreeMap<usize, &Entry> = BTreeMap::new();
        for (c, u) in p.units.iter().enumerate() {
            if let Atom::Keep(k) = &u.atom {
                let ke = self.keep_entry(&k.id)?;
                if ke.meta.run.is_none() {
                    pkeep.insert(c, ke);
                }
            }
        }
        let bounds: std::collections::HashSet<usize> = zruns.iter().map(|z| z.start.unwrap()).collect();
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
        // Zero-width items (markers, `hp:t` starts, marks inside `hp:t`) go
        // into their own run when it still covers them.
        let mut zw: Vec<&Entry> = self.at(path, Kind::Marker);
        zw.extend(self.at(path, Kind::Rmarker));
        zw.sort_by_key(|e| e.seq);
        let mut at: HashMap<Target, Vec<&Entry>> = HashMap::new();
        for m in zw {
            let s = m.start.unwrap();
            let own = |g: &(usize, usize, Option<&Entry>)| g.2.map(|o| o.id) == m.meta.run;
            let t = segs
                .iter()
                .position(|g| own(g) && g.0 <= s && s <= g.1)
                .map(Target::Seg)
                .or_else(|| {
                    zruns.iter().find(|z| Some(z.id) == m.meta.run && z.start == Some(s)).map(|z| Target::Zrun(z.id))
                })
                .or_else(|| segs.iter().position(|g| g.0 <= s && s < g.1).map(Target::Seg))
                .or_else(|| segs.iter().rposition(|g| g.1 == s).map(Target::Seg))
                .unwrap_or(Target::New(s));
            at.entry(t).or_default().push(m);
        }
        #[derive(Clone, Copy)]
        enum It<'b> {
            Zrun(&'b Entry),
            New(usize),
            Pkeep(&'b Entry),
            Seg(usize),
        }
        let mut items: Vec<(usize, u8, u64, It)> = vec![];
        for z in &zruns {
            items.push((z.start.unwrap(), 0, z.seq, It::Zrun(z)));
        }
        for t in at.keys() {
            if let Target::New(s) = t {
                let seq = at[t].first().map_or(0, |e| e.seq);
                items.push((*s, 0, seq, It::New(*s)));
            }
        }
        for (c, k) in &pkeep {
            items.push((*c, 1, k.seq, It::Pkeep(k)));
        }
        for (si, s) in segs.iter().enumerate() {
            items.push((s.0, 1, s.2.map_or(u64::MAX, |o| o.seq), It::Seg(si)));
        }
        items.sort_by_key(|t| (t.0, t.1, t.2));
        let mut tabs: HashMap<u64, usize> = HashMap::new();
        let none: Vec<&Entry> = vec![];
        let mut out = vec![];
        for (_, _, _, it) in items {
            match it {
                It::Pkeep(k) => out.extend(k.xml.iter().map(|x| node(fragment(x)))),
                It::Zrun(z) => {
                    let mut r = fragment(&z.xml[0]);
                    let base = self.base_char_pr(&r, style, old_style);
                    self.set_char_pr(&mut r, base, old_style, style);
                    let ms = at.get(&Target::Zrun(z.id)).unwrap_or(&none);
                    self.fill_run(&mut r, p, z.start.unwrap(), z.start.unwrap(), ms, None, &mut tabs)?;
                    out.push(node(r));
                }
                It::New(s) => {
                    let ms = &at[&Target::New(s)];
                    let mut r =
                        ms.iter().find_map(|m| m.xml.get(1)).map_or_else(|| empty_run(style.char_pr), |x| fragment(x));
                    r.children.clear();
                    self.fill_run(&mut r, p, s, s, ms, None, &mut tabs)?;
                    out.push(node(r));
                }
                It::Seg(si) => {
                    let (a, b, own) = segs[si];
                    let mut r = match own {
                        Some(o) => fragment(&o.xml[0]),
                        None => empty_run(style.char_pr),
                    };
                    let base = self.base_char_pr(&r, style, old_style);
                    let sflags = self.header.flags(style.char_pr);
                    let want = Marks(eff[a].0 | (sflags.0 & self.header.flags(base).0));
                    let cp = self.header.char_pr_with(base, want)?;
                    self.set_char_pr(&mut r, cp, old_style, style);
                    let ms = at.get(&Target::Seg(si)).unwrap_or(&none);
                    self.fill_run(&mut r, p, a, b, ms, own, &mut tabs)?;
                    out.push(node(r));
                }
            }
        }
        pel.children.extend(out);
        Ok(())
    }

    /// The character shape a run starts from: its own, or, when its
    /// paragraph changed style and the run had the old style's shape, the new one's.
    fn base_char_pr(&self, r: &Element, style: &Style, old_style: Option<&Style>) -> u32 {
        let cur = num_attr(r, "charPrIDRef").unwrap_or(old_style.unwrap_or(style).char_pr);
        match old_style {
            Some(o) if o.id != style.id && o.char_pr == cur => style.char_pr,
            _ => cur,
        }
    }

    /// Writes `charPrIDRef` when the run has one or its shape changed.
    fn set_char_pr(&self, r: &mut Element, cp: u32, old_style: Option<&Style>, style: &Style) {
        let cur = num_attr(r, "charPrIDRef").unwrap_or(old_style.unwrap_or(style).char_pr);
        if r.attr("charPrIDRef").is_some() || cp != cur {
            r.set("charPrIDRef", &cp.to_string());
        }
    }

    /// A run's children: units `a..b` and the zero-width items placed in it.
    #[allow(clippy::too_many_arguments)]
    fn fill_run(
        &self,
        r: &mut Element,
        p: &Inline,
        a: usize,
        b: usize,
        zw: &[&Entry],
        own: Option<&Entry>,
        tabs: &mut HashMap<u64, usize>,
    ) -> Result<(), String> {
        let mut by_pos: BTreeMap<usize, Vec<&Entry>> = BTreeMap::new();
        for m in zw {
            by_pos.entry(m.start.unwrap().clamp(a, b)).or_default().push(m);
        }
        let mut t: Option<Element> = None;
        let close = |t: &mut Option<Element>, r: &mut Element| {
            if let Some(x) = t.take() {
                r.children.push(node(x));
            }
        };
        let open = |t: &mut Option<Element>| {
            t.get_or_insert_with(|| Element::new("hp:t"));
        };
        for c in a..=b {
            for m in by_pos.get(&c).into_iter().flatten() {
                match (m.kind, m.meta.aux.first().map(String::as_str)) {
                    (Kind::Rmarker, _) if m.meta.tag == "hp:t" => {
                        close(&mut t, r);
                        t = Some(fragment(&m.xml[0]));
                    }
                    (Kind::Rmarker, Some("in-t")) => {
                        open(&mut t);
                        t.as_mut().unwrap().children.push(node(fragment(&m.xml[0])));
                    }
                    _ => {
                        close(&mut t, r);
                        r.children.push(node(fragment(&m.xml[0])));
                    }
                }
            }
            if c == b {
                break;
            }
            match &p.units[c].atom {
                Atom::Char(ch) => {
                    open(&mut t);
                    let x = t.as_mut().unwrap();
                    match char_element(*ch) {
                        Some(name) => {
                            let stored = (*ch == '\t')
                                .then(|| {
                                    let o = own?;
                                    let k = tabs.entry(o.id).or_default();
                                    *k += 1;
                                    o.meta.aux.get(*k - 1).map(|s| fragment(s))
                                })
                                .flatten();
                            x.children.push(node(stored.unwrap_or_else(|| Element::new(name))));
                        }
                        None => match x.children.last_mut() {
                            Some(Node::Text(s)) => s.push_str(&escape_text(&ch.to_string())),
                            _ => x.children.push(Node::Text(escape_text(&ch.to_string()))),
                        },
                    }
                }
                Atom::Break => {
                    open(&mut t);
                    t.as_mut().unwrap().children.push(node(Element::new("hp:lineBreak")));
                }
                Atom::Keep(k) => {
                    close(&mut t, r);
                    for x in &self.keep_entry(&k.id)?.xml {
                        r.children.push(node(fragment(x)));
                    }
                }
                Atom::PageBreak => {
                    return Err(
                        "a page break inside a paragraph cannot be written to hwpx; write <pagebreak/> on its own line"
                            .into(),
                    )
                }
                Atom::NoteRef(_) | Atom::Math(_) => {
                    return Err("footnotes and math are not supported by the hwpx engine yet".into())
                }
            }
        }
        close(&mut t, r);
        Ok(())
    }

    // ------------------------------------------------------------ tables

    /// The paragraph holding the table at `bi` and the side-by-side tables
    /// after it that shared its paragraph; how many blocks it took.
    fn table_group(&mut self, blocks: &[Block], bi: usize) -> Result<(Element, usize), String> {
        let tag = |k: usize, t: &str| self.at(&[k], Kind::Tbl).into_iter().find(|e| e.meta.tag == t);
        let (mut anchor, group) = match tag(bi, "hwpx:anchor") {
            Some(a) => (fragment(&a.xml[0]), Some(a.meta.aux[0].clone())),
            None => (self.fresh_anchor(tag(bi, "hwpx:join"))?, None),
        };
        let mut members = vec![bi];
        if let Some(g) = &group {
            let mut k = bi + 1;
            while k < blocks.len()
                && matches!(blocks[k], Block::Table(_))
                && self.at(&[k], Kind::Bmarker).is_empty()
                && tag(k, "hwpx:join").is_some_and(|j| &j.meta.aux[0] == g)
            {
                members.push(k);
                k += 1;
            }
        }
        let mut tables = vec![];
        for &k in &members {
            let Block::Table(t) = &blocks[k] else { unreachable!() };
            tables.push(self.table(t, k)?);
        }
        // Fill the `hp:tbl` slots in order; slots of tables that went are dropped.
        let mut tables = tables.into_iter();
        for r in anchor.elements_mut().filter(|r| r.is("hp:run")) {
            let mut kids = vec![];
            for n in std::mem::take(&mut r.children) {
                match n {
                    Node::El(e) if e.is("hp:tbl") && e.children.is_empty() => {
                        if let Some(t) = tables.next() {
                            kids.push(node(t));
                        }
                    }
                    n => kids.push(n),
                }
            }
            r.children = kids;
        }
        Ok((anchor, members.len()))
    }

    /// A paragraph for one table: the shell of its group's paragraph, or a
    /// new one.
    fn fresh_anchor(&self, join: Option<&Entry>) -> Result<Element, String> {
        let from = join.and_then(|j| self.anchors.get(j.meta.aux[0].as_str())).map(|a| fragment(&a.xml[0]));
        let default = self.default_style()?;
        let (mut p, run) = match from {
            Some(a) => {
                let run = a.elements().find(|r| r.is("hp:run") && r.child("hp:tbl").is_some()).map(|r| r.shell());
                (a.shell(), run)
            }
            None => (self.new_para(&default), None),
        };
        p.children.clear();
        let mut run = run.unwrap_or_else(|| empty_run(default.char_pr));
        run.children = vec![node(Element::new("hp:tbl")), node(Element::new("hp:t"))];
        p.children.push(node(run));
        Ok(p)
    }

    fn table(&mut self, t: &hanji_core::Table, bi: usize) -> Result<Element, String> {
        if t.style.is_some() {
            return Err("hwpx has no table styles; remove the {style=…} line".into());
        }
        let own = self.at(&[bi], Kind::Tbl).into_iter().find(|e| e.meta.tag.is_empty());
        let Some(e) = own.or(self.table_template) else {
            return Err("this file has no table to take a new table's layout from, so the table cannot be written; tables can be added to hwpx files that already have one".into());
        };
        let mut tbl = assemble(&e.xml);
        if own.is_none() {
            // A new table takes the first table's layout, not its caption.
            tbl.children.retain(|n| !matches!(n, Node::El(x) if x.is("hp:caption")));
        }
        let (rows_n, cols_n) = (t.rows.len(), t.rows.first().map_or(0, Vec::len));
        let (r0, c0) = dims(&tbl);
        tbl.set("rowCnt", &rows_n.to_string());
        tbl.set("colCnt", &cols_n.to_string());
        if own.is_none() || (r0, c0) != (rows_n, cols_n) {
            // Cell zones address the old grid.
            tbl.children.retain(|n| !matches!(n, Node::El(x) if x.is("hp:cellzoneList")));
        }
        for (ri, row) in t.rows.iter().enumerate() {
            let mut tr =
                self.at(&[bi, ri], Kind::Tr).first().map_or_else(|| Element::new("hp:tr"), |e| assemble(&e.xml));
            for (ci, cell) in row.iter().enumerate() {
                let path = [bi, ri, ci];
                let ps = match cell {
                    Cell::Left if ci == 0 => return Err(format!("row {}: || with no cell to its left", ri + 1)),
                    Cell::Text(ps) => ps,
                    _ => {
                        // hwpx has no cell where a merge covers one: what the old cell held would go.
                        let held = self
                            .by
                            .iter()
                            .any(|((p, _), es)| p.len() >= 3 && p[..3] == path && es.iter().any(|e| !e.is_trivial()));
                        if held {
                            return Err(format!("the cell at row {}, column {} is now covered by a merge, and hwpx keeps no cell there, so its properties and content would be lost; merge cells in Hancom, or write the merged table as a new one", ri + 1, ci + 1));
                        }
                        continue;
                    }
                };
                let span = 1 + row[ci + 1..].iter().take_while(|c| **c == Cell::Left).count();
                let down = 1 + t.rows[ri + 1..].iter().take_while(|r| r.get(ci) == Some(&Cell::Up)).count();
                let te = self.at(&path, Kind::Tc).first().copied().or_else(|| self.cell_neighbour(t, bi, ri, ci));
                let xml = te.ok_or("this file has no table cell to take a new cell's properties from")?.xml.clone();
                let mut tc = fragment(&xml[0]);
                let mut sub = fragment(&xml[1]);
                let styles = self.styles;
                for (k, p) in ps.iter().enumerate() {
                    let style = p.style.as_deref().unwrap_or(&styles.default_paragraph);
                    sub.children.push(node(self.para(&p.content, Some(style), &[bi, ri, ci, k], None)?));
                }
                if !self.at(&[bi, ri, ci, ps.len()], Kind::Bmarker).is_empty() {
                    return Err("a marker between cell paragraphs cannot be written to hwpx".into());
                }
                tc.children.push(node(sub));
                let mut rest: Vec<Element> = xml[2..].iter().map(|x| fragment(x)).collect();
                // The cell's place, written after its address (which goes first).
                for (name, attrs, after) in [
                    ("hp:cellAddr", [("colAddr", ci), ("rowAddr", ri)], None),
                    ("hp:cellSpan", [("colSpan", span), ("rowSpan", down)], Some("hp:cellAddr")),
                ] {
                    let k = rest.iter().position(|x| x.is(name)).unwrap_or_else(|| {
                        let at = after.and_then(|a| rest.iter().position(|x| x.is(a))).map_or(0, |k| k + 1);
                        rest.insert(at, Element::new(name));
                        at
                    });
                    for (a, v) in attrs {
                        rest[k].set(a, &v.to_string());
                    }
                }
                tc.children.extend(rest.into_iter().map(node));
                tr.children.push(node(tc));
            }
            tbl.children.push(node(tr));
        }
        Ok(tbl)
    }

    /// Properties for a new cell: the nearest cell above, then to the left,
    /// in the same table, then the file's first cell.
    fn cell_neighbour(&self, t: &hanji_core::Table, bi: usize, ri: usize, ci: usize) -> Option<&'a Entry> {
        let find = |r: usize, c: usize| self.at(&[bi, r, c], Kind::Tc).first().copied();
        (0..ri)
            .rev()
            .find_map(|r| find(r, ci))
            .or_else(|| (0..ci).rev().find_map(|c| find(ri, c)))
            .or_else(|| (0..t.rows.len()).find_map(|r| (0..t.rows[r].len()).find_map(|c| find(r, c))))
            .or(self.cell_template)
    }
}

/// A zero-width item whose paragraph went, as a run of its own: a marker
/// in its old run's shell, an `hp:t` start or mark in a run and `hp:t`, a
/// zero-width run as itself.
fn wrap_run(m: &Entry) -> Element {
    let el = fragment(&m.xml[0]);
    if el.is("hp:run") {
        return el.shell();
    }
    let mut r = m.xml.get(1).map_or_else(|| empty_run(0), |x| fragment(x));
    let el = if m.meta.aux.first().is_some_and(|a| a == "in-t") {
        let mut t = Element::new("hp:t");
        t.children.push(node(el));
        t
    } else {
        el
    };
    r.children = vec![node(el)];
    r
}

pub(crate) fn is_page_break(i: &Inline) -> bool {
    i.units.len() == 1 && i.units[0].atom == Atom::PageBreak
}
