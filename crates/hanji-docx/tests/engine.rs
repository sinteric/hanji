//! Engine behaviour on small synthetic packages: exact edits, refusals,
//! §8 neutralisation and the surface-before-export list.

use hanji_core::{edit, rewrite, Capabilities, Engine, EngineError, ImportOptions, Kind, Part, Refusal};
use hanji_docx::{package, DocxEngine};

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const CAPS: Capabilities = Capabilities { links: false, fields: false, footnotes: false, math: false };

fn part(name: &str, data: &str) -> Part {
    Part { name: name.into(), data: data.as_bytes().to_vec(), dos_time: 0x5b21_0000, external_attr: 0, deflate: true }
}

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style><w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/></w:style><w:style w:type="paragraph" w:styleId="Note"><w:name w:val="Note"/></w:style><w:style w:type="table" w:default="1" w:styleId="TableNormal"><w:name w:val="Normal Table"/></w:style><w:style w:type="table" w:styleId="Grid"><w:name w:val="Table Grid"/></w:style></w:styles>"#;

/// A package whose body is `body` (WordprocessingML), with extra parts.
fn docx(body: &str, extra: Vec<Part>) -> Vec<u8> {
    let doc = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="{W}" xmlns:r="{R}" xmlns:o="urn:schemas-microsoft-com:office:office" xmlns:v="urn:schemas-microsoft-com:vml"><w:body>{body}<w:sectPr/></w:body></w:document>"#
    );
    let mut parts = vec![
        part(
            "[Content_Types].xml",
            r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.ms-word.document.macroEnabled.main+xml"/><Override PartName="/word/vbaProject.bin" ContentType="application/vnd.ms-office.vbaProject"/></Types>"#,
        ),
        part(
            "_rels/.rels",
            r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
        ),
        part("word/document.xml", &doc),
        part("word/styles.xml", STYLES),
    ];
    parts.extend(extra);
    package::write(&parts).unwrap()
}

fn p(text: &str) -> String {
    format!("<w:p><w:r><w:t xml:space=\"preserve\">{text}</w:t></w:r></w:p>")
}

#[test]
fn exact_edit_lands_and_refuses_ambiguous_old_text() {
    let body = format!(
        "{}<w:p><w:r><w:rPr><w:color w:val=\"FF0000\"/></w:rPr><w:t>매출 2023</w:t></w:r><w:bookmarkStart w:id=\"0\" w:name=\"b\"/><w:r><w:t xml:space=\"preserve\"> 증가</w:t></w:r><w:bookmarkEnd w:id=\"0\"/></w:p>{}",
        p("반복"),
        p("반복")
    );
    let pkg = docx(&body, vec![]);
    let imp = DocxEngine.import(&pkg, &ImportOptions::default()).unwrap();
    let r = edit(&imp.remainder, &imp.text, "2023", "2024", CAPS).unwrap();
    assert!(r.text.contains("매출 2024 증가"), "{}", r.text);
    assert!(r.report.refused.is_empty() && r.report.removed.is_empty(), "{:?}", r.report);
    let out = DocxEngine.export(&r.text, &r.remainder).unwrap();
    let again = DocxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert_eq!(again.text, r.text);
    // the colour and the bookmark stay on their text
    let xml =
        String::from_utf8(package::get(&package::read(&out).unwrap(), "word/document.xml").unwrap().to_vec()).unwrap();
    assert!(xml.contains("<w:color w:val=\"FF0000\"/></w:rPr><w:t>매출 2024</w:t>"), "{xml}");
    assert!(xml.contains("</w:r><w:bookmarkStart"), "{xml}");
    match edit(&imp.remainder, &imp.text, "반복", "x", CAPS) {
        Err(Refusal::Edit(m)) => assert!(m.contains("occurs 2 times"), "{m}"),
        other => panic!("{other:?}"),
    }
    assert!(matches!(edit(&imp.remainder, &imp.text, "없음", "x", CAPS), Err(Refusal::Edit(_))));
}

#[test]
fn a_wrapper_split_across_paragraphs_is_refused() {
    let body =
        r#"<w:p><w:hyperlink r:id="rId9"><w:r><w:t xml:space="preserve">alpha beta</w:t></w:r></w:hyperlink></w:p>"#;
    let pkg = docx(body, vec![]);
    let imp = DocxEngine.import(&pkg, &ImportOptions::default()).unwrap();
    let new = imp.text.replace("alpha beta", "alpha\n\nbeta");
    match rewrite(&imp.remainder, &imp.text, &new, CAPS) {
        Err(Refusal::Unplaceable(r)) => {
            assert!(r.refused.iter().any(|x| x.2.contains("different paragraphs")), "{r:?}")
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn identical_blocks_with_different_properties_are_refused_not_swapped() {
    let spacing = |n: u32| format!("<w:p><w:pPr><w:spacing w:after=\"{n}\"/></w:pPr></w:p>");
    let body = format!("{}{}{}{}{}", p("A"), spacing(100), p("B"), spacing(200), p("C"));
    let imp = DocxEngine.import(&docx(&body, vec![]), &ImportOptions::default()).unwrap();
    assert!(imp.text.contains("A\n\n<p/>\n\nB"), "{}", imp.text);
    // Both empty paragraphs move; the text cannot say which is which.
    let new = imp.text.replace("A\n\n<p/>\n\nB\n\n<p/>\n\nC", "<p/>\n<p/>\n\nA\n\nB\n\nC");
    match rewrite(&imp.remainder, &imp.text, &new, CAPS) {
        Err(Refusal::Unplaceable(r)) => assert!(r.refused.iter().any(|x| x.2.contains("identical blocks")), "{r:?}"),
        other => panic!("{other:?}"),
    }
    // Moving just one of them, next to a unique neighbour, is fine.
    let one = imp.text.replace("\n\nC", "\n\nC\n\n<p/>");
    assert!(rewrite(&imp.remainder, &imp.text, &one, CAPS).is_ok());
}

#[test]
fn empty_paragraphs_and_multi_paragraph_cells_round_trip() {
    let cell = |inner: &str| format!("<w:tc>{inner}</w:tc>");
    let tbl = format!(
        "<w:tbl><w:tblPr><w:tblStyle w:val=\"Grid\"/></w:tblPr><w:tblGrid><w:gridCol/><w:gridCol/></w:tblGrid><w:tr>{}{}</w:tr><w:tr>{}{}</w:tr></w:tbl>",
        cell(&p("지역")),
        cell(&p("비고")),
        cell(&p("부산")),
        cell(&format!("{}<w:p/>{}", p("해운대 1곳"), "<w:p><w:pPr><w:pStyle w:val=\"Note\"/></w:pPr><w:r><w:t>잠정치</w:t></w:r></w:p>"))
    );
    let body = format!("{}<w:p/><w:p><w:pPr><w:pStyle w:val=\"Note\"/></w:pPr></w:p>{tbl}", p("앞"));
    let pkg = docx(&body, vec![]);
    let imp = DocxEngine.import(&pkg, &ImportOptions::default()).unwrap();
    assert!(
        imp.text.contains("앞\n\n<p/>\n<p style=\"Note\"/>\n\n{style=\"Table Grid\"}\n| 지역 | 비고 |"),
        "{}",
        imp.text
    );
    assert!(imp.text.contains("| 부산 | 해운대 1곳<p/><p style=\"Note\"/>잠정치 |"), "{}", imp.text);
    let out = DocxEngine.export(&imp.text, &imp.remainder).unwrap();
    assert_eq!(DocxEngine.import(&out, &ImportOptions::default()).unwrap().text, imp.text);
    // split a cell paragraph with an exact edit
    let r = edit(&imp.remainder, &imp.text, "해운대 1곳", "해운대<p/>1곳", CAPS).unwrap();
    let out = DocxEngine.export(&r.text, &r.remainder).unwrap();
    assert_eq!(DocxEngine.import(&out, &ImportOptions::default()).unwrap().text, r.text);
}

#[test]
fn invalid_text_is_refused_with_the_allowed_names() {
    let imp = DocxEngine.import(&docx(&p("hello"), vec![]), &ImportOptions::default()).unwrap();
    let bad = imp.text.replace("hello", "<div style=\"Nope\">hello</div>");
    match DocxEngine.export(&bad, &imp.remainder) {
        Err(EngineError::Invalid(d)) => {
            let m = d[0].to_string();
            assert!(m.contains("Allowed paragraph styles: \"Normal\", \"heading 1\", \"Note\""), "{m}");
        }
        other => panic!("{other:?}"),
    }
    let bad = imp.text.replace("hello", "[link](https://example.org)");
    assert!(matches!(DocxEngine.export(&bad, &imp.remainder), Err(EngineError::Invalid(_))));
}

#[test]
fn active_and_remote_content_is_neutralised_and_reported() {
    let body = format!(
        "{}{}{}{}{}{}",
        r#"<w:p><w:fldSimple w:instr=" DDEAUTO Excel Sheet1 R1C1 "><w:r><w:t>42</w:t></w:r></w:fldSimple></w:p>"#,
        r#"<w:p><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> INCLUDEPICTURE "http://example.org/a.png" \d </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>picture</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p>"#,
        r#"<w:p><w:r><w:object><v:shape id="s1"><v:imagedata r:id="rId3"/></v:shape><o:OLEObject Type="Embed" ProgID="Excel.Sheet.12" ShapeID="s1" r:id="rId4"/></w:object></w:r></w:p>"#,
        r#"<w:p><w:r><w:rPr><w:vanish/></w:rPr><w:t>secret</w:t></w:r></w:p>"#,
        r#"<w:p><w:del w:id="1" w:author="Kim"><w:r><w:delText>gone</w:delText></w:r></w:del></w:p>"#,
        p("kept"),
    );
    let rels = r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/p.png"/><Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/package" Target="embeddings/x.xlsx"/><Relationship Id="rId5" Type="http://schemas.microsoft.com/office/2006/relationships/vbaProject" Target="vbaProject.bin"/><Relationship Id="rId6" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="http://example.org/linked.png" TargetMode="External"/><Relationship Id="rId7" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="http://example.org/" TargetMode="External"/></Relationships>"#;
    let settings_rels = r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/attachedTemplate" Target="http://evil.example/t.dotm" TargetMode="External"/></Relationships>"#;
    let settings = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><w:settings xmlns:w="{W}" xmlns:r="{R}"><w:attachedTemplate r:id="rId1"/></w:settings>"#
    );
    let comments = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><w:comments xmlns:w="{W}"><w:comment w:id="0" w:author="Park"><w:p><w:r><w:t>check this</w:t></w:r></w:p></w:comment></w:comments>"#
    );
    let core = r#"<?xml version="1.0" encoding="UTF-8"?><cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:creator>Hong</dc:creator></cp:coreProperties>"#;
    let pkg = docx(
        &body,
        vec![
            part("word/_rels/document.xml.rels", rels),
            part("word/settings.xml", &settings),
            part("word/_rels/settings.xml.rels", settings_rels),
            part("word/vbaProject.bin", "MACRO"),
            part("word/embeddings/x.xlsx", "OLE"),
            part("word/media/p.png", "PNG"),
            part("word/comments.xml", &comments),
            part("docProps/core.xml", core),
        ],
    );
    let imp = DocxEngine.import(&pkg, &ImportOptions::default()).unwrap();
    let kinds: Vec<&str> = imp.report.neutralised.iter().map(|n| n.kind.as_str()).collect();
    for k in ["fetching-field", "ole-object", "macros", "linked-image", "remote-template"] {
        assert!(kinds.contains(&k), "{k} not in {kinds:?}");
    }
    assert_eq!(kinds.iter().filter(|k| **k == "fetching-field").count(), 2, "{kinds:?}");
    let surface: Vec<&str> = imp.report.surface.iter().map(|n| n.kind.as_str()).collect();
    for k in ["hidden-text", "tracked-deletion", "comment", "metadata"] {
        assert!(surface.contains(&k), "{k} not in {surface:?}");
    }
    // Field results stay as text; the instructions are gone.
    assert!(imp.text.contains("42") && imp.text.contains("picture"), "{}", imp.text);
    assert!(!imp.text.contains("field"), "{}", imp.text);
    let out = DocxEngine.export(&imp.text, &imp.remainder).unwrap();
    let parts = package::read(&out).unwrap();
    let names: Vec<&str> = parts.iter().map(|p| p.name.as_str()).collect();
    assert!(!names.contains(&"word/vbaProject.bin") && !names.contains(&"word/embeddings/x.xlsx"), "{names:?}");
    assert!(names.contains(&"word/media/p.png"));
    let all: String = parts
        .iter()
        .filter(|p| p.name.ends_with(".xml") || p.name.ends_with(".rels"))
        .map(|p| String::from_utf8_lossy(&p.data).into_owned())
        .collect();
    for gone in [
        "DDEAUTO",
        "INCLUDEPICTURE",
        "OLEObject",
        "vbaProject",
        "attachedTemplate",
        "linked.png",
        "evil.example",
        "macroEnabled",
    ] {
        assert!(!all.contains(gone), "{gone} still in the export");
    }
    assert!(all.contains("http://example.org/\""), "the hyperlink relationship is kept");
    let again = DocxEngine.import(&out, &ImportOptions::default()).unwrap();
    assert!(again.report.neutralised.is_empty(), "{:?}", again.report.neutralised);
    assert_eq!(again.text, imp.text);
}

#[test]
fn markers_in_table_structure_keep_the_pipe_table() {
    // Markers after tblGrid, between rows, between cells, after the last cell
    // and after the last row (survey: 15.6% of docx tables have one).
    let cell = |t: &str| format!("<w:tc><w:tcPr><w:tcW w:w=\"2000\" w:type=\"dxa\"/></w:tcPr>{}</w:tc>", p(t));
    let body = format!(
        "<w:tbl><w:tblPr><w:tblW w:w=\"0\" w:type=\"auto\"/></w:tblPr><w:tblGrid><w:gridCol w:w=\"2000\"/><w:gridCol w:w=\"2000\"/></w:tblGrid>\
         <w:bookmarkStart w:id=\"1\" w:name=\"t\"/>\
         <w:tr>{}<w:proofErr w:type=\"spellStart\"/>{}<w:bookmarkEnd w:id=\"1\"/></w:tr>\
         <w:bookmarkStart w:id=\"2\" w:name=\"r2\"/>\
         <w:tr><w:trPr><w:cantSplit/></w:trPr>{}{}</w:tr>\
         <w:bookmarkEnd w:id=\"2\"/></w:tbl>{}",
        cell("a"),
        cell("b"),
        cell("c"),
        cell("d"),
        p("after")
    );
    let pkg = docx(&body, vec![]);
    let (_, _, _, stats) = DocxEngine::split(&pkg, &ImportOptions::default()).unwrap();
    assert_eq!((stats.tables_modelled, stats.tables_kept), (1, 0), "{:?}", stats.kept_reasons);
    let imp = DocxEngine.import(&pkg, &ImportOptions::default()).unwrap();
    assert!(imp.text.contains("| a | b |\n|---|---|\n| c | d |"), "{}", imp.text);
    let doc = |pkg: &[u8]| package::get(&package::read(pkg).unwrap(), "word/document.xml").unwrap().to_vec();
    let canon = |pkg: &[u8]| hanji_docx::xml::canon_part(&doc(pkg)).unwrap();
    // GetPut: the document comes back canonically equal.
    let out = DocxEngine.export(&imp.text, &imp.remainder).unwrap();
    assert_eq!(canon(&out), canon(&pkg));
    // An edit in a cell keeps every marker where it was.
    let r = edit(&imp.remainder, &imp.text, "| c |", "| see |", CAPS).unwrap();
    assert!(r.report.refused.is_empty() && r.report.removed.is_empty(), "{:?}", r.report);
    let out = DocxEngine.export(&r.text, &r.remainder).unwrap();
    assert_eq!(canon(&out), canon(&pkg).replace(">c</", ">see</"));
    // A rewrite that adds a row keeps them too, and nothing is refused.
    let new = r.text.replace("| see | d |", "| see | d |\n| e | f |");
    let r2 = rewrite(&r.remainder, &r.text, &new, CAPS).unwrap();
    let xml = String::from_utf8(doc(&DocxEngine.export(&r2.text, &r2.remainder).unwrap())).unwrap();
    let order = [
        "<w:tblGrid>",
        "w:name=\"t\"",
        ">a<",
        "w:proofErr",
        ">b<",
        "w:id=\"1\"/></w:tr>",
        "w:name=\"r2\"",
        ">see<",
        ">e<",
        "w:id=\"2\"/></w:tbl>",
    ];
    let at: Vec<usize> = order.iter().map(|s| xml.find(s).unwrap_or_else(|| panic!("{s} missing: {xml}"))).collect();
    assert!(at.windows(2).all(|w| w[0] < w[1]), "{order:?} out of order: {xml}");
}

#[test]
fn a_marker_among_table_properties_keeps_the_table_whole() {
    let body = "<w:tbl><w:tblPr/><w:bookmarkStart w:id=\"1\" w:name=\"t\"/><w:tblGrid><w:gridCol w:w=\"2000\"/></w:tblGrid><w:tr><w:tc><w:p/></w:tc></w:tr></w:tbl><w:bookmarkEnd w:id=\"1\"/>";
    let pkg = docx(body, vec![]);
    let (_, _, _, stats) = DocxEngine::split(&pkg, &ImportOptions::default()).unwrap();
    assert_eq!(stats.tables_kept, 1);
    assert!(stats.kept_reasons[0].contains("tblGrid after"), "{:?}", stats.kept_reasons);
    let imp = DocxEngine.import(&pkg, &ImportOptions::default()).unwrap();
    let out = DocxEngine.export(&imp.text, &imp.remainder).unwrap();
    let canon = |pkg: &[u8]| {
        hanji_docx::xml::canon_part(package::get(&package::read(pkg).unwrap(), "word/document.xml").unwrap()).unwrap()
    };
    assert_eq!(canon(&out), canon(&pkg));
}

#[test]
fn a_bold_page_break_keeps_its_bold() {
    // Survey GetPut failures (7 of 167 files): `<pagebreak/>` cannot state
    // marks, so the run's own marks must survive export.
    let body = format!("{}<w:p><w:r><w:rPr><w:b/></w:rPr><w:br w:type=\"page\"/></w:r></w:p>{}", p("a"), p("b"));
    let pkg = docx(&body, vec![]);
    let imp = DocxEngine.import(&pkg, &ImportOptions::default()).unwrap();
    assert!(imp.text.contains("\n<pagebreak/>\n"), "{}", imp.text);
    let out = DocxEngine.export(&imp.text, &imp.remainder).unwrap();
    let canon = |pkg: &[u8]| {
        hanji_docx::xml::canon_part(package::get(&package::read(pkg).unwrap(), "word/document.xml").unwrap()).unwrap()
    };
    assert_eq!(canon(&out), canon(&pkg));
}

#[test]
fn overlapping_old_text_is_ambiguous_and_bad_spans_are_refused() {
    let pkg = docx(&p("x aaa"), vec![]);
    let imp = DocxEngine.import(&pkg, &ImportOptions::default()).unwrap();
    match edit(&imp.remainder, &imp.text, "aa", "b", CAPS) {
        Err(Refusal::Edit(m)) => assert!(m.contains("occurs 2 times"), "{m}"),
        other => panic!("{other:?}"),
    }
    let k = imp.text.find("aaa").unwrap();
    let n = imp.text.len();
    for (s, e) in [(k + 1, k), (k, n + 1)] {
        let r = hanji_core::reanchor_span(&imp.remainder, &imp.text, s, e, "b", CAPS);
        assert!(matches!(r, Err(Refusal::Edit(_))), "{s}..{e}: {r:?}");
    }
}

#[test]
fn a_zip_bomb_is_refused() {
    let big = "a".repeat(1 << 20);
    let pkg = docx(&p(&big), vec![]);
    assert!(pkg.len() < 1 << 16, "the test package compresses well");
    let e = package::read_limited(&pkg, 1 << 19).unwrap_err();
    assert!(e.contains("expands to more than"), "{e}");
    assert!(package::read(&pkg).is_ok());
}

#[test]
fn spaces_only_paragraphs_keep_their_spaces_and_runs() {
    // An original spaces-only paragraph is `<div style="Name">spaces</div>`,
    // named by the file's own style (never a made-up "Normal").
    let styles_de = STYLES.replace(
        "w:styleId=\"Normal\"><w:name w:val=\"Normal\"/>",
        "w:styleId=\"Normal\"><w:name w:val=\"Standard\"/>",
    );
    let body =
        format!("{}<w:p><w:r><w:rPr><w:b/></w:rPr><w:t xml:space=\"preserve\">   </w:t></w:r></w:p>{}", p("a"), p("b"));
    let mut pkg = package::read(&docx(&body, vec![])).unwrap();
    pkg.iter_mut().find(|x| x.name == "word/styles.xml").unwrap().data = styles_de.into_bytes();
    let pkg = package::write(&pkg).unwrap();
    let imp = DocxEngine.import(&pkg, &ImportOptions::default()).unwrap();
    assert!(imp.text.contains("\n<div style=\"Standard\">   </div>\n"), "{}", imp.text);
    let doc = |pkg: &[u8]| package::get(&package::read(pkg).unwrap(), "word/document.xml").unwrap().to_vec();
    let canon = |pkg: &[u8]| hanji_docx::xml::canon_part(&doc(pkg)).unwrap();
    let out = DocxEngine.export(&imp.text, &imp.remainder).unwrap();
    assert_eq!(canon(&out), canon(&pkg));

    // Survey E8 case: splitting a paragraph so that one piece is spaces only
    // keeps the run of those spaces (with its bold) on them.
    let body =
        "<w:p><w:r><w:rPr><w:b/></w:rPr><w:t xml:space=\"preserve\">    </w:t></w:r><w:r><w:t>TEXT</w:t></w:r></w:p>";
    let pkg = docx(body, vec![]);
    let imp = DocxEngine.import(&pkg, &ImportOptions::default()).unwrap();
    assert!(imp.text.ends_with("\n    TEXT\n"), "{}", imp.text);
    let split = imp.text.replace("    TEXT", "<div style=\"Normal\">    </div>\n\nTEXT");
    for r in [
        rewrite(&imp.remainder, &imp.text, &split, CAPS).unwrap(),
        edit(&imp.remainder, &imp.text, "    TEXT", "<div style=\"Normal\">    </div>\n\nTEXT", CAPS).unwrap(),
    ] {
        assert!(r.report.refused.is_empty() && r.report.removed.is_empty(), "{:?}", r.report);
        let run = r
            .remainder
            .entries
            .iter()
            .find(|e| e.kind == Kind::Run && e.xml.iter().any(|x| x.contains("w:b")))
            .unwrap();
        assert_eq!((run.path.clone(), run.start, run.end), (vec![0], Some(0), Some(4)));
        let out = DocxEngine.export(&r.text, &r.remainder).unwrap();
        let xml = String::from_utf8(doc(&out)).unwrap();
        assert!(xml.contains("<w:rPr><w:b/></w:rPr><w:t xml:space=\"preserve\">    </w:t>"), "{xml}");
        assert_eq!(DocxEngine.import(&out, &ImportOptions::default()).unwrap().text, r.text, "PutGet");
    }
}

#[test]
fn embedded_macro_packages_are_neutralised() {
    let inner = |names: &[&str]| {
        let parts: Vec<Part> = names.iter().map(|n| part(n, "x")).collect();
        package::write(&parts).unwrap()
    };
    let bin = |name: &str, data: Vec<u8>| Part { data, ..part(name, "") };
    let rels = |t: &str| {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/package" Target="{t}"/></Relationships>"#
        )
    };
    let chart = |n: u8| {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:r="{R}"><c:chart/><c:externalData r:id="rId1"><c:autoUpdate val="0"/></c:externalData><c:note n="{n}"/></c:chartSpace>"#
        )
    };
    let pkg = docx(
        &p("a"),
        vec![
            part("word/charts/chart1.xml", &chart(1)),
            part("word/charts/_rels/chart1.xml.rels", &rels("../embeddings/Book1.xlsm")),
            bin("word/embeddings/Book1.xlsm", inner(&["[Content_Types].xml", "xl/workbook.xml", "xl/vbaProject.bin"])),
            part("word/charts/chart2.xml", &chart(2)),
            part("word/charts/_rels/chart2.xml.rels", &rels("../embeddings/Book2.xlsx")),
            bin("word/embeddings/Book2.xlsx", inner(&["[Content_Types].xml", "xl/workbook.xml", "xl/vbaProject.bin"])),
            bin("word/embeddings/Other.docm", inner(&["[Content_Types].xml", "word/document.xml"])),
            part("word/charts/chart3.xml", &chart(3)),
            part("word/charts/_rels/chart3.xml.rels", &rels("../embeddings/Clean.xlsx")),
            bin("word/embeddings/Clean.xlsx", inner(&["[Content_Types].xml", "xl/workbook.xml"])),
        ],
    );
    let imp = DocxEngine.import(&pkg, &ImportOptions::default()).unwrap();
    let got: Vec<(&str, &str)> = imp
        .report
        .neutralised
        .iter()
        .filter(|n| n.kind == "macro-package")
        .map(|n| (n.location.as_str(), n.detail.as_str()))
        .collect();
    assert_eq!(got.len(), 3, "{:?}", imp.report.neutralised);
    let out = DocxEngine.export(&imp.text, &imp.remainder).unwrap();
    let parts = package::read(&out).unwrap();
    let names: Vec<&str> = parts.iter().map(|p| p.name.as_str()).collect();
    for gone in ["word/embeddings/Book1.xlsm", "word/embeddings/Book2.xlsx", "word/embeddings/Other.docm"] {
        assert!(!names.contains(&gone), "{gone} survived: {names:?}");
    }
    assert!(names.contains(&"word/embeddings/Clean.xlsx"), "{names:?}");
    let text = |n: &str| String::from_utf8(package::get(&parts, n).unwrap().to_vec()).unwrap();
    for n in ["word/charts/chart1.xml", "word/charts/chart2.xml"] {
        assert!(!text(n).contains("externalData"), "{n}: {}", text(n));
        assert!(text(n).contains("<c:note"), "{n}: the rest of the chart stays");
    }
    assert!(text("word/charts/chart3.xml").contains("<c:externalData r:id=\"rId1\">"));
    assert!(text("word/charts/_rels/chart3.xml.rels").contains("Clean.xlsx"));
    assert!(!text("word/charts/_rels/chart1.xml.rels").contains("Book1"));
}

const NUMBERING: &str = r#"<?xml version="1.0" encoding="UTF-8"?><w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:abstractNum w:abstractNumId="0"><w:lvl w:ilvl="0"><w:numFmt w:val="bullet"/></w:lvl><w:lvl w:ilvl="1"><w:numFmt w:val="bullet"/></w:lvl></w:abstractNum><w:abstractNum w:abstractNumId="1"><w:lvl w:ilvl="0"><w:numFmt w:val="decimal"/></w:lvl><w:lvl w:ilvl="1"><w:numFmt w:val="lowerLetter"/></w:lvl></w:abstractNum><w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num><w:num w:numId="2"><w:abstractNumId w:val="1"/></w:num></w:numbering>"#;

fn li(num: u32, ilvl: u32, text: &str) -> String {
    format!("<w:p><w:pPr><w:pStyle w:val=\"ListParagraph\"/><w:numPr><w:ilvl w:val=\"{ilvl}\"/><w:numId w:val=\"{num}\"/></w:numPr></w:pPr><w:r><w:t>{text}</w:t></w:r></w:p>")
}

fn list_docx(body: &str, numbering: bool) -> Vec<u8> {
    let styles = STYLES.replace(
        "</w:styles>",
        "<w:style w:type=\"paragraph\" w:styleId=\"ListParagraph\"><w:name w:val=\"List Paragraph\"/></w:style><w:style w:type=\"paragraph\" w:styleId=\"ListBullet\"><w:name w:val=\"List Bullet\"/><w:pPr><w:numPr><w:numId w:val=\"1\"/></w:numPr></w:pPr></w:style></w:styles>",
    );
    let mut parts =
        package::read(&docx(body, if numbering { vec![part("word/numbering.xml", NUMBERING)] } else { vec![] }))
            .unwrap();
    parts.iter_mut().find(|x| x.name == "word/styles.xml").unwrap().data = styles.into_bytes();
    package::write(&parts).unwrap()
}

fn doc_xml(pkg: &[u8]) -> String {
    String::from_utf8(package::get(&package::read(pkg).unwrap(), "word/document.xml").unwrap().to_vec()).unwrap()
}

#[test]
fn numbered_paragraphs_are_list_items() {
    let heading = "<w:p><w:pPr><w:pStyle w:val=\"Heading1\"/><w:numPr><w:ilvl w:val=\"0\"/><w:numId w:val=\"2\"/></w:numPr></w:pPr><w:r><w:t>Title</w:t></w:r></w:p>";
    let cell = format!(
        "<w:tbl><w:tblPr/><w:tblGrid><w:gridCol w:w=\"2000\"/></w:tblGrid><w:tr><w:tc>{}</w:tc></w:tr></w:tbl>",
        li(1, 0, "in cell")
    );
    let style_item = "<w:p><w:pPr><w:pStyle w:val=\"ListBullet\"/></w:pPr><w:r><w:t>by style</w:t></w:r></w:p>";
    let body = [
        heading.to_string(),
        li(1, 0, "a"),
        li(1, 1, "b"),
        li(1, 0, "c"),
        p("plain"),
        li(2, 0, "d"),
        li(2, 1, "d1"),
        li(2, 0, "e"),
        li(1, 0, "other list"),
        style_item.to_string(),
        cell,
    ]
    .concat();
    let pkg = list_docx(&body, true);
    let imp = DocxEngine.import(&pkg, &ImportOptions::default()).unwrap();
    let want = "# Title\n\n- a\n  - b\n- c\n\nplain\n\n1. d\n   1. d1\n1. e\n\n- other list\n- by style\n\n| <p style=\"List Paragraph\"/>in cell |\n|---|\n";
    assert!(imp.text.ends_with(want), "{}", imp.text);
    // GetPut.
    let canon = |pkg: &[u8]| hanji_docx::xml::canon_part(doc_xml(pkg).as_bytes()).unwrap();
    let out = DocxEngine.export(&imp.text, &imp.remainder).unwrap();
    assert_eq!(canon(&out), canon(&pkg));
}

#[test]
fn list_edits_take_numbering_from_siblings_or_the_default_list() {
    let body =
        [li(1, 0, "a"), li(1, 1, "b"), li(1, 0, "c"), p("plain"), li(2, 0, "d"), li(2, 0, "e"), p("end")].concat();
    let pkg = list_docx(&body, true);
    let imp = DocxEngine.import(&pkg, &ImportOptions::default()).unwrap();
    let new = imp
        .text
        .replace("- c\n", "- c\n- new sibling\n  - new child\n")
        .replace("  - b\n", "- b promoted\n")
        .replace("1. e\n", "\ne unlisted\n")
        .replace("\nend\n", "\nend\n\n1. fresh\n   1. fresh child\n\n- fresh bullet\n");
    let r = rewrite(&imp.remainder, &imp.text, &new, CAPS).unwrap();
    let out = DocxEngine.export(&r.text, &r.remainder).unwrap();
    let xml = doc_xml(&out);
    let num_of = |t: &str| {
        let at = xml.find(&format!(">{t}<")).unwrap_or_else(|| panic!("{t}: {xml}"));
        let p = &xml[xml[..at].rfind("<w:p>").or_else(|| xml[..at].rfind("<w:p ")).unwrap()..at];
        let v = |k: &str| {
            let pat = format!("<w:{k} w:val=\"");
            p.find(&pat).map(|i| p[i + pat.len()..].split('"').next().unwrap().to_string())
        };
        (v("numId"), v("ilvl"))
    };
    let s = |x: &str| Some(x.to_string());
    assert_eq!(num_of("a"), (s("1"), s("0")));
    assert_eq!(num_of("b promoted"), (s("1"), s("0")), "a promoted item moves its own ilvl");
    assert_eq!(num_of("new sibling"), (s("1"), s("0")));
    assert_eq!(num_of("new child"), (s("1"), s("1")));
    assert_eq!(num_of("e unlisted"), (None, None), "a former item loses its numbering");
    // A new numbered list restarts: a new w:num over the default decimal list.
    assert_eq!(num_of("fresh"), (s("3"), s("0")));
    assert_eq!(num_of("fresh child"), (s("3"), s("1")));
    assert_eq!(num_of("fresh bullet"), (s("1"), s("0")));
    let numbering =
        String::from_utf8(package::get(&package::read(&out).unwrap(), "word/numbering.xml").unwrap().to_vec()).unwrap();
    assert!(numbering.contains("<w:num w:numId=\"3\"><w:abstractNumId w:val=\"1\"/><w:lvlOverride w:ilvl=\"0\"><w:startOverride w:val=\"1\"/></w:lvlOverride></w:num></w:numbering>"), "{numbering}");
    assert!(
        xml.contains("<w:pStyle w:val=\"ListParagraph\"/><w:numPr><w:ilvl w:val=\"0\"/><w:numId w:val=\"3\"/>"),
        "{xml}"
    );
    // PutGet.
    assert_eq!(DocxEngine.import(&out, &ImportOptions::default()).unwrap().text, r.text);
    if let Ok(dir) = std::env::var("HANJI_SAVE") {
        std::fs::write(format!("{dir}/lists.docx"), &out).unwrap();
    }
    // An exact edit adds an item next to its sibling.
    let r = edit(&imp.remainder, &imp.text, "- c\n", "- c\n- c2\n", CAPS).unwrap();
    assert!(r.report.refused.is_empty() && r.report.removed.is_empty(), "{:?}", r.report);
    let xml = doc_xml(&DocxEngine.export(&r.text, &r.remainder).unwrap());
    assert!(xml.contains("<w:numId w:val=\"1\"/></w:numPr></w:pPr><w:r><w:t>c2</w:t>"), "{xml}");
}

#[test]
fn a_new_list_without_numbering_in_the_file_is_refused() {
    let pkg = list_docx(&p("a"), false);
    let imp = DocxEngine.import(&pkg, &ImportOptions::default()).unwrap();
    let new = imp.text.replace("\na\n", "\na\n\n- b\n");
    match DocxEngine.export(&new, &imp.remainder) {
        Err(EngineError::Refused(m)) => assert!(m.contains("no bulleted list to take numbering from"), "{m}"),
        other => panic!("{other:?}"),
    }
}
