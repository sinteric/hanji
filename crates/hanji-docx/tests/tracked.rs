//! Tracked-change export (§10.2) on small packages: the revision XML Word
//! writes, Accept All / Reject All back to the edited and imported text, and
//! the refusals.

mod review;

use hanji_core::{edit, reanchor_rewrite, Capabilities, Engine, EngineError, ImportOptions, Part};
use hanji_docx::{package, DocxEngine, ExportOptions, History, Reviewer};

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const CAPS: Capabilities = Capabilities { links: false, fields: false, footnotes: false, math: false };
const AUTHOR: &str = "hanji";

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style><w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/></w:style><w:style w:type="paragraph" w:styleId="Note"><w:name w:val="Note"/></w:style><w:style w:type="table" w:default="1" w:styleId="TableNormal"><w:name w:val="Normal Table"/></w:style></w:styles>"#;

fn part(name: &str, data: &str) -> Part {
    Part { name: name.into(), data: data.as_bytes().to_vec(), dos_time: 0x5b21_0000, external_attr: 0, deflate: true }
}

fn docx(body: &str) -> Vec<u8> {
    let doc = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="{W}"><w:body>{body}<w:sectPr/></w:body></w:document>"#
    );
    package::write(&[
        part(
            "[Content_Types].xml",
            r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#,
        ),
        part("word/document.xml", &doc),
        part("word/styles.xml", STYLES),
    ])
    .unwrap()
}

fn p(text: &str) -> String {
    format!("<w:p><w:r><w:t xml:space=\"preserve\">{text}</w:t></w:r></w:p>")
}

fn tracked() -> ExportOptions {
    ExportOptions { tracked_changes: Some(Reviewer { author: AUTHOR.into(), date: "2026-09-29T00:00:00Z".into() }) }
}

fn doc_xml(pkg: &[u8]) -> String {
    String::from_utf8(package::get(&package::read(pkg).unwrap(), "word/document.xml").unwrap().to_vec()).unwrap()
}

fn text(pkg: &[u8]) -> String {
    DocxEngine.import(pkg, &ImportOptions::default()).unwrap().text
}

/// Exact edits `(old, new)` in turn, exported as tracked changes; Accept All
/// and Reject All give the edited and the imported text back.
fn run(body: &str, edits: &[(&str, &str)]) -> Result<String, EngineError> {
    let pkg = docx(body);
    let imp = DocxEngine.import(&pkg, &ImportOptions::default()).unwrap();
    let mut h = History::new(&imp.text, &imp.remainder)?;
    let (mut t, mut rem) = (imp.text.clone(), imp.remainder.clone());
    for (old, new) in edits {
        let r = edit(&rem, &t, old, new, CAPS).unwrap_or_else(|e| panic!("{e}"));
        h.push(&r)?;
        (t, rem) = (r.text, r.remainder);
    }
    let out = DocxEngine.export_with(&t, &rem, &tracked(), Some(&h))?;
    assert!(review::count(&out, AUTHOR) > 0);
    assert_eq!(text(&review::resolve(&out, AUTHOR, true).unwrap()), t, "accept all");
    assert_eq!(text(&review::resolve(&out, AUTHOR, false).unwrap()), imp.text, "reject all");
    Ok(doc_xml(&out))
}

#[test]
fn a_replaced_word_is_a_deleted_run_and_an_inserted_one_with_their_formatting() {
    let body = r#"<w:p><w:r><w:rPr><w:color w:val="FF0000"/></w:rPr><w:t xml:space="preserve">매출 2023 증가</w:t></w:r></w:p>"#;
    let x = run(body, &[("2023", "2024")]).unwrap();
    let rpr = r#"<w:rPr><w:color w:val="FF0000"/></w:rPr>"#;
    assert!(x.contains(&format!(r#"<w:del w:id="1" w:author="hanji" w:date="2026-09-29T00:00:00Z"><w:r>{rpr}<w:delText xml:space="preserve">2023</w:delText></w:r></w:del>"#)), "{x}");
    assert!(x.contains(&format!(r#"<w:ins w:id="2" w:author="hanji" w:date="2026-09-29T00:00:00Z"><w:r>{rpr}<w:t xml:space="preserve">2024</w:t></w:r></w:ins>"#)), "{x}");
    assert!(x.contains(&format!("<w:r>{rpr}<w:t xml:space=\"preserve\">매출 </w:t></w:r>")), "{x}");
}

#[test]
fn an_inserted_paragraph_marks_the_mark_before_it_as_word_does() {
    let body = format!("{}{}", p("first"), p("second"));
    let x = run(&body, &[("first", "first\n\nnew one")]).unwrap();
    // The first paragraph's mark is the inserted one; the new text sits in
    // the paragraph that keeps the first one's mark.
    assert!(x.contains(r#"<w:p><w:pPr><w:rPr><w:ins w:id="1""#), "{x}");
    assert!(x.contains(">first</w:t></w:r></w:p><w:p><w:ins"), "{x}");
    assert!(x.contains(">new one</w:t></w:r></w:ins></w:p>"), "{x}");
}

#[test]
fn a_deleted_paragraph_has_its_runs_and_its_own_mark_deleted() {
    let body = format!("{}{}{}", p("keep"), p("gone"), p("last"));
    let x = run(&body, &[("gone\n\n", "")]).unwrap();
    assert!(x.contains(r#"<w:p><w:pPr><w:rPr><w:del w:id="2""#), "{x}");
    assert!(x.contains(r#"<w:delText xml:space="preserve">gone</w:delText>"#), "{x}");
    // Before the end of the document, the mark before it is the deleted one.
    let x = run(&format!("{}{}", p("keep"), p("gone")), &[("keep\n\ngone", "keep")]).unwrap();
    assert!(
        x.contains(r#"<w:p><w:pPr><w:rPr><w:del w:id="1""#)
            && x.contains("<w:t xml:space=\"preserve\">keep</w:t></w:r></w:p>"),
        "{x}"
    );
}

#[test]
fn a_restyle_is_a_paragraph_property_change_and_bold_a_run_property_change() {
    let body = format!("{}{}", p("title"), p("plain text"));
    let x = run(&body, &[("title", "# title"), ("plain text", "plain **text**")]).unwrap();
    assert!(x.contains(r#"<w:pPr><w:pStyle w:val="Heading1"/><w:pPrChange w:id="1" w:author="hanji" w:date="2026-09-29T00:00:00Z"><w:pPr><w:pStyle w:val="Normal"/></w:pPr></w:pPrChange></w:pPr>"#), "{x}");
    assert!(x.contains(r#"<w:rPr><w:b/><w:rPrChange w:id="2" w:author="hanji" w:date="2026-09-29T00:00:00Z"><w:rPr/></w:rPrChange></w:rPr><w:t xml:space="preserve">text</w:t>"#), "{x}");
}

#[test]
fn table_cells_track_and_edits_compose() {
    let cell = |t: &str| format!("<w:tc>{}</w:tc>", p(t));
    let body = format!(
        "{}<w:tbl><w:tblPr/><w:tblGrid><w:gridCol/><w:gridCol/></w:tblGrid><w:tr>{}{}</w:tr><w:tr>{}{}</w:tr></w:tbl>{}",
        p("before"),
        cell("지역"),
        cell("매출"),
        cell("서울"),
        cell("120"),
        p("after")
    );
    let x = run(&body, &[("| 120 |", "| 1205 |"), ("before", "before this"), ("this", "that")]).unwrap();
    assert!(x.contains("<w:tc><w:p><w:del") || x.contains("<w:t>120</w:t></w:r><w:ins"), "{x}");
    assert!(!x.contains("this"), "an insertion edited again is one insertion: {x}");
}

#[test]
fn off_is_export_and_on_needs_the_history() {
    let pkg = docx(&format!("{}{}", p("alpha"), p("beta")));
    let imp = DocxEngine.import(&pkg, &ImportOptions::default()).unwrap();
    let r = edit(&imp.remainder, &imp.text, "alpha", "gamma", CAPS).unwrap();
    let off = DocxEngine.export_with(&r.text, &r.remainder, &ExportOptions::default(), None).unwrap();
    assert_eq!(off, DocxEngine.export(&r.text, &r.remainder).unwrap());
    assert!(matches!(DocxEngine.export_with(&r.text, &r.remainder, &tracked(), None), Err(EngineError::Refused(_))));
    // An edit that does not start from the history's revision.
    let mut h = History::new(&imp.text, &imp.remainder).unwrap();
    h.push(&r).unwrap();
    assert!(h.push(&r).is_err());
}

#[test]
fn what_cannot_be_tracked_is_refused_with_the_reason() {
    let refused = |r: Result<String, EngineError>| match r {
        Err(EngineError::Refused(m)) => m,
        other => panic!("{other:?}"),
    };
    // Another author's tracked insertion, deleted: it would nest.
    let other = r#"<w:p><w:r><w:t xml:space="preserve">one </w:t></w:r><w:ins w:id="7" w:author="kim" w:date="2026-01-01T00:00:00Z"><w:r><w:t>x</w:t></w:r></w:ins><w:r><w:t xml:space="preserve"> two</w:t></w:r></w:p>"#;
    let pkg = docx(other);
    let imp = DocxEngine.import(&pkg, &ImportOptions::default()).unwrap();
    let keep = imp.text.lines().find(|l| l.contains("<keep")).unwrap().to_string();
    let m = refused(run(other, &[(keep.as_str(), "one two")]));
    assert!(m.contains("another author"), "{m}");
    // Text next to it is fine.
    run(other, &[(" two", " three")]).unwrap();
    // A field's result.
    let field = r#"<w:p><w:r><w:t xml:space="preserve">page </w:t></w:r><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText> PAGE </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r></w:p><w:p><w:r><w:t>3</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p>"#;
    run(field, &[("page ", "")]).unwrap();
    let m = refused(run(field, &[("3", "4")]));
    assert!(m.contains("field"), "{m}");
    // A rewrite that chooses among identical paragraphs.
    let body = format!("{}{}{}", p("same"), p("same"), p("same"));
    let pkg = docx(&body);
    let imp = DocxEngine.import(&pkg, &ImportOptions::default()).unwrap();
    let new = imp.text.replacen("same\n\n", "", 1);
    let r = reanchor_rewrite(&imp.remainder, &imp.text, &new, CAPS).unwrap();
    assert!(!r.alignment.ambiguous.is_empty());
    match History::new(&imp.text, &imp.remainder).unwrap().push(&r) {
        Err(EngineError::Refused(m)) => assert!(m.contains("exact edits"), "{m}"),
        other => panic!("{other:?}"),
    }
}
