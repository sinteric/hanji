use crate::ast::{Keep, StyleLine};

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
    /// A flow document's paragraph styles as style lines (§5.2), the default
    /// style's first: the values of a style the text uses without a line.
    pub style_lines: Option<Vec<StyleLine>>,
    /// The paragraph styles `#` to `######` stand for.
    pub headings: [Option<String>; 6],
    /// The style of a list item that names none; `None`: the default style.
    pub item_style: Option<String>,
}

/// A slide layout: its name and the slots its placeholders give, in order.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Layout {
    pub name: String,
    pub slots: Vec<String>,
    /// Its type code (pptx: `p:sldLayout@type`, such as `objTx`), which
    /// `layout:` may give instead of the name when no layout has that name and
    /// exactly one has that type (§5.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ty: Option<String>,
}
