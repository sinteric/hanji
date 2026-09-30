//! Text formatting (§5.3): each run's font, size and colour shown as the
//! run shows them, and written back only where the text changes them.

use hanji_core::edit::edit_in;
use hanji_core::{Capabilities, Engine, EngineError, ImportOptions, Imported, Part, Remainder};
use hanji_package::{opc, package};
use hanji_pptx::{PptxEngine, PptxModel};

const CAPS: Capabilities =
    Capabilities { links: false, fields: false, footnotes: false, math: false, formatting: false };

fn deck(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/corpus/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn import(pkg: &[u8]) -> Imported {
    PptxEngine.import(pkg, &ImportOptions::default()).unwrap_or_else(|e| panic!("{e}"))
}

/// An exact edit, exported: the export reads back as the edited text.
fn edited(imp: &Imported, old: &str, new: &str) -> (Vec<Part>, String) {
    let r = edit_in(&PptxModel, &imp.remainder, &imp.text, old, new, CAPS).unwrap_or_else(|e| panic!("{old:?}: {e}"));
    assert!(r.report.removed.is_empty() && r.report.refused.is_empty(), "{:?}", r.report);
    let out = PptxEngine.export(&r.text, &r.remainder).unwrap_or_else(|e| panic!("{e}"));
    let back = import(&out).text;
    assert_eq!(back, canonical(&r.text, &r.remainder), "PutGet");
    (package::read(&out).unwrap(), back)
}

/// What a write returns for `text` (§5.1).
fn canonical(text: &str, rem: &Remainder) -> String {
    use hanji_core::TextModel;
    let (blocks, _) = PptxModel.resolve(text, rem, CAPS).unwrap_or_else(|e| panic!("{e:?}"));
    PptxEngine::text_of(&blocks, rem, None)
}

fn export_err(text: &str, rem: &Remainder) -> String {
    match PptxEngine.export(text, rem) {
        Err(EngineError::Invalid(d)) => hanji_format::diag::render(&d),
        Err(EngineError::Refused(m)) => m,
        other => panic!(
            "expected an error, got {:?} for {}",
            other.map(|_| ()),
            text.lines().filter(|l| l.contains("s5") || l.contains("::notes")).collect::<Vec<_>>().join("\n")
        ),
    }
}

fn xml(parts: &[Part], name: &str) -> String {
    String::from_utf8(package::get(parts, name).unwrap().to_vec()).unwrap()
}

fn slide(parts: &[Part], n: usize) -> String {
    let pres = xml(parts, "ppt/presentation.xml");
    let list = &pres[pres.find("<p:sldIdLst>").unwrap()..pres.find("</p:sldIdLst>").unwrap()];
    let rid = list.split("r:id=\"").nth(n).unwrap();
    xml(parts, &opc::target_of(parts, "ppt/presentation.xml", &rid[..rid.find('"').unwrap()]).unwrap())
}

/// The text shape named `name` in a slide's XML.
fn shape<'a>(x: &'a str, name: &str) -> &'a str {
    let at = x.find(&format!("name=\"{name}\"")).unwrap_or_else(|| panic!("no {name} in {x}"));
    &x[at..at + x[at..].find("</p:sp>").unwrap()]
}

const SOURCE: &str =
    "<shape id=\"s5\" name=\"출처\" box=\"36 475 288 29\" font=Calibri size=18pt color=tx1>출처: 내부 집계</shape>";

#[test]
fn every_run_shows_what_it_inherits() {
    let imp = import(&deck("korean-deck.pptx"));
    // A slot's formatting on its marker, what a paragraph adds at its end.
    assert!(imp.text.contains(concat!(
        "::title box=\"36 22 648 90\" font=Calibri size=44pt color=tx1::\n핵심 지표\n",
        "::body box=\"36 126 648 356\" font=Calibri color=tx1::\n- 매출 **12% 증가** {size=32pt}\n"
    )));
    // A text box's from the deck's defaults and its theme.
    assert!(imp.text.contains(SOURCE), "{}", imp.text);
    // Notes show none.
    assert!(imp.text.contains("::notes::\n전년 대비 증가폭을 강조한다.\n"));
    // A colour the file adjusts is shown with `*`; one lighter by a whole percent as +N%.
    let s = import(&deck("shapes.pptx")).text;
    assert!(s.contains("color=accent2+40%::") && s.contains("color=tx1*::"), "{s}");
}

#[test]
fn a_word_restyled_writes_only_its_run() {
    let imp = import(&deck("korean-deck.pptx"));
    let before = package::read(&deck("korean-deck.pptx")).unwrap();
    let (parts, _) = edited(
        &imp,
        SOURCE,
        "<shape id=\"s5\" name=\"출처\" box=\"36 475 288 29\" font=Calibri size=18pt color=tx1>[출처]{size=24pt color=#FF7F50}: 내부 집계</shape>",
    );
    let sh = shape(&slide(&parts, 3), "출처").to_string();
    assert!(
        sh.contains("<a:r><a:rPr lang=\"ko-KR\" altLang=\"en-US\" sz=\"2400\"><a:solidFill><a:srgbClr val=\"FF7F50\"/></a:solidFill></a:rPr><a:t>출처</a:t></a:r><a:r><a:rPr lang=\"ko-KR\" altLang=\"en-US\"/><a:t>: 내부 집계</a:t></a:r>"),
        "{sh}"
    );
    // Every other slide is as it was.
    for n in [1, 2, 4, 5, 6, 7] {
        assert_eq!(slide(&parts, n), slide(&before, n), "slide {n}");
    }
}

#[test]
fn a_shape_font_and_a_slot_size_are_written_and_taken_back() {
    let imp = import(&deck("korean-deck.pptx"));
    let before = package::read(&deck("korean-deck.pptx")).unwrap();
    let (parts, _) =
        edited(&imp, "font=Calibri size=18pt color=tx1>출처", "font=\"Noto Sans KR\" size=18pt color=tx1>출처");
    let sh = shape(&slide(&parts, 3), "출처").to_string();
    assert!(
        sh.contains("<a:rPr lang=\"ko-KR\" altLang=\"en-US\"><a:latin typeface=\"Noto Sans KR\"/></a:rPr>"),
        "{sh}"
    );
    // A title at 40 pt, then back at the size it inherits: the run is as it was.
    let old = "::title box=\"36 22 648 90\" font=Calibri size=44pt color=tx1::\n핵심 지표";
    let (parts, text) = edited(&imp, old, &old.replace("44pt", "40pt"));
    assert!(slide(&parts, 2).contains("sz=\"4000\""), "{}", slide(&parts, 2));
    let imp2 = import(&package::write(&parts).unwrap());
    assert_eq!(imp2.text, text);
    let (parts, _) = edited(&imp2, &old.replace("44pt", "40pt"), old);
    assert_eq!(slide(&parts, 2), slide(&before, 2));
    // Left unsaid, a value is the one the run inherits: the direct size goes.
    let (parts, _) = {
        let r = edit_in(
            &PptxModel,
            &imp2.remainder,
            &imp2.text,
            " size=40pt color=tx1::\n핵심",
            " color=tx1::\n핵심",
            CAPS,
        )
        .unwrap();
        let out = PptxEngine.export(&r.text, &r.remainder).unwrap();
        assert!(import(&out).text.contains(old), "the inherited size is shown again");
        (package::read(&out).unwrap(), ())
    };
    assert_eq!(slide(&parts, 2), slide(&before, 2));
}

#[test]
fn a_paragraph_size_and_theme_colours_are_written() {
    let imp = import(&deck("korean-deck.pptx"));
    let (parts, back) = edited(&imp, "- 신규 고객 34곳 {size=32pt}", "- 신규 고객 34곳 {size=36pt color=accent1+40%}");
    assert!(back.contains("- 신규 고객 34곳 {size=36pt color=accent1+40%}\n"), "{back}");
    let x = slide(&parts, 2);
    assert!(
        x.contains("sz=\"3600\"><a:solidFill><a:schemeClr val=\"accent1\"><a:lumMod val=\"60000\"/><a:lumOff val=\"40000\"/></a:schemeClr></a:solidFill>"),
        "{x}"
    );
    let (parts, back) =
        edited(&imp, "- 영업이익률 8.4% {size=32pt}", "- 영업이익률 8.4% {size=32pt color=#1F4E79/50%}");
    assert!(
        back.contains("- 영업이익률 8.4% {color=#1F4E79/50%}\n") || back.contains("8.4% {size=32pt color=#1F4E79/50%}"),
        "{back}"
    );
    assert!(slide(&parts, 2).contains("<a:srgbClr val=\"1F4E79\"><a:alpha val=\"50000\"/></a:srgbClr>"));
}

#[test]
fn what_cannot_be_written_is_refused_with_the_form() {
    let imp = import(&deck("korean-deck.pptx"));
    let rem = &imp.remainder;
    let with = |new: &str| imp.text.replace(SOURCE, new);
    for (bad, want) in [
        ("<shape id=\"s5\" name=\"출처\" box=\"36 475 288 29\" color=red>출처: 내부 집계</shape>", "#RRGGBB"),
        ("<shape id=\"s5\" name=\"출처\" box=\"36 475 288 29\" size=0pt>출처: 내부 집계</shape>", "size"),
        ("<shape id=\"s5\" name=\"출처\" box=\"36 475 288 29\" colour=tx1>출처: 내부 집계</shape>", "color"),
        ("<shape id=\"s5\" name=\"출처\" box=\"36 475 288 29\">[출처]{size=big}: 내부 집계</shape>", "size=big"),
    ] {
        let m = export_err(&with(bad), rem);
        assert!(m.contains(want), "{bad}: {m}");
    }
    // A colour kept as the file stores it cannot be written where it is not.
    let m = export_err(&with(&SOURCE.replace("color=tx1", "color=accent1*")), rem);
    assert!(m.contains("color=accent1*") && m.contains("cannot be written"), "{m}");
    // Notes have no formatting.
    let m = export_err(&imp.text.replace("::notes::\n전년", "::notes size=12pt::\n전년"), rem);
    assert!(m.contains("notes"), "{m}");
}

/// korean-deck.pptx with the 출처 text box's paragraph replaced by `runs`.
fn with_runs(runs: &str) -> Vec<u8> {
    let mut parts = package::read(&deck("korean-deck.pptx")).unwrap();
    let p = parts.iter_mut().find(|p| p.name == "ppt/slides/slide3.xml").unwrap();
    let x = String::from_utf8(p.data.clone()).unwrap();
    let old = "<a:r><a:rPr lang=\"ko-KR\" altLang=\"en-US\"/><a:t>출처: 내부 집계</a:t></a:r>";
    assert!(x.contains(old));
    p.data = x.replace(old, runs).into_bytes();
    package::write(&parts).unwrap()
}

#[test]
fn marks_on_spaces_between_differently_formatted_runs_are_kept() {
    // Spaces in a run of their own, italic like the text around them but
    // without its colour; and a bold line break between runs of two sizes.
    let accent = "<a:solidFill><a:schemeClr val=\"accent1\"/></a:solidFill>";
    let runs = [
        format!("<a:r><a:rPr lang=\"en-US\" i=\"1\">{accent}</a:rPr><a:t>–</a:t></a:r><a:r><a:rPr lang=\"en-US\" i=\"1\"/><a:t>  </a:t></a:r><a:r><a:rPr lang=\"en-US\" i=\"1\">{accent}</a:rPr><a:t>ORC 3301</a:t></a:r>"),
        "<a:r><a:rPr lang=\"en-US\" sz=\"1400\" b=\"1\"/><a:t>End User </a:t></a:r><a:br><a:rPr lang=\"en-US\" b=\"1\"/></a:br><a:r><a:rPr lang=\"en-US\" sz=\"700\" b=\"1\"/><a:t>Community</a:t></a:r>".to_string(),
    ];
    for r in runs {
        let pkg = with_runs(&r);
        let imp = import(&pkg);
        let out = PptxEngine.export(&imp.text, &imp.remainder).unwrap_or_else(|e| panic!("{e}\n{}", imp.text));
        let (a, b) = (package::read(&pkg).unwrap(), package::read(&out).unwrap());
        assert_eq!(slide(&b, 3), slide(&a, 3), "{}", imp.text);
        assert_eq!(import(&out).text, imp.text);
    }
}
