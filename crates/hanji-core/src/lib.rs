//! hanji core (DESIGN.md §4, §7, §10.3): the engine interface, the resolved
//! model, the remainder store, and re-anchoring of the remainder after an
//! edit. No I/O.

pub mod diff;
pub mod edit;
pub mod engine;
pub mod lists;
pub mod model;
pub mod place;
pub mod remainder;

pub use edit::{edit, model_of, reanchor, reanchor_rewrite, reanchor_span, rewrite, Reanchored, Refusal, Report};
pub use engine::{Engine, EngineError, ImportOptions, ImportReport, Imported, Notice};
pub use lists::{plan_lists, ListDefs, ListPlan};
pub use model::{Block, Capabilities, ListItem, Para, Path, StyleDef, StyleSet, Table};
pub use remainder::{Entry, KeepIds, Kind, Meta, Part, Remainder};
