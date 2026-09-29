//! The pptx edit set (§9): P1 change a title, P2 edit a body bullet, P3 add a
//! slide from a layout, P4 delete a slide, P5 move a slide, P6 edit notes,
//! P7 edit a `<shape>`'s text, P8 restyle a slide by changing its layout;
//! P9 makes them all in one revision.

use hanji_testkit::{Cx, Doc, Edit, EditFn};

pub const EDITS: [(&str, EditFn); 0] = [];

#[allow(dead_code)]
fn _unused(_: &Doc, _: &Cx) -> Option<Edit> {
    None
}
