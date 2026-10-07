//! Bounded library worksheet jobs: actual package bytes, fonts and SVG/PNG output.
#[path = "fixtures/xlsx_grid.rs"]
mod fixture;
#[path = "fixtures/xlsx_inherited.rs"]
mod inherited;
#[path = "fixtures/xlsx_numeric.rs"]
mod numeric;
use hanji_preview::{
    xlsx::{open_xlsx, FormulaResult, XlsxOptions},
    FontOptions, PageData, PageFormat,
};

fn book(calc: &str) -> hanji_preview::xlsx::Workbook {
    open_xlsx(&fixture::build(fixture::GRID, calc, fixture::STYLES, &[]), XlsxOptions::default()).unwrap()
}
fn error<T>(r: Result<T, String>) -> String {
    match r {
        Ok(_) => panic!("expected refusal"),
        Err(e) => e,
    }
}

#[test]
fn multilingual_styles_merge_dimensions_and_cached_results_are_visible_and_explicit() {
    let mut b = book(r#"<calcPr fullCalcOnLoad="1"/>"#);
    assert_eq!(b.sheets.len(), 2);
    assert_eq!(b.sheets[1].state, "hidden");
    let w = b.render_window_with_fonts(0, "A1:C5", &FontOptions::default()).unwrap();
    assert_eq!(w.cells.iter().find(|c| c.address == "A1").unwrap().display, "매출 & 売上");
    assert_eq!(w.cells.iter().find(|c| c.address == "B2").unwrap().display, "1,234.50");
    assert!(!w.cells.iter().any(|c| c.address == "B1" || c.address == "C2" || c.address == "A5"));
    assert!(w.fonts().missing_glyphs_total > 0); // Bundled fonts do not claim Korean/CJK coverage.
    let p = w.page_info();
    assert!((p.width - 201.0).abs() < 0.01);
    assert_eq!(p.height, 93.0);
    let PageData::Svg(svg) = w.render(PageFormat::Svg).unwrap().data else { panic!() };
    for expected in
        ["매출 &amp; 売上", "1,234.50", "#CCEEDD", "#123456", "stroke-width=\"1\"", "@font-face", "clipPath"]
    {
        assert!(svg.contains(expected), "{expected}");
    }
    assert!(!svg.contains("HIDDEN"));
    assert!(w.render(PageFormat::Svg).unwrap().diagnostics.iter().any(|d| d.path == "sheets[0].cells[B3].formula"));
    let cell = |a: &str| w.cells.iter().find(|c| c.address == a).unwrap();
    assert_eq!(cell("A3").display, "1235.5");
    assert_eq!(cell("A3").formula_result, FormulaResult::CachedPossiblyStale);
    assert_eq!(cell("B3").display, "#UNEVALUATED");
    assert_eq!(cell("B3").formula_result, FormulaResult::Missing);
    assert_eq!(cell("A4").display, ""); // Empty cached string is a valid result.
    assert_eq!(cell("A4").formula_result, FormulaResult::CachedPossiblyStale);
    assert_eq!(cell("B4").formula.as_deref(), Some("")); // Shared follower is not fabricated.
    for feature in ["conditionalFormatting", "drawing", "pageSetup"] {
        assert!(w.diagnostics().iter().any(|d| d.path.ends_with(feature)));
    }
    assert!(w.diagnostics().iter().any(|d| d.message.contains("no evaluation was performed")));
    let PageData::Png(png) = w.render(PageFormat::Png { dpi: 72.0 }).unwrap().data else { panic!() };
    let image = resvg::tiny_skia::Pixmap::decode_png(&png).unwrap();
    assert_eq!((image.width(), image.height()), (201, 93));
    let html = w.html("<script>title</script>");
    assert!(!html.contains("<script>"));
    assert!(html.contains("&lt;script&gt;title&lt;/script&gt;"));
    assert!(html.contains("worksheet window, cached formula results only"));
}

#[test]
fn arbitrary_sparse_addresses_do_not_expand_the_reported_dimension() {
    let mut b = book("");
    let w = b.render_window_with_fonts(0, "XFD1048576", &FontOptions::default()).unwrap();
    assert_eq!(w.cells.len(), 1);
    assert_eq!(w.cells[0].display, "LAST");
    assert!(w.page_info().width < 100.0);
    assert!(error(b.render_window_with_fonts(0, "A1:XFD1048576", &FontOptions::default())).contains("budget"));
    assert!(error(b.render_window_with_fonts(0, "A1", &FontOptions::default())).contains("cuts merged range A1:B1"));
    assert!(error(b.render_window_with_fonts(9, "A1", &FontOptions::default())).contains("index"));
    assert!(error(b.render_window_with_fonts(0, "XFE1", &FontOptions::default())).contains("invalid"));
    assert!(b.render_window_with_fonts(1, "A1", &FontOptions::default()).unwrap().cells.is_empty());
}

#[test]
fn clipped_numeric_values_report_the_cell_and_keep_the_complete_display() {
    let sheet = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><cols><col min="1" max="1" width="2"/><col min="2" max="2" width="20"/></cols><sheetData><row r="1"><c r="A1"><v>123456789</v></c><c r="B1"><v>7</v></c></row></sheetData></worksheet>"#;
    let mut b = open_xlsx(&fixture::build(sheet, "", fixture::STYLES, &[]), XlsxOptions::default()).unwrap();
    let w = b.render_window_with_fonts(0, "A1:B1", &FontOptions::default()).unwrap();
    assert_eq!(w.cells[0].display, "123456789");
    assert!(w
        .diagnostics()
        .iter()
        .any(|d| d.path == "sheets[0].cells[A1].clipping" && d.message.contains("overflow indicator")));
    assert!(!w.diagnostics().iter().any(|d| d.path == "sheets[0].cells[B1].clipping"));
    assert!(w.render(PageFormat::Svg).unwrap().diagnostics.iter().any(|d| d.path == "sheets[0].cells[A1].clipping"));
    assert!(w.quality().enforce(hanji_preview::quality::Strictness::CriticalLosses).is_ok());
}

#[test]
fn numeric_overflow_never_draws_a_truncated_value_and_keeps_accessible_full_values() {
    let bytes = fixture::build(numeric::GRID, "", numeric::STYLES, &[]);
    let original = bytes.clone();
    let mut book = open_xlsx(&bytes, XlsxOptions::default()).unwrap();
    let window = book.render_window_with_fonts(0, "A1:D11", &FontOptions::default()).unwrap();
    let PageData::Svg(svg) = window.render(PageFormat::Svg).unwrap().data else { panic!() };
    let tree = hanji_package::xml::parse(svg.as_bytes()).unwrap();
    let mut drawn = vec![];
    tree.root.walk(&mut |element| {
        if element.local() == "text" {
            drawn.push(
                element
                    .children
                    .iter()
                    .filter_map(|node| match node {
                        hanji_package::xml::Node::Text(text) => Some(hanji_package::xml::unescape(text)),
                        _ => None,
                    })
                    .collect::<String>(),
            );
        }
    });
    assert_eq!(
        drawn.iter().filter(|s| s.as_str() == "-123456789").count(),
        1,
        "only the wide control may draw its complete number: {drawn:?}"
    );
    for address in ["A1", "A2", "A3", "A4", "A5", "A6", "A7", "A8", "A9", "D11"] {
        assert!(
            window.diagnostics().iter().any(|d| d.path == format!("sheets[0].cells[{address}].clipping")
                && d.message.contains("overflow indicator")),
            "{address}"
        );
    }
    assert!(drawn.iter().filter(|s| !s.is_empty() && s.chars().all(|c| c == '#')).count() >= 10, "{drawn:?}");
    let cell = |a: &str| window.cells.iter().find(|c| c.address == a).unwrap();
    assert_eq!(cell("A1").display, "-123456789");
    assert_eq!(cell("A3").display, "-12345.00%");
    assert_eq!(cell("A4").display, "-1.23E+08");
    assert_eq!(cell("A5").display, "2023-03-15");
    assert_eq!(cell("A6").display, "2026-10-03T13:00:00Z");
    assert_eq!(cell("A10").display, "#UNEVALUATED");
    assert_eq!(cell("A10").formula_result, FormulaResult::Missing);
    assert_eq!(cell("B10").display, "0");
    assert_ne!(cell("B10").formula_result, FormulaResult::Missing);
    assert!(drawn.iter().any(|s| s == "?"));
    assert!(drawn.iter().any(|s| s == "0"));
    assert!(drawn.iter().any(|s| s == "#UNEVALUATED"));
    assert!(svg.contains("aria-describedby="));
    let html = window.html("numeric overflow");
    assert!(html.contains("Complete cell values"));
    for cell in &window.cells {
        assert!(html.contains(&format!("<td>{}</td>", hanji_package::xml::escape_text(&cell.display))));
        assert!(svg.contains(&hanji_package::xml::escape_text(&format!("{}: {}", cell.address, cell.display))));
    }
    let PageData::Png(png) = window.render(PageFormat::Png { dpi: 96.0 }).unwrap().data else { panic!() };
    assert!(resvg::tiny_skia::Pixmap::decode_png(&png).is_ok());
    assert_eq!(bytes, original);
}

#[test]
fn an_overflow_font_without_marker_glyphs_uses_an_unambiguous_vector_fallback() {
    let chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 ".chars().collect();
    let data = hanji_preview::subset::subset(oxml_layout::bundled_fonts::bundled_font_data()[0].1, 0, &chars).unwrap();
    assert!(ttf_parser::Face::parse(&data, 0).unwrap().glyph_index('#').is_none());
    let fonts = FontOptions { fonts: vec![hanji_preview::FontData::new(data)], ..Default::default() };
    let sheet = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><cols><col min="1" max="1" width="2"/></cols><sheetData><row r="1"><c r="A1"><v>123456789</v></c></row></sheetData></worksheet>"#;
    let mut b = open_xlsx(&fixture::build(sheet, "", fixture::STYLES, &[]), XlsxOptions::default()).unwrap();
    let window = b.render_window_with_fonts(0, "A1", &fonts).unwrap();
    let PageData::Svg(svg) = window.render(PageFormat::Svg).unwrap().data else { panic!() };
    assert!(!svg.contains(">123456789</text>"));
    assert!(svg.contains("A1: 123456789"));
    let tree = hanji_package::xml::parse(svg.as_bytes()).unwrap();
    let mut diagonal_lines = 0;
    tree.root.walk(&mut |e| {
        if e.local() == "line" && e.get("x1") != e.get("x2") && e.get("y1") != e.get("y2") {
            diagonal_lines += 1;
        }
    });
    assert_eq!(diagonal_lines, 2);
    assert_eq!(window.fonts().missing_glyphs_total, 0);
}

#[test]
fn accessible_metadata_keeps_svg_valid_when_cell_text_contains_xml_invalid_controls() {
    let sheet = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>A_x0001_B</t></is></c></row></sheetData></worksheet>"#;
    let mut b = open_xlsx(&fixture::build(sheet, "", fixture::STYLES, &[]), XlsxOptions::default()).unwrap();
    let window = b.render_window_with_fonts(0, "A1", &FontOptions::default()).unwrap();
    assert_eq!(window.cells[0].display, "A\u{1}B");
    let PageData::Svg(svg) = window.render(PageFormat::Svg).unwrap().data else { panic!() };
    assert!(!svg.contains('\u{1}'));
    assert!(svg.contains("A1: A\u{FFFD}B"));
    hanji_package::xml::parse(svg.as_bytes()).unwrap();
    assert!(!window.html("controls").contains('\u{1}'));
    assert!(window.quality().diagnostics.iter().any(|d| d.code == hanji_preview::quality::DiagnosticCode::InvalidText));
}

#[test]
fn inherited_number_formats_use_the_effective_cell_row_or_column_style() {
    let bytes = fixture::build(inherited::INHERITED_GRID, "", inherited::INHERITED_STYLES, &[]);
    let mut b = open_xlsx(&bytes, XlsxOptions::default()).unwrap();
    let w = b.render_window_with_fonts(0, "A1:E6", &FontOptions::default()).unwrap();
    let cells = serde_json::to_value(&w.cells).unwrap();
    for (address, display) in [
        ("A1", "50%"),
        ("B1", "1900-01-02"),
        ("C1", "$1,234.50"),
        ("A2", "50%"),
        ("B2", "1900-01-02"),
        ("C2", "$1,234.50"),
        ("A3", "50%"),
        ("B3", "50%"),
        ("C3", "50%"),
        ("D3", "0.5"),
        ("E3", "0.500"),
        ("A4", "1900-01-02"),
        ("B4", "1900-01-02"),
        ("C4", "1900-01-02"),
        ("D4", "50%"),
        ("A5", "$1,234.50"),
        ("B5", "$1,234.50"),
        ("C5", "$1,234.50"),
        ("D5", "1234.5"),
        ("A6", "50%"),
        ("B6", "1900-01-02"),
        ("C6", "$1,234.50"),
    ] {
        let cell = cells.as_array().unwrap().iter().find(|c| c["address"] == address).unwrap();
        assert_eq!(cell["display"], display, "{address}");
    }
    for address in ["A2", "B2", "C2", "C3", "E3", "B4", "B5"] {
        assert_eq!(
            w.cells.iter().find(|c| c.address == address).unwrap().formula_result,
            FormulaResult::CachedUnverified
        );
    }
    let PageData::Svg(svg) = w.render(PageFormat::Svg).unwrap().data else { panic!() };
    for display in ["50%", "1900-01-02", "$1,234.50", "0.500"] {
        assert!(svg.contains(&format!(">{display}</text>")), "{display}");
    }
}

#[test]
fn inherited_date_formats_respect_the_workbook_date_system() {
    let bytes = fixture::build(inherited::INHERITED_GRID, "", inherited::INHERITED_STYLES, &[]);
    let mut parts = hanji_package::package::read(&bytes).unwrap();
    let workbook = parts.iter_mut().find(|p| p.name == "xl/workbook.xml").unwrap();
    workbook.data = String::from_utf8(workbook.data.clone())
        .unwrap()
        .replace("<sheets>", "<workbookPr date1904=\"1\"/><sheets>")
        .into_bytes();
    let mut b = open_xlsx(&hanji_package::package::write(&parts).unwrap(), XlsxOptions::default()).unwrap();
    let w = b.render_window_with_fonts(0, "A1:E6", &FontOptions::default()).unwrap();
    for address in ["B1", "B2", "A4", "B4", "C4", "B6"] {
        assert_eq!(w.cells.iter().find(|c| c.address == address).unwrap().display, "1904-01-03", "{address}");
    }
}

#[test]
fn nonempty_text_omitted_from_a_narrow_visible_cell_is_diagnosed() {
    let sheet = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><cols><col min="1" max="2" width="0.01"/></cols><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>OMITTED</t></is></c><c r="B1" t="inlineStr"><is><t/></is></c></row></sheetData></worksheet>"#;
    let mut b = open_xlsx(&fixture::build(sheet, "", fixture::STYLES, &[]), XlsxOptions::default()).unwrap();
    let w = b.render_window_with_fonts(0, "A1:B1", &FontOptions::default()).unwrap();
    assert_eq!(w.cells[0].display, "OMITTED");
    assert!(w.diagnostics().iter().any(|d| d.path == "sheets[0].cells[A1].clipping" && d.message.contains("omitted")));
    assert!(!w.diagnostics().iter().any(|d| d.path == "sheets[0].cells[B1].clipping"));
    assert!(w.warnings().iter().any(|w| w.contains("cell text is omitted")));
    let PageData::Svg(svg) = w.render(PageFormat::Svg).unwrap().data else { panic!() };
    assert!(!svg.contains(">OMITTED</text>"));
    assert!(svg.contains("A1: OMITTED")); // Accessibility keeps the complete omitted value.
    assert!(w.html("narrow").contains("cell text is omitted"));
}

#[test]
fn resource_budgets_refuse_instead_of_truncating_or_allocating_large_rasters() {
    let bytes = fixture::build(fixture::GRID, "", fixture::STYLES, &[]);
    assert!(error(open_xlsx(&bytes, XlsxOptions { max_unpacked_bytes: 100, ..Default::default() })).contains("expands"));
    assert!(error(open_xlsx(&bytes, XlsxOptions { max_window_rows: 0, ..Default::default() })).contains("positive"));
    assert!(error(open_xlsx(&bytes, XlsxOptions { max_font_family_bytes: 2, ..Default::default() }))
        .contains("metadata budget"));
    assert!(error(open_xlsx(&bytes, XlsxOptions { max_number_format_bytes: 2, ..Default::default() }))
        .contains("metadata budget"));
    let mut b = open_xlsx(&bytes, XlsxOptions { max_window_text_bytes: 4, ..Default::default() }).unwrap();
    assert!(error(b.render_window_with_fonts(0, "A1:B2", &FontOptions::default())).contains("text budget"));
    let mut b = book("");
    let w = b.render_window_with_fonts(0, "A1:B4", &FontOptions::default()).unwrap();
    for dpi in [0.0, -1.0, f64::NAN, f64::INFINITY, 1e30] {
        assert!(w.render(PageFormat::Png { dpi }).is_err());
    }
    assert!(error(w.render(PageFormat::Png { dpi: 10_000.0 })).contains("pixel budget"));
    assert!(w.render(PageFormat::Svg).is_ok());
}

#[test]
fn macros_links_and_formula_calls_are_never_executed_or_refreshed() {
    let rels = br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="external" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/externalLink" Target="https://example.invalid/book.xlsx" TargetMode="External"/></Relationships>"#;
    let sheet = fixture::GRID.replace("SUM(B2,1)", "WEBSERVICE(&quot;https://example.invalid/&quot;)");
    let bytes = fixture::build(
        &sheet,
        "",
        fixture::STYLES,
        &[
            ("xl/vbaProject.bin", b"synthetic nonexecutable fixture"),
            ("xl/externalLinks/_rels/externalLink1.xml.rels", rels),
        ],
    );
    let original = bytes.clone();
    let mut b = open_xlsx(&bytes, XlsxOptions::default()).unwrap();
    let w = b.render_window_with_fonts(0, "A1:B4", &FontOptions::default()).unwrap();
    assert_eq!(w.cells.iter().find(|c| c.address == "A3").unwrap().display, "1235.5");
    assert_eq!(w.cells.iter().find(|c| c.address == "A3").unwrap().formula_result, FormulaResult::CachedUnverified);
    assert!(w.diagnostics().iter().any(|d| d.path == "workbook.macros"));
    assert!(w.diagnostics().iter().any(|d| d.path == "workbook.externalLinks"));
    assert!(!w.html("safe").contains("href=\"https://"));
    assert_eq!(bytes, original);
}

#[test]
fn malformed_rows_and_overlapping_merges_are_reported() {
    let sheet = fixture::GRID.replace("r=\"B2\"", "r=\"B0\"");
    let mut b = open_xlsx(&fixture::build(&sheet, "", fixture::STYLES, &[]), XlsxOptions::default()).unwrap();
    assert!(error(b.render_window_with_fonts(0, "A1:B4", &FontOptions::default())).contains("invalid cell column"));
    let sheet =
        fixture::GRID.replace("<mergeCell ref=\"A1:B1\"/>", "<mergeCell ref=\"A1:B1\"/><mergeCell ref=\"B1:C1\"/>");
    let mut b = open_xlsx(&fixture::build(&sheet, "", fixture::STYLES, &[]), XlsxOptions::default()).unwrap();
    assert!(error(b.render_window_with_fonts(0, "A1:C4", &FontOptions::default())).contains("overlapping"));
    let styles = fixture::STYLES.replace("FF123456", "éééé");
    let mut b = open_xlsx(&fixture::build(fixture::GRID, "", &styles, &[]), XlsxOptions::default()).unwrap();
    let w = b.render_window_with_fonts(0, "A1:B4", &FontOptions::default()).unwrap();
    assert!(w.diagnostics().iter().any(|d| d.message.contains("invalid color")));
}

#[test]
fn sizable_sheet_indexes_rows_but_lays_out_only_the_requested_window() {
    let mut sheet =
        String::from(r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#);
    for r in 1..=20_000 {
        sheet.push_str(&format!("<row r=\"{r}\"><c r=\"A{r}\"><v>{r}</v></c></row>"));
    }
    sheet.push_str("</sheetData></worksheet>");
    let bytes = fixture::build(&sheet, "", fixture::STYLES, &[]);
    let mut b = open_xlsx(&bytes, XlsxOptions::default()).unwrap();
    let w = b.render_window_with_fonts(0, "A19990:B20000", &FontOptions::default()).unwrap();
    assert_eq!(w.cells.len(), 11);
    assert_eq!(w.cells.last().unwrap().display, "20000");
    assert_eq!(w.page_info().height, 183.0);
}

#[test]
#[cfg(not(target_family = "wasm"))]
fn supplied_korean_font_renders_multilingual_cells_without_missing_glyphs() {
    let Some(dir) = std::env::var_os("HANJI_TEST_FONT_DIR") else {
        return;
    };
    let data = std::fs::read(std::path::PathBuf::from(dir).join("Arial Unicode.ttf")).unwrap();
    let opts = FontOptions {
        fonts: vec![hanji_preview::FontData::new(data)],
        aliases: Some(
            hanji_preview::fonts::Aliases::parse(
                "[[family]]\nnames = [\"Calibri\"]\nsubstitute = [\"Arial Unicode MS\"]\n",
            )
            .unwrap(),
        ),
    };
    let mut b = book("");
    let w = b.render_window_with_fonts(0, "A1:B1", &opts).unwrap();
    assert_eq!(w.fonts().missing_glyphs_total, 0);
    assert!(w.fonts().substituted.iter().any(|f| f.source == Some(hanji_preview::fonts::Source::Supplied)));
    assert!(w.render(PageFormat::Png { dpi: 72.0 }).is_ok());
}

#[test]
fn utf16_escaped_supplementary_characters_survive_the_shared_reader() {
    assert_eq!(hanji_xlsx::sst::decode_xstring("_xD83D__xDE00_"), "😀");
    assert_eq!(hanji_xlsx::sst::decode_xstring("_xD83D_text"), "\u{FFFD}text");
    assert_eq!(hanji_xlsx::sst::decode_xstring("_x005F_xD83D_"), "_xD83D_");
    let sheet = fixture::GRID.replace("HIDDEN", "_xD83D__xDE00_").replace("hidden=\"1\" width=\"10\"", "width=\"10\"");
    let mut b = open_xlsx(&fixture::build(&sheet, "", fixture::STYLES, &[]), XlsxOptions::default()).unwrap();
    let w = b.render_window_with_fonts(0, "A1:C4", &FontOptions::default()).unwrap();
    assert_eq!(w.cells.iter().find(|c| c.address == "C2").unwrap().display, "😀");
}

#[test]
fn invalid_values_are_reported_and_complex_text_is_diagnosed() {
    for (from, to, expected) in [("<v>1</v>", "<v>99999</v>", "shared-string"), ("1234.5", "NaN", "nonfinite")] {
        let sheet = fixture::GRID.replace(from, to);
        let mut b = open_xlsx(&fixture::build(&sheet, "", fixture::STYLES, &[]), XlsxOptions::default()).unwrap();
        assert!(error(b.render_window_with_fonts(0, "A1:B4", &FontOptions::default())).contains(expected));
    }
    let sheet = fixture::GRID
        .replace("<v>1234.5</v>", "<is><t>مرحبا é</t></is>")
        .replace("r=\"B2\" s=\"2\"", "r=\"B2\" t=\"inlineStr\"");
    let mut b = open_xlsx(&fixture::build(&sheet, "", fixture::STYLES, &[]), XlsxOptions::default()).unwrap();
    let w = b.render_window_with_fonts(0, "A1:B4", &FontOptions::default()).unwrap();
    assert!(w
        .diagnostics()
        .iter()
        .any(|d| d.path == "sheets[0].cells[B2].text" && d.message.contains("bidirectional")));
}

#[test]
fn owned_windows_render_after_the_workbook_is_dropped() {
    fn worker_safe<T: Send + Sync>() {}
    worker_safe::<hanji_preview::xlsx::Workbook>();
    worker_safe::<hanji_preview::xlsx::Window>();
    let mut workbook = book("");
    let window = workbook.render_window_with_fonts(0, "A1:B4", &FontOptions::default()).unwrap();
    drop(workbook);
    assert!(matches!(window.render(PageFormat::Svg).unwrap().data, PageData::Svg(ref s) if s.contains("#UNEVALUATED")));
    assert!(
        matches!(window.render(PageFormat::Png { dpi: 72.0 }).unwrap().data, PageData::Png(ref b) if b.starts_with(b"\x89PNG"))
    );
}

#[test]
fn unsupported_font_advance_policy_does_not_claim_applied_metrics() {
    struct AdvancePolicy;
    impl hanji_preview::FontResolver for AdvancePolicy {
        fn resolve_font(
            &self,
            _: &str,
            _: hanji_preview::fonts::Script,
            _: bool,
            _: bool,
        ) -> Option<hanji_preview::ResolvedFont> {
            Some(hanji_preview::ResolvedFont {
                family: "Carlito".into(),
                source: hanji_preview::fonts::Source::Supplied,
                metrics: hanji_preview::fonts::Metrics::Table,
                ea_advance: Some(1.0),
                font: hanji_preview::FontData::new(oxml_layout::bundled_fonts::bundled_font_data()[0].1),
            })
        }
    }
    let mut workbook = book("");
    let window = workbook.render_window_with_resolver(0, "A1:B4", &AdvancePolicy).unwrap();
    assert!(window.diagnostics().iter().any(|d| d.message.contains("advance overrides are not applied")));
    assert!(window.fonts().substituted.iter().all(|f| f.metrics == hanji_preview::fonts::Metrics::Substitute));
    assert!(window.render(PageFormat::Svg).unwrap().diagnostics.iter().any(|d| d.path.starts_with("window.fonts[")));
}

fn themed_book(styles: &str, theme: &str, target: &str, external: bool) -> Vec<u8> {
    let bytes = fixture::build(fixture::GRID, "", styles, &[]);
    let mut parts = hanji_package::package::read(&bytes).unwrap();
    let rels = parts.iter_mut().find(|p| p.name == "xl/_rels/workbook.xml.rels").unwrap();
    let mode = if external { " TargetMode=\"External\"" } else { "" };
    let rel = format!(
        r#"<Relationship Id="theme" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="{target}"{mode}/>"#
    );
    rels.data = String::from_utf8(rels.data.clone())
        .unwrap()
        .replace("</Relationships>", &format!("{rel}</Relationships>"))
        .into_bytes();
    parts.push(hanji_core::Part {
        name: "custom/colors.xml".into(),
        data: theme.as_bytes().to_vec(),
        dos_time: 0,
        external_attr: 0,
        deflate: true,
    });
    hanji_package::package::write(&parts).unwrap()
}

const WORKSHEET_THEME: &str = r#"<d:theme xmlns:d="http://schemas.openxmlformats.org/drawingml/2006/main"><d:themeElements><d:clrScheme name="Owned"><d:accent1><d:srgbClr val="FF0000"/></d:accent1><d:dk2><d:srgbClr val="123456"/></d:dk2><d:lt2><d:srgbClr val="000000"/></d:lt2><d:lt1><d:sysClr val="window" lastClr="FFFFFF"/></d:lt1><d:dk1><d:sysClr val="windowText" lastClr="000000"/></d:dk1></d:clrScheme></d:themeElements></d:theme>"#;

#[test]
fn worksheet_theme_and_tint_colors_match_explicit_rgb_in_all_outputs() {
    let themed = fixture::STYLES
        .replace("rgb=\"FF000000\"", "theme=\"1\"")
        .replace("rgb=\"FF123456\"", "theme=\"0\"")
        .replace("rgb=\"FFCCEEDD\"", "theme=\"4\" tint=\"0.5\"")
        .replace("rgb=\"FF005500\"", "theme=\"2\" tint=\"0.5\"");
    let explicit =
        fixture::STYLES.replace("FF123456", "FFFFFFFF").replace("FFCCEEDD", "FFFF8080").replace("FF005500", "FF808080");
    let input = themed_book(&themed, WORKSHEET_THEME, "../custom/colors.xml", false);
    let original = input.clone();
    let mut a = open_xlsx(&input, XlsxOptions::default()).unwrap();
    let mut b = open_xlsx(&fixture::build(fixture::GRID, "", &explicit, &[]), XlsxOptions::default()).unwrap();
    let a = a.render_window_with_fonts(0, "A1:C5", &FontOptions::default()).unwrap();
    let b = b.render_window_with_fonts(0, "A1:C5", &FontOptions::default()).unwrap();
    assert_eq!(a.page_info(), b.page_info());
    assert_eq!(serde_json::to_value(&a.cells).unwrap(), serde_json::to_value(&b.cells).unwrap());
    assert_eq!(serde_json::to_value(a.fonts()).unwrap(), serde_json::to_value(b.fonts()).unwrap());
    assert!(!a.diagnostics().iter().any(|d| d.message.contains("invalid color")));
    for format in [PageFormat::Svg, PageFormat::Png { dpi: 96.0 }] {
        let bytes = |window: &hanji_preview::xlsx::Window| match window.render(format).unwrap().data {
            PageData::Svg(s) => s.into_bytes(),
            PageData::Png(png) => png,
        };
        assert_eq!(bytes(&a), bytes(&b));
        assert_eq!(bytes(&a), bytes(&a));
    }
    assert_eq!(a.html("owned"), b.html("owned"));
    assert_eq!(input, original);
}

#[test]
fn worksheet_rgb_tints_preserve_hue_and_cover_luminance_endpoints() {
    for (base, tint, expected) in [
        ("FFFF0000", "-0.5", "FF800000"),
        ("FFFF0000", "0.5", "FFFF8080"),
        ("FF008000", "0.5", "FF40FF40"),
        ("FF808080", "0.5", "FFC0C0C0"),
        ("FF123456", "0", "FF123456"),
        ("FF123456", "-1", "FF000000"),
        ("FF123456", "1", "FFFFFFFF"),
        ("FF000000", "0.5", "FF808080"),
        ("FFFFFFFF", "-0.5", "FF808080"),
    ] {
        let tinted = fixture::STYLES.replace("rgb=\"FF123456\"", &format!("rgb=\"{base}\" tint=\"{tint}\""));
        let explicit = fixture::STYLES.replace("FF123456", expected);
        let render = |styles: &str| {
            let mut book = open_xlsx(&fixture::build(fixture::GRID, "", styles, &[]), XlsxOptions::default()).unwrap();
            let window = book.render_window_with_fonts(0, "A1:C5", &FontOptions::default()).unwrap();
            let PageData::Svg(svg) = window.render(PageFormat::Svg).unwrap().data else { panic!() };
            svg
        };
        assert!(render(&tinted) == render(&explicit), "{base} tint={tint} must match {expected}");
    }
}

#[test]
fn unavailable_and_invalid_worksheet_colors_keep_diagnostics_and_defaults() {
    let explicit = fixture::STYLES.replace("FF123456", "FF000000");
    let mut control = open_xlsx(&fixture::build(fixture::GRID, "", &explicit, &[]), XlsxOptions::default()).unwrap();
    let control = control.render_window_with_fonts(0, "A1:C5", &FontOptions::default()).unwrap();
    let PageData::Svg(default_svg) = control.render(PageFormat::Svg).unwrap().data else { panic!() };
    for color in [
        "theme=\"12\"",
        "theme=\"-1\"",
        "theme=\"NaN\"",
        "theme=\"6\"",
        "indexed=\"64\"",
        "rgb=\"GG123456\"",
        "rgb=\"12345\"",
        "rgb=\"00000Z\"",
        "theme=\"4\" tint=\"NaN\"",
        "theme=\"4\" tint=\"inf\"",
        "theme=\"4\" tint=\"-inf\"",
        "theme=\"4\" tint=\"1.00001\"",
        "theme=\"4\" tint=\"-1.00001\"",
        "theme=\"4\" tint=\"text\"",
        "rgb=\"FF123456\" tint=\"NaN\"",
    ] {
        let styles = fixture::STYLES.replace("rgb=\"FF123456\"", color);
        let mut book =
            open_xlsx(&themed_book(&styles, WORKSHEET_THEME, "../custom/colors.xml", false), XlsxOptions::default())
                .unwrap();
        let window = book.render_window_with_fonts(0, "A1:C5", &FontOptions::default()).unwrap();
        assert!(window.diagnostics().iter().any(|d| d.message.contains("invalid color")), "{color}");
        let PageData::Svg(svg) = window.render(PageFormat::Svg).unwrap().data else { panic!() };
        assert!(svg == default_svg, "{color} must retain the complete default-color control output");
    }
    let styles = fixture::STYLES.replace("rgb=\"FF123456\"", "theme=\"4\"");
    for (theme, target, external) in [
        (WORKSHEET_THEME, "https://invalid.example/colors.xml", true),
        (WORKSHEET_THEME, "../custom/missing.xml", false),
        ("malformed XML", "../custom/colors.xml", false),
        ("<theme/>", "../custom/colors.xml", false),
    ] {
        let mut book = open_xlsx(&themed_book(&styles, theme, target, external), XlsxOptions::default()).unwrap();
        let window = book.render_window_with_fonts(0, "A1:C5", &FontOptions::default()).unwrap();
        assert!(window.diagnostics().iter().any(|d| d.message.contains("invalid color")));
        if external {
            assert!(window.diagnostics().iter().any(|d| d.message.contains("external relationships")));
        }
    }
    for invalid in [
        "<d:srgbClr val=\"GG0000\"/>",
        "<d:srgbClr val=\"FFFF0000\"/>",
        "<d:sysClr val=\"windowText\"/>",
        "<d:srgbClr val=\"FF0000\"><d:tint val=\"50000\"/></d:srgbClr>",
    ] {
        let theme = WORKSHEET_THEME.replace("<d:srgbClr val=\"FF0000\"/>", invalid);
        let mut book =
            open_xlsx(&themed_book(&styles, &theme, "../custom/colors.xml", false), XlsxOptions::default()).unwrap();
        let window = book.render_window_with_fonts(0, "A1:C5", &FontOptions::default()).unwrap();
        assert!(window.diagnostics().iter().any(|d| d.message.contains("invalid color")), "{invalid}");
    }
}
