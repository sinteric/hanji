//! Engine behaviour on small synthetic packages: the §5.2 mapping, exact
//! edits, side-by-side tables (§10.8), tracked changes (§10.2), §8
//! neutralisation and the surface-before-export list.

use hanji_core::{edit, rewrite, Capabilities, Engine, EngineError, ImportOptions, Part, Refusal};
use hanji_hwpx::{package, xml, HwpxEngine};

const CAPS: Capabilities =
    Capabilities { links: false, fields: false, footnotes: false, math: false, formatting: true, table_place: false };
const NS: &str = r#"xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph" xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hh="http://www.hancom.co.kr/hwpml/2011/head" xmlns:hc="http://www.hancom.co.kr/hwpml/2011/core""#;

fn part(name: &str, data: &str) -> Part {
    Part {
        name: name.into(),
        data: data.as_bytes().to_vec(),
        dos_time: 0x5b21_0000,
        external_attr: 0,
        deflate: name != "mimetype",
    }
}

fn char_pr(id: u32, extra: &str) -> String {
    format!(
        r##"<hh:charPr id="{id}" height="1000" textColor="#000000"><hh:fontRef hangul="0" latin="0"/>{extra}<hh:underline type="NONE" shape="SOLID" color="#000000"/><hh:strikeout shape="NONE" color="#000000"/></hh:charPr>"##
    )
}

fn para_pr(id: u32, heading: &str) -> String {
    format!(r#"<hh:paraPr id="{id}"><hh:align horizontal="JUSTIFY"/><hh:heading {heading}/></hh:paraPr>"#)
}

/// Styles: 0 바탕글 (default), 1 본문, 2 개요 1 (outline level 1 → `#`).
/// Character shapes: 0 plain, 1 bold, 2 red (not a mark). Paragraph shapes:
/// 0 none, 1 outline 1, 2 bullet 1, 3 number 1.
fn header(extra: &str) -> String {
    let chars = [
        char_pr(0, ""),
        char_pr(1, "<hh:bold/>"),
        char_pr(2, "").replace("#000000\"><hh:fontRef", "#FF0000\"><hh:fontRef"),
    ]
    .concat();
    let paras = [
        para_pr(0, r#"type="NONE" idRef="0" level="0""#),
        para_pr(1, r#"type="OUTLINE" idRef="0" level="0""#),
        para_pr(2, r#"type="BULLET" idRef="1" level="0""#),
        para_pr(3, r#"type="NUMBER" idRef="1" level="0""#),
    ]
    .concat();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes" ?><hh:head {NS} version="1.4" secCnt="1"><hh:refList><hh:charProperties itemCnt="3">{chars}</hh:charProperties><hh:numberings itemCnt="1"><hh:numbering id="1" start="0"><hh:paraHead start="1" level="1" numFormat="DIGIT">^1.</hh:paraHead><hh:paraHead start="1" level="2" numFormat="HANGUL_SYLLABLE">^2.</hh:paraHead></hh:numbering></hh:numberings><hh:bullets itemCnt="1"><hh:bullet id="1" char="-"/></hh:bullets><hh:paraProperties itemCnt="4">{paras}</hh:paraProperties><hh:styles itemCnt="3"><hh:style id="0" type="PARA" name="바탕글" engName="Normal" paraPrIDRef="0" charPrIDRef="0"/><hh:style id="1" type="PARA" name="본문" engName="Body" paraPrIDRef="0" charPrIDRef="0"/><hh:style id="2" type="PARA" name="개요 1" engName="Outline 1" paraPrIDRef="1" charPrIDRef="1"/></hh:styles>{extra}</hh:refList></hh:head>"#
    )
}

const SEC_START: &str = r#"<hp:run charPrIDRef="0"><hp:secPr id="" textDirection="HORIZONTAL"><hp:pagePr landscape="WIDELY" width="59528" height="84186"/></hp:secPr><hp:ctrl><hp:colPr id="" type="NEWSPAPER" colCount="1"/></hp:ctrl></hp:run>"#;

/// An hwpx package whose section holds `paras` (the first gets the section settings).
fn hwpx_with(paras: &[String], header_extra: &str, extra: Vec<Part>, manifest_extra: &str) -> Vec<u8> {
    let mut body = String::new();
    for (k, p) in paras.iter().enumerate() {
        body.push_str(&if k == 0 { p.replacen("<hp:run", &format!("{SEC_START}<hp:run"), 1) } else { p.clone() });
    }
    let sec = format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes" ?><hs:sec {NS}>{body}</hs:sec>"#);
    let hpf = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes" ?><opf:package xmlns:opf="http://www.idpf.org/2007/opf/"><opf:metadata><opf:meta name="creator" content="text">홍길동</opf:meta></opf:metadata><opf:manifest><opf:item id="header" href="Contents/header.xml" media-type="application/xml"/><opf:item id="section0" href="Contents/section0.xml" media-type="application/xml"/>{manifest_extra}</opf:manifest><opf:spine><opf:itemref idref="header" linear="no"/><opf:itemref idref="section0" linear="yes"/></opf:spine></opf:package>"#
    );
    let mut parts = vec![
        part("mimetype", "application/hwp+zip"),
        part("Contents/header.xml", &header(header_extra)),
        part("Contents/section0.xml", &sec),
        part("Contents/content.hpf", &hpf),
        part(
            "META-INF/manifest.xml",
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes" ?><odf:manifest xmlns:odf="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0"/>"#,
        ),
        part("Preview/PrvText.txt", "preview"),
    ];
    parts.extend(extra);
    package::write(&parts).unwrap()
}

fn hwpx(paras: &[String]) -> Vec<u8> {
    hwpx_with(paras, "", vec![], "")
}

/// A paragraph: `(charPrIDRef, text)` runs in style `style`, shape `ppr`.
fn p_in(style: u32, ppr: u32, runs: &[(u32, &str)]) -> String {
    let runs: String =
        runs.iter().map(|(c, t)| format!(r#"<hp:run charPrIDRef="{c}"><hp:t>{t}</hp:t></hp:run>"#)).collect();
    format!(
        r#"<hp:p id="0" paraPrIDRef="{ppr}" styleIDRef="{style}" pageBreak="0" columnBreak="0" merged="0">{runs}<hp:linesegarray><hp:lineseg textpos="0"/></hp:linesegarray></hp:p>"#
    )
}

fn p(text: &str) -> String {
    p_in(0, 0, &[(0, text)])
}

/// A table of `rows`; a cell is `(text, colSpan, rowSpan)`, placed left to right.
fn tbl(rows: &[&[(&str, usize, usize)]], cols: usize) -> String {
    let mut grid = vec![vec![false; cols]; rows.len()];
    let mut trs = String::new();
    for (r, row) in rows.iter().enumerate() {
        let mut tcs = String::new();
        let mut c = 0;
        for (text, cs, rs) in *row {
            while grid[r][c] {
                c += 1;
            }
            for row in grid.iter_mut().skip(r).take(*rs) {
                row[c..c + cs].fill(true);
            }
            let paras: String = text.split("<p/>").map(p).collect();
            tcs.push_str(&format!(r#"<hp:tc name="" header="0" borderFillIDRef="3"><hp:subList id="" vertAlign="CENTER">{paras}</hp:subList><hp:cellAddr colAddr="{c}" rowAddr="{r}"/><hp:cellSpan colSpan="{cs}" rowSpan="{rs}"/><hp:cellSz width="1000" height="282"/><hp:cellMargin left="510" right="510" top="141" bottom="141"/></hp:tc>"#));
            c += cs;
        }
        trs.push_str(&format!("<hp:tr>{tcs}</hp:tr>"));
    }
    format!(
        r#"<hp:tbl id="1" rowCnt="{}" colCnt="{cols}" borderFillIDRef="3"><hp:sz width="4000" height="1000"/><hp:pos treatAsChar="0"/><hp:outMargin left="283"/><hp:inMargin left="510"/>{trs}</hp:tbl>"#,
        rows.len()
    )
}

/// A paragraph holding `tables` and an empty `hp:t`.
fn anchor(tables: &[String]) -> String {
    format!(
        r#"<hp:p id="0" paraPrIDRef="0" styleIDRef="0" pageBreak="0" columnBreak="0" merged="0"><hp:run charPrIDRef="0">{}<hp:t/></hp:run></hp:p>"#,
        tables.concat()
    )
}

/// Keep a package next to the corpus exports, so the rhwp check (`validate/`) re-opens it too.
fn save(dir: &str, what: &str, pkg: &[u8]) {
    let d = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("hwpx-corpus-out").join(dir);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join(format!("{what}.hwpx")), pkg).unwrap();
}

/// `pkg` with its `header.xml` changed by `f`.
fn with_header(pkg: &[u8], f: impl Fn(&str) -> String) -> Vec<u8> {
    let mut parts = package::read(pkg).unwrap();
    let h = parts.iter_mut().find(|p| p.name == "Contents/header.xml").unwrap();
    h.data = f(std::str::from_utf8(&h.data).unwrap()).into_bytes();
    package::write(&parts).unwrap()
}

/// A rewritten text as the store returns it: completed by the engine (§5.2).
fn completed(text: &str, rem: &hanji_core::Remainder) -> (String, hanji_core::Remainder) {
    let (_, mut blocks) = hanji_core::model_of(text, rem, CAPS).unwrap();
    let mut rem = rem.clone();
    HwpxEngine::complete(&mut blocks, &mut rem);
    (HwpxEngine::text_of(&blocks, &rem, None), rem)
}

fn section(pkg: &[u8]) -> String {
    String::from_utf8(package::get(&package::read(pkg).unwrap(), "Contents/section0.xml").unwrap().to_vec()).unwrap()
}

fn canon(pkg: &[u8]) -> String {
    xml::canon_part(section(pkg).as_bytes()).unwrap()
}

fn import(pkg: &[u8]) -> hanji_core::Imported {
    HwpxEngine.import(pkg, &ImportOptions::default()).unwrap_or_else(|e| panic!("{e}"))
}

fn export(text: &str, rem: &hanji_core::Remainder) -> Vec<u8> {
    HwpxEngine.export(text, rem).unwrap_or_else(|e| panic!("{e}\n{text}"))
}

#[test]
fn styles_headings_marks_tables_and_breaks_map_onto_the_model() {
    let pagebreak = p("다음 쪽").replace("pageBreak=\"0\"", "pageBreak=\"1\"");
    let body = [
        p_in(2, 1, &[(1, "3분기 보고")]),
        p_in(0, 0, &[(0, "매출은 "), (1, "12%"), (0, " 증가했다.")]),
        p_in(1, 0, &[(2, "빨간 글자")]),
        r#"<hp:p id="0" paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"/></hp:p>"#.to_string(),
        r#"<hp:p id="0" paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:t>첫 줄<hp:lineBreak/>둘째 줄<hp:tab width="4000" leader="0" type="1"/>탭</hp:t></hp:run></hp:p>"#.to_string(),
        anchor(&[tbl(&[&[("지역", 1, 1), ("매출", 1, 1)], &[("서울", 1, 2), ("120", 1, 1)], &[("95", 1, 1)], &[("합계", 2, 1)]], 2)]),
        anchor(&[tbl(&[&[("비고", 1, 1)], &[("부산<p/>해운대", 1, 1)]], 1)]),
        pagebreak,
    ];
    let pkg = hwpx(&body);
    let imp = import(&pkg);
    // The style section (§5.2): the default style's line complete, the others what differ.
    let styles = "<style name=\"바탕글\" align=justify line-spacing=160% size=10pt color=#000000/>\n<style name=\"개요 1\" bold/>\n<style name=\"본문\"/>\n";
    assert!(imp.text.contains(styles), "{}", imp.text);
    let want = "# 3분기 보고\n\n매출은 **12%** 증가했다.\n\n<div style=\"본문\">빨간 글자</div> {color=#FF0000}\n\n<p/>\n\n첫 줄<br/>둘째 줄\t탭\n\n{valign=middle}\n| 지역 | 매출 |\n|---|---|\n| 서울 | 120 |\n| ^^ | 95 |\n| 합계 ||\n\n{valign=middle}\n| 비고 |\n|---|\n| 부산<p/>해운대 |\n\n<pagebreak/>\n\n다음 쪽\n";
    assert!(imp.text.ends_with(want), "{}", imp.text);
    // GetPut: the section comes back canonically equal, the header byte-equal.
    let out = export(&imp.text, &imp.remainder);
    assert_eq!(canon(&out), canon(&pkg));
    let hdr = |p: &[u8]| package::get(&package::read(p).unwrap(), "Contents/header.xml").unwrap().to_vec();
    assert_eq!(hdr(&out), hdr(&pkg));
    // hwpx has no table styles: the validator says so.
    let bad = imp.text.replace("{valign=middle}\n| 비고 |", "{style=\"Grid\" valign=middle}\n| 비고 |");
    match HwpxEngine.export(&bad, &imp.remainder) {
        Err(EngineError::Invalid(d)) => assert!(d[0].to_string().contains("table style"), "{}", d[0]),
        other => panic!("{other:?}"),
    }
}

#[test]
fn exact_edits_keep_formatting_and_new_marks_get_a_shape() {
    let pkg = hwpx(&[p_in(0, 0, &[(2, "매출 2023"), (0, " 증가")]), p("끝")]);
    let imp = import(&pkg);
    let r = edit(&imp.remainder, &imp.text, "2023", "2024", CAPS).unwrap();
    assert!(r.report.refused.is_empty() && r.report.removed.is_empty(), "{:?}", r.report);
    let out = export(&r.text, &r.remainder);
    let sec = section(&out);
    assert!(sec.contains(r#"<hp:run charPrIDRef="2"><hp:t>매출 2024</hp:t></hp:run>"#), "{sec}");
    // The edited paragraph drops its stale layout cache; the other keeps it.
    assert_eq!(sec.matches("<hp:linesegarray>").count(), 1, "{sec}");
    // Bold on red text needs a shape the file does not have: a new charPr.
    let r = edit(&imp.remainder, &imp.text, "매출 2023", "**매출 2023**", CAPS).unwrap();
    let out = export(&r.text, &r.remainder);
    let hdr = String::from_utf8(package::get(&package::read(&out).unwrap(), "Contents/header.xml").unwrap().to_vec())
        .unwrap();
    assert!(hdr.contains(r#"<hh:charProperties itemCnt="4">"#), "{hdr}");
    assert!(
        hdr.contains(
            r##"<hh:charPr id="3" height="1000" textColor="#FF0000"><hh:fontRef hangul="0" latin="0"/><hh:bold/>"##
        ),
        "{hdr}"
    );
    assert!(section(&out).contains(r#"<hp:run charPrIDRef="3"><hp:t>매출 2023</hp:t>"#));
    assert_eq!(import(&out).text, r.text, "PutGet");
    // Bold already in the file is reused, not added again.
    let r = edit(&imp.remainder, &imp.text, "끝", "**끝**", CAPS).unwrap();
    let out = export(&r.text, &r.remainder);
    assert!(section(&out).contains(r#"<hp:run charPrIDRef="1"><hp:t>끝</hp:t>"#), "{}", section(&out));
    // Restyle to the outline style: the paragraph and its runs take the style's shapes.
    let r = edit(&imp.remainder, &imp.text, "\n끝\n", "\n# 끝\n", CAPS).unwrap();
    let sec = section(&export(&r.text, &r.remainder));
    assert!(sec.contains(r#"paraPrIDRef="1" styleIDRef="2" pageBreak="0" columnBreak="0" merged="0"><hp:run charPrIDRef="1"><hp:t>끝"#), "{sec}");
}

#[test]
fn joining_paragraphs_keeps_the_runs_the_join_rewrites_around() {
    // "2." at the start of a line is escaped (`2\.`, not a list item); after
    // the join it is not. The exact span then covers the paragraph mark, the
    // escape and the units between them, which are the same on both sides.
    let pkg = hwpx(&[p("1부."), p_in(0, 0, &[(2, "  "), (2, " 2."), (0, " 나")])]);
    // Character shape 2 differs from 0 in what the text does not show (a shade).
    let pkg = with_header(&pkg, |h| h.replace("#FF0000\"><hh:fontRef", "#000000\" shadeColor=\"#FFFF00\"><hh:fontRef"));
    let imp = import(&pkg);
    assert!(imp.text.contains("1부.\n\n   2\\. 나"), "{}", imp.text);
    let r = edit(&imp.remainder, &imp.text, "1부.\n\n   2\\.", "1부.   2.", CAPS).unwrap();
    // Only the second paragraph's own properties go with it.
    let removed: Vec<_> = r.report.removed.iter().map(|x| x.1).collect();
    assert_eq!(removed, [hanji_core::Kind::Ppr], "{:?}", r.report);
    let sec = section(&export(&r.text, &r.remainder));
    assert!(
        sec.contains(
            r#"<hp:run charPrIDRef="2"><hp:t>  </hp:t></hp:run><hp:run charPrIDRef="2"><hp:t> 2.</hp:t></hp:run>"#
        ),
        "{sec}"
    );
}

#[test]
fn a_style_missing_from_the_header_keeps_its_reference() {
    let pkg = hwpx(&[p_in(9, 0, &[(1, "유령 스타일")]), p("끝")]);
    let imp = import(&pkg);
    assert_eq!(canon(&export(&imp.text, &imp.remainder)), canon(&pkg), "GetPut\n{}", imp.text);
    let r = edit(&imp.remainder, &imp.text, "유령 스타일", "유령 문단", CAPS).unwrap();
    let out = export(&r.text, &r.remainder);
    assert!(section(&out).contains(r#"paraPrIDRef="0" styleIDRef="9""#), "{}", section(&out));
    assert_eq!(import(&out).text, r.text, "PutGet");
    // A new paragraph in that style takes the same reference.
    let block = imp.text.lines().find(|b| b.contains("유령")).unwrap().to_string();
    let added = imp.text.replace("끝", &block.replace("유령 스타일", "새 문단"));
    let out = export(&added, &imp.remainder);
    assert_eq!(section(&out).matches(r#"styleIDRef="9""#).count(), 2, "{}", section(&out));
}

#[test]
fn side_by_side_tables_are_separate_blocks_in_one_paragraph() {
    let a = tbl(&[&[("담당 부서", 1, 1), ("기획과", 1, 1)]], 2);
    let b = tbl(&[&[("담당자", 1, 1), ("홍길동", 1, 1)]], 2);
    let pkg = hwpx(&[p("앞"), anchor(&[a, b]), p("뒤")]);
    let (blocks, _, _, stats) = HwpxEngine::split(&pkg, &ImportOptions::default()).unwrap();
    assert_eq!((blocks.len(), stats.side_by_side), (4, 1));
    let imp = import(&pkg);
    assert!(
        imp.text.contains("| 담당 부서 | 기획과 |\n|---|---|\n\n{valign=middle}\n| 담당자 | 홍길동 |\n|---|---|\n"),
        "{}",
        imp.text
    );
    let out = export(&imp.text, &imp.remainder);
    assert_eq!(canon(&out), canon(&pkg), "GetPut writes both tables back into their paragraph");
    // An edit in the second table keeps them together.
    let r = edit(&imp.remainder, &imp.text, "홍길동", "김철수", CAPS).unwrap();
    let sec = section(&export(&r.text, &r.remainder));
    assert_eq!(sec.matches("<hp:tbl ").count(), 2);
    assert!(sec.contains("</hp:tbl><hp:tbl "), "still side by side: {sec}");
    // Moving the second table after 뒤 gives it a paragraph of its own.
    let moved = imp
        .text
        .replace("\n\n{valign=middle}\n| 담당자 | 홍길동 |\n|---|---|\n", "\n")
        .replace("\n뒤\n", "\n뒤\n\n{valign=middle}\n| 담당자 | 홍길동 |\n|---|---|\n");
    let r = rewrite(&imp.remainder, &imp.text, &moved, CAPS).unwrap();
    let out = export(&r.text, &r.remainder);
    let sec = section(&out);
    assert!(!sec.contains("</hp:tbl><hp:tbl "), "{sec}");
    assert!(
        sec.find("기획과").unwrap() < sec.find("뒤").unwrap() && sec.find("뒤").unwrap() < sec.find("홍길동").unwrap()
    );
    assert_eq!(import(&out).text, r.text, "PutGet");
    // Deleting the first table leaves the second in the shared paragraph.
    let gone = imp.text.replace("{valign=middle}\n| 담당 부서 | 기획과 |\n|---|---|\n\n", "");
    let r = rewrite(&imp.remainder, &imp.text, &gone, CAPS).unwrap();
    let out = export(&r.text, &r.remainder);
    assert_eq!(section(&out).matches("<hp:tbl ").count(), 1);
    assert_eq!(import(&out).text, r.text, "PutGet");
}

#[test]
fn merges_are_written_as_spans() {
    let pkg = hwpx(&[anchor(&[tbl(&[&[("a", 1, 1), ("b", 1, 1)], &[("c", 1, 1), ("d", 1, 1)]], 2)])]);
    let imp = import(&pkg);
    // New rows merge freely: spans come from the markers.
    let new = imp.text.replace("| c | d |", "| c | d |\n| e ||\n| f | g |\n| ^^ | h |");
    let r = rewrite(&imp.remainder, &imp.text, &new, CAPS).unwrap();
    let out = export(&r.text, &r.remainder);
    let sec = section(&out);
    assert!(sec.contains(r#"<hp:cellAddr colAddr="0" rowAddr="2"/><hp:cellSpan colSpan="2" rowSpan="1"/>"#), "{sec}");
    assert!(sec.contains(r#"<hp:cellAddr colAddr="0" rowAddr="3"/><hp:cellSpan colSpan="1" rowSpan="2"/>"#), "{sec}");
    assert!(sec.contains(r#"rowCnt="5" colCnt="2""#), "{sec}");
    assert_eq!(sec.matches("<hp:tc ").count(), 8, "no cell where a merge covers one: {sec}");
    assert_eq!(import(&out).text, r.text, "PutGet");
    // Merging existing cells would lose the covered cell: refused, as in docx.
    let across = imp.text.replace("| c | d |", "| c ||");
    assert!(matches!(rewrite(&imp.remainder, &imp.text, &across, CAPS), Err(Refusal::Unplaceable(_))));
    let down = imp.text.replace("| c | d |", "| c | ^^ |");
    let r = rewrite(&imp.remainder, &imp.text, &down, CAPS).unwrap();
    match HwpxEngine.export(&r.text, &r.remainder) {
        Err(EngineError::Refused(m)) => assert!(m.contains("row 2, column 2 is now covered by a merge"), "{m}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn bullets_and_numbers_are_list_items() {
    let body = [p_in(0, 2, &[(0, "가")]), p_in(0, 2, &[(0, "나")]), p("본문"), p_in(0, 3, &[(0, "하나")]), p("끝")];
    let pkg = hwpx(&body);
    let imp = import(&pkg);
    assert!(imp.text.contains("- 가\n- 나\n\n본문\n\n1. 하나\n\n끝\n"), "{}", imp.text);
    assert_eq!(canon(&export(&imp.text, &imp.remainder)), canon(&pkg));
    // A new item takes its sibling's shape; a paragraph that stops being an item loses its heading.
    let new = imp
        .text
        .replace("- 나\n", "- 나\n- 다\n")
        .replace("1. 하나\n", "하나\n")
        .replace("\n끝\n", "\n끝\n\n1. 새 목록\n");
    let r = rewrite(&imp.remainder, &imp.text, &new, CAPS).unwrap();
    let out = export(&r.text, &r.remainder);
    let sec = section(&out);
    assert!(sec.contains(r#"paraPrIDRef="2" styleIDRef="0" pageBreak="0" columnBreak="0" merged="0"><hp:run charPrIDRef="0"><hp:t>다"#), "{sec}");
    assert!(sec.contains(r#"paraPrIDRef="0" styleIDRef="0" pageBreak="0" columnBreak="0" merged="0"><hp:run charPrIDRef="0"><hp:t>하나"#), "{sec}");
    // A new numbered list restarts: a copy of the numbering under a new id, and a shape that uses it.
    let hdr = String::from_utf8(package::get(&package::read(&out).unwrap(), "Contents/header.xml").unwrap().to_vec())
        .unwrap();
    assert!(hdr.contains(r#"<hh:numbering id="2" start="0">"#), "{hdr}");
    assert!(hdr.contains(r#"<hh:heading type="NUMBER" idRef="2" level="0"/>"#), "{hdr}");
    assert_eq!(import(&out).text, r.text, "PutGet");
}

/// A package with `body` and a second bullet: shape 2 is bullet 1 at the
/// margin, shape 4 bullet 2 one level down, indented.
fn nested_lists(body: &[String]) -> Vec<u8> {
    // Shape 4: a second bullet one level down, indented.
    let nested = r#"<hh:paraPr id="4"><hh:align horizontal="JUSTIFY"/><hh:heading type="BULLET" idRef="2" level="1"/><hh:margin><hc:left value="2000" unit="HWPUNIT"/></hh:margin></hh:paraPr>"#;
    let mut parts = package::read(&hwpx(body)).unwrap();
    let h = parts.iter_mut().find(|p| p.name == "Contents/header.xml").unwrap();
    h.data = String::from_utf8(std::mem::take(&mut h.data))
        .unwrap()
        .replace(
            r#"<hh:bullets itemCnt="1"><hh:bullet id="1" char="-"/>"#,
            r#"<hh:bullets itemCnt="2"><hh:bullet id="1" char="-"/><hh:bullet id="2" char="o"/>"#,
        )
        .replace(r#"<hh:paraProperties itemCnt="4">"#, r#"<hh:paraProperties itemCnt="5">"#)
        .replace("</hh:paraProperties>", &format!("{nested}</hh:paraProperties>"))
        .into_bytes();
    package::write(&parts).unwrap()
}

/// A new item takes the shape of an item at its own level: a level's shape
/// holds its indent and bullet, which Hancom shows (the kit's 73: the first
/// half of a split nested item took the outer item's shape with only its
/// heading level changed, and Hancom showed it at the outer level).
#[test]
fn a_new_nested_item_takes_the_shape_of_its_level() {
    let body = [
        p_in(0, 2, &[(0, "상위")]),
        p_in(0, 4, &[(0, "짧게 그리고 아주 길게 이어지는 문장")]),
        p_in(0, 2, &[(0, "다음 상위")]),
    ];
    let pkg = nested_lists(&body);
    let imp = import(&pkg);
    assert!(
        imp.text.contains("- 상위\n  - 짧게 그리고 아주 길게 이어지는 문장 {indent-left=10pt}\n- 다음 상위\n"),
        "{}",
        imp.text
    );
    // Split the nested item: the longer half keeps the paragraph, so the first half is the new
    // one, between the outer item and it. Both are nested items in shape 4; no shape is added.
    // The new one takes its level's indent, and the returned text shows it.
    let split = imp.text.replace("짧게 그리고", "짧게\n  - 그리고");
    let r = rewrite(&imp.remainder, &imp.text, &split, CAPS).unwrap();
    let (text, rem) = completed(&r.text, &r.remainder);
    assert!(text.contains("  - 짧게 {indent-left=10pt}\n  - 그리고"), "{text}");
    let out = export(&text, &rem);
    let sec = section(&out);
    for t in ["짧게", "그리고 아주 길게 이어지는 문장"] {
        let root = xml::parse(sec.as_bytes()).unwrap().root;
        let p = root.elements().find(|p| p.is("hp:p") && p.text_of(&["hp:t"]) == t).unwrap().clone();
        assert_eq!(p.get("paraPrIDRef").as_deref(), Some("4"), "{t}: {sec}");
    }
    let parts = package::read(&out).unwrap();
    assert_eq!(
        package::get(&parts, "Contents/header.xml"),
        package::get(&package::read(&pkg).unwrap(), "Contents/header.xml")
    );
    assert_eq!(import(&out).text, text, "PutGet");
    save("engine-lists", "ORIGINAL", &pkg);
    save("engine-lists", "split-nested", &out);
}

/// An item the text moves to another level takes that level's shape from an
/// item there, bullet and indent (the kit's 73: E4 moved a level-3 item out
/// to head a list; only its heading level changed, and Hancom showed it at
/// its old indent).
#[test]
fn an_item_moved_to_another_level_takes_that_levels_shape() {
    let body = [
        p_in(0, 2, &[(0, "상위")]),
        p_in(0, 4, &[(0, "하위")]),
        p_in(0, 4, &[(0, "하위 둘")]),
        p_in(0, 2, &[(0, "다음 상위")]),
        p("본문"),
        p_in(0, 2, &[(0, "끝 항목")]),
    ];
    let pkg = nested_lists(&body);
    let imp = import(&pkg);
    let nested = "  - 하위 {indent-left=10pt}\n  - 하위 둘 {indent-left=10pt}\n";
    assert!(imp.text.contains(&format!("- 상위\n{nested}- 다음 상위\n\n본문\n\n- 끝 항목\n")), "{}", imp.text);
    let shape = |out: &[u8], t: &str| {
        let root = xml::parse(section(out).as_bytes()).unwrap().root;
        let p = root.elements().find(|p| p.is("hp:p") && p.text_of(&["hp:t"]) == t).unwrap().get("paraPrIDRef");
        p
    };
    let header = |pkg: &[u8]| package::get(&package::read(pkg).unwrap(), "Contents/header.xml").unwrap().to_vec();
    // The nested item moves out to the margin, into the list after "본문"; "다음 상위" moves
    // under "상위". Each takes its new level's indent (the moved line carries its old one).
    let moved = imp
        .text
        .replace(&format!("{nested}- 다음 상위\n"), "  - 하위 둘 {indent-left=10pt}\n  - 다음 상위\n")
        .replace("\n- 끝 항목\n", "\n- 하위 {indent-left=10pt}\n- 끝 항목\n");
    let r = rewrite(&imp.remainder, &imp.text, &moved, CAPS).unwrap();
    let (text, rem) = completed(&r.text, &r.remainder);
    assert!(text.contains("  - 다음 상위 {indent-left=10pt}\n") && text.contains("\n- 하위\n- 끝 항목\n"), "{text}");
    let out = export(&text, &rem);
    assert_eq!(shape(&out, "하위").as_deref(), Some("2"), "{}", section(&out));
    assert_eq!(shape(&out, "다음 상위").as_deref(), Some("4"), "{}", section(&out));
    assert_eq!(header(&out), header(&pkg));
    assert_eq!(import(&out).text, text, "PutGet");
    save("engine-lists", "moved-levels", &out);
}

#[test]
fn adjacent_lists_of_another_kind_or_definition_stay_separate() {
    // A bullet list right before a numbered one: two lists, as in docx (§5.2).
    let body =
        [p_in(0, 2, &[(0, "가")]), p_in(0, 2, &[(0, "나")]), p_in(0, 3, &[(0, "하나")]), p_in(0, 3, &[(0, "둘")])];
    let pkg = hwpx(&body);
    let imp = import(&pkg);
    assert!(imp.text.ends_with("\n- 가\n- 나\n\n1. 하나\n1. 둘\n"), "{}", imp.text);
    assert_eq!(canon(&export(&imp.text, &imp.remainder)), canon(&pkg), "GetPut");
    // Written as new text, each reads back as it was written.
    let base = import(&hwpx(&[p("끝")]));
    for lists in [
        "- 가\n- 나\n\n1. 하나\n1. 둘\n",
        "1. 하나\n1. 둘\n\n- 가\n- 나\n",
        // Two numbered lists: the second restarts, on a numbering of its own.
        "1. 하나\n1. 둘\n\n1. 새 하나\n",
        // A list nests another kind, and the outer list goes on after it.
        "- 가\n  1. 하나\n  1. 둘\n- 나\n\n1. 셋\n",
    ] {
        let r =
            rewrite(&base.remainder, &base.text, &base.text.replace("끝\n", &format!("{lists}\n끝\n")), CAPS).unwrap();
        let out = export(&r.text, &r.remainder);
        assert_eq!(import(&out).text, r.text, "PutGet\n{}", section(&out));
    }
}

/// A border fill with no lines (a page border), as a file with no table has.
const NO_LINES: &str = r##"<hh:borderFills itemCnt="1"><hh:borderFill id="1" threeD="0" shadow="0" centerLine="NONE" breakCellSeparateLine="0"><hh:slash type="NONE" Crooked="0" isCounter="0"/><hh:backSlash type="NONE" Crooked="0" isCounter="0"/><hh:leftBorder type="NONE" width="0.1 mm" color="#000000"/><hh:rightBorder type="NONE" width="0.1 mm" color="#000000"/><hh:topBorder type="NONE" width="0.1 mm" color="#000000"/><hh:bottomBorder type="NONE" width="0.1 mm" color="#000000"/><hh:diagonal type="SOLID" width="0.1 mm" color="#000000"/></hh:borderFill></hh:borderFills>"##;

const ONE_COLUMN: &str = r#"colCount="1""#;

#[test]
fn a_new_table_in_a_file_without_one_takes_the_default_look() {
    let table = "| 지역 | 매출 |\n|---|---|\n| 서울 | 120 |\n| 합계 ||\n";
    let with_table = |header_extra: &str, columns: &str| {
        let pkg = hwpx_with(&[p("앞"), p("뒤")], header_extra, vec![], "");
        let mut parts = package::read(&pkg).unwrap();
        let sec = parts.iter_mut().find(|p| p.name == "Contents/section0.xml").unwrap();
        sec.data = String::from_utf8(sec.data.clone()).unwrap().replace(r#"colCount="1""#, columns).into_bytes();
        let pkg = package::write(&parts).unwrap();
        let imp = import(&pkg);
        let r =
            rewrite(&imp.remainder, &imp.text, &imp.text.replace("\n뒤\n", &format!("\n{table}\n뒤\n")), CAPS).unwrap();
        // The new table takes the default look, and the returned text shows it.
        let (text, rem) = completed(&r.text, &r.remainder);
        assert!(text.contains("{border=\"0.34pt solid #000000\" valign=middle}\n| 지역 |"), "{text}");
        let out = export(&text, &rem);
        assert_eq!(import(&out).text, text, "PutGet");
        save("engine-new-table", "ORIGINAL", &pkg);
        let hdr = |p: &[u8]| package::get(&package::read(p).unwrap(), "Contents/header.xml").unwrap().to_vec();
        (String::from_utf8(hdr(&out)).unwrap(), hdr(&pkg) == hdr(&out), out)
    };
    // The file has no border fill with lines: a solid one is added for the table and its cells.
    let (hdr, same, out) = with_table(NO_LINES, ONE_COLUMN);
    assert!(!same && hdr.contains(r#"<hh:borderFills itemCnt="2">"#), "{hdr}");
    assert!(hdr.contains(r##"<hh:borderFill id="2" threeD="0" shadow="0" centerLine="NONE" breakCellSeparateLine="0"><hh:slash type="NONE" Crooked="0" isCounter="0"/><hh:backSlash type="NONE" Crooked="0" isCounter="0"/><hh:leftBorder type="SOLID" width="0.12 mm" color="#000000"/>"##), "{hdr}");
    let sec = section(&out);
    assert!(sec.contains(r#"rowCnt="3" colCnt="2" cellSpacing="0" borderFillIDRef="2""#), "{sec}");
    assert_eq!(
        sec.matches(
            r#"<hp:tc name="" header="0" hasMargin="0" protect="0" editable="0" dirty="0" borderFillIDRef="2">"#
        )
        .count(),
        5,
        "{sec}"
    );
    // Across the text width (the page here has no margins), less the outer margins.
    assert!(sec.contains(r#"<hp:sz width="58962" widthRelTo="ABSOLUTE" height="3846""#), "{sec}");
    assert!(sec.contains(r#"<hp:cellSpan colSpan="1" rowSpan="1"/><hp:cellSz width="29481" height="282"/>"#), "{sec}");
    assert!(sec.contains(r#"<hp:cellSpan colSpan="2" rowSpan="1"/><hp:cellSz width="58962" height="282"/>"#), "{sec}");
    save("engine-new-table", "added-border-fill", &out);
    // In two columns 1134 apart, across one column.
    let (_, _, out) = with_table(NO_LINES, r#"colCount="2" sameSz="1" sameGap="1134""#);
    assert!(section(&out).contains(r#"<hp:sz width="28630""#), "{}", section(&out));
    save("engine-new-table", "two-columns", &out);
    // One that draws a table already is reused: the header is unchanged.
    let (_, same, out) =
        with_table(&NO_LINES.replace(r#"NONE" width="0.1 mm""#, r#"SOLID" width="0.12 mm""#), ONE_COLUMN);
    assert!(same && section(&out).contains(r#"colCnt="2" cellSpacing="0" borderFillIDRef="1""#), "{}", section(&out));
    save("engine-new-table", "reused-border-fill", &out);
}

#[test]
fn deleting_the_first_paragraph_keeps_the_section_settings() {
    let pkg = hwpx(&[p("첫째"), p("둘째")]);
    let imp = import(&pkg);
    let r = rewrite(&imp.remainder, &imp.text, &imp.text.replace("첫째\n\n", ""), CAPS).unwrap();
    let sec = section(&export(&r.text, &r.remainder));
    assert!(sec.contains("<hp:secPr") && sec.find("<hp:secPr").unwrap() < sec.find("둘째").unwrap(), "{sec}");
    // Two page breaks in a row cannot be paragraph properties.
    let two = imp.text.replace("\n둘째\n", "\n<pagebreak/>\n\n<pagebreak/>\n\n둘째\n");
    match HwpxEngine.export(&two, &imp.remainder) {
        Err(EngineError::Refused(m)) => assert!(m.contains("two page breaks"), "{m}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_section_whose_paragraphs_all_went_keeps_its_settings() {
    // Two sections: section1.xml is section0.xml with other text.
    let pkg = hwpx(&[p("가"), p("나")]);
    let mut parts = package::read(&pkg).unwrap();
    let mut second = parts.iter().find(|p| p.name == "Contents/section0.xml").unwrap().clone();
    second.name = "Contents/section1.xml".into();
    second.data = String::from_utf8(second.data).unwrap().replace('가', "다").replace('나', "라").into_bytes();
    parts.push(second);
    let pkg = package::write(&parts).unwrap();
    let imp = import(&pkg);
    let r = rewrite(&imp.remainder, &imp.text, &imp.text.replace("\n\n다\n\n라\n", "\n"), CAPS).unwrap();
    let out = export(&r.text, &r.remainder);
    let parts = package::read(&out).unwrap();
    let sec1 = String::from_utf8(package::get(&parts, "Contents/section1.xml").unwrap().to_vec()).unwrap();
    assert!(sec1.contains("<hp:secPr") && !sec1.contains('다'), "{sec1}");
    // The section keeps one empty paragraph for its settings, which the text then shows.
    assert_eq!(import(&out).text, format!("{}\n<p/>\n", r.text));
}

/// The top-level paragraphs of a section part.
fn top_paras(sec: &str) -> Vec<xml::Element> {
    xml::parse(sec.as_bytes()).unwrap().root.elements().filter(|e| e.is("hp:p")).cloned().collect()
}

/// A paragraph's layout cache counts the section's start run (`hp:secPr`,
/// `hp:colPr`: 16 characters) when it holds it. A paragraph put before the
/// first one takes the run, and the old first paragraph's cache pointed
/// past its end (Hancom's repair prompt on the kit's fdi e10): that cache
/// goes, and every other stays.
#[test]
fn a_layout_cache_past_the_paragraph_end_is_not_written() {
    // secPr 8 + colPr 8 + pageNum 8: the second line starts at 24, on "abc".
    let first = r#"<hp:p id="0" paraPrIDRef="0" styleIDRef="0" pageBreak="0" columnBreak="0" merged="0"><hp:run charPrIDRef="0"><hp:ctrl><hp:pageNum pos="BOTTOM_CENTER" formatType="DIGIT" sideChar="-"/></hp:ctrl><hp:t>abc</hp:t></hp:run><hp:linesegarray><hp:lineseg textpos="0"/><hp:lineseg textpos="24"/></hp:linesegarray></hp:p>"#;
    let pkg = hwpx(&[first.into(), p("둘째"), p("셋째")]);
    assert_eq!(hanji_hwpx::layout_problems(&pkg).unwrap(), Vec::<String>::new());
    let imp = import(&pkg);
    // GetPut keeps every cache.
    assert_eq!(section(&export(&imp.text, &imp.remainder)), section(&pkg));
    let lines = |p: &xml::Element| p.child("hp:linesegarray").is_some();

    // A new paragraph before the first takes the start run; "abc" gives it up.
    let r = rewrite(&imp.remainder, &imp.text, &imp.text.replacen("abc", "새 첫 문단\n\nabc", 1), CAPS).unwrap();
    let out = export(&r.text, &r.remainder);
    let sec = section(&out);
    let ps = top_paras(&sec);
    assert!(!ps[0].descendants("hp:secPr").is_empty(), "{sec}");
    assert_eq!(ps[1].text_of(&["hp:t"]), "abc");
    assert!(!lines(&ps[1]) && lines(&ps[2]) && lines(&ps[3]), "{sec}");
    assert_eq!(hanji_hwpx::layout_problems(&out).unwrap(), Vec::<String>::new());
    save("engine-section-start", "ORIGINAL", &pkg);
    save("engine-section-start", "new-first", &out);

    // Without the start run, "abc"'s own cache is the fault the check reports.
    let mut stale = xml::parse(first.as_bytes()).unwrap().root;
    assert!(hanji_hwpx::owpml::stale_layout(&stale).unwrap().contains("starts at 24"));
    hanji_hwpx::owpml::drop_stale_layouts(&mut stale);
    assert!(!lines(&stale));
}

/// The layout check: a line past the paragraph's end (the characters, then
/// the end mark) is stale; controls and objects count 8, a line break 1.
#[test]
fn a_layout_cache_past_the_paragraph_end_is_stale() {
    use hanji_hwpx::owpml::{char_count, stale_layout};
    let para = |inner: &str, pos: &[usize]| {
        let lines: String = pos.iter().map(|t| format!(r#"<hp:lineseg textpos="{t}"/>"#)).collect();
        let x = format!(r#"<hp:p {NS}>{inner}<hp:linesegarray>{lines}</hp:linesegarray></hp:p>"#);
        xml::parse(x.as_bytes()).unwrap().root
    };
    let inner = r#"<hp:run><hp:ctrl><hp:colPr/></hp:ctrl><hp:tbl/><hp:t>a&amp;b<hp:tab/>c<hp:lineBreak/>d<hp:markpenBegin/>😀</hp:t></hp:run>"#;
    // colPr 8, tbl 8, "a&b" 3, tab 8, "c" 1, line break 1, "d" 1, 😀 2 (UTF-16).
    assert_eq!(char_count(&para(inner, &[0])), 32);
    assert!(stale_layout(&para(inner, &[0, 20, 33])).is_none());
    assert!(stale_layout(&para(inner, &[0, 34])).unwrap().contains("starts at 34"));
    assert!(stale_layout(&para("<hp:run/>", &[0])).is_none());
}

// ---------------------------------------------------------------- §10.2 tracked changes

const TRACKED: &str = r##"<hh:trackChanges itemCnt="2"><hh:trackChange type="Delete" date="2026-09-28T00:00:00Z" authorID="1" hide="0" id="1"/><hh:trackChange type="Insert" date="2026-09-28T00:00:00Z" authorID="1" hide="0" id="2"/></hh:trackChanges><hh:trackChangeAuthors itemCnt="1"><hh:trackChangeAuthor name="김검토" mark="1" color="#FF0000" id="1"/></hh:trackChangeAuthors>"##;

fn tracked_pkg() -> Vec<u8> {
    let changed = r#"<hp:p id="0" paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:t>매출은 <hp:deleteBegin Id="1" TcId="1" paraend="0"/>12%<hp:deleteEnd Id="1" TcId="1" paraend="0"/><hp:insertBegin Id="2" TcId="2" paraend="0"/>15%<hp:insertEnd Id="2" TcId="2" paraend="0"/> 증가</hp:t></hp:run></hp:p>"#;
    // An insertion that spans two paragraphs.
    let open = r#"<hp:p id="0" paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:t>새 <hp:insertBegin Id="3" TcId="2" paraend="1"/>문단</hp:t></hp:run></hp:p>"#;
    let close = r#"<hp:p id="0" paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:t>이어짐<hp:insertEnd Id="3" TcId="2" paraend="1"/></hp:t></hp:run></hp:p>"#;
    hwpx_with(&[p("머리"), changed.into(), p("가운데"), open.into(), close.into(), p("끝")], TRACKED, vec![], "")
}

#[test]
fn tracked_changes_are_read_only_placeholders_and_never_dropped() {
    let pkg = tracked_pkg();
    let (_, _, report, stats) = HwpxEngine::split(&pkg, &ImportOptions::default()).unwrap();
    assert_eq!(stats.tracked_paragraphs, 3);
    assert!(report.surface.iter().any(|n| n.kind == "tracked-deletion" && n.detail == "12%"), "{:?}", report.surface);
    let imp = import(&pkg);
    assert!(
        imp.text.contains(r#"kind="tracked-change" summary="tracked deletion by 김검토: 매출은 12%15% 증가""#),
        "{}",
        imp.text
    );
    assert_eq!(imp.text.matches("kind=\"tracked-change\"").count(), 3, "{}", imp.text);
    // GetPut, and an edit elsewhere keeps every mark byte for byte.
    let getput = export(&imp.text, &imp.remainder);
    assert_eq!(canon(&getput), canon(&pkg));
    let r = edit(&imp.remainder, &imp.text, "가운데", "중간", CAPS).unwrap();
    let out = export(&r.text, &r.remainder);
    save("engine-tracked", "ORIGINAL", &pkg);
    save("engine-tracked", "getput", &getput);
    save("engine-tracked", "E1-exact", &out);
    let marks = |s: &str| s.matches("Begin Id=").count() + s.matches("End Id=").count();
    assert_eq!(marks(&section(&out)), 6);
    assert_eq!(import(&out).text, r.text, "PutGet");
    // Deleting a tracked change's placeholder is refused at export: nothing is dropped silently.
    let keep_line = imp.text.lines().find(|l| l.contains("tracked deletion")).unwrap();
    let gone = imp.text.replace(&format!("{keep_line}\n\n"), "");
    let r = rewrite(&imp.remainder, &imp.text, &gone, CAPS).unwrap();
    match HwpxEngine.export(&r.text, &r.remainder) {
        Err(EngineError::Refused(m)) => {
            assert!(m.contains("tracked deletion by 김검토") && m.contains("cannot be dropped"), "{m}")
        }
        other => panic!("{other:?}"),
    }
    // Nor can a paragraph inside a change that spans paragraphs go: it holds no mark.
    let pkg3 = {
        let open = r#"<hp:p id="0" paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:t>새 <hp:insertBegin Id="3" TcId="2" paraend="1"/>문단</hp:t></hp:run></hp:p>"#;
        let close = r#"<hp:p id="0" paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:t>이어짐<hp:insertEnd Id="3" TcId="2" paraend="1"/></hp:t></hp:run></hp:p>"#;
        hwpx_with(&[p("머리"), open.into(), p("안쪽"), close.into()], TRACKED, vec![], "")
    };
    let imp3 = import(&pkg3);
    let inside = imp3.text.lines().find(|l| l.contains("inside a tracked change: 안쪽")).unwrap();
    let r = rewrite(&imp3.remainder, &imp3.text, &imp3.text.replace(&format!("{inside}\n\n"), ""), CAPS).unwrap();
    match HwpxEngine.export(&r.text, &r.remainder) {
        Err(EngineError::Refused(m)) => assert!(m.contains("removes a paragraph of a tracked change"), "{m}"),
        other => panic!("{other:?}"),
    }
    // A change over two paragraphs cannot be cut: nothing may go between them.
    let lines: Vec<&str> = imp.text.lines().filter(|l| l.contains("tracked insertion")).collect();
    assert_eq!(lines.len(), 2);
    let cut = imp.text.replace(&format!("{}\n\n", lines[1]), &format!("새 글\n\n{}\n\n", lines[1]));
    let r = rewrite(&imp.remainder, &imp.text, &cut, CAPS).unwrap();
    match HwpxEngine.export(&r.text, &r.remainder) {
        Err(EngineError::Refused(m)) => assert!(m.contains("cut or reordered"), "{m}"),
        other => panic!("{other:?}"),
    }
}

// ---------------------------------------------------------------- §8

#[test]
fn scripts_ole_and_linked_files_are_neutralised_and_reported() {
    let ole = r#"<hp:p id="0" paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:ole id="5" binaryItemIDRef="ole1"><hp:sz width="100" height="100"/></hp:ole><hp:t>표 앞</hp:t></hp:run></hp:p>"#;
    let pic = r#"<hp:p id="0" paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:pic id="6"><hc:img binaryItemIDRef="img1"/></hp:pic><hp:t>그림</hp:t></hp:run></hp:p>"#;
    let hidden = r#"<hp:p id="0" paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:ctrl><hp:hiddenComment><hp:subList><hp:p><hp:run><hp:t>숨은 설명</hp:t></hp:run></hp:p></hp:subList></hp:hiddenComment></hp:ctrl><hp:t>본문</hp:t></hp:run></hp:p>"#;
    let manifest = r#"<opf:item id="headersc" href="Scripts/headerScripts" media-type="application/x-javascript ;charset=utf-16"/><opf:item id="ole1" href="BinData/ole1.OLE" media-type="application/ole"/><opf:item id="ole2" href="BinData/ole2.OLE" media-type="application/ole"/><opf:item id="img1" href="C:\\Users\\a\\photo.png" media-type="image/png" isEmbeded="0"/><opf:item id="img2" href="BinData/image2.png" media-type="image/png"/>"#;
    let fills = r#"<hh:borderFills itemCnt="3"><hh:borderFill id="8"><hc:fillBrush><hc:imgBrush mode="TOTAL"><hc:img binaryItemIDRef="img1"/></hc:imgBrush></hc:fillBrush></hh:borderFill><hh:borderFill id="9"><hc:fillBrush><hc:winBrush/><hc:imgBrush mode="TOTAL"><hc:img binaryItemIDRef="img1"/></hc:imgBrush></hc:fillBrush></hh:borderFill><hh:borderFill id="10"><hc:fillBrush/></hh:borderFill></hh:borderFills>"#;
    let docopt = r#"</hh:refList><hh:docOption><hh:linkinfo path="\\\\server\\share\\base.hwpx" pageInherit="1" footnoteInherit="0"/></hh:docOption><hh:refList>"#;
    let pkg = hwpx_with(
        &[p("시작"), ole.into(), pic.into(), hidden.into()],
        &format!("{fills}{docopt}"),
        vec![
            part("Scripts/headerScripts", "function OnDocument_New() {}"),
            part("BinData/ole1.OLE", "OLE"),
            part("BinData/ole2.OLE", "OLE"),
            part("BinData/image2.png", "PNG"),
            // A master page (copied through) with an object of its own.
            part(
                "Contents/masterpage0.xml",
                &format!(
                    r#"<masterPage {NS}><hp:subList><hp:p><hp:run><hp:ole id="7" binaryItemIDRef="ole2"/></hp:run></hp:p></hp:subList></masterPage>"#
                ),
            ),
        ],
        manifest,
    );
    let imp = import(&pkg);
    let kinds: Vec<&str> = imp.report.neutralised.iter().map(|n| n.kind.as_str()).collect();
    for k in ["macros", "ole-object", "linked-image", "linked-document"] {
        assert!(kinds.contains(&k), "{k} not in {kinds:?}");
    }
    let surface: Vec<&str> = imp.report.surface.iter().map(|n| n.kind.as_str()).collect();
    for k in ["hidden-text", "metadata", "preview-text"] {
        assert!(surface.contains(&k), "{k} not in {surface:?}");
    }
    let out = export(&imp.text, &imp.remainder);
    let parts = package::read(&out).unwrap();
    let names: Vec<&str> = parts.iter().map(|p| p.name.as_str()).collect();
    for gone in ["Scripts/headerScripts", "BinData/ole1.OLE", "BinData/ole2.OLE"] {
        assert!(!names.contains(&gone), "{gone} in {names:?}");
    }
    assert!(names.contains(&"BinData/image2.png"), "embedded pictures stay");
    let all: String = parts
        .iter()
        .filter(|p| p.name.ends_with(".xml") || p.name.ends_with(".hpf"))
        .map(|p| String::from_utf8_lossy(&p.data).into_owned())
        .collect();
    for gone in ["Scripts/", "<hp:ole", "ole1", "ole2", "photo.png", "server", "img1", "<hp:pic"] {
        assert!(!all.contains(gone), "{gone} still in the export");
    }
    assert!(imp.text.contains("표 앞") && imp.text.contains("그림"), "{}", imp.text);
    // The picture of the linked image and the header's image fill of it go too.
    let located: Vec<&str> = imp.report.neutralised.iter().map(|n| n.location.as_str()).collect();
    assert!(located.contains(&"Contents/section0.xml <hp:pic>"), "{located:?}");
    assert!(located.contains(&"Contents/header.xml <hc:imgBrush>"), "{located:?}");
    assert!(located.contains(&"Contents/masterpage0.xml <hp:ole>"), "{located:?}");
    // A fill that was only the image goes whole; others keep the rest and are otherwise untouched.
    for kept in [
        r#"<hh:borderFill id="8"/>"#,
        r#"<hh:borderFill id="9"><hc:fillBrush><hc:winBrush/></hc:fillBrush></hh:borderFill>"#,
        r#"<hh:borderFill id="10"><hc:fillBrush/></hh:borderFill>"#,
    ] {
        assert!(all.contains(kept), "{kept} not in {all}");
    }
    let again = import(&out);
    assert!(again.report.neutralised.is_empty(), "{:?}", again.report.neutralised);
    assert_eq!(again.text, imp.text);
    // A reference hanji does not know how to remove is refused, not left dangling.
    let odd = r#"<hp:p id="0" paraPrIDRef="0" styleIDRef="0"><hp:run charPrIDRef="0"><hp:video videotype="Local" fileIDRef="img1" imageIDRef=""/><hp:t>영상</hp:t></hp:run></hp:p>"#;
    let pkg = hwpx_with(&[p("시작"), odd.into()], "", vec![], manifest);
    match HwpxEngine.import(&pkg, &ImportOptions::default()) {
        Err(EngineError::Package(m)) => assert!(m.contains("<hp:video> refers to img1"), "{m}"),
        other => panic!("{:?}", other.map(|i| i.text)),
    }
}

#[test]
fn a_package_that_is_not_hwpx_is_refused() {
    let docxish = package::write(&[part("[Content_Types].xml", "<Types/>")]).unwrap();
    assert!(
        matches!(HwpxEngine.import(&docxish, &ImportOptions::default()), Err(EngineError::Package(m)) if m.contains("not an hwpx package"))
    );
    let ambiguous = import(&hwpx(&[p("반복"), p("반복")]));
    assert!(
        matches!(edit(&ambiguous.remainder, &ambiguous.text, "반복", "x", CAPS), Err(Refusal::Edit(m)) if m.contains("occurs 2 times"))
    );
}
