//! The hanji text format for Documents and Presentations (DESIGN.md §5.1–5.3):
//! a typed AST, a parser with a source map, a canonical serializer, and
//! validator errors written for the model. No I/O.

pub mod ast;
pub mod diag;
pub mod names;
pub mod parse;
pub mod pres;
pub mod serialize;

pub use ast::*;
pub use diag::Diagnostic;
pub use names::{Layout, Names};
pub use parse::{merge_problem, parse, parse_with, BlockMap, BlockMapKind, ParaMap, Parsed, SourceMap};
pub use pres::{
    parse_presentation, serialize_presentation, HeadMap, ItemMap, ParsedPresentation, PresentationMap, SlideMap,
};
pub use serialize::{serialize, serialize_inline};

/// Schema version this crate reads and writes.
pub const SCHEMA_VERSION: u32 = 1;
