//! Canonical serializer (§5.1): one paragraph per line, no table padding,
//! fixed attribute order, boundary spaces outside emphasis markers, and
//! escapes only where the parser would otherwise misread a character.

use crate::ast::*;
use crate::parse::{closer, delim};
use crate::styled::{self, StyleTable};

pub fn serialize(doc: &Document) -> String {
    let mut out = front_lines(&doc.front);
    let table = StyleTable { default: doc.styles.first().map(|l| l.name.clone()), lines: doc.styles.clone() };
    for l in &doc.styles {
        let p = l.props.write();
        out.push(format!("<style name=\"{}\"{}{p}/>", attr(&l.name), if p.is_empty() { "" } else { " " }));
    }
    if !doc.styles.is_empty() && !doc.blocks.is_empty() {
        out.push(String::new());
    }
    blocks_in(&mut out, &doc.blocks, &table);
    out.join("\n") + "\n"
}

/// The front matter's lines.
pub(crate) fn front_lines(f: &FrontMatter) -> Vec<String> {
    let mut out = vec!["---".to_string(), format!("type: {}", f.doc_type), format!("format: {}", f.format)];
    if let Some(t) = &f.template {
        out.push(format!("template: {t}"));
    }
    out.push(format!("schema: {}", f.schema));
    if let Some((w, h)) = f.size {
        out.push(format!("size: {} x {} pt", crate::pres::pt(w), crate::pres::pt(h)));
    }
    out.push("---".into());
    out
}

/// Blocks, a blank line between them.
fn blocks_in(out: &mut Vec<String>, blocks: &[Block], st: &StyleTable) {
    for (k, b) in blocks.iter().enumerate() {
        // Consecutive empty paragraphs are consecutive lines; a blank line
        // separates the group from other blocks.
        if k > 0 && !(is_empty_para(b) && is_empty_para(&blocks[k - 1])) {
            out.push(String::new());
        }
        block(out, b, st);
    }
}

/// [`blocks`]; `pres`: a Presentation slot's text (§5.3), whose `[text]{…}`
/// is formatting, each of its lines ending in `ends[block][line]` (a
/// paragraph's own formatting, ` {…}`) when there is one.
pub(crate) fn blocks_with(out: &mut Vec<String>, blocks: &[Block], pres: bool, ends: &[Vec<String>]) {
    let st = StyleTable::default();
    for (k, b) in blocks.iter().enumerate() {
        if k > 0 && !(is_empty_para(b) && is_empty_para(&blocks[k - 1])) {
            out.push(String::new());
        }
        let from = out.len();
        if pres {
            block_pres(out, b);
        } else {
            block(out, b, &st);
        }
        if let Some(e) = ends.get(k) {
            for (l, end) in out[from..].iter_mut().zip(e) {
                l.push_str(end);
            }
        }
    }
}

/// A Presentation slot's block: its text's formatting is `[text]{…}` spans.
fn block_pres(out: &mut Vec<String>, b: &Block) {
    match b {
        Block::Para(p) => out.push(para_body(p, serialize_inline_with(&p.content, true))),
        Block::List(items) => {
            let mut widths: Vec<usize> = vec![];
            for it in items {
                widths.truncate(it.level);
                let indent: usize = widths.iter().sum();
                let body = serialize_inline_with(&it.content, true);
                let line = if body.is_empty() { it.marker().to_string() } else { format!("{} {body}", it.marker()) };
                out.push(format!("{}{line}", " ".repeat(indent)));
                widths.push(it.marker().len() + 1);
            }
        }
        other => block(out, other, &StyleTable::default()),
    }
}

fn block(out: &mut Vec<String>, b: &Block, st: &StyleTable) {
    match b {
        Block::Para(p) => out.push(para_line(p)),
        Block::Table(t) => table(out, t, st),
        Block::Keep(k) => out.push(keep_tag(k)),
        Block::List(items) => {
            // Each item is indented to its parent's content column: the
            // widths of its ancestors' markers plus one space each.
            let mut widths: Vec<usize> = vec![];
            for it in items {
                widths.truncate(it.level);
                let indent: usize = widths.iter().sum();
                let (body, own) = with_own(&it.content, &it.props, &Props::new(), &Props::new());
                let mut line =
                    if body.is_empty() { it.marker().to_string() } else { format!("{} {body}", it.marker()) };
                let style = it.style.as_ref().map(|s| format!("style=\"{}\"", attr(s)));
                let brace: Vec<String> = style.into_iter().chain((!own.is_empty()).then(|| own.write())).collect();
                if !brace.is_empty() {
                    line.push_str(&format!(" {{{}}}", brace.join(" ")));
                }
                out.push(format!("{}{line}", " ".repeat(indent)));
                widths.push(it.marker().len() + 1);
            }
        }
        Block::PageBreak => out.push("<pagebreak/>".into()),
        Block::FootnoteDef(f) => {
            let body = serialize_inline(&f.content);
            out.push(if body.is_empty() { format!("[^{}]:", f.label) } else { format!("[^{}]: {body}", f.label) });
        }
    }
}

/// `<p/>` / `<p style="Name"/>` lines.
fn is_empty_para(b: &Block) -> bool {
    matches!(b, Block::Para(p) if p.content.is_empty() && !matches!(p.style, ParaStyle::Heading(_)))
}

fn p_tag(style: Option<&str>) -> String {
    match style {
        Some(s) => format!("<p style=\"{}\"/>", attr(s)),
        None => "<p/>".into(),
    }
}

/// A cell's paragraphs: the first one's text, then `<p/>` before each other
/// one. A leading plain `<p/>` is written only where it is needed (an empty
/// first paragraph followed by others).
pub(crate) fn cell_text(ps: &[CellPara]) -> String {
    cell_text_in(ps, &StyleTable::default(), &Props::new())
}

/// [`cell_text`]; `pres`: a Presentation shape's text, each paragraph
/// ending in `ends[paragraph]` (its own formatting, ` {…}`).
pub(crate) fn cell_text_with(ps: &[CellPara], pres: bool, ends: &[String]) -> String {
    if !pres {
        return cell_text(ps);
    }
    let mut s = String::new();
    for (k, p) in ps.iter().enumerate() {
        let lead_needed = k > 0 || p.style.is_some() || (p.content.is_empty() && ps.len() > 1);
        if lead_needed {
            s.push_str(&p_tag(p.style.as_deref()));
        }
        s.push_str(&serialize_inline_with(&p.content, true));
        if let Some(e) = ends.get(k) {
            s.push_str(e);
        }
    }
    s
}

/// [`cell_text`] with each paragraph's `{…}`: what differs from its style
/// under the table line's `tl`.
fn cell_text_in(ps: &[CellPara], st: &StyleTable, tl: &Props) -> String {
    let mut s = String::new();
    for (k, p) in ps.iter().enumerate() {
        let lead_needed = k > 0 || p.style.is_some() || (p.content.is_empty() && ps.len() > 1);
        if lead_needed {
            s.push_str(&p_tag(p.style.as_deref()));
        }
        let base = if tl.is_empty() { Props::new() } else { st.values(p.style.as_deref()) };
        let (body, own) = with_own(&p.content, &p.props, &base, tl);
        s.push_str(&body);
        if !own.is_empty() {
            s.push_str(&format!(" {{{}}}", own.write()));
        }
    }
    s
}

/// A paragraph's text and its `{…}`: `props` (its own keys) and `units`'
/// text keys are what differs from `base` (its style's values, or nothing
/// where only differences matter); `tl` is the table line's. The `{…}`
/// holds what differs from `base` under `tl`, and the text keys every unit
/// has; each unit's other text keys are a `[text]{…}`.
fn with_own(content: &Inline, props: &Props, base: &Props, tl: &Props) -> (String, Props) {
    if !styled::shows(content) {
        return (serialize_inline(content), Props::new());
    }
    let reference = base.overlay(tl);
    let mut own = base.overlay(props).only(&styled::PARA_OWN).diff(&reference.only(&styled::PARA_OWN));
    let text_base = base.only(&Key::TEXT);
    let eff = |u: &Unit| text_base.overlay(&u.props);
    let mut common: Option<Props> = None;
    for u in content.units.iter().filter(|u| u.takes_props()) {
        let e = eff(u);
        common = Some(match common {
            None => e,
            Some(mut c) => {
                c.0.retain(|k, v| e.get(*k) == Some(v));
                c
            }
        });
    }
    let text_ref = reference.only(&Key::TEXT);
    let lift = common.unwrap_or_default().diff(&text_ref);
    own = own.overlay(&lift);
    let span_ref = text_ref.overlay(&lift);
    (inline_spans(content, &|u| eff(u).diff(&span_ref)), own)
}

/// A table: its line, then its rows. Canonical form lifts what is shared
/// (§5.2): a box value more than half of the cells have is on the table
/// line, more than half of a row's (differing from the line's) on the row;
/// a layout or text value more than half of the cell paragraphs with text
/// set beyond their style is on the table line (a table of one cell has no
/// table line values). Each cell and paragraph writes what differs.
fn table(out: &mut Vec<String>, t: &Table, st: &StyleTable) {
    let dflt = styled::box_default();
    let text_cells: Vec<(usize, usize)> = t
        .rows
        .iter()
        .enumerate()
        .flat_map(|(r, row)| {
            row.iter().enumerate().filter(|(_, c)| matches!(c, Cell::Text(_))).map(move |(c, _)| (r, c))
        })
        .collect();
    let eff_box = |r: usize, c: usize| dflt.overlay(t.box_at(r, c));
    let lift = |cells: &[(usize, usize)], reference: &Props| {
        let mut out = Props::new();
        if cells.len() < 2 {
            return out;
        }
        for k in Key::BOX {
            let vals: Vec<Option<Value>> = cells.iter().map(|&(r, c)| eff_box(r, c).get(k).cloned()).collect();
            if let Some((Some(v), n)) = styled::mode(&vals) {
                if 2 * n > vals.len() && reference.get(k) != Some(&v) {
                    out.set(k, v);
                }
            }
        }
        out
    };
    let table_box = lift(&text_cells, &dflt);
    // The cell paragraphs' layout and text keys.
    let paras: Vec<(&CellPara, Props)> = t
        .rows
        .iter()
        .flatten()
        .filter_map(|c| if let Cell::Text(ps) = c { Some(ps) } else { None })
        .flatten()
        .filter(|p| styled::shows(&p.content))
        .map(|p| (p, st.values(p.style.as_deref())))
        .collect();
    let mut table_para = Props::new();
    if paras.len() >= 2 && text_cells.len() >= 2 {
        for k in styled::TABLE_PARA {
            let text = Key::TEXT.contains(&k);
            // A paragraph's value beyond its style; whether every one can state its own.
            let mut vals: Vec<Option<Value>> = vec![];
            let mut all_known = true;
            for (p, base) in &paras {
                if text {
                    let effs: Vec<Option<Value>> = p
                        .content
                        .units
                        .iter()
                        .filter(|u| u.takes_props())
                        .map(|u| u.props.get(k).or(base.get(k)).cloned())
                        .collect();
                    all_known &= effs.iter().all(Option::is_some);
                    let same = effs.windows(2).all(|w| w[0] == w[1]);
                    let v = effs.first().cloned().flatten().filter(|_| same);
                    vals.push(v.filter(|v| base.get(k) != Some(v)));
                } else {
                    all_known &= p.props.get(k).or(base.get(k)).is_some();
                    vals.push(p.props.get(k).cloned());
                }
            }
            if let Some((Some(v), n)) = styled::mode(&vals).filter(|_| all_known) {
                if 2 * n > vals.len() {
                    table_para.set(k, v);
                }
            }
        }
    }
    let head = table_box.overlay(&table_para);
    let style = t.style.as_ref().map(|s| format!("style=\"{}\"", attr(s)));
    let parts: Vec<String> = style.into_iter().chain((!head.is_empty()).then(|| head.write())).collect();
    if !parts.is_empty() {
        out.push(format!("{{{}}}", parts.join(" ")));
    }
    for (r, row) in t.rows.iter().enumerate() {
        let in_row: Vec<(usize, usize)> = text_cells.iter().copied().filter(|x| x.0 == r).collect();
        let line_ref = dflt.overlay(&table_box);
        let row_box = lift(&in_row, &line_ref);
        let cell_ref = line_ref.overlay(&row_box);
        let mut line = String::from("|");
        for (c, cell) in row.iter().enumerate() {
            match cell {
                Cell::Left => line.push('|'),
                Cell::Up => line.push_str(" ^^ |"),
                Cell::Text(ps) => {
                    let mut s = cell_text_in(ps, st, &table_para).replace('|', "\\|");
                    if s.trim() == "^^" {
                        let k = s.find('^').unwrap();
                        s.insert(k, '\\');
                    }
                    let own = eff_box(r, c).diff(&cell_ref);
                    if !own.is_empty() {
                        s = if s.is_empty() {
                            format!("{{{}}}", own.write())
                        } else {
                            format!("{{{}}} {s}", own.write())
                        };
                    }
                    line.push(' ');
                    line.push_str(&s);
                    line.push_str(" |");
                }
            }
        }
        if !row_box.is_empty() {
            line.push_str(&format!(" {{{}}}", row_box.write()));
        }
        out.push(line);
        if r == 0 {
            out.push(format!("|{}", "---|".repeat(row.len())));
        }
    }
}

pub(crate) fn para_line(p: &Para) -> String {
    let (body, own) = with_own(&p.content, &p.props, &Props::new(), &Props::new());
    let line = para_body(p, body);
    if own.is_empty() {
        line
    } else {
        format!("{line} {{{}}}", own.write())
    }
}

fn para_body(p: &Para, body: String) -> String {
    match &p.style {
        ParaStyle::Heading(n) => {
            let hashes = "#".repeat(*n as usize);
            if body.is_empty() {
                hashes
            } else {
                format!("{hashes} {body}")
            }
        }
        ParaStyle::Named(s) if body.is_empty() => p_tag(Some(s)),
        ParaStyle::Named(s) => format!("<div style=\"{}\">{body}</div>", attr(s)),
        // Spaces alone have no plain line form (`Document::normalize`).
        ParaStyle::Plain if body.is_empty() || p.content.spaces_only() => p_tag(None),
        ParaStyle::Plain => escape_line_start(body),
    }
}

/// A plain line must not read as another block.
fn escape_line_start(mut s: String) -> String {
    let lead = s.len() - s.trim_start().len();
    let t = &s[lead..];
    let (first, second) = (t.chars().next(), t.chars().nth(1));
    // A list marker, even indented: `-`, `+ `, `1.`, `1)` (the parser reads
    // indented ones as nested items).
    let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
    let numbered = digits > 0
        && matches!(t.chars().nth(digits), Some('.') | Some(')'))
        && matches!(t.chars().nth(digits + 1), Some(' ') | None);
    if numbered {
        s.insert(lead + digits, '\\');
        return s;
    }
    let bullet =
        (first == Some('-') && matches!(second, Some(' ') | None)) || (first == Some('+') && second == Some(' '));
    if first == Some('|') || bullet {
        s.insert(lead, '\\');
        return s;
    }
    let first = s.chars().next().unwrap();
    let thematic = matches!(first, '-' | '_') && s.len() >= 3 && s.trim_end().chars().all(|c| c == first || c == ' ');
    if first == '#' || first == '>' || thematic {
        s.insert(0, '\\');
    }
    s
}

pub fn serialize_inline(inl: &Inline) -> String {
    serialize_inline_with(inl, false)
}

/// [`serialize_inline`]; `pres`: a Presentation's slot or shape text, where
/// a style span is `[text]{…}` (§5.3).
pub fn serialize_inline_with(inl: &Inline, pres: bool) -> String {
    inline_spans_with(inl, &|u| u.props.clone(), pres)
}

/// Inline text, a unit's `want` (its text keys beyond what the paragraph
/// states) written as `[text]{…}`.
fn inline_spans(inl: &Inline, want: &dyn Fn(&Unit) -> Props) -> String {
    inline_spans_with(inl, want, false)
}

fn inline_spans_with(inl: &Inline, want: &dyn Fn(&Unit) -> Props, pres: bool) -> String {
    let mut out = Out { dollar: dollar_escapes(inl), pres, ..Out::default() };
    for (a, b, span) in inl.segments() {
        let spans = prop_spans(&inl.units, a, b, want);
        match span.map(|s| &s.kind) {
            Some(SpanKind::Style(st)) => {
                out.s.push('[');
                run(&mut out, &inl.units, a, b, true, &spans);
                out.s.push_str("]{");
                out.s.push_str(&st.attrs());
                out.s.push('}');
            }
            Some(SpanKind::Link(url)) => {
                out.s.push('[');
                run(&mut out, &inl.units, a, b, true, &spans);
                out.s.push_str("](");
                out.s.push_str(&link_url(url));
                out.s.push(')');
            }
            Some(SpanKind::Field(name)) => {
                out.s.push_str(&format!("<field name=\"{}\">", attr(name)));
                run(&mut out, &inl.units, a, b, false, &spans);
                out.s.push_str("</field>");
            }
            None => run(&mut out, &inl.units, a, b, false, &spans),
        }
    }
    out.s
}

/// The `[text]{…}` stretches of `units[a..b]`: maximal runs of units with
/// the same non-empty `want`. A space or atom (which states no properties)
/// is inside a stretch when the units around it are.
fn prop_spans(units: &[Unit], a: usize, b: usize, want: &dyn Fn(&Unit) -> Props) -> Vec<(usize, usize, Props)> {
    let mut w: Vec<Option<Props>> =
        (a..b).map(|i| units[i].takes_props().then(|| want(&units[i]).only(&Key::TEXT))).collect();
    for i in 0..w.len() {
        if w[i].is_some() {
            continue;
        }
        let prev = w[..i].iter().rev().find_map(|x| x.clone());
        let next = w[i + 1..].iter().find(|x| x.is_some()).cloned().flatten();
        w[i] = Some(match (prev, next) {
            (Some(p), Some(n)) if p == n => p,
            _ => Props::new(),
        });
    }
    let mut out: Vec<(usize, usize, Props)> = vec![];
    for (k, p) in w.into_iter().enumerate() {
        let p = p.unwrap_or_default();
        if p.is_empty() {
            continue;
        }
        match out.last_mut() {
            Some(l) if l.1 == a + k && l.2 == p => l.1 += 1,
            _ => out.push((a + k, a + k + 1, p)),
        }
    }
    out
}

/// Which literal `$` units need `\$`: those that could open math (a
/// non-space follows) while some later `$` in the output could close it.
/// Conservative, since delimiters, tags and urls also put characters
/// between units.
fn dollar_escapes(inl: &Inline) -> Vec<bool> {
    let u = &inl.units;
    let span_starts: Vec<usize> = inl.spans.iter().flat_map(|s| [s.start, s.end]).collect();
    let span_dollar = |j: usize| {
        inl.spans
            .iter()
            .any(|s| s.start >= j && matches!(&s.kind, SpanKind::Link(x) | SpanKind::Field(x) if x.contains('$')))
    };
    let closer_after = |i: usize| {
        (i + 1..u.len()).any(|j| match &u[j].atom {
            Atom::Char('$') => !u[j - 1].atom.is_space() || u[j - 1].marks != u[j].marks || span_starts.contains(&j),
            Atom::Math(_) => true,
            Atom::Keep(k) => (k.id.clone() + &k.kind + &k.summary).contains('$'),
            _ => false,
        }) || span_dollar(i + 1)
    };
    (0..u.len())
        .map(|i| {
            let opens = u
                .get(i + 1)
                .is_some_and(|n| !n.atom.is_space() || n.marks != u[i].marks || span_starts.contains(&(i + 1)));
            u[i].atom == Atom::Char('$') && opens && closer_after(i)
        })
        .collect()
}

#[derive(Default)]
struct Out {
    s: String,
    dollar: Vec<bool>,
    /// Byte index of a trailing unescaped literal `~`, if any.
    bare_tilde: Option<usize>,
    /// A Presentation's text: `]{` and ` {` would read as formatting.
    pres: bool,
}

impl Out {
    fn delim(&mut self, d: &str) {
        if d == "~~" {
            if let Some(k) = self.bare_tilde.take() {
                self.s.insert(k, '\\');
            }
        }
        self.bare_tilde = None;
        self.s.push_str(d);
    }
    fn text(&mut self, t: &str) {
        self.bare_tilde = None;
        self.s.push_str(t);
    }
    fn tilde(&mut self) {
        if self.s.ends_with('~') {
            self.text("\\~");
        } else {
            self.bare_tilde = Some(self.s.len());
            self.s.push('~');
        }
    }
}

/// One marked stretch `units[a..b]`: nested where possible (the
/// longest-lasting mark opens first), toggled where marks cross.
fn run(out: &mut Out, all: &[Unit], a: usize, b: usize, in_link: bool, spans: &[(usize, usize, Props)]) {
    let units = &all[a..b];
    let mut open: Vec<Marks> = vec![];
    let close_at = |at: usize| spans.iter().find(|s| s.1 == at).map(|s| format!("]{{{}}}", s.2.write()));
    let mut in_span = false;
    for i in 0..units.len() {
        let want = units[i].marks;
        for k in (0..open.len()).rev() {
            if !want.has(open[k]) {
                out.delim(closer(open[k]));
                open.remove(k);
            }
        }
        if let Some(c) = close_at(a + i) {
            out.text(&c);
            in_span = false;
        }
        if spans.iter().any(|s| s.0 == a + i) {
            out.text("[");
            in_span = true;
        }
        let mut new: Vec<Marks> = Marks::ALL.into_iter().filter(|&m| want.has(m) && !open.contains(&m)).collect();
        let end = |m: Marks| (i..units.len()).find(|&j| !units[j].marks.has(m)).unwrap_or(units.len());
        new.sort_by_key(|&m| std::cmp::Reverse(end(m)));
        for m in new {
            out.delim(delim(m));
            open.push(m);
        }
        unit(out, all, a + i, in_link || in_span);
    }
    for m in open.into_iter().rev() {
        out.delim(closer(m));
    }
    if let Some(c) = close_at(b) {
        out.text(&c);
    }
}

fn unit(out: &mut Out, units: &[Unit], i: usize, in_link: bool) {
    let next_char = match units.get(i + 1).map(|u| &u.atom) {
        Some(Atom::Char(c)) => Some(*c),
        _ => None,
    };
    match &units[i].atom {
        Atom::Char(c) => match c {
            '\\' => out.text("\\\\"),
            '*' => out.text("\\*"),
            '~' => out.tilde(),
            '<' if next_char.is_some_and(|c| c.is_ascii_alphabetic() || matches!(c, '/' | '!' | '?')) => {
                out.text("\\<")
            }
            '[' if in_link || next_char == Some('^') => out.text("\\["),
            '^' if in_link && out.s.ends_with('[') => out.text("\\^"),
            ']' if in_link || next_char == Some('(') || (out.pres && next_char == Some('{')) => out.text("\\]"),
            '$' if out.dollar[i] => out.text("\\$"),
            // A Document escapes every `{`; a Presentation's text, one that
            // could open a paragraph's formatting.
            '{' if !out.pres || out.s.chars().last().is_none_or(char::is_whitespace) => out.text("\\{"),
            c => {
                let mut b = [0u8; 4];
                out.text(c.encode_utf8(&mut b));
            }
        },
        Atom::Break => out.text("<br/>"),
        Atom::Keep(k) => out.text(&keep_tag(k)),
        Atom::NoteRef(l) => out.text(&format!("[^{l}]")),
        Atom::Math(m) => {
            let next_digit = next_char.is_some_and(|c| c.is_ascii_digit());
            if dollar_math_ok(m) && !next_digit {
                out.text(&format!("${m}$"))
            } else {
                out.text(&format!("<math>{m}</math>"))
            }
        }
        Atom::PageBreak => out.text("<pagebreak/>"),
    }
}

/// Whether `$m$` reads back as this math: `$` opens before a non-space and
/// closes after one, a closing `$` followed by a digit is text (`$5 and
/// $10`), and `$`, a trailing `\` or a `<p` inside would end or split it.
/// Otherwise the math is written `<math>m</math>`.
fn dollar_math_ok(m: &str) -> bool {
    let (Some(first), Some(last)) = (m.chars().next(), m.chars().last()) else { return false };
    !first.is_whitespace() && !last.is_whitespace() && last != '\\' && !m.contains('$') && !m.contains("<p")
}

pub(crate) fn keep_tag(k: &Keep) -> String {
    format!("<keep id=\"{}\" kind=\"{}\" summary=\"{}\"/>", attr(&k.id), attr(&k.kind), attr(&k.summary))
}

/// An attribute value as the text writes it between quotes.
pub fn attr_value(s: &str) -> String {
    attr(s)
}

pub(crate) fn attr(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;")
}

fn link_url(url: &str) -> String {
    let mut o = String::new();
    for c in url.chars() {
        match c {
            '\\' | ')' => {
                o.push('\\');
                o.push(c);
            }
            c if c.is_whitespace() => o.push_str(&format!("%{:02X}", c as u32)),
            c => o.push(c),
        }
    }
    o
}
