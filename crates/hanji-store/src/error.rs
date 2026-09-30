//! Errors written for the model (DESIGN.md §5.1): what went wrong, where
//! (line and column), what was expected, and what to do next. A refusal
//! (§2 rule 1) is an error with its reasons, never a silent success.

use std::fmt;

use hanji_core::{EngineError, Kind, Notice, Refusal};
use hanji_format::chars::name_in;
use hanji_format::Diagnostic;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Code {
    /// No such document, revision or file.
    NotFound,
    /// The request itself is malformed (a window, an argument).
    BadRequest,
    /// The edit names a revision that is not the current one, and a model
    /// edit came after it: read again.
    StaleRevision,
    /// `old` does not occur in the revision.
    NoMatch,
    /// `old` occurs more than once.
    AmbiguousMatch,
    /// The text is not valid; see `diagnostics`.
    Invalid,
    /// The engine cannot write what the text asks for; nothing is dropped.
    Refused,
    /// The edit would lose content the text does not show; see `lost`.
    Unplaceable,
    /// The edit and a person's re-imported edits change the same lines.
    MergeConflict,
    /// The export would carry content to surface first; see `surfaced`.
    SurfacedNotAcknowledged,
    /// The package cannot be read.
    Package,
    Unsupported,
    /// Reading or writing a file or the store failed.
    Io,
}

/// A remainder entry an edit would lose.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Loss {
    pub kind: String,
    pub reason: String,
}

impl Loss {
    pub fn of(kind: Kind, reason: &str) -> Loss {
        Loss { kind: format!("{kind:?}").to_lowercase(), reason: name_in(reason).into_owned() }
    }
}

/// A §8 item: neutralised on import, or to surface before export.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Item {
    pub kind: String,
    pub location: String,
    pub detail: String,
}

impl From<&Notice> for Item {
    fn from(n: &Notice) -> Item {
        Item { kind: n.kind.clone(), location: n.location.clone(), detail: n.detail.clone() }
    }
}

pub fn items(ns: &[Notice]) -> Vec<Item> {
    ns.iter().map(Item::from).collect()
}

/// A validator error: 1-based line and column (in characters).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Diag {
    pub line: usize,
    pub col: usize,
    pub message: String,
}

pub fn diags(ds: &[Diagnostic]) -> Vec<Diag> {
    ds.iter().map(|d| Diag { line: d.line, col: d.col, message: name_in(&d.message).into_owned() }).collect()
}

/// Lines of a revision the edit and a person's edits both change.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Conflict {
    /// 1-based, inclusive, in the revision the edit was made against.
    pub first_line: usize,
    pub last_line: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Error {
    pub code: Code,
    pub message: String,
    #[serde(flatten)]
    pub detail: Box<Detail>,
}

/// What an error points at, beyond its message.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Detail {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diag>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub lost: Vec<Loss>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub surfaced: Vec<Item>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<Conflict>,
    /// How often `old` occurs (`no_match`: 0, `ambiguous_match`: 2 or more).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matches: Option<usize>,
    /// Lines where `old` occurs (or, for `no_match`, where its first line does).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<usize>,
    /// The failing edit of a list (1-based).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edit: Option<usize>,
    /// The document's current revision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head: Option<u32>,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Unusual characters in `message` are named by code point
    /// ([`hanji_format::chars::name_in`]).
    pub fn new(code: Code, message: impl Into<String>) -> Error {
        let message = message.into();
        let message = match name_in(&message) {
            std::borrow::Cow::Borrowed(_) => message,
            std::borrow::Cow::Owned(m) => m,
        };
        Error { code, message, detail: Box::default() }
    }

    pub fn bad(message: impl Into<String>) -> Error {
        Error::new(Code::BadRequest, message)
    }

    pub fn not_found(message: impl Into<String>) -> Error {
        Error::new(Code::NotFound, message)
    }

    pub fn io(message: impl Into<String>) -> Error {
        Error::new(Code::Io, message)
    }

    pub fn invalid(ds: &[Diagnostic]) -> Error {
        let mut e = Error::new(Code::Invalid, format!("the text is not valid:\n{}", hanji_format::diag::render(ds)));
        e.detail.diagnostics = diags(ds);
        e
    }

    /// The failing edit `k` (0-based) of a list of `n`.
    pub fn at_edit(mut self, k: usize, n: usize) -> Error {
        if n > 1 {
            self.detail.edit = Some(k + 1);
            self.message = format!("edit {} of {n}: {}", k + 1, self.message);
        }
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

impl From<EngineError> for Error {
    fn from(e: EngineError) -> Error {
        match e {
            EngineError::Invalid(d) => Error::invalid(&d),
            EngineError::Package(m) => Error::new(Code::Package, format!("the package cannot be read: {m}")),
            EngineError::Refused(m) => Error::new(Code::Refused, format!("refused: {m}")),
            EngineError::Unsupported(m) => Error::new(Code::Unsupported, format!("not supported: {m}")),
        }
    }
}

impl From<Refusal> for Error {
    fn from(r: Refusal) -> Error {
        match r {
            Refusal::Invalid(d) => Error::invalid(&d),
            Refusal::Edit(m) => Error::bad(m),
            Refusal::Unplaceable(rep) => {
                let mut e = Error::new(Code::Unplaceable, Refusal::Unplaceable(rep.clone()).to_string());
                e.detail.lost = rep.refused.iter().map(|(_, k, why)| Loss::of(*k, why)).collect();
                e
            }
        }
    }
}
