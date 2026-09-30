//! Formatting in a docx text (DESIGN.md §5.2, F2): the style section and the
//! direct formatting a file has, read (GetPut) and written back child by
//! child (PutGet), style edits and new styles in `styles.xml`, and what the
//! vocabulary or Word cannot hold, refused with the reason.

use hanji_core::{edit, rewrite, Capabilities, Engine, EngineError, ImportOptions, Refusal, Remainder};
use hanji_docx::DocxEngine;
use hanji_package::package;

const CAPS: Capabilities =
    Capabilities { links: false, fields: false, footnotes: false, math: false, formatting: true };

fn corpus(name: &str) -> Vec<u8> {
    let path = format!("{}/../../prototype/remainder/corpus/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn part(pkg: &[u8], name: &str) -> String {
    String::from_utf8(package::get(&package::read(pkg).unwrap(), name).unwrap().to_vec()).unwrap()
}

/// The element `<tag…>…</tag>` (or `<tag…/>`) that holds `text` in `xml`.
fn around(xml: &str, tag: &str, text: &str) -> String {
    let at = xml.find(text).unwrap_or_else(|| panic!("{text:?} not in {xml}"));
    let open = [format!("<{tag}>"), format!("<{tag} ")]
        .iter()
        .filter_map(|o| xml[..at].rfind(o.as_str()))
        .max()
        .unwrap_or_else(|| panic!("no <{tag}> before {text:?}"));
    let close = format!("</{tag}>");
    let end = xml[at..].find(&close).unwrap() + at + close.len();
    xml[open..end].to_string()
}

/// Exact edits `(old, new)` in turn, from the imported text.
fn edits(text: &str, rem: &Remainder, list: &[(&str, &str)]) -> hanji_core::Reanchored {
    let (mut t, mut r) = (text.to_string(), rem.clone());
    let mut last = None;
    for (old, new) in list {
        let e = edit(&r, &t, old, new, CAPS).unwrap_or_else(|e| panic!("{old:?} → {new:?}: {e}"));
        assert!(e.report.refused.is_empty() && e.report.removed.is_empty(), "{old:?}: {:?}", e.report);
        (t, r) = (e.text.clone(), e.remainder.clone());
        last = Some(e);
    }
    last.unwrap()
}

#[test]
fn a_file_s_styles_and_direct_formatting_read_back_unchanged() {
    let pkg = corpus("korean-report.docx");
    let imp = DocxEngine.import(&pkg, &ImportOptions::default()).unwrap();
    let t = &imp.text;
    // The default style's line is complete, the others hold what differs.
    assert!(t.contains("---\n<style name=\"Normal\" line-spacing=100% font=\"Noto Sans CJK KR\" size=12pt color=#000000/>\n<style name=\"Heading 1\" size=16pt bold/>\n"), "{t}");
    assert!(t.contains("<style name=\"Note\" fill=#EEEEEE/>\n"), "{t}");
    assert!(t.contains("[15% 성장]{size=14pt color=#1F4E79}했다."), "{t}");
    // GetPut: styles.xml is not written, document.xml is the same XML.
    let out = DocxEngine.export(t, &imp.remainder).unwrap();
    assert_eq!(part(&out, "word/styles.xml"), part(&pkg, "word/styles.xml"));
    let canon = |p: &[u8]| hanji_docx::xml::canon_part(part(p, "word/document.xml").as_bytes()).unwrap();
    assert_eq!(canon(&out), canon(&pkg));
    // Theme colours are shown by name.
    let tables = DocxEngine.import(&corpus("docx4j-tables.docx"), &ImportOptions::default()).unwrap();
    assert!(tables.text.contains("fill=tx2+90%") && tables.text.contains("fill=bg1"), "{}", tables.text);
}

#[test]
fn formatting_edits_rewrite_only_what_they_change() {
    let pkg = corpus("korean-report.docx");
    let imp = DocxEngine.import(&pkg, &ImportOptions::default()).unwrap();
    let r = edits(
        &imp.text,
        &imp.remainder,
        &[
            // A paragraph's first-line indent.
            ("에 지점을 연다.\n", "에 지점을 연다. {first-line=10pt}\n"),
            // A run's colour; its size stays.
            ("[15% 성장]{size=14pt color=#1F4E79}", "[15% 성장]{size=14pt color=#C00000}"),
            // A style line: every Heading 2 paragraph changes.
            ("<style name=\"Heading 2\" size=13pt bold/>", "<style name=\"Heading 2\" size=14pt color=accent1 bold/>"),
            // A new style, then a paragraph in it.
            (
                "<style name=\"Block Quotation\" indent-left=28.35pt/>\n",
                "<style name=\"Block Quotation\" indent-left=28.35pt/>\n<style name=\"Callout\" fill=#FFF2CC border-left=\"2.25pt solid #C00000\"/>\n",
            ),
            ("\n부록: 지점 목록\n", "\n<div style=\"Callout\">부록: 지점 목록</div>\n"),
            // A cell's fill.
            ("| 지역 | 지점 | 매출 |", "| {fill=#DDEBF7} 지역 | 지점 | 매출 |"),
        ],
    );
    let out = DocxEngine.export(&r.text, &r.remainder).unwrap();
    // PutGet: the export reads back as the canonical text.
    let canonical = DocxEngine::text_of(&r.new, &r.remainder, None);
    assert_eq!(DocxEngine.import(&out, &ImportOptions::default()).unwrap().text, canonical);
    // The source writes empty elements open and closed; the export, as `<x/>`.
    let before = part(&pkg, "word/document.xml").replace("<w:rPr></w:rPr>", "<w:rPr/>");
    let after = part(&out, "word/document.xml");
    // A paragraph no edit touched keeps its XML.
    assert_eq!(around(&after, "w:p", "아래 표는"), around(&before, "w:p", "아래 표는"));
    // The indent is one child more; the rest of the paragraph's pPr stays.
    let (p0, p1) = (around(&before, "w:pPr", ">4</w:t>"), around(&after, "w:pPr", ">4</w:t>"));
    assert!(p1.contains("w:firstLine=\"200\""), "{p1}");
    assert_eq!(p1.replace("<w:ind w:firstLine=\"200\"/>", ""), p0, "only w:ind is new");
    let run = around(&after, "w:r", ">성장<");
    assert!(run.contains("<w:color w:val=\"C00000\"/>") && run.contains("w:sz w:val=\"28\""), "{run}");
    assert!(around(&after, "w:tc", ">지역<").contains("<w:shd w:val=\"clear\" w:color=\"auto\" w:fill=\"DDEBF7\"/>"));
    // The style edits are in styles.xml: Heading 2 changed, Callout new.
    let styles = part(&out, "word/styles.xml");
    let h2 = around(&styles, "w:style", "w:val=\"Heading 2\"");
    assert!(
        h2.contains("<w:color w:val=\"") && h2.contains("w:themeColor=\"accent1\"") && h2.contains("w:sz w:val=\"28\""),
        "{h2}"
    );
    let callout = around(&styles, "w:style", "w:val=\"Callout\"");
    assert!(callout.contains("w:customStyle=\"1\"") && callout.contains("w:fill=\"FFF2CC\""), "{callout}");
    assert!(around(&after, "w:p", "부록").contains("<w:pStyle w:val=\"Callout\"/>"));
}

#[test]
fn what_the_vocabulary_or_word_cannot_hold_is_refused() {
    let pkg = corpus("korean-report.docx");
    let imp = DocxEngine.import(&pkg, &ImportOptions::default()).unwrap();
    let (t, rem) = (&imp.text, &imp.remainder);
    let invalid = |old: &str, new: &str| match rewrite(rem, t, &t.replacen(old, new, 1), CAPS) {
        Err(Refusal::Invalid(d)) => d[0].message.clone(),
        other => panic!("{new}: {:?}", other.map(|r| r.text)),
    };
    let refused = |old: &str, new: &str| {
        let r = rewrite(rem, t, &t.replacen(old, new, 1), CAPS).unwrap_or_else(|e| panic!("{new}: {e}"));
        match DocxEngine.export(&r.text, &r.remainder) {
            Err(EngineError::Refused(m)) => m,
            other => panic!("{new}: {:?}", other.map(|_| ())),
        }
    };
    let para = "\n아래 표는 지점별 매출을 정리한 것이다.\n";
    // Word has no transparent text or shading colours.
    let m = refused(para, "\n아래 표는 지점별 매출을 정리한 것이다. {fill=#FF0000/50%}\n");
    assert!(m.contains("transparent"), "{m}");
    // A gradient is kept as written, never written.
    let m = refused(para, "\n아래 표는 지점별 매출을 정리한 것이다. {fill=gradient}\n");
    assert!(m.contains("gradient"), "{m}");
    // A mark is a mark: `bold=no` is not a property.
    let m = invalid("[15% 성장]{size=14pt", "[15% 성장]{bold=no size=14pt");
    assert!(m.contains("bold"), "{m}");
    // A new style needs a name no style has, compared without case.
    let line = "<style name=\"Block Quotation\" indent-left=28.35pt/>\n";
    let m = invalid(line, &format!("{line}<style name=\"note\" size=9pt/>\n"));
    assert!(m.contains("\"note\" is taken") && m.contains("\"Note\""), "{m}");
    // The line of a style the text does not use was not shown: it is not
    // changed in the same revision that starts to use it.
    let m = invalid(line, &format!("{line}<style name=\"Body Text\" size=9pt/>\n"));
    assert!(m.contains("\"Body Text\" is already a style of this file"), "{m}");
}
