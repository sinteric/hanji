//! Authored indentation controls use real spaces; production cell text stays untouched.
#[path = "fixtures/xlsx_grid.rs"]
mod fixture;
use hanji_preview::{
    xlsx::{open_xlsx, XlsxOptions},
    FontOptions, PageData, PageFormat,
};

fn styles(alignment: &str, cell_font: usize, normal_font: usize) -> String {
    format!(
        r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><fonts count="3"><font><name val="Calibri"/><sz val="11"/></font><font><name val="Caladea"/><sz val="22"/><b/></font><font><name val="Calibri"/><sz val="9"/><i/></font></fonts><fills><fill><patternFill patternType="none"/></fill></fills><borders><border/></borders><cellStyleXfs><xf fontId="1"/><xf fontId="{normal_font}"/></cellStyleXfs><cellXfs><xf fontId="{cell_font}" fillId="0" borderId="0">{alignment}</xf></cellXfs><cellStyles><cellStyle name="표준" builtinId="0" xfId="1"/></cellStyles></styleSheet>"#
    )
}
fn render(
    text: &str,
    alignment: &str,
    cell_font: usize,
    normal_font: usize,
    width: f64,
) -> (hanji_preview::xlsx::Window, Vec<u8>) {
    let sheet = format!(
        r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><cols><col min="1" max="1" width="{width}"/></cols><sheetData><row r="1" ht="120" customHeight="1"><c r="A1" t="inlineStr"><is><t xml:space="preserve">{text}</t></is></c></row></sheetData></worksheet>"#
    );
    let bytes = fixture::build(&sheet, "", &styles(alignment, cell_font, normal_font), &[]);
    let original = bytes.clone();
    let mut book = open_xlsx(&bytes, XlsxOptions::default()).unwrap();
    let window = book.render_window_with_fonts(0, "A1", &FontOptions::default()).unwrap();
    assert_eq!(bytes, original);
    (window, bytes)
}
fn runs(window: &hanji_preview::xlsx::Window) -> Vec<(String, Vec<f64>, f64)> {
    let PageData::Svg(svg) = window.render(PageFormat::Svg).unwrap().data else { panic!() };
    let tree = hanji_package::xml::parse(svg.as_bytes()).unwrap();
    let mut out = vec![];
    tree.root.walk(&mut |e| {
        if e.local() == "text" {
            let text: String = e
                .children
                .iter()
                .filter_map(|n| match n {
                    hanji_package::xml::Node::Text(s) => Some(hanji_package::xml::unescape(s)),
                    _ => None,
                })
                .collect();
            let x = e.get("x").unwrap().split_whitespace().map(|s| s.parse().unwrap()).collect();
            out.push((text, x, e.get("y").unwrap().parse().unwrap()));
        }
    });
    out
}
fn cell_run(window: &hanji_preview::xlsx::Window, text: &str) -> (Vec<f64>, f64) {
    let r = runs(window).into_iter().find(|r| r.0 == text).unwrap();
    (r.1, r.2)
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 0.00001, "{a} != {b}");
}
fn pixels(window: &hanji_preview::xlsx::Window) -> Vec<u8> {
    let PageData::Png(png) = window.render(PageFormat::Png { dpi: 96.0 }).unwrap().data else { panic!() };
    resvg::tiny_skia::Pixmap::decode_png(&png).unwrap().data().to_vec()
}

#[test]
fn left_and_right_indent_match_explicit_spaces_without_changing_cell_text() {
    for (horizontal, text, index) in [("left", "      ABC", 6), ("right", "ABC      ", 0)] {
        let (actual, _) = render("ABC", &format!(r#"<alignment horizontal="{horizontal}" indent="2"/>"#), 0, 0, 30.0);
        let (control, _) = render(text, &format!(r#"<alignment horizontal="{horizontal}"/>"#), 0, 0, 30.0);
        let a = cell_run(&actual, "ABC");
        let c = cell_run(&control, text);
        near(a.0[0], c.0[index]);
        near(a.1, c.1);
        assert_eq!(pixels(&actual), pixels(&control));
        assert_eq!(actual.cells[0].display, "ABC");
        assert!(actual.html("indent").contains("<td>ABC</td>"));
        assert!(!actual.diagnostics().iter().any(|d| d.message.contains("indentation")));
    }
}

#[test]
fn indentation_uses_the_localized_normal_style_font_not_the_cell_font() {
    let (space, _) = render("   X", r#"<alignment horizontal="left"/>"#, 2, 2, 40.0);
    let (plain_space, _) = render("X", r#"<alignment horizontal="left"/>"#, 2, 2, 40.0);
    let expected = cell_run(&space, "   X").0[3] - cell_run(&plain_space, "X").0[0];
    for h in ["left", "right"] {
        let (indented, _) = render("ABC", &format!(r#"<alignment horizontal="{h}" indent="1"/>"#), 1, 2, 40.0);
        let (plain, _) = render("ABC", &format!(r#"<alignment horizontal="{h}"/>"#), 1, 2, 40.0);
        let delta = cell_run(&indented, "ABC").0[0] - cell_run(&plain, "ABC").0[0];
        near(delta, if h == "left" { expected } else { -expected });
        assert_eq!(serde_json::to_value(indented.fonts()).unwrap(), serde_json::to_value(plain.fonts()).unwrap());
    }
}

#[test]
fn wrapping_and_newlines_use_the_indented_content_width_on_every_line() {
    for h in ["left", "right"] {
        let (plain, _) =
            render("ABCDEFGHIJABCDEFGHIJ", &format!(r#"<alignment horizontal="{h}" wrapText="1"/>"#), 0, 0, 8.0);
        let (indented, _) = render(
            "ABCDEFGHIJABCDEFGHIJ",
            &format!(r#"<alignment horizontal="{h}" indent="2" wrapText="1"/>"#),
            0,
            0,
            8.0,
        );
        let p: Vec<_> = runs(&plain).into_iter().filter(|r| r.0.len() > 1).collect();
        let i: Vec<_> = runs(&indented).into_iter().filter(|r| r.0.len() > 1).collect();
        assert!(i.len() > p.len(), "indent must reduce the available wrapping width");
        assert_eq!(i.iter().map(|r| r.0.as_str()).collect::<String>(), "ABCDEFGHIJABCDEFGHIJ");
        let (multi, _) = render("ABC\nABC", &format!(r#"<alignment horizontal="{h}" indent="2"/>"#), 0, 0, 30.0);
        let positions: Vec<_> = runs(&multi).into_iter().filter(|r| r.0 == "ABC").collect();
        assert_eq!(positions.len(), 2);
        near(positions[0].1[0], positions[1].1[0]);
    }
}

#[test]
fn unsupported_alignment_and_malformed_indent_remain_diagnosed() {
    for h in ["center", "general", "distributed", "justify"] {
        let (w, _) = render("ABC", &format!(r#"<alignment horizontal="{h}" indent="1"/>"#), 0, 0, 30.0);
        assert!(w.diagnostics().iter().any(|d| d.message.contains("indentation")), "{h}");
    }
    for indent in ["-1", "1.5", "4294967296", "NaN"] {
        let (w, _) = render("ABC", &format!(r#"<alignment horizontal="left" indent="{indent}"/>"#), 0, 0, 30.0);
        assert!(w.diagnostics().iter().any(|d| d.message.contains("invalid indentation")), "{indent}");
    }
    let (huge, _) = render("ABC", r#"<alignment horizontal="left" indent="4294967295" wrapText="1"/>"#, 0, 0, 30.0);
    assert_eq!(huge.cells[0].display, "ABC");
    assert!(!runs(&huge).iter().any(|r| r.0.contains("ABC")));
    assert!(huge.diagnostics().iter().any(|d| d.path.ends_with(".clipping")));
}

#[test]
fn rtl_reading_order_does_not_exchange_explicit_left_and_right_indentation() {
    for h in ["left", "right"] {
        let (rtl, _) =
            render("ABC", &format!(r#"<alignment horizontal="{h}" indent="1" readingOrder="2"/>"#), 0, 0, 30.0);
        let (ltr, _) =
            render("ABC", &format!(r#"<alignment horizontal="{h}" indent="1" readingOrder="1"/>"#), 0, 0, 30.0);
        let (plain, _) = render("ABC", &format!(r#"<alignment horizontal="{h}" indent="1"/>"#), 0, 0, 30.0);
        near(cell_run(&rtl, "ABC").0[0], cell_run(&plain, "ABC").0[0]);
        near(cell_run(&ltr, "ABC").0[0], cell_run(&plain, "ABC").0[0]);
        assert!(rtl.diagnostics().iter().any(|d| d.message.contains("reading order")));
        let (hebrew, _) =
            render("אבג", &format!(r#"<alignment horizontal="{h}" indent="1" readingOrder="2"/>"#), 0, 0, 30.0);
        assert!(hebrew.diagnostics().iter().any(|d| d.message.contains("bidirectional")));
    }
}

#[test]
fn indentation_preserves_numeric_and_missing_formula_overflow_guards() {
    let sheet = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><cols><col min="1" max="1" width="10"/></cols><sheetData><row r="1" ht="30"><c r="A1"><v>12345</v></c></row><row r="2" ht="30"><c r="A2"><f>1+2</f><v/></c></row><row r="3" ht="30"><c r="A3"><f>1+2</f><v>3</v></c></row></sheetData></worksheet>"#;
    let package =
        fixture::build(sheet, "", &styles(r#"<alignment horizontal="right" indent="10" wrapText="1"/>"#, 0, 0), &[]);
    let mut b = open_xlsx(&package, XlsxOptions::default()).unwrap();
    let w = b.render_window_with_fonts(0, "A1:A3", &FontOptions::default()).unwrap();
    assert_eq!(w.cells.iter().map(|c| c.display.as_str()).collect::<Vec<_>>(), ["12345", "#UNEVALUATED", "3"]);
    assert_eq!(w.cells[1].formula.as_deref(), Some("1+2"));
    assert_eq!(w.cells[2].formula_result, hanji_preview::xlsx::FormulaResult::CachedUnverified);
    let drawn = runs(&w);
    assert!(!drawn.iter().any(|r| ["12345", "#UNEVALUATED", "3"].contains(&r.0.as_str()) && r.1[0] >= 36.0));
    assert!(drawn.iter().any(|r| !r.0.is_empty() && r.0.chars().all(|c| c == '#')));
    assert!(drawn.iter().any(|r| r.0 == "?"));
    for address in ["A1", "A2", "A3"] {
        assert!(w
            .diagnostics()
            .iter()
            .any(|d| d.path == format!("sheets[0].cells[{address}].clipping") && d.message.contains("indicator")));
    }
    assert!(w.html("guarded").contains("<td>12345</td>"));
}

#[test]
fn inactive_alignment_and_zero_indent_preserve_output_and_resource_budgets() {
    let (zero, _) = render("ABC", r#"<alignment horizontal="left" indent="0"/>"#, 0, 0, 30.0);
    let (plain, _) = render("ABC", r#"<alignment horizontal="left"/>"#, 0, 0, 30.0);
    assert_eq!(pixels(&zero), pixels(&plain));
    let inactive = styles(r#"<alignment horizontal="left" indent="4294967295"/>"#, 0, 0).replace(
        "<xf fontId=\"0\" fillId=\"0\" borderId=\"0\">",
        "<xf fontId=\"0\" fillId=\"0\" borderId=\"0\" applyAlignment=\"0\">",
    );
    let sheet = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><cols><col min="1" max="1" width="30"/></cols><sheetData><row r="1" ht="120"><c r="A1" t="inlineStr"><is><t>ABC</t></is></c></row></sheetData></worksheet>"#;
    let bytes = fixture::build(sheet, "", &inactive, &[]);
    let mut b = open_xlsx(&bytes, XlsxOptions::default()).unwrap();
    let w = b.render_window_with_fonts(0, "A1", &FontOptions::default()).unwrap();
    near(cell_run(&w, "ABC").0[0], cell_run(&plain, "ABC").0[0]);
    assert!(!w.diagnostics().iter().any(|d| d.message.contains("indentation")));
    let sample = fixture::build(fixture::GRID, "", fixture::STYLES, &[]);
    let options = XlsxOptions { max_window_text_bytes: 1, ..XlsxOptions::default() };
    let mut b = open_xlsx(&sample, options).unwrap();
    assert!(b.render_window_with_fonts(0, "A1:B1", &FontOptions::default()).err().unwrap().contains("text budget"));
    let options = XlsxOptions { max_png_pixels: 1, ..XlsxOptions::default() };
    let mut b = open_xlsx(&bytes, options).unwrap();
    let w = b.render_window_with_fonts(0, "A1", &FontOptions::default()).unwrap();
    assert!(w.render(PageFormat::Png { dpi: 96.0 }).err().unwrap().contains("pixel budget"));
}

#[test]
fn unresolved_normal_style_metrics_are_reported_and_normal_metadata_is_budgeted() {
    let missing =
        styles(r#"<alignment horizontal="left" indent="1"/>"#, 0, 0).replace("builtinId=\"0\"", "builtinId=\"1\"");
    let mut b = open_xlsx(&fixture::build(fixture::GRID, "", &missing, &[]), XlsxOptions::default()).unwrap();
    // Use a single cell with style zero, avoiding the grid fixture's other styles.
    let w = b.render_window_with_fonts(0, "A2", &FontOptions::default()).unwrap();
    assert!(w.diagnostics().iter().any(|d| d.message.contains("Normal style font is unresolved")));
    let overlong = styles(r#"<alignment horizontal="left" indent="1"/>"#, 0, 2)
        .replace("name val=\"Calibri\"/><sz val=\"9\"", "name val=\"Overlong Normal font family\"/><sz val=\"9\"");
    let mut b = open_xlsx(&fixture::build(fixture::GRID, "", &overlong, &[]), XlsxOptions::default()).unwrap();
    let w = b.render_window_with_fonts(0, "A2", &FontOptions::default()).unwrap();
    let report = w.quality();
    let metric = report.diagnostics.iter().find(|d| d.path == "window.indentation").unwrap();
    assert_eq!(metric.code, hanji_preview::quality::DiagnosticCode::WorksheetTextApproximation);
    assert_eq!(metric.consequence, hanji_preview::quality::Consequence::Approximation);
    let implicit = styles(r#"<alignment horizontal="left" indent="1"/>"#, 0, 0).replace("<xf fontId=\"0\"/>", "<xf/>");
    let mut b = open_xlsx(&fixture::build(fixture::GRID, "", &implicit, &[]), XlsxOptions::default()).unwrap();
    let w = b.render_window_with_fonts(0, "A2", &FontOptions::default()).unwrap();
    assert!(!w.diagnostics().iter().any(|d| d.message.contains("Normal style font is unresolved")));
    let options = XlsxOptions { max_font_family_bytes: 10, ..XlsxOptions::default() };
    assert!(open_xlsx(&fixture::build(fixture::GRID, "", &overlong, &[]), options)
        .err()
        .unwrap()
        .contains("font family"));
}
