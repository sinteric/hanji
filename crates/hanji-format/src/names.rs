use crate::ast::Keep;

/// Names a file allows. `None` skips that check.
#[derive(Clone, Debug, Default)]
pub struct Names {
    pub paragraph_styles: Option<Vec<String>>,
    pub table_styles: Option<Vec<String>>,
    pub fields: Option<Vec<String>>,
    /// The file's placeholders; the text may keep, move or delete them only.
    pub keeps: Option<Vec<Keep>>,
    pub formats: Option<Vec<String>>,
}
