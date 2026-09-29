//! Structure edits.

use hanji_format::sheet::Spreadsheet;

/// Whether two structures are the same (front matter aside).
pub fn same(a: &Spreadsheet, b: &Spreadsheet) -> bool {
    a.sheets == b.sheets
}
