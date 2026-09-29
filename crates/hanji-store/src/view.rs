//! Partial views (DESIGN.md §2 rule 11): a large file is read a part at a
//! time, by heading, slide range or line range, and a Spreadsheet's cells a
//! row window at a time. A part is the revision's own lines, so an edit can
//! copy `old` from it exactly.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::formats::DocType;

/// Most bytes one read returns; a larger file or window is cut at a line
/// (at a blank line where one is near) and says where to go on.
pub const MAX_READ_BYTES: usize = 60_000;

/// Rows a Spreadsheet read shows when no window is asked for.
pub const DEFAULT_ROWS: u32 = 50;

/// What part of a revision to read. At most one of `lines`, `section` and
/// `slides`; `table` (with `rows`) or `sheet` with `range` for cells.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Window {
    /// Line range, 1-based and inclusive: `"120:180"`, or `"120"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<String>,
    /// A Document's section: the text of its heading (`"3분기 실적"` or
    /// `"## 3분기 실적"`), up to the next heading of the same or a higher level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    /// A Presentation's slides, 1-based and inclusive: `"3:5"`, or `"3"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slides: Option<String>,
    /// A Spreadsheet table whose rows to show.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    /// Sheet rows of `table` to show: `"2:101"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<String>,
    /// A sheet whose `range` to show.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet: Option<String>,
    /// Cells of `sheet` to show: `"A1:F50"` (or `"Sheet1!A1:F50"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<String>,
}

impl Window {
    pub fn is_cells(&self) -> bool {
        self.table.is_some() || self.rows.is_some() || self.sheet.is_some() || self.range.is_some()
    }
}

/// A heading, slide, sheet or table line, for finding a part to read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OutlineEntry {
    pub line: usize,
    pub text: String,
}

/// `"a:b"`, `"a-b"` or `"a"` → 1-based inclusive `(a, b)`.
pub fn span(s: &str, what: &str) -> Result<(usize, usize)> {
    let bad = || Error::bad(format!("{what} {s:?} is not a range such as \"3:8\" or \"3\" (1-based, inclusive)."));
    let s = s.trim();
    let (a, b) = s.split_once([':', '-']).unwrap_or((s, s));
    let (a, b): (usize, usize) = (a.trim().parse().map_err(|_| bad())?, b.trim().parse().map_err(|_| bad())?);
    if a == 0 || b < a {
        return Err(bad());
    }
    Ok((a, b))
}

/// Index of the first line after the front matter.
fn body_start(ls: &[&str]) -> usize {
    if ls.first().map(|l| l.trim_end()) != Some("---") {
        return 0;
    }
    ls.iter().skip(1).position(|l| l.trim_end() == "---").map_or(0, |k| k + 2)
}

fn heading(line: &str) -> Option<(usize, &str)> {
    let t = line.trim_end();
    let n = t.chars().take_while(|&c| c == '#').count();
    ((1..=6).contains(&n) && t[n..].starts_with(' ')).then(|| (n, t[n + 1..].trim()))
}

/// Line index (0-based) where each slide starts: its `layout:` line.
fn slide_starts(ls: &[&str]) -> Vec<usize> {
    let from = body_start(ls);
    let mut out = vec![];
    let mut want = true;
    for (k, l) in ls.iter().enumerate().skip(from) {
        let t = l.trim_end();
        if t == "---" {
            want = true;
        } else if want && !t.trim().is_empty() {
            out.push(k);
            want = false;
        }
    }
    out
}

/// The headings, slides, or sheets and tables of a text.
pub fn outline(ty: DocType, text: &str) -> Vec<OutlineEntry> {
    let ls: Vec<&str> = text.lines().collect();
    let entry = |k: usize, t: String| OutlineEntry { line: k + 1, text: t };
    match ty {
        DocType::Document => {
            ls.iter().enumerate().filter(|(_, l)| heading(l).is_some()).map(|(k, l)| entry(k, l.to_string())).collect()
        }
        DocType::Presentation => slide_starts(&ls)
            .iter()
            .enumerate()
            .map(|(n, &k)| {
                let end = ls[k..].iter().position(|l| l.trim_end() == "---").map_or(ls.len(), |e| k + e);
                let title = ls[k..end]
                    .iter()
                    .position(|l| {
                        let t = l.trim();
                        t == "::title::" || (t.starts_with("::title ") && t.ends_with("::"))
                    })
                    .and_then(|t| ls.get(k + t + 1))
                    .map(|t| format!(" · {}", t.trim()))
                    .unwrap_or_default();
                entry(k, format!("slide {}: {}{title}", n + 1, ls[k].trim()))
            })
            .collect(),
        DocType::Spreadsheet => ls
            .iter()
            .enumerate()
            .filter(|(_, l)| l.starts_with("<sheet ") || l.starts_with("<table "))
            .map(|(k, l)| entry(k, l.to_string()))
            .collect(),
    }
}

/// The 0-based line range `[a, b)` a window asks for.
pub fn part(ty: DocType, text: &str, w: &Window) -> Result<Option<(usize, usize)>> {
    let asked = [w.lines.is_some(), w.section.is_some(), w.slides.is_some()].iter().filter(|x| **x).count();
    if asked > 1 {
        return Err(Error::bad("ask for one of lines, section and slides."));
    }
    let ls: Vec<&str> = text.split_inclusive('\n').collect();
    let n = ls.len();
    if let Some(s) = &w.lines {
        let (a, b) = span(s, "lines")?;
        if a > n {
            return Err(Error::bad(format!("line {a} is past the end: the revision has {n} lines.")));
        }
        return Ok(Some((a - 1, b.min(n))));
    }
    if let Some(name) = &w.section {
        if ty != DocType::Document {
            return Err(Error::bad("sections are a Document's; read a Presentation by slides, or any text by lines."));
        }
        let want = heading(name).map_or(name.trim(), |h| h.1);
        let hits: Vec<(usize, usize)> =
            ls.iter().enumerate().filter_map(|(k, l)| heading(l).filter(|h| h.1 == want).map(|h| (k, h.0))).collect();
        let (k, level) = match hits.as_slice() {
            [one] => *one,
            [] => {
                let all: Vec<String> =
                    outline(ty, text).iter().map(|e| format!("{} (line {})", e.text, e.line)).collect();
                let mut e = Error::not_found(format!(
                    "no heading reads {want:?}. The headings are: {}.",
                    if all.is_empty() { "none (read by lines)".into() } else { all.join("; ") }
                ));
                e.detail.matches = Some(0);
                return Err(e);
            }
            many => {
                let mut e = Error::new(
                    crate::error::Code::AmbiguousMatch,
                    format!("{} headings read {want:?}; read one by lines.", many.len()),
                );
                e.detail.matches = Some(many.len());
                e.detail.lines = many.iter().map(|h| h.0 + 1).collect();
                return Err(e);
            }
        };
        let end = ls.iter().enumerate().skip(k + 1).find(|(_, l)| heading(l).is_some_and(|h| h.0 <= level));
        return Ok(Some((k, end.map_or(n, |x| x.0))));
    }
    if let Some(s) = &w.slides {
        if ty != DocType::Presentation {
            return Err(Error::bad("slides are a Presentation's; read a Document by section or lines."));
        }
        let (a, b) = span(s, "slides")?;
        let starts = slide_starts(&ls);
        if a > starts.len() {
            return Err(Error::bad(format!("slide {a} is past the end: the deck has {} slides.", starts.len())));
        }
        let b = b.min(starts.len());
        let end = starts.get(b).map_or(n, |&next| {
            // Up to the separator before the next slide.
            (starts[b - 1]..next).rev().find(|&k| ls[k].trim_end() == "---").unwrap_or(next)
        });
        return Ok(Some((starts[a - 1], end)));
    }
    Ok(None)
}

/// Cut lines `[a, b)` to at most `limit` bytes, ending at a blank line when one
/// is in the last quarter: the new end.
pub fn clip(ls: &[&str], a: usize, b: usize, limit: usize) -> usize {
    let mut size = 0;
    for k in a..b {
        size += ls[k].len();
        if size > limit {
            let floor = a + ((k - a) * 3 / 4);
            let blank = (floor..k).rev().find(|&j| ls[j].trim().is_empty()).map(|j| j + 1);
            return blank.unwrap_or(k.max(a + 1));
        }
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "---\ntype: document\nformat: docx\nschema: 1\n---\n# A\na\n\n## A1\nx\n\n# B\nb\n";
    const DECK: &str = "---\ntype: presentation\nformat: pptx\nschema: 1\n---\n\nlayout: Title Slide\n::title::\n하나\n\n---\n\nlayout: Title and Content\n::title::\n둘\n::body::\n- x\n\n---\n\nlayout: Blank\n";

    fn lines_of(ty: DocType, text: &str, w: Window) -> String {
        let (a, b) = part(ty, text, &w).unwrap().unwrap();
        text.split_inclusive('\n').skip(a).take(b - a).collect()
    }

    #[test]
    fn sections_slides_and_lines() {
        let sec = |s: &str| Window { section: Some(s.into()), ..Default::default() };
        assert_eq!(lines_of(DocType::Document, DOC, sec("A")), "# A\na\n\n## A1\nx\n\n");
        assert_eq!(lines_of(DocType::Document, DOC, sec("## A1")), "## A1\nx\n\n");
        assert_eq!(lines_of(DocType::Document, DOC, sec("B")), "# B\nb\n");
        assert!(part(DocType::Document, DOC, &sec("C")).unwrap_err().message.contains("# A (line 6)"));
        let sl = |s: &str| Window { slides: Some(s.into()), ..Default::default() };
        assert_eq!(lines_of(DocType::Presentation, DECK, sl("1")), "layout: Title Slide\n::title::\n하나\n\n");
        assert_eq!(
            lines_of(DocType::Presentation, DECK, sl("2:3")),
            "layout: Title and Content\n::title::\n둘\n::body::\n- x\n\n---\n\nlayout: Blank\n"
        );
        let o = outline(DocType::Presentation, DECK);
        assert_eq!(o[1].text, "slide 2: layout: Title and Content · 둘");
        let ln = Window { lines: Some("6:7".into()), ..Default::default() };
        assert_eq!(lines_of(DocType::Document, DOC, ln), "# A\na\n");
        assert!(span("0:3", "lines").is_err() && span("5:3", "lines").is_err());
    }

    #[test]
    fn clipping_prefers_a_blank_line() {
        let text: String = (0..100).map(|k| format!("paragraph {k:03}\n\n")).collect();
        let ls: Vec<&str> = text.split_inclusive('\n').collect();
        let end = clip(&ls, 0, 200, 300);
        assert!(end < 200);
        let kept: String = text.split_inclusive('\n').take(end).collect();
        assert!(kept.len() <= 300 && kept.ends_with("\n\n"), "{kept:?}");
    }
}
