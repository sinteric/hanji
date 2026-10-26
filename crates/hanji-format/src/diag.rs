use std::fmt;

/// A validator error written for the model: where, what was expected, and
/// the allowed names when a list applies.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Diagnostic {
    /// 1-based line.
    pub line: usize,
    /// 1-based column, in characters.
    pub col: usize,
    pub message: String,
}

impl Diagnostic {
    pub fn new(line: usize, col: usize, message: impl Into<String>) -> Diagnostic {
        Diagnostic {
            line,
            col,
            message: message.into(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {}, column {}: {}",
            self.line, self.col, self.message
        )
    }
}

impl std::error::Error for Diagnostic {}

/// All diagnostics, one per line, unusual characters named
/// ([`crate::chars::name_in`]).
pub fn render(diags: &[Diagnostic]) -> String {
    let all = diags
        .iter()
        .map(|d| d.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    crate::chars::name_in(&all).into_owned()
}

pub(crate) fn quoted(names: &[String]) -> String {
    names
        .iter()
        .map(|n| format!("\"{n}\""))
        .collect::<Vec<_>>()
        .join(", ")
}
