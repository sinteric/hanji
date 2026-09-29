//! Range operations.

use hanji_core::{EngineError, Remainder};

#[derive(Clone, Debug, Default)]
pub struct OpReport {}

pub fn apply(_text: &str, _rem: &Remainder, _ops: &str) -> Result<crate::Applied, EngineError> {
    Err(EngineError::Unsupported("range operations"))
}
