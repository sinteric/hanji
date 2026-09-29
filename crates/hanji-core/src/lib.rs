//! hanji core (DESIGN.md §4, §7, §10.3): the engine interface, the resolved
//! model (a Document's blocks, or a Presentation's slides, slots and shapes
//! as heads over blocks), the remainder store, and re-anchoring of the
//! remainder after an edit. No I/O.

pub mod diff;
pub mod edit;
pub mod engine;
pub mod lists;
pub mod model;
pub mod place;
pub mod presentation;
pub mod remainder;

pub use edit::{
    edit, edit_in, model_of, reanchor, reanchor_rewrite, reanchor_rewrite_in, reanchor_span, reanchor_span_in, rewrite,
    rewrite_in, DocumentModel, Reanchored, Refusal, Report, TextModel,
};
pub use engine::{notice, Engine, EngineError, ImportOptions, ImportReport, Imported, Notice};
pub use lists::{plan_lists, ListDefs, ListPlan};
pub use model::{Block, BlockSrc, Capabilities, Head, ListItem, Para, Path, SrcKind, StyleDef, StyleSet, Table};
pub use remainder::{Entry, KeepIds, Kind, Meta, Part, Remainder};
