//! A cell's value as the model sees it.

/// What a cell holds (a formula cell: its computed value).
#[derive(Clone, Debug, PartialEq)]
pub enum CellValue {
    Empty,
    Number(f64),
    Text(String),
    Bool(bool),
    /// `#DIV/0!`, `#N/A`, …
    Error(String),
}

impl CellValue {
    pub fn is_empty(&self) -> bool {
        matches!(self, CellValue::Empty)
    }
}
