use hanji_preview::{render_document_with_fonts, DocumentKind, FontOptions, PageData, PageFormat};

const ENGLISH: &[u8] = include_bytes!("../../../prototype/preview/baseline/08-docx-new-report-en.docx");
const KOREAN: &[u8] = include_bytes!("../../../prototype/preview/baseline/01-docx-untouched-korean-report.docx");

#[test]
fn renderer_upgrade_uses_the_pptx_shape_style_font_color() {
    let bytes = include_bytes!("../../hanji-pptx/corpus/shapes.pptx");
    let p = render_document_with_fonts(DocumentKind::Pptx, bytes, &FontOptions::default()).unwrap();
    let PageData::Svg(svg) = p.render_page(0, PageFormat::Svg).unwrap().data else { panic!("SVG") };
    let root = hanji_package::xml::parse(svg.as_bytes()).unwrap().root;
    let mut colors = vec![];
    root.walk(&mut |element| {
        if element.local() == "text" && element.text_of(&["text", "tspan"]) == "Cloud" {
            colors.push(element.get("fill"));
        }
    });
    // The shape's fontRef chooses the white theme color. Older renderers
    // inherited black instead, despite retaining the shape's geometry.
    assert_eq!(colors, vec![Some("#FFFFFF".into())]);
}

#[test]
fn renderer_upgrade_numbers_the_first_footnote_independently_of_its_storage_id() {
    let parts = hanji_package::package::read(KOREAN).unwrap();
    let document = hanji_package::package::get(&parts, "word/document.xml").unwrap();
    let root = hanji_package::xml::parse(document).unwrap().root;
    let mut storage_ids = vec![];
    root.walk(&mut |element| {
        if element.local() == "footnoteReference" {
            storage_ids.push(element.get("w:id"));
        }
    });
    assert_eq!(storage_ids, vec![Some("2".into())]);

    let p = render_document_with_fonts(DocumentKind::Docx, KOREAN, &FontOptions::default()).unwrap();
    let PageData::Svg(svg) = p.render_page(0, PageFormat::Svg).unwrap().data else { panic!("SVG") };
    let root = hanji_package::xml::parse(svg.as_bytes()).unwrap().root;
    let mut markers = vec![];
    root.walk(&mut |element| {
        if element.local() == "text"
            && element.get("font-size").and_then(|size| size.parse::<f64>().ok()).is_some_and(|size| size < 6.0)
        {
            let text = element.text_of(&["text", "tspan"]);
            if text.chars().all(|c| c.is_ascii_digit()) && !text.is_empty() {
                markers.push(text);
            }
        }
    });
    // Both the body reference and the note marker use the visible label 1.
    assert_eq!(markers, vec!["1", "1"]);
}

#[test]
fn real_pptx_doughnut_has_distinct_wedges_and_category_legend() {
    use std::collections::BTreeSet;

    let bytes = include_bytes!("../../../prototype/preview/baseline/14-pptx-untouched-korean-report-deck.pptx");
    let p = render_document_with_fonts(DocumentKind::Pptx, bytes, &FontOptions::default()).unwrap();
    let PageData::Svg(svg) = p.render_page(5, PageFormat::Svg).unwrap().data else { panic!("SVG") };
    let root = hanji_package::xml::parse(svg.as_bytes()).unwrap().root;
    let mut wedge_colors = BTreeSet::new();
    let mut chart_labels = BTreeSet::new();
    root.walk(&mut |e| {
        // The three large curved wedges are distinct from the rectangular
        // legend swatches, including the slide's separate manual legend.
        if e.local() == "path" && e.get("d").is_some_and(|d| d.contains('C')) {
            if let Some(fill) = e.get("fill") {
                wedge_colors.insert(fill);
            }
        }
        if e.local() == "text" {
            chart_labels.insert(e.text_of(&["text", "tspan"]));
        }
    });
    assert_eq!(wedge_colors, BTreeSet::from(["#4F81BD".into(), "#C0504D".into(), "#9BBB59".into()]));
    // The manual labels include percentages and are split into text runs;
    // these exact category labels must come from the chart's own legend.
    for label in ["구독", "라이선스", "서비스"] {
        assert!(chart_labels.contains(label), "missing chart category {label}");
    }
    assert!(!chart_labels.contains("비중"), "a series legend loses the category labels");
}

#[test]
fn portable_docx_pages_are_owned_self_contained_and_render_all_formats() {
    let bytes = ENGLISH.to_vec();
    let p = render_document_with_fonts(DocumentKind::Docx, &bytes, &FontOptions::default()).unwrap();
    assert_eq!(bytes.as_slice(), ENGLISH);
    drop(bytes);
    assert!(p.page_count() > 0);
    assert_eq!(p.fonts().missing_glyphs_total, 0, "{}", p.fonts().summary());
    let diagnostics = serde_json::to_string(p.diagnostics()).unwrap();
    for index in 0..p.page_count() {
        let page = p.page_info(index).unwrap();
        assert!(page.width > 100.0 && page.height > 100.0);
        let PageData::Svg(svg) = p.render_page(index, PageFormat::Svg).unwrap().data else { panic!("SVG") };
        assert!(svg.contains("<text") && svg.contains("data:font/"));
        let PageData::Png(png) = p.render_page(index, PageFormat::Png { dpi: 96.0 }).unwrap().data else {
            panic!("PNG")
        };
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }
    assert!(p.html("<report>").contains("&lt;report&gt;"));
    assert!(p.render_page(p.page_count(), PageFormat::Svg).is_err());
    assert!(p.render_page(0, PageFormat::Png { dpi: f64::NAN }).is_err());
    assert_eq!(serde_json::to_string(p.diagnostics()).unwrap(), diagnostics);
}

#[test]
fn docx_without_a_korean_font_reports_the_actual_missing_glyphs() {
    let p = render_document_with_fonts(DocumentKind::Docx, KOREAN, &FontOptions::default()).unwrap();
    assert_eq!(p.page_count(), 2);
    assert!(p.fonts().missing_glyphs_total > 0);
    assert!(p.warnings().iter().any(|w| w == hanji_preview::NO_KOREAN_FONT));
}

#[cfg(feature = "hwpx")]
#[test]
fn portable_hwpx_pages_embed_subsets_and_report_missing_korean_fonts() {
    let bytes = include_bytes!("../../../prototype/preview/baseline/30-hwpx-new-plan-ko.hwpx");
    let p = render_document_with_fonts(DocumentKind::Hwpx, bytes, &FontOptions::default()).unwrap();
    assert!(p.page_count() > 0);
    assert!(p.fonts().missing_glyphs_total > 0);
    assert!(p.warnings().iter().any(|w| w == hanji_preview::NO_KOREAN_FONT));
    for index in 0..p.page_count() {
        let PageData::Svg(svg) = p.render_page(index, PageFormat::Svg).unwrap().data else { panic!("SVG") };
        assert!(svg.contains("data:font/") && svg.contains(&format!("hanji-page-{index}-font-")));
        let PageData::Png(png) = p.render_page(index, PageFormat::Png { dpi: 96.0 }).unwrap().data else {
            panic!("PNG")
        };
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }
    let html = p.html("plan");
    assert!(html.contains("hanji-font-") && html.contains("experimental"));
    assert!(p.render_page(0, PageFormat::Png { dpi: 0.0 }).is_err());
    assert!(render_document_with_fonts(DocumentKind::Hwpx, ENGLISH, &FontOptions::default()).is_err());
}

#[cfg(feature = "hwpx")]
#[test]
fn hwpx_html_clip_references_resolve_to_their_own_page_geometry() {
    use hanji_package::xml;
    use std::collections::{BTreeMap, BTreeSet};

    fn definitions(root: &xml::Element) -> BTreeMap<String, String> {
        let mut definitions = BTreeMap::new();
        root.walk(&mut |e| {
            if let Some(id) = e.get("id") {
                let geometry = e.elements().map(xml::Element::to_xml).collect::<String>();
                assert!(definitions.insert(id, geometry).is_none(), "IDs are unique within a page");
            }
        });
        definitions
    }
    fn clips(root: &xml::Element) -> Vec<String> {
        let mut references = vec![];
        root.walk(&mut |e| {
            if let Some(value) = e.get("clip-path") {
                references.push(value.strip_prefix("url(#").unwrap().strip_suffix(')').unwrap().into());
            }
        });
        references
    }

    let bytes = include_bytes!("../../../prototype/preview/baseline/26-hwpx-untouched-footnote-01.hwpx");
    let p = render_document_with_fonts(DocumentKind::Hwpx, bytes, &FontOptions::default()).unwrap();
    assert_eq!(p.page_count(), 6);
    let original: Vec<_> = (0..p.page_count())
        .map(|i| {
            let PageData::Svg(svg) = p.render_page(i, PageFormat::Svg).unwrap().data else { panic!("SVG") };
            xml::parse(svg.as_bytes()).unwrap().root
        })
        .collect();
    let first = definitions(&original[0]);
    let second = definitions(&original[1]);
    assert!(first["cell-clip-6"].contains("width=\"634.986"));
    assert!(second["cell-clip-6"].contains("width=\"49.400"));
    assert_ne!(first["cell-clip-6"], second["cell-clip-6"]);
    assert_ne!(first["body-clip-3"], second["body-clip-3"]);

    let html = p.html("real footnotes");
    let inline: Vec<_> = html
        .split("<section id=\"page-")
        .skip(1)
        .map(|section| {
            let (_, section) = section.split_once('>').unwrap();
            let (svg, _) = section.split_once("</section>").unwrap();
            xml::parse(svg.as_bytes()).unwrap().root
        })
        .collect();
    assert_eq!(inline.len(), original.len());
    let mut document_ids = BTreeSet::new();
    for (before, after) in original.iter().zip(&inline) {
        let local = definitions(after);
        for id in local.keys() {
            assert!(document_ids.insert(id.clone()), "inline SVG ID {id} must not collide across pages");
        }
        let prior = definitions(before);
        let (prior_refs, refs) = (clips(before), clips(after));
        assert_eq!(prior_refs.len(), refs.len());
        for (old, new) in prior_refs.iter().zip(&refs) {
            assert_eq!(local.get(new), prior.get(old), "each clip resolves to its own page's original geometry");
            assert!(local.contains_key(new), "fragment reference resolves within this page");
        }
        assert_eq!(before.text_of(&["text", "tspan"]), after.text_of(&["text", "tspan"]));
    }
}

#[path = "fixtures/docx_vertical.rs"]
mod docx_vertical;

fn table_bounds(group: &resvg::usvg::Group, out: &mut Vec<resvg::usvg::Rect>) {
    use resvg::usvg::{Node, Paint};
    for node in group.children() {
        match node {
            Node::Group(group) => table_bounds(group, out),
            Node::Path(path)
                if path.fill().is_some_and(|fill| {
                    matches!(fill.paint(), Paint::Color(c)
                        if (c.red, c.green, c.blue) == (0xCC, 0xEE, 0xFF))
                }) =>
            {
                out.push(path.abs_bounding_box());
            }
            _ => {}
        }
    }
}
#[test]
fn vertical_docx_table_svg_stays_inside_the_header_footer_body_band() {
    for alignment in ["left", "center", "right"] {
        for vertical_alignment in ["top", "center", "bottom"] {
            let bytes = docx_vertical::build(alignment, vertical_alignment);
            let p = render_document_with_fonts(DocumentKind::Docx, &bytes, &FontOptions::default()).unwrap();
            assert_eq!(p.page_count(), 1);
            let info = p.page_info(0).unwrap();
            let PageData::Svg(svg) = p.render_page(0, PageFormat::Svg).unwrap().data else { panic!("SVG") };
            let text = hanji_package::xml::parse(svg.as_bytes()).unwrap().root.text_of(&["text", "tspan"]);
            assert!(text.contains("Tall header") && text.contains("Tall footer") && text.contains("Body table"));
            let tree = resvg::usvg::Tree::from_str(&svg, &Default::default()).unwrap();
            let mut bounds = Vec::new();
            table_bounds(tree.root(), &mut bounds);
            assert!(!bounds.is_empty(), "shaded table is present");
            let to_points = info.height / f64::from(tree.size().height());
            for rect in bounds {
                let (top, bottom) = (f64::from(rect.top()) * to_points, f64::from(rect.bottom()) * to_points);
                assert!(
                    top >= 155.0 && bottom <= 661.0,
                    "{alignment}/{vertical_alignment}: table y={top}..{bottom} must fit between 156 and 660 pt"
                );
            }
        }
    }
}

#[test]
fn docx_selected_story_pages_restore_continuation_room_and_preserve_body_text() {
    let mut horizontal_second_pages = Vec::new();
    for (vertical, overflow) in [(true, false), (false, false), (true, true)] {
        for first_height in [12, 120] {
            let bytes = docx_vertical::selected_story(first_height, vertical, overflow);
            let p = render_document_with_fonts(DocumentKind::Docx, &bytes, &FontOptions::default()).unwrap();
            assert_eq!(p.page_count(), if overflow { 7 } else { 2 });
            let mut body = String::new();
            for page in 0..p.page_count() {
                let PageData::Svg(svg) = p.render_page(page, PageFormat::Svg).unwrap().data else { panic!("SVG") };
                let text = hanji_package::xml::parse(svg.as_bytes()).unwrap().root.text_of(&["text", "tspan"]);
                assert!(text.contains(if page == 0 { "First header" } else { "Default header" }));
                body.push_str(&text);
                if !overflow {
                    assert!(text.contains(if page == 0 { "Page one body" } else { "Page two body" }));
                    assert!(text.contains(if page == 0 { "Page one table" } else { "Page two table" }));
                    let tree = resvg::usvg::Tree::from_str(&svg, &Default::default()).unwrap();
                    let mut bounds = Vec::new();
                    table_bounds(tree.root(), &mut bounds);
                    assert_eq!(bounds.len(), 1);
                    let to_points = p.page_info(page).unwrap().height / f64::from(tree.size().height());
                    let rect = bounds[0];
                    if vertical {
                        let top = if page == 0 && first_height == 120 { 156.0 } else { 72.0 };
                        assert!((f64::from(rect.top()) * to_points - top).abs() < 0.01);
                        assert!((f64::from(rect.bottom()) * to_points - 720.0).abs() < 0.01);
                    } else {
                        assert!((f64::from(rect.left()) * to_points - 72.0).abs() < 0.01);
                        assert!((f64::from(rect.right()) * to_points - 540.0).abs() < 0.01);
                        if page == 1 {
                            horizontal_second_pages.push(svg);
                        }
                    }
                }
            }
            if overflow {
                assert_eq!(body.matches("Word").count(), 4000);
            }
            assert!(!p.diagnostics().iter().any(|d| d.message.contains("largest active header/footer band")));
        }
    }
    assert_eq!(horizontal_second_pages[0], horizontal_second_pages[1]);
}

#[test]
fn fractional_landscape_docx_tables_use_selected_story_bands() {
    let bytes = docx_vertical::fractional_landscape();
    let p = render_document_with_fonts(DocumentKind::Docx, &bytes, &FontOptions::default()).unwrap();
    assert_eq!(p.page_count(), 2);
    let mut body = String::new();
    for page in 0..p.page_count() {
        let PageData::Svg(svg) = p.render_page(page, PageFormat::Svg).unwrap().data else { panic!("SVG") };
        let text = hanji_package::xml::parse(svg.as_bytes()).unwrap().root.text_of(&["text", "tspan"]);
        assert!(text.contains(if page == 0 { "First header" } else { "Default header" }));
        assert!(text.contains("Shared footer"));
        body.push_str(&text);
        let tree = resvg::usvg::Tree::from_str(&svg, &Default::default()).unwrap();
        let mut bounds = Vec::new();
        table_bounds(tree.root(), &mut bounds);
        assert_eq!(bounds.len(), 1);
        let to_points = p.page_info(page).unwrap().height / f64::from(tree.size().height());
        let top = if page == 0 { 36.0 + 300.05 } else { 36.0 + 400.05 };
        let bottom = 612.0 - 36.0 - 100.1;
        assert!((f64::from(bounds[0].top()) * to_points - top).abs() < 0.01);
        assert!((f64::from(bounds[0].bottom()) * to_points - bottom).abs() < 0.01);
    }
    for text in ["Page one body", "Page one table", "Page two body", "Page two table"] {
        assert_eq!(body.matches(text).count(), 1);
    }
}
