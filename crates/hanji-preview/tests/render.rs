//! The preview end to end on corpus decks (DESIGN.md §7, §7.1): the
//! exported package of a stored revision is rendered; slides, SVG validity,
//! subset fonts and the fonts report are checked.
//!
//! Tests do not read system fonts. The Korean ones need a Hangul font file:
//! `HANJI_TEST_FONT_DIR`, else one under `/usr/share/fonts` (CI installs
//! fonts-noto-cjk). Without one they are skipped, except on CI.

#![cfg(not(target_family = "wasm"))]

use std::collections::BTreeSet;
use std::path::PathBuf;

use base64::Engine as _;
use hanji_preview::store::{self, Output};
use hanji_preview::{fonts::Script, Options, Preview};
use hanji_store::{Code, MemStorage, TextEdit, Workspace};

fn corpus(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../hanji-pptx/corpus").join(name)
}

/// `name` opened into a fresh in-memory store: (workspace, doc id).
fn opened(name: &str) -> (Workspace<MemStorage>, String) {
    let mut ws = Workspace::new(MemStorage::new());
    let bytes = std::fs::read(corpus(name)).unwrap();
    let id = ws.open_bytes(name, &bytes, None).unwrap().doc_id;
    (ws, id)
}

fn no_fonts() -> Options {
    Options { font_dirs: vec![], system_fonts: false }
}

/// A font file that draws Hangul, for the Korean tests.
fn korean_font() -> Option<PathBuf> {
    let mut db = fontdb::Database::new();
    match std::env::var_os("HANJI_TEST_FONT_DIR") {
        Some(d) => db.load_fonts_dir(d),
        None => db.load_fonts_dir("/usr/share/fonts"),
    }
    let mut found: Vec<(bool, PathBuf)> = db
        .faces()
        .filter_map(|f| {
            let fontdb::Source::File(path) = &f.source else { return None };
            let has = db.with_face_data(f.id, |d, i| {
                ttf_parser::Face::parse(d, i)
                    .is_ok_and(|x| x.glyph_index('가').is_some() && x.glyph_index('A').is_some())
            });
            let noto = f.families.iter().any(|(n, _)| n == "Noto Sans CJK KR");
            (has == Some(true)).then(|| (!noto, path.clone()))
        })
        .collect();
    found.sort();
    let font = found.into_iter().next().map(|(_, p)| p);
    if font.is_none() {
        assert!(std::env::var_os("CI").is_none(), "CI has no Korean font: install fonts-noto-cjk");
        eprintln!("no Korean font found: set HANJI_TEST_FONT_DIR to test Hangul rendering");
    }
    font
}

/// The `@font-face` data of an SVG or HTML page, decoded.
fn embedded_fonts(page: &str) -> Vec<Vec<u8>> {
    page.split("url('data:font/")
        .skip(1)
        .filter_map(|rest| rest.split_once(";base64,").and_then(|(_, r)| r.split('\'').next()))
        .map(|b64| base64::engine::general_purpose::STANDARD.decode(b64).unwrap())
        .collect()
}

/// Every page parses as SVG, uses only the fonts it embeds, and embeds
/// subsets (each well under the bundled faces' sizes).
fn check_pages(p: &Preview, slides: usize, max_page: usize) {
    assert_eq!(p.slide_count(), slides);
    for k in 0..slides {
        let svg = p.slide_svg(k);
        assert!(svg.len() < max_page, "slide {} is {} bytes", k + 1, svg.len());
        resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default())
            .unwrap_or_else(|e| panic!("slide {} is not valid SVG: {e}", k + 1));
        let used: BTreeSet<&str> = svg.split("font-family=\"").skip(1).filter_map(|r| r.split('"').next()).collect();
        let declared: BTreeSet<&str> =
            svg.split("@font-face{font-family:'").skip(1).filter_map(|r| r.split('\'').next()).collect();
        assert_eq!(used, declared, "slide {}: every font drawn is embedded", k + 1);
        for font in embedded_fonts(&svg) {
            let face = ttf_parser::Face::parse(&font, 0).expect("an embedded font parses");
            assert!(
                face.number_of_glyphs() < 600,
                "slide {}: a face of {} glyphs is not a subset",
                k + 1,
                face.number_of_glyphs()
            );
            assert!(face.tables().cmap.is_some(), "the subset keeps its cmap");
        }
        let png = p.slide_png(k, 48.0).unwrap();
        assert!(png.starts_with(b"\x89PNG"), "slide {} rasterises", k + 1);
    }
}

#[test]
fn the_korean_deck_without_a_korean_font_warns_and_reports_what_is_missing() {
    let (ws, id) = opened("korean-deck.pptx");
    let (out, p) = store::render(&ws, &id, None, &no_fonts()).unwrap();
    check_pages(&p, 7, 1 << 20);
    assert_eq!(out.slides, 7);
    assert!(p.warnings.iter().any(|w| w == hanji_preview::NO_KOREAN_FONT), "{:?}", p.warnings);
    let hangul = p.fonts.substituted.iter().find(|s| s.requested == "맑은 고딕" && s.script == Script::Hangul).unwrap();
    assert_eq!(hangul.drawn, None);
    assert!(hangul.chars > 100 && hangul.pages == (1..=7).collect::<Vec<_>>(), "{hangul:?}");
    assert!(p.fonts.missing_glyphs.iter().any(|m| m.char == "U+D575"), "핵 is missing: {:?}", p.fonts.missing_glyphs);
    assert_eq!(p.fonts.missing_glyphs_total, p.fonts.missing_glyphs.len());
    // Latin text is drawn by the faces compiled in.
    let calibri = p.fonts.substituted.iter().find(|s| s.requested == "Calibri").unwrap();
    assert_eq!((calibri.drawn.as_deref(), calibri.metrics.name()), (Some("Carlito"), "compatible:Carlito".to_string()));
    assert_eq!(calibri.source, Some(hanji_preview::fonts::Source::Bundled));
    assert!(out.summary.starts_with("fonts: ") && out.summary.contains("Calibri → Carlito"), "{}", out.summary);
    let json = serde_json::to_value(&out).unwrap();
    assert_eq!(json["fonts"]["substituted"][0]["requested"], "맑은 고딕");
    assert!(json["fonts"]["missing_glyphs"][0]["char"].as_str().unwrap().starts_with("U+"));
}

#[test]
fn the_korean_deck_with_a_korean_font_draws_every_character() {
    let Some(font) = korean_font() else { return };
    let (ws, id) = opened("korean-deck.pptx");
    let opts = Options { font_dirs: vec![font], system_fonts: false };
    let (_, p) = store::render(&ws, &id, None, &opts).unwrap();
    // A Korean slide with its subset fonts stays small (a whole CJK face is 16 MB or more).
    check_pages(&p, 7, 300 << 10);
    assert!(p.warnings.is_empty(), "{:?}", p.warnings);
    assert!(p.fonts.missing_glyphs.is_empty(), "{:?}", p.fonts.missing_glyphs);
    let hangul = p.fonts.substituted.iter().find(|s| s.requested == "맑은 고딕" && s.script == Script::Hangul).unwrap();
    assert_eq!(hangul.source, Some(hanji_preview::fonts::Source::FontDir));
    assert!(hangul.drawn.is_some());
    // The PNG is drawn with the same subset faces.
    let png = p.slide_png(1, 96.0).unwrap();
    let img = resvg::tiny_skia::Pixmap::decode_png(&png).unwrap();
    assert_eq!((img.width(), img.height()), (960, 720));
    assert!(img.pixels().iter().any(|px| px.red() < 64), "text is drawn");
}

#[test]
fn the_shapes_deck_renders_every_slide() {
    let (ws, id) = opened("shapes.pptx");
    let (_, p) = store::render(&ws, &id, None, &no_fonts()).unwrap();
    check_pages(&p, 6, 1 << 20);
    assert!(p.fonts.missing_glyphs.is_empty(), "{:?}", p.fonts.missing_glyphs);
    let total: usize = p.fonts.substituted.iter().map(|s| s.chars).sum::<usize>()
        + p.fonts.drawn_as_requested.iter().map(|s| s.chars).sum::<usize>();
    assert!(total > 100, "{:?}", p.fonts);
}

#[test]
fn the_viewer_embeds_each_font_once_and_marks_substituted_text() {
    let (ws, id) = opened("korean-deck.pptx");
    let (mut out, p) = store::render(&ws, &id, None, &no_fonts()).unwrap();
    let html = p.html("korean-deck");
    assert_eq!(html.matches("<section class=\"slide\"").count(), 7);
    assert_eq!(html.matches("<svg ").count(), 7);
    let faces: Vec<&str> =
        html.split("@font-face{font-family:'").skip(1).filter_map(|r| r.split('\'').next()).collect();
    let distinct: BTreeSet<&&str> = faces.iter().collect();
    assert_eq!(faces.len(), distinct.len(), "one @font-face per face for the whole deck");
    assert!(html.contains("<g class=\"hanji-subst\"><title>Calibri → Carlito (metrics: compatible:Carlito)</title>"));
    assert!(html.contains("class=\"hanji-mark\""));
    assert!(html.contains("<li>Calibri → Carlito, "), "the banner lists substitutions");
    assert!(html.contains("data-font-requested=\"맑은 고딕\""));
    // Definition ids are per slide, so slides inlined together do not share them.
    assert!(!html.contains("rdocx-"));
    let dir = std::env::temp_dir().join(format!("hanji-preview-test-{}", std::process::id()));
    store::write(&p, &mut out, Output::Svg, &dir).unwrap();
    assert_eq!(out.files.len(), 7);
    assert!(out.files[0].ends_with("korean-deck-r1-slide-1.svg"));
    store::write(&p, &mut out, Output::Html, &dir).unwrap();
    assert!(out.files[0].ends_with("korean-deck-r1-preview.html"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_revision_is_previewed_from_its_export_not_the_original() {
    let (mut ws, id) = opened("korean-deck.pptx");
    ws.edit(&id, 1, &[TextEdit { old: "3분기 영업 보고".into(), new: "4분기 영업 보고".into() }]).unwrap();
    let text = |rev| {
        let (_, p) = store::render(&ws, &id, Some(rev), &no_fonts()).unwrap();
        p.slide_svg(0)
    };
    // The title is split at the script change: "4" and "분기 영업 보고".
    let (old, new) = (text(1), text(2));
    assert!(new.contains(">4</text>") && !new.contains(">3</text>"), "{new}");
    assert!(old.contains(">3</text>"));
}

#[test]
fn other_formats_are_not_supported_yet() {
    let mut ws = Workspace::new(MemStorage::new());
    let id = ws.create(hanji_store::DocType::Document, None, None).unwrap().doc_id;
    let e = store::render(&ws, &id, None, &no_fonts()).err().unwrap();
    assert_eq!(e.code, Code::Unsupported);
    assert!(e.message.starts_with("preview not supported yet for docx"), "{}", e.message);
}
