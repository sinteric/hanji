//! Canonical serializer (§5.1): one paragraph per line, no table padding,
//! fixed attribute order, boundary spaces outside emphasis markers, and
//! escapes only where the parser would otherwise misread a character.

use crate::ast::*;
use crate::parse::{closer, delim};

pub fn serialize(doc: &Document) -> String {
    let f = &doc.front;
    let mut out = vec!["---".to_string(), format!("type: {}", f.doc_type), format!("format: {}", f.format)];
    if let Some(t) = &f.template {
        out.push(format!("template: {t}"));
    }
    out.push(format!("schema: {}", f.schema));
    out.push("---".into());
    for (k, b) in doc.blocks.iter().enumerate() {
        // Consecutive empty paragraphs are consecutive lines; a blank line
        // separates the group from other blocks.
        if k > 0 && !(is_empty_para(b) && is_empty_para(&doc.blocks[k - 1])) {
            out.push(String::new());
        }
        block(&mut out, b);
    }
    out.join("\n") + "\n"
}

fn block(out: &mut Vec<String>, b: &Block) {
    match b {
        Block::Para(p) => out.push(para_line(p)),
        Block::Table(t) => {
            if let Some(s) = &t.style {
                out.push(format!("{{style=\"{}\"}}", attr(s)));
            }
            for (r, row) in t.rows.iter().enumerate() {
                let mut line = String::from("|");
                for c in row {
                    match c {
                        Cell::Left => line.push('|'),
                        Cell::Up => line.push_str(" ^^ |"),
                        Cell::Text(ps) => {
                            let mut s = cell_text(ps).replace('|', "\\|");
                            if s.trim() == "^^" {
                                let k = s.find('^').unwrap();
                                s.insert(k, '\\');
                            }
                            line.push(' ');
                            line.push_str(&s);
                            line.push_str(" |");
                        }
                    }
                }
                out.push(line);
                if r == 0 {
                    out.push(format!("|{}", "---|".repeat(row.len())));
                }
            }
        }
        Block::Keep(k) => out.push(keep_tag(k)),
        Block::List(items) => {
            // Each item is indented to its parent's content column: the
            // widths of its ancestors' markers plus one space each.
            let mut widths: Vec<usize> = vec![];
            for it in items {
                widths.truncate(it.level);
                let indent: usize = widths.iter().sum();
                let body = serialize_inline(&it.content);
                let line = if body.is_empty() { it.marker().to_string() } else { format!("{} {body}", it.marker()) };
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
fn cell_text(ps: &[CellPara]) -> String {
    let mut s = String::new();
    for (k, p) in ps.iter().enumerate() {
        let lead_needed = k > 0 || p.style.is_some() || (p.content.is_empty() && ps.len() > 1);
        if lead_needed {
            s.push_str(&p_tag(p.style.as_deref()));
        }
        s.push_str(&serialize_inline(&p.content));
    }
    s
}

fn para_line(p: &Para) -> String {
    let body = serialize_inline(&p.content);
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
    if matches!(first, Some('|') | Some('{')) || bullet {
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
    let mut out = Out { dollar: dollar_escapes(inl), ..Out::default() };
    for (a, b, span) in inl.segments() {
        match span.map(|s| &s.kind) {
            Some(SpanKind::Link(url)) => {
                out.s.push('[');
                run(&mut out, &inl.units, a, b, true);
                out.s.push_str("](");
                out.s.push_str(&link_url(url));
                out.s.push(')');
            }
            Some(SpanKind::Field(name)) => {
                out.s.push_str(&format!("<field name=\"{}\">", attr(name)));
                run(&mut out, &inl.units, a, b, false);
                out.s.push_str("</field>");
            }
            None => run(&mut out, &inl.units, a, b, false),
        }
    }
    out.s
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
fn run(out: &mut Out, all: &[Unit], a: usize, b: usize, in_link: bool) {
    let units = &all[a..b];
    let mut open: Vec<Marks> = vec![];
    for i in 0..units.len() {
        let want = units[i].marks;
        for k in (0..open.len()).rev() {
            if !want.has(open[k]) {
                out.delim(closer(open[k]));
                open.remove(k);
            }
        }
        let mut new: Vec<Marks> = Marks::ALL.into_iter().filter(|&m| want.has(m) && !open.contains(&m)).collect();
        let end = |m: Marks| (i..units.len()).find(|&j| !units[j].marks.has(m)).unwrap_or(units.len());
        new.sort_by_key(|&m| std::cmp::Reverse(end(m)));
        for m in new {
            out.delim(delim(m));
            open.push(m);
        }
        unit(out, all, a + i, in_link);
    }
    for m in open.into_iter().rev() {
        out.delim(closer(m));
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
            ']' if in_link || next_char == Some('(') => out.text("\\]"),
            '$' if out.dollar[i] => out.text("\\$"),
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

fn attr(s: &str) -> String {
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
