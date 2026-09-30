//! The hanji text format for Documents, Presentations and Spreadsheets
//! (DESIGN.md §5.1–5.4):
//! a typed AST, a parser with a source map, a canonical serializer, and
//! validator errors written for the model. No I/O.

pub mod ast;
pub mod chars;
pub mod diag;
pub mod formula;
pub mod names;
pub mod ops;
pub mod parse;
pub mod pres;
pub mod presets;
pub mod props;
pub mod serialize;
pub mod sheet;
pub mod styled;
pub mod vocab;

pub use ast::*;
pub use diag::Diagnostic;
pub use names::{Layout, Names};
pub use parse::{merge_problem, parse, parse_with, BlockMap, BlockMapKind, ParaMap, Parsed, SourceMap};
pub use pres::{
    parse_presentation, serialize_presentation, HeadMap, ItemMap, ParsedPresentation, PresentationMap, SlideMap,
};
pub use serialize::{serialize, serialize_inline};
pub use sheet::{parse_spreadsheet, serialize_spreadsheet, window_text, Spreadsheet, WindowOf};

/// Schema version this crate reads and writes.
pub const SCHEMA_VERSION: u32 = 1;
