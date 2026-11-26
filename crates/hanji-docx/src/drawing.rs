//! DOCX drawings whose inner text the model can edit: a text box
//! (`w:txbxContent` / VML `v:textbox`) whose content is ordinary text in
//! one paragraph is that text; geometry, anchoring, styling and the original
//! XML stay as a `Wrap`. Pictures, charts, groups, OLE previews, empty
//! shapes, and boxes that hold more than one paragraph, a table or nested
//! objects stay `<keep kind="drawing"/>`.

use hanji_core::{Entry, Kind};
use hanji_package::xml::{self, is_blank, Element, Node};

/// Locals a shown text-box paragraph may hold. Nested drawings, tables,
/// fields, notes, content controls and breaks stay placeholders.
const SHOWN: &[&str] = &[
    "p",
    "pPr",
    "r",
    "rPr",
    "t",
    "pStyle",
    "rStyle",
    "rFonts",
    "sz",
    "szCs",
    "lang",
    "b",
    "bCs",
    "i",
    "iCs",
    "u",
    "color",
    "shd",
    "spacing",
    "ind",
    "jc",
    "kern",
    "highlight",
    "vertAlign",
    "w",
    "position",
    "em",
    "vanish",
    "noProof",
    "snapToGrid",
    "smallCaps",
    "caps",
    "strike",
    "dstrike",
    "rtl",
    "cs",
    "keepNext",
    "keepLines",
    "widowControl",
    "autoSpaceDE",
    "autoSpaceDN",
    "adjustRightInd",
    "contextualSpacing",
    "overflowPunct",
    "bidi",
    "textAlignment",
    "outlineLvl",
    "framePr",
    "tabs",
    "wordWrap",
    "kinsoku",
    "topLinePunct",
    "fitText",
];

/// Compound drawings the text cannot show as one span of characters.
const COMPOUND: &[&str] = &["grpSp", "wgp", "group", "wpc", "lockedCanvas"];

/// A wrap whose XML is a drawing, a VML picture or an AlternateContent
/// that holds a text box.
pub(crate) fn is_drawing(e: &Entry) -> bool {
    e.kind == Kind::Wrap
        && matches!(
            e.meta.tag.as_str(),
            "w:drawing" | "w:pict" | "mc:AlternateContent"
        )
}

/// The body a drawing's text box can show, when that is ordinary one-line
/// text in a single paragraph.
pub fn shown(el: &Element) -> Option<String> {
    if !matches!(el.local(), "drawing" | "pict" | "AlternateContent") {
        return None;
    }
    let primary = primary(el);
    if compound(primary) {
        return None;
    }
    let txbx = txbx_content(primary)?;
    let para = one_para(txbx)?;
    para_text(para)
}

/// `xml` of a stored drawing, with every text box's `w:t` set to `body`.
/// Unchanged text keeps the original fragment (Choice and Fallback stay
/// as stored).
pub fn with_body(xml: &str, body: &str) -> String {
    let mut e = xml::fragment(xml);
    if shown(&e).as_deref() == Some(body) {
        return xml.to_string();
    }
    set_all_txbx(&mut e, body);
    e.to_xml()
}

/// A copy whose text-box runs are empty, so the wrap's fingerprint is the
/// object, not the words.
pub fn fingerprint_el(el: &Element) -> Element {
    let mut x = el.clone();
    x.walk_mut(&mut |e| {
        if e.local() == "t" {
            e.children.clear();
        }
    });
    x
}

fn primary(el: &Element) -> &Element {
    if el.local() != "AlternateContent" {
        return el;
    }
    el.elements()
        .find(|e| e.local() == "Choice")
        .and_then(|c| c.elements().next())
        .unwrap_or(el)
}

fn compound(el: &Element) -> bool {
    let mut found = false;
    el.walk(&mut |e| {
        if COMPOUND.contains(&e.local()) {
            found = true;
        }
    });
    found
}

fn txbx_content(el: &Element) -> Option<&Element> {
    let mut found = None;
    el.walk(&mut |e| {
        if found.is_none() && e.local() == "txbxContent" {
            found = Some(e);
        }
    });
    found
}

fn one_para(txbx: &Element) -> Option<&Element> {
    let mut para = None;
    for n in &txbx.children {
        match n {
            Node::El(e) if e.is("w:p") => {
                if para.is_some() {
                    return None;
                }
                para = Some(e);
            }
            n if is_blank(n) => {}
            _ => return None,
        }
    }
    para
}

fn para_text(p: &Element) -> Option<String> {
    let mut outer = true;
    let mut ok = true;
    p.walk(&mut |e| {
        if outer {
            outer = false;
            return;
        }
        if !SHOWN.contains(&e.local()) {
            ok = false;
        }
    });
    if !ok {
        return None;
    }
    let body = p.text_of(&["w:t"]);
    if body.is_empty() || body.contains('\n') || body.contains('\r') {
        return None;
    }
    Some(body)
}

fn set_all_txbx(e: &mut Element, body: &str) {
    if e.local() == "txbxContent" {
        set_txbx_text(e, body);
        return;
    }
    for c in e.elements_mut() {
        set_all_txbx(c, body);
    }
}

fn set_txbx_text(txbx: &mut Element, body: &str) {
    let mut used = false;
    set_t(txbx, body, &mut used);
    if used {
        return;
    }
    let mut t = Element::new("w:t");
    if needs_preserve(body) {
        t.set("xml:space", "preserve");
    }
    t.children.push(Node::Text(xml::escape_text(body)));
    let mut r = Element::new("w:r");
    r.children.push(Node::El(t));
    let mut p = Element::new("w:p");
    p.children.push(Node::El(r));
    txbx.children.push(Node::El(p));
}

fn set_t(e: &mut Element, body: &str, used: &mut bool) {
    if e.local() == "t" {
        if !*used {
            e.children = vec![Node::Text(xml::escape_text(body))];
            if needs_preserve(body) && e.attr("xml:space").is_none() {
                e.set("xml:space", "preserve");
            }
            *used = true;
        } else {
            e.children.clear();
        }
        return;
    }
    for c in e.elements_mut() {
        set_t(c, body, used);
    }
}

fn needs_preserve(s: &str) -> bool {
    s.trim() != s || s.contains("  ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use hanji_package::xml::fragment;

    fn box_xml(inner: &str) -> String {
        format!(
            r#"<mc:AlternateContent><mc:Choice Requires="wps"><w:drawing><wp:inline><wp:docPr id="1" name="Text Box 1"/><a:graphic><a:graphicData><wps:wsp><wps:txbx><w:txbxContent>{inner}</w:txbxContent></wps:txbx></wps:wsp></a:graphicData></a:graphic></wp:inline></w:drawing></mc:Choice><mc:Fallback><w:pict><v:shape><v:textbox><w:txbxContent>{inner}</w:txbxContent></v:textbox></v:shape></w:pict></mc:Fallback></mc:AlternateContent>"#
        )
    }

    fn p(text: &str) -> String {
        format!("<w:p><w:r><w:t>{text}</w:t></w:r></w:p>")
    }

    #[test]
    fn a_one_paragraph_text_box_is_shown() {
        let e = fragment(&box_xml(&p("Hello world")));
        assert_eq!(shown(&e).as_deref(), Some("Hello world"));
    }

    #[test]
    fn two_paragraphs_stay_unshown() {
        let e = fragment(&box_xml(&format!("{}{}", p("a"), p("b"))));
        assert!(shown(&e).is_none());
    }

    #[test]
    fn an_empty_box_stays_unshown() {
        let e = fragment(&box_xml("<w:p/>"));
        assert!(shown(&e).is_none());
    }

    #[test]
    fn a_table_inside_stays_unshown() {
        let e = fragment(&box_xml(
            "<w:tbl><w:tr><w:tc><w:p><w:r><w:t>x</w:t></w:r></w:p></w:tc></w:tr></w:tbl>",
        ));
        assert!(shown(&e).is_none());
    }

    #[test]
    fn a_picture_drawing_stays_unshown() {
        let e = fragment(
            r#"<w:drawing><wp:inline><wp:docPr id="1" name="Picture 1"/><a:graphic><a:graphicData><pic:pic><pic:blipFill><a:blip r:embed="rId1"/></pic:blipFill></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing>"#,
        );
        assert!(shown(&e).is_none());
    }

    #[test]
    fn with_body_keeps_the_fragment_when_the_text_is_unchanged() {
        let xml = box_xml(&p("Hello world"));
        assert_eq!(with_body(&xml, "Hello world"), xml);
    }

    #[test]
    fn with_body_rewrites_choice_and_fallback() {
        let xml = box_xml(&p("Hello world"));
        let out = with_body(&xml, "Hi");
        let e = fragment(&out);
        assert_eq!(shown(&e).as_deref(), Some("Hi"));
        assert_eq!(e.text_of(&["w:t"]).matches("Hi").count(), 2, "{out}");
        assert!(!out.contains("Hello world"), "{out}");
    }

    #[test]
    fn a_vml_textbox_is_shown() {
        let e = fragment(
            r#"<w:pict><v:shape><v:textbox><w:txbxContent><w:p><w:r><w:t>VML</w:t></w:r></w:p></w:txbxContent></v:textbox></v:shape></w:pict>"#,
        );
        assert_eq!(shown(&e).as_deref(), Some("VML"));
    }

    #[test]
    fn a_paragraph_with_ppr_is_shown() {
        let inner = concat!(
            "<w:p><w:pPr><w:pStyle w:val=\"Normal\"/><w:spacing w:before=\"0\"/></w:pPr>",
            "<w:r><w:rPr><w:sz w:val=\"24\"/></w:rPr><w:t>Here is a text box</w:t></w:r></w:p>",
        );
        let e = fragment(&box_xml(inner));
        assert_eq!(shown(&e).as_deref(), Some("Here is a text box"));
    }

    #[test]
    fn a_content_control_inside_stays_unshown() {
        let inner = concat!(
            "<w:p><w:sdt><w:sdtPr/><w:sdtContent>",
            "<w:r><w:t>x</w:t></w:r></w:sdtContent></w:sdt></w:p>",
        );
        assert!(shown(&fragment(&box_xml(inner))).is_none());
    }
}
