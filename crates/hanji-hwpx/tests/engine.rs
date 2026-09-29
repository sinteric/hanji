//! Engine behaviour on small synthetic packages: the §5.2 mapping, exact
//! edits, side-by-side tables (§10.8), tracked changes (§10.2), §8
//! neutralisation and the surface-before-export list.

use hanji_core::{edit, rewrite, Capabilities, Engine, EngineError, ImportOptions, Part, Refusal};
use hanji_hwpx::{package, xml, HwpxEngine};

const CAPS: Capabilities = Capabilities { links: false, fields: false, footnotes: false, math: false };
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
    let want = "# 3분기 보고\n\n매출은 **12%** 증가했다.\n\n<div style=\"본문\">빨간 글자</div>\n\n<p/>\n\n첫 줄<br/>둘째 줄\t탭\n\n| 지역 | 매출 |\n|---|---|\n| 서울 | 120 |\n| ^^ | 95 |\n| 합계 ||\n\n| 비고 |\n|---|\n| 부산<p/>해운대 |\n\n<pagebreak/>\n\n다음 쪽\n";
    assert!(imp.text.ends_with(want), "{}", imp.text);
    // GetPut: the section comes back canonically equal, the header byte-equal.
    let out = export(&imp.text, &imp.remainder);
    assert_eq!(canon(&out), canon(&pkg));
    let hdr = |p: &[u8]| package::get(&package::read(p).unwrap(), "Contents/header.xml").unwrap().to_vec();
    assert_eq!(hdr(&out), hdr(&pkg));
    // hwpx has no table styles: the validator says so.
    let bad = imp.text.replace("| 비고 |", "{style=\"Grid\"}\n| 비고 |");
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
fn side_by_side_tables_are_separate_blocks_in_one_paragraph() {
    let a = tbl(&[&[("담당 부서", 1, 1), ("기획과", 1, 1)]], 2);
    let b = tbl(&[&[("담당자", 1, 1), ("홍길동", 1, 1)]], 2);
    let pkg = hwpx(&[p("앞"), anchor(&[a, b]), p("뒤")]);
    let (blocks, _, _, stats) = HwpxEngine::split(&pkg, &ImportOptions::default()).unwrap();
    assert_eq!((blocks.len(), stats.side_by_side), (4, 1));
    let imp = import(&pkg);
    assert!(imp.text.contains("| 담당 부서 | 기획과 |\n|---|---|\n\n| 담당자 | 홍길동 |\n|---|---|\n"), "{}", imp.text);
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
        .replace("\n\n| 담당자 | 홍길동 |\n|---|---|\n", "\n")
        .replace("\n뒤\n", "\n뒤\n\n| 담당자 | 홍길동 |\n|---|---|\n");
    let r = rewrite(&imp.remainder, &imp.text, &moved, CAPS).unwrap();
    let out = export(&r.text, &r.remainder);
    let sec = section(&out);
    assert!(!sec.contains("</hp:tbl><hp:tbl "), "{sec}");
    assert!(
        sec.find("기획과").unwrap() < sec.find("뒤").unwrap() && sec.find("뒤").unwrap() < sec.find("홍길동").unwrap()
    );
    assert_eq!(import(&out).text, r.text, "PutGet");
    // Deleting the first table leaves the second in the shared paragraph.
    let gone = imp.text.replace("| 담당 부서 | 기획과 |\n|---|---|\n\n", "");
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
    let manifest = r#"<opf:item id="headersc" href="Scripts/headerScripts" media-type="application/x-javascript ;charset=utf-16"/><opf:item id="ole1" href="BinData/ole1.OLE" media-type="application/ole"/><opf:item id="img1" href="C:\\Users\\a\\photo.png" media-type="image/png" isEmbeded="0"/><opf:item id="img2" href="BinData/image2.png" media-type="image/png"/>"#;
    let docopt = r#"</hh:refList><hh:docOption><hh:linkinfo path="\\\\server\\share\\base.hwpx" pageInherit="1" footnoteInherit="0"/></hh:docOption><hh:refList>"#;
    let pkg = hwpx_with(
        &[p("시작"), ole.into(), pic.into(), hidden.into()],
        docopt,
        vec![
            part("Scripts/headerScripts", "function OnDocument_New() {}"),
            part("BinData/ole1.OLE", "OLE"),
            part("BinData/image2.png", "PNG"),
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
    assert!(!names.contains(&"Scripts/headerScripts") && !names.contains(&"BinData/ole1.OLE"), "{names:?}");
    assert!(names.contains(&"BinData/image2.png"), "embedded pictures stay");
    let all: String = parts
        .iter()
        .filter(|p| p.name.ends_with(".xml") || p.name.ends_with(".hpf"))
        .map(|p| String::from_utf8_lossy(&p.data).into_owned())
        .collect();
    for gone in ["Scripts/", "<hp:ole", "ole1", "photo.png", "server"] {
        assert!(!all.contains(gone), "{gone} still in the export");
    }
    assert!(imp.text.contains("표 앞") && imp.text.contains("그림"), "{}", imp.text);
    let again = import(&out);
    assert!(again.report.neutralised.is_empty(), "{:?}", again.report.neutralised);
    assert_eq!(again.text, imp.text);
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
