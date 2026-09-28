//! The hanji text format for Documents (DESIGN.md §5.1, §5.2): a typed AST,
//! a parser with a source map, a canonical serializer, and validator errors
//! written for the model. No I/O.
//!
//! Extension points held open by DESIGN.md §10.8 (undecided): empty
//! paragraphs have no Markdown form (engines write `<div style="Normal"></div>`),
//! and multi-paragraph table cells have no pipe-table form (engines keep such
//! a table whole as a block `<keep/>`).

pub mod ast;
pub mod diag;
pub mod names;
pub mod parse;
pub mod serialize;

pub use ast::*;
pub use diag::Diagnostic;
pub use names::Names;
pub use parse::{parse, parse_with, BlockMap, BlockMapKind, ParaMap, Parsed, SourceMap};
pub use serialize::{serialize, serialize_inline};

/// Schema version this crate reads and writes.
pub const SCHEMA_VERSION: u32 = 1;
