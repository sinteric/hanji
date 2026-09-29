//! Cell-range anchors (§4: "block, run range, slide, shape, cell range"): A1
//! addresses, ranges and range lists, and how a range moves when rows are
//! inserted into, deleted from or reordered in a span of columns — the
//! spreadsheet counterpart of re-anchoring by an exact edit span. An entry
//! anchored to a range lands at its new range, goes with its rows, or is
//! refused when the move would split it.

use std::fmt;

/// Most rows and columns a sheet holds (Excel 2007 and later).
pub const MAX_ROW: u32 = 1_048_576;
pub const MAX_COL: u32 = 16_384;

/// A cell address: 0-based column, 1-based row, as A1 writes them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellRef {
    pub col: u32,
    pub row: u32,
}

/// An inclusive rectangle of cells, `first` top-left and `last` bottom-right.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellRange {
    pub first: CellRef,
    pub last: CellRef,
}

/// Column letters for a 0-based column (`0` → `A`, `26` → `AA`).
pub fn col_letters(col: u32) -> String {
    let mut s = Vec::new();
    let mut n = col + 1;
    while n > 0 {
        let r = (n - 1) % 26;
        s.push(b'A' + r as u8);
        n = (n - 1) / 26;
    }
    s.reverse();
    String::from_utf8(s).unwrap()
}

/// The 0-based column of letters (`A` → 0), case-insensitive; `None` past `XFD`.
pub fn col_index(letters: &str) -> Option<u32> {
    if letters.is_empty() || letters.len() > 3 {
        return None;
    }
    let mut n: u32 = 0;
    for b in letters.bytes() {
        if !b.is_ascii_alphabetic() {
            return None;
        }
        n = n * 26 + (b.to_ascii_uppercase() - b'A' + 1) as u32;
    }
    (1..=MAX_COL).contains(&n).then(|| n - 1)
}

impl CellRef {
    pub fn new(col: u32, row: u32) -> CellRef {
        CellRef { col, row }
    }

    /// `B7` or `$B$7`.
    pub fn parse(s: &str) -> Option<CellRef> {
        let s = s.trim();
        let s = s.strip_prefix('$').unwrap_or(s);
        let k = s.find(|c: char| !c.is_ascii_alphabetic())?;
        let (letters, rest) = s.split_at(k);
        let rest = rest.strip_prefix('$').unwrap_or(rest);
        if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) || rest.starts_with('0') {
            return None;
        }
        let row: u32 = rest.parse().ok()?;
        (row <= MAX_ROW).then_some(CellRef { col: col_index(letters)?, row })
    }
}

impl fmt::Display for CellRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", col_letters(self.col), self.row)
    }
}

impl CellRange {
    pub fn new(first: CellRef, last: CellRef) -> CellRange {
        CellRange {
            first: CellRef::new(first.col.min(last.col), first.row.min(last.row)),
            last: CellRef::new(first.col.max(last.col), first.row.max(last.row)),
        }
    }

    pub fn cell(c: CellRef) -> CellRange {
        CellRange { first: c, last: c }
    }

    /// `A1`, `A1:C3` (either corner first) or with `$` signs.
    pub fn parse(s: &str) -> Option<CellRange> {
        match s.split_once(':') {
            Some((a, b)) => Some(CellRange::new(CellRef::parse(a)?, CellRef::parse(b)?)),
            None => CellRef::parse(s).map(CellRange::cell),
        }
    }

    pub fn rows(&self) -> u32 {
        self.last.row - self.first.row + 1
    }

    pub fn cols(&self) -> u32 {
        self.last.col - self.first.col + 1
    }

    pub fn contains(&self, c: CellRef) -> bool {
        (self.first.col..=self.last.col).contains(&c.col) && (self.first.row..=self.last.row).contains(&c.row)
    }

    pub fn intersects(&self, o: &CellRange) -> bool {
        self.first.col <= o.last.col
            && o.first.col <= self.last.col
            && self.first.row <= o.last.row
            && o.first.row <= self.last.row
    }

    /// The smallest range holding both.
    pub fn union(&self, o: &CellRange) -> CellRange {
        CellRange {
            first: CellRef::new(self.first.col.min(o.first.col), self.first.row.min(o.first.row)),
            last: CellRef::new(self.last.col.max(o.last.col), self.last.row.max(o.last.row)),
        }
    }
}

impl fmt::Display for CellRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.first == self.last {
            write!(f, "{}", self.first)
        } else {
            write!(f, "{}:{}", self.first, self.last)
        }
    }
}

/// A space-separated range list (`sqref`: `A1:B2 D4`).
pub fn parse_sqref(s: &str) -> Option<Vec<CellRange>> {
    let v: Option<Vec<CellRange>> = s.split_whitespace().map(CellRange::parse).collect();
    v.filter(|v| !v.is_empty())
}

pub fn write_sqref(ranges: &[CellRange]) -> String {
    ranges.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(" ")
}

/// Rows of a span of columns (`cols`, inclusive, 0-based) moving on one sheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowShift {
    /// `n` new rows at sheet row `at`; the rows from `at` down move down by `n`.
    Insert { cols: (u32, u32), at: u32, n: u32 },
    /// Rows `first..=last` go; the rows below move up.
    Delete { cols: (u32, u32), first: u32, last: u32 },
}

/// Where a range goes under a [`RowShift`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moved {
    /// Not touched.
    Same,
    /// Moved or resized.
    To(CellRange),
    /// Every row of it was deleted.
    Gone,
    /// It reaches outside the moving columns into rows that move: it would be
    /// torn in two.
    Split,
}

impl RowShift {
    pub fn cols(&self) -> (u32, u32) {
        match *self {
            RowShift::Insert { cols, .. } | RowShift::Delete { cols, .. } => cols,
        }
    }

    /// The first row that moves.
    pub fn from_row(&self) -> u32 {
        match *self {
            RowShift::Insert { at, .. } => at,
            RowShift::Delete { first, .. } => first,
        }
    }

    /// Rows of the sheet from `from_row` down change by this much (negative: up).
    pub fn delta(&self) -> i64 {
        match *self {
            RowShift::Insert { n, .. } => n as i64,
            RowShift::Delete { first, last, .. } => -((last - first + 1) as i64),
        }
    }

    fn in_cols(&self, c: u32) -> bool {
        let (a, b) = self.cols();
        (a..=b).contains(&c)
    }

    /// The new row of a cell at (`col`, `row`); `None` when it is deleted.
    pub fn row(&self, col: u32, row: u32) -> Option<u32> {
        if !self.in_cols(col) {
            return Some(row);
        }
        match *self {
            RowShift::Insert { at, n, .. } => Some(if row >= at { row + n } else { row }),
            RowShift::Delete { first, last, .. } => match row {
                r if r < first => Some(r),
                r if r <= last => None,
                r => Some(r - (last - first + 1)),
            },
        }
    }

    /// Where range `r` goes. A range inside the moving columns grows when
    /// rows are inserted inside it and shrinks when some of its rows go.
    pub fn range(&self, r: &CellRange) -> Moved {
        let (c0, c1) = self.cols();
        if r.last.col < c0 || r.first.col > c1 {
            return Moved::Same;
        }
        let inside = r.first.col >= c0 && r.last.col <= c1;
        if r.last.row < self.from_row() {
            return Moved::Same;
        }
        if !inside {
            return Moved::Split;
        }
        let at = |row| CellRef::new(r.first.col, row);
        let end = |row| CellRef::new(r.last.col, row);
        match *self {
            RowShift::Insert { at: a, n, .. } => {
                let first = if r.first.row >= a { r.first.row + n } else { r.first.row };
                let to = CellRange { first: at(first), last: end(r.last.row + n) };
                if to.last.row > MAX_ROW {
                    return Moved::Split;
                }
                Moved::To(to)
            }
            RowShift::Delete { first, last, .. } => {
                let cnt = last - first + 1;
                if r.first.row >= first && r.last.row <= last {
                    return Moved::Gone;
                }
                let nf = if r.first.row < first {
                    r.first.row
                } else if r.first.row > last {
                    r.first.row - cnt
                } else {
                    first
                };
                let nl = if r.last.row > last { r.last.row - cnt } else { first - 1 };
                Moved::To(CellRange { first: at(nf), last: end(nl) })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(s: &str) -> CellRange {
        CellRange::parse(s).unwrap()
    }

    #[test]
    fn addresses() {
        for (c, s) in [(0, "A"), (25, "Z"), (26, "AA"), (701, "ZZ"), (702, "AAA"), (16383, "XFD")] {
            assert_eq!(col_letters(c), s);
            assert_eq!(col_index(s), Some(c));
        }
        assert_eq!(col_index("XFE"), None);
        assert_eq!(CellRef::parse("$B$7"), Some(CellRef::new(1, 7)));
        assert_eq!(CellRef::parse("B0"), None);
        assert_eq!(CellRef::parse("B07"), None);
        assert_eq!(r("C3:A1").to_string(), "A1:C3");
        assert_eq!(parse_sqref("A1:B2 D4").map(|v| write_sqref(&v)).as_deref(), Some("A1:B2 D4"));
    }

    #[test]
    fn inserted_rows_move_grow_or_split() {
        let s = RowShift::Insert { cols: (0, 5), at: 10, n: 2 };
        assert_eq!(s.range(&r("A2:F9")), Moved::Same);
        assert_eq!(s.range(&r("B10:C12")), Moved::To(r("B12:C14")));
        assert_eq!(s.range(&r("A2:F20")), Moved::To(r("A2:F22")));
        assert_eq!(s.range(&r("H1:H30")), Moved::Same);
        assert_eq!(s.range(&r("E12:H12")), Moved::Split);
        assert_eq!(s.range(&r("E2:H3")), Moved::Same);
        assert_eq!(s.row(7, 40), Some(40));
        assert_eq!(s.row(3, 40), Some(42));
    }

    #[test]
    fn deleted_rows_take_ranges_or_shrink_them() {
        let s = RowShift::Delete { cols: (0, 5), first: 10, last: 12 };
        assert_eq!(s.range(&r("A10:F12")), Moved::Gone);
        assert_eq!(s.range(&r("A11")), Moved::Gone);
        assert_eq!(s.range(&r("A2:A20")), Moved::To(r("A2:A17")));
        assert_eq!(s.range(&r("A11:A20")), Moved::To(r("A10:A17")));
        assert_eq!(s.range(&r("A5:A11")), Moved::To(r("A5:A9")));
        assert_eq!(s.range(&r("A13:B14")), Moved::To(r("A10:B11")));
        assert_eq!(s.range(&r("F11:G11")), Moved::Split);
        assert_eq!(s.row(0, 11), None);
        assert_eq!(s.row(0, 13), Some(10));
    }
}
