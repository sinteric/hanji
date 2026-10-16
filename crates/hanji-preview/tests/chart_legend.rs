//! Omitted pie/doughnut legends surface as preview diagnostics and warnings
//! without rewriting the source package.

use hanji_preview::{
    docx, render_document_with_fonts, render_pptx_with_fonts, DocumentKind, DocumentPreview, FontOptions,
    CHART_LEGEND_OMITTED_WARNING,
};

fn oversized_korean_pie() -> rdocx::ChartData {
    rdocx::ChartData {
        categories: [
            "구독 서비스 매출",
            "라이선스",
            "유지보수 및 기술 지원",
            "교육",
            "컨설팅 프로젝트",
            "기타 수익",
            "하드웨어 판매",
            "클라우드 호스팅",
            "광고",
            "파트너 수수료",
        ]
        .map(str::to_owned)
        .to_vec(),
        series: vec![("매출".to_owned(), (1..=10).map(f64::from).collect())],
        ..rdocx::ChartData::default()
    }
}

fn assert_legend_omitted(preview: &DocumentPreview, source: &str) {
    let diagnostic = preview
        .diagnostics()
        .iter()
        .find(|d| rdocx::ChartLegendOmitted::from_diagnostic_message(&d.message).is_some())
        .unwrap_or_else(|| panic!("missing chart-legend-omitted diagnostic: {:?}", preview.diagnostics()));
    assert!(diagnostic.path.starts_with("layout.diagnostics["), "{}", diagnostic.path);
    assert!(diagnostic.message.starts_with(rdocx::CHART_LEGEND_OMITTED), "{}", diagnostic.message);
    let notice = rdocx::ChartLegendOmitted::from_diagnostic_message(&diagnostic.message).unwrap();
    assert_eq!(notice.reason.as_str(), "entries-exceed-height");
    assert_eq!(notice.entry_count, 10);
    assert!(diagnostic.message.contains(&format!("source={source}")), "{}", diagnostic.message);
    assert!(preview.warnings().iter().any(|w| w.contains(CHART_LEGEND_OMITTED_WARNING)), "{:?}", preview.warnings());
    assert!(preview.html("legend").contains(CHART_LEGEND_OMITTED_WARNING));
    let kind = if source.starts_with("word:") { DocumentKind::Docx } else { DocumentKind::Pptx };
    assert!(preview
        .quality(kind)
        .diagnostics
        .iter()
        .any(|d| d.code == hanji_preview::quality::DiagnosticCode::ChartLegendOmitted));
}

#[test]
fn small_docx_pie_warns_that_the_legend_was_omitted_without_rewriting_bytes() {
    let data = oversized_korean_pie();
    let mut document = rdocx::Document::new();
    document.add_chart(rdocx::ChartKind::Pie, rdocx::Length::pt(200.0), rdocx::Length::pt(140.0), &data).unwrap();
    let bytes = document.to_bytes().unwrap();
    let original = bytes.clone();
    let preview = docx::render_with_fonts(&bytes, &FontOptions::default()).unwrap();
    assert_eq!(bytes, original);
    assert_eq!(hanji_package::package::read(&bytes).unwrap(), hanji_package::package::read(&original).unwrap());
    assert_legend_omitted(&DocumentPreview::Layout(Box::new(preview)), "word:rId");
}

#[test]
fn fitting_docx_pie_does_not_warn_about_an_omitted_legend() {
    let data = oversized_korean_pie();
    let mut document = rdocx::Document::new();
    document.add_chart(rdocx::ChartKind::Pie, rdocx::Length::inches(6.0), rdocx::Length::inches(3.5), &data).unwrap();
    let bytes = document.to_bytes().unwrap();
    let original = bytes.clone();
    let preview = render_document_with_fonts(DocumentKind::Docx, &bytes, &FontOptions::default()).unwrap();
    assert_eq!(bytes, original);
    assert!(!preview.diagnostics().iter().any(|d| d.message.starts_with(rdocx::CHART_LEGEND_OMITTED)));
    assert!(!preview.warnings().iter().any(|w| w.contains(CHART_LEGEND_OMITTED_WARNING)));
}

#[test]
fn small_pptx_pie_warns_that_the_legend_was_omitted_without_rewriting_bytes() {
    let data = oversized_korean_pie();
    let template = include_bytes!("../../hanji-pptx/corpus/shapes.pptx");
    let mut deck = rpptx::Presentation::from_bytes(template).unwrap();
    deck.add_chart(
        0,
        rpptx::ChartKind::Pie,
        rpptx::Emu(0),
        rpptx::Emu(0),
        rpptx::Emu(2_540_000),
        rpptx::Emu(1_778_000),
        &data,
    )
    .unwrap();
    let bytes = deck.to_bytes().unwrap();
    let original = bytes.clone();
    let preview = render_pptx_with_fonts(&bytes, &FontOptions::default()).unwrap();
    assert_eq!(bytes, original);
    assert_eq!(hanji_package::package::read(&bytes).unwrap(), hanji_package::package::read(&original).unwrap());
    let notice = preview
        .diagnostics
        .iter()
        .find_map(|d| rdocx::ChartLegendOmitted::from_diagnostic_message(&d.message))
        .expect("pptx chart-legend-omitted");
    assert_eq!(notice.width_pt, 200.0);
    assert_eq!(notice.height_pt, 140.0);
    assert!(preview.warnings.iter().any(|w| w.contains(CHART_LEGEND_OMITTED_WARNING)), "{:?}", preview.warnings);
    assert!(preview.html("legend").contains(CHART_LEGEND_OMITTED_WARNING));
    let source = preview
        .diagnostics
        .iter()
        .find(|d| d.message.starts_with(rdocx::CHART_LEGEND_OMITTED))
        .and_then(|d| d.message.split("source=").nth(1))
        .unwrap();
    assert!(source.starts_with("slide:") || source.starts_with("layout:") || source.starts_with("master:"), "{source}");
}

#[test]
fn fitting_pptx_pie_does_not_warn_about_an_omitted_legend() {
    let data = oversized_korean_pie();
    let template = include_bytes!("../../hanji-pptx/corpus/shapes.pptx");
    let mut deck = rpptx::Presentation::from_bytes(template).unwrap();
    deck.add_chart(
        0,
        rpptx::ChartKind::Pie,
        rpptx::Emu(0),
        rpptx::Emu(0),
        rpptx::Emu(5_486_400),
        rpptx::Emu(3_200_400),
        &data,
    )
    .unwrap();
    let bytes = deck.to_bytes().unwrap();
    let original = bytes.clone();
    let preview = render_pptx_with_fonts(&bytes, &FontOptions::default()).unwrap();
    assert_eq!(bytes, original);
    assert!(!preview.diagnostics.iter().any(|d| d.message.starts_with(rdocx::CHART_LEGEND_OMITTED)));
    assert!(!preview.warnings.iter().any(|w| w.contains(CHART_LEGEND_OMITTED_WARNING)));
}
