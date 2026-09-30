//! A flow document's formatting (DESIGN.md §5.2, F2): the style section,
//! `{…}` on paragraphs, cells, rows and tables, `[text]{…}` spans, the
//! canonical lifting rules, and the validator's refusals.

use hanji_format::*;

const FM: &str = "---\ntype: document\nformat: docx\nschema: 1\n---\n";
const STYLES: &str = "<style name=\"Normal\" line-spacing=115% font=Calibri size=11pt color=#000000/>\n<style name=\"Heading 1\" space-before=12pt size=16pt color=accent1 bold/>\n\n";

fn doc(body: &str) -> String {
    format!("{FM}{STYLES}{body}")
}

/// The canonical form of `text` (parsed, written, and parsed again to the same AST).
fn canonical(text: &str) -> String {
    let d = parse(text).unwrap_or_else(|e| panic!("{}\n---\n{text}", diag::render(&e)));
    let s = serialize(&d);
    let d2 = parse(&s).unwrap_or_else(|e| panic!("reparse: {}\n---\n{s}", diag::render(&e)));
    assert_eq!(d, d2, "parse → serialize → parse changed the AST\n{s}");
    s
}

fn error(body: &str) -> String {
    match parse(&doc(body)) {
        Ok(_) => panic!("{body}: valid"),
        Err(e) => e[0].to_string(),
    }
}

#[test]
fn canonical_formatting_reads_back_as_written() {
    let body = "# 개요 {align=center}\n\n본문 [강조]{size=12pt color=#C00000} 부분. {first-line=10pt space-after=6pt}\n\n<div style=\"Heading 1\">제목처럼</div> {indent-left=20pt}\n\n- 항목 {style=\"Heading 1\" first-line=-10pt}\n\n{border=\"0.5pt solid #000000\"}\n| 구분 {align=center} | 내용 {align=center} | {fill=#D9D9D9}\n|---|---|\n| {fill=#FFF2CC valign=middle} 서울 | 120 |\n| 합계 | 215 | {border-top=\"1.5pt double #000000\"}\n";
    assert_eq!(canonical(&doc(body)), doc(body));
    let d = parse(&doc(body)).unwrap();
    assert_eq!(d.styles.len(), 2);
    assert_eq!(d.styles[1].props.write(), "space-before=12pt size=16pt color=accent1 bold");
}

#[test]
fn shared_values_are_lifted() {
    // Every run of a paragraph has it: the paragraph's.
    assert_eq!(canonical(&doc("[전부 빨강]{color=#FF0000}\n")), doc("전부 빨강 {color=#FF0000}\n"));
    // More than half of a table's cells: the table line's; a row's: the row's.
    let cells = "| {fill=#EEEEEE} a | {fill=#EEEEEE} b |\n|---|---|\n| {fill=#EEEEEE} c | d |\n";
    assert_eq!(canonical(&doc(cells)), doc("{fill=#EEEEEE}\n| a | b |\n|---|---|\n| c | {fill=none} d |\n"));
    let row = "| {fill=#EEEEEE} a | {fill=#EEEEEE} b |\n|---|---|\n| c | d |\n| e | f |\n";
    assert_eq!(canonical(&doc(row)), doc("| a | b | {fill=#EEEEEE}\n|---|---|\n| c | d |\n| e | f |\n"));
}

#[test]
fn braces_in_text_are_escaped() {
    assert_eq!(canonical(&doc("집합 \\{a, b\\}\n")), doc("집합 \\{a, b}\n"));
    assert_eq!(canonical(&doc("[괄호 \\[1\\]]{size=9pt} 뒤\n")), doc("[괄호 \\[1\\]]{size=9pt} 뒤\n"));
}

#[test]
fn what_the_syntax_cannot_say_is_refused() {
    assert!(error("굵게 아님 {bold=no}\n").contains("bold"), "a mark is written as a mark");
    assert!(error("문단 {valign=middle}\n").contains("valign is a cell's"));
    let e = error("<br/> {first-line=10pt}\n");
    assert!(e.contains("without text"), "{e}");
    let e = error("본문\n\n<style name=\"Late\" size=9pt/>\n");
    assert!(e.contains("style"), "{e}");
    let twice = format!(
        "{FM}<style name=\"Normal\" size=11pt/>\n<style name=\"A\" size=9pt/>\n<style name=\"A\" size=10pt/>\n\n본문\n"
    );
    assert!(parse(&twice).is_err(), "a style has one line");
    assert!(error("본문 {colour=#FF0000}\n").contains("colour"));
}
