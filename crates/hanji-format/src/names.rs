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
    /// A presentation's layouts and the slots each has (§5.3).
    pub layouts: Option<Vec<Layout>>,
    /// A presentation's shapes that are not placeholders: `(id, name)`.
    pub shapes: Option<Vec<(String, String)>>,
    /// Ids of the placeholders (in `keeps`) that stand for a slide's objects:
    /// a `<keep/>` line of one of these is the object, not a slot's text.
    /// `None`: a `<keep/>` line outside a slot is an object.
    pub objects: Option<Vec<String>>,
}

/// A slide layout: its name and the slots its placeholders give, in order.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Layout {
    pub name: String,
    pub slots: Vec<String>,
}
