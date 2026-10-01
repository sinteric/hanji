// SPDX-License-Identifier: MIT OR Apache-2.0
// Copied from rdocx 0.14.0 (https://github.com/tensorbee/rdocx, crates.io `rdocx`), file src/svg.rs,
// lines 1-1056, by Atul Sharma and the rdocx contributors, licensed MIT OR Apache-2.0 (rdocx's Cargo.toml).
// The upstream file carries no header of its own; this header records its origin and licence.
// Only change: `render_page` is `pub` instead of `pub(crate)`, so this CLI can lower a layout it made
// with caller fonts (rdocx) or with rpptx's layout (same oxml-layout type) to SVG.
use std::collections::HashSet;
use std::fmt::Write as _;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use oxml_layout::{
    Color, Effect, FillRule, FontData, FontId, GlyphRun, GroupElement, LayoutResult, LineCap,
    LineJoin, Paint, Path, PathCommand, PathElement, PositionedElement, Rect, Stroke, Transform,
};

/// One stable diagnostic produced while lowering a page to SVG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvgDiagnostic {
    /// Location in the layout result that required a fallback or was omitted.
    pub path: String,
    /// Stable description of the lossy conversion.
    pub message: String,
}

/// One self-contained SVG page and its ordered diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvgRenderResult {
    /// Complete UTF-8 SVG document.
    pub svg: String,
    /// Layout diagnostics followed by SVG lowering diagnostics.
    pub diagnostics: Vec<SvgDiagnostic>,
}

pub fn render_page(layout: &LayoutResult, page_index: usize) -> Option<SvgRenderResult> {
    let page = layout.pages.get(page_index)?;
    let mut state = SvgState::new(
        layout,
        page.width,
        page.height,
        layout
            .diagnostics
            .iter()
            .enumerate()
            .map(|(index, diagnostic)| SvgDiagnostic {
                path: format!("layout.diagnostics[{index}]"),
                message: diagnostic.message.clone(),
            })
            .collect(),
    );

    let mut used_fonts = Vec::new();
    let mut seen_fonts = HashSet::new();
    collect_used_fonts(&page.elements, &mut seen_fonts, &mut used_fonts);
    state.emit_font_definitions(&used_fonts);

    let mut body = String::new();
    write!(
        body,
        "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" fill=\"#FFFFFF\"/>",
        number(page.width),
        number(page.height)
    )
    .unwrap();
    if let Some(background) = &page.background {
        let path = format!("pages[{page_index}].background");
        if let Some(attributes) = state.paint_attributes(background, "fill", &path) {
            write!(
                body,
                "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" {attributes}/>",
                number(page.width),
                number(page.height)
            )
            .unwrap();
        }
    }
    state.emit_elements(
        &page.elements,
        &format!("pages[{page_index}].elements"),
        &mut body,
        Transform::IDENTITY,
    );

    let mut svg = String::new();
    write!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}pt\" height=\"{}pt\" viewBox=\"0 0 {} {}\">",
        number(page.width),
        number(page.height),
        number(page.width),
        number(page.height)
    )
    .unwrap();
    if !state.defs.is_empty() {
        write!(svg, "<defs>{}</defs>", state.defs).unwrap();
    }
    svg.push_str(&body);
    svg.push_str("</svg>");

    Some(SvgRenderResult {
        svg,
        diagnostics: state.diagnostics,
    })
}

struct SvgState<'a> {
    layout: &'a LayoutResult,
    page_width: f64,
    page_height: f64,
    defs: String,
    diagnostics: Vec<SvgDiagnostic>,
    next_definition: usize,
    diagnosed_fonts: HashSet<FontId>,
}

impl<'a> SvgState<'a> {
    fn new(
        layout: &'a LayoutResult,
        page_width: f64,
        page_height: f64,
        diagnostics: Vec<SvgDiagnostic>,
    ) -> Self {
        Self {
            layout,
            page_width,
            page_height,
            defs: String::new(),
            diagnostics,
            next_definition: 0,
            diagnosed_fonts: HashSet::new(),
        }
    }

    fn definition_id(&mut self) -> String {
        let id = format!("rdocx-def-{}", self.next_definition);
        self.next_definition += 1;
        id
    }

    fn diagnose(&mut self, path: &str, message: &str) {
        self.diagnostics.push(SvgDiagnostic {
            path: path.to_owned(),
            message: message.to_owned(),
        });
    }

    fn emit_font_definitions(&mut self, used_fonts: &[FontId]) {
        if used_fonts.is_empty() {
            return;
        }

        let mut rules = String::new();
        for font_id in used_fonts {
            let Some(font) = self.layout.fonts.iter().find(|font| font.id == *font_id) else {
                continue;
            };
            let (mime, format) = font_content_type(font);
            write!(
                rules,
                "@font-face{{font-family:'rdocx-font-{}';src:url('data:{mime};base64,{}') format('{format}');font-weight:{};font-style:{}}}",
                font.id.0,
                BASE64.encode(font.data.as_ref()),
                if font.bold { "bold" } else { "normal" },
                if font.italic { "italic" } else { "normal" }
            )
            .unwrap();
        }
        if !rules.is_empty() {
            write!(self.defs, "<style>{rules}</style>").unwrap();
        }
    }

    fn emit_elements(
        &mut self,
        elements: &[PositionedElement],
        path: &str,
        output: &mut String,
        accumulated: Transform,
    ) {
        for (index, element) in elements.iter().enumerate() {
            let element_path = format!("{path}[{index}]");
            match element {
                PositionedElement::Text(run) => self.emit_text(run, &element_path, output),
                PositionedElement::MultilingualText(run) => {
                    if !run.is_valid() {
                        self.diagnose(
                            &element_path,
                            "invalid multilingual glyph positioning was omitted from SVG output",
                        );
                        continue;
                    }
                    let diagnostic_count = self.diagnostics.len();
                    self.emit_text(&run.legacy_projection(), &element_path, output);
                    if self.diagnostics.len() == diagnostic_count {
                        self.diagnose(
                            &element_path,
                            "multilingual shaping kept searchable text with browser-positioned glyph approximation",
                        );
                    }
                }
                PositionedElement::Line {
                    start,
                    end,
                    width,
                    color,
                    dash_pattern,
                } => {
                    write!(
                        output,
                        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-opacity=\"{}\" stroke-width=\"{}\"",
                        number(start.x),
                        number(start.y),
                        number(end.x),
                        number(end.y),
                        color_hex(*color),
                        number(color.a.clamp(0.0, 1.0)),
                        number(*width)
                    )
                    .unwrap();
                    if let Some((on, off)) = dash_pattern {
                        write!(
                            output,
                            " stroke-dasharray=\"{} {}\"",
                            number(*on),
                            number(*off)
                        )
                        .unwrap();
                    }
                    output.push_str("/>");
                }
                PositionedElement::FilledRect { rect, color } => {
                    write!(
                        output,
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" fill-opacity=\"{}\"/>",
                        number(rect.x),
                        number(rect.y),
                        number(rect.width),
                        number(rect.height),
                        color_hex(*color),
                        number(color.a.clamp(0.0, 1.0))
                    )
                    .unwrap();
                }
                PositionedElement::Image { rect, data, .. } => {
                    self.emit_image(*rect, data, &element_path, output)
                }
                PositionedElement::LinkAnnotation { rect, url } => {
                    self.emit_link(*rect, url, &element_path, output)
                }
                PositionedElement::Path(path_element) => {
                    self.emit_path_element(path_element, &element_path, output)
                }
                PositionedElement::Group(group) => {
                    self.emit_group(group, &element_path, output, accumulated)
                }
                PositionedElement::MarkedContent { children, .. } => {
                    output.push_str("<g>");
                    self.emit_elements(
                        children,
                        &format!("{element_path}.children"),
                        output,
                        accumulated,
                    );
                    output.push_str("</g>");
                }
                _ => self.diagnose(
                    &element_path,
                    "unsupported positioned element was omitted from SVG output",
                ),
            }
        }
    }

    fn emit_text(&mut self, run: &GlyphRun, path: &str, output: &mut String) {
        if run.text.is_empty() {
            return;
        }
        self.diagnose_font_on_first_use(run.font_id);

        write!(
            output,
            "<text xml:space=\"preserve\" y=\"{}\" font-family=\"rdocx-font-{}\" font-size=\"{}\" font-weight=\"{}\" font-style=\"{}\" fill=\"{}\" fill-opacity=\"{}\"",
            number(run.origin.y),
            run.font_id.0,
            number(run.font_size),
            if run.bold { "bold" } else { "normal" },
            if run.italic { "italic" } else { "normal" },
            color_hex(run.color),
            number(run.color.a.clamp(0.0, 1.0))
        )
        .unwrap();

        let scalar_count = run.text.chars().count();
        if scalar_count == run.glyph_ids.len() && run.advances.len() == run.glyph_ids.len() {
            let mut x = run.origin.x;
            output.push_str(" x=\"");
            for (index, advance) in run.advances.iter().enumerate() {
                if index != 0 {
                    output.push(' ');
                }
                output.push_str(&number(x));
                x += advance;
            }
            output.push('"');
        } else {
            let advance = run.advances.iter().copied().sum::<f64>().max(0.0);
            write!(
                output,
                " x=\"{}\" textLength=\"{}\" lengthAdjust=\"spacingAndGlyphs\"",
                number(run.origin.x),
                number(advance)
            )
            .unwrap();
            self.diagnose(
                path,
                "complex shaping kept searchable text with total-advance positioning",
            );
        }
        let (text, replaced_invalid_xml) = sanitize_xml_text(&run.text);
        if replaced_invalid_xml {
            self.diagnose(
                path,
                "XML-invalid text characters were replaced with U+FFFD",
            );
        }
        write!(output, ">{}</text>", escape_text(&text)).unwrap();
    }

    fn diagnose_font_on_first_use(&mut self, font_id: FontId) {
        if !self.diagnosed_fonts.insert(font_id) {
            return;
        }
        let Some(font) = self.layout.fonts.iter().find(|font| font.id == font_id) else {
            self.diagnose(
                &format!("fonts[{}]", font_id.0),
                "text references font data that is absent from the layout result",
            );
            return;
        };
        if font.face_index != 0 {
            self.diagnose(
                &format!("fonts[{}]", font.id.0),
                "font collection face index cannot be selected by SVG and uses the default face",
            );
        }
    }

    fn emit_image(&mut self, rect: Rect, data: &[u8], path: &str, output: &mut String) {
        let Some(mime) = image_mime(data) else {
            self.diagnose(
                path,
                "image bytes are neither PNG nor JPEG and were omitted",
            );
            return;
        };
        write!(
            output,
            "<image x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"none\" href=\"data:{mime};base64,{}\"/>",
            number(rect.x),
            number(rect.y),
            number(rect.width),
            number(rect.height),
            BASE64.encode(data)
        )
        .unwrap();
    }

    fn emit_link(&mut self, rect: Rect, url: &str, path: &str, output: &mut String) {
        if !is_safe_link(url) {
            self.diagnose(path, "active or unsupported link target was omitted");
            return;
        }
        write!(
            output,
            "<a href=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" pointer-events=\"all\"/></a>",
            escape_attribute(url),
            number(rect.x),
            number(rect.y),
            number(rect.width),
            number(rect.height)
        )
        .unwrap();
    }

    fn emit_path_element(&mut self, element: &PathElement, path: &str, output: &mut String) {
        write!(
            output,
            "<path d=\"{}\" fill-rule=\"{}\"",
            path_data(&element.path),
            fill_rule(element.path.fill_rule)
        )
        .unwrap();
        match &element.fill {
            Some(paint) => {
                if let Some(attributes) =
                    self.paint_attributes(paint, "fill", &format!("{path}.fill"))
                {
                    write!(output, " {attributes}").unwrap();
                } else {
                    output.push_str(" fill=\"none\"");
                }
            }
            None => output.push_str(" fill=\"none\""),
        }
        if let Some(stroke) = &element.stroke {
            self.emit_stroke_attributes(stroke, &format!("{path}.stroke"), output);
        }
        output.push_str("/>");
    }

    fn emit_stroke_attributes(&mut self, stroke: &Stroke, path: &str, output: &mut String) {
        if let Some(attributes) = self.paint_attributes(&stroke.paint, "stroke", path) {
            write!(
                output,
                " {attributes} stroke-width=\"{}\" stroke-linecap=\"{}\" stroke-linejoin=\"{}\"",
                number(stroke.width),
                line_cap(stroke.cap),
                line_join(stroke.join)
            )
            .unwrap();
            if let Some(dash) = &stroke.dash {
                output.push_str(" stroke-dasharray=\"");
                for (index, value) in dash.iter().enumerate() {
                    if index != 0 {
                        output.push(' ');
                    }
                    output.push_str(&number(*value));
                }
                output.push('"');
            }
        } else {
            output.push_str(" stroke=\"none\"");
        }
    }

    fn emit_group(
        &mut self,
        group: &GroupElement,
        path: &str,
        output: &mut String,
        accumulated: Transform,
    ) {
        let mut attributes = String::new();
        let group_to_page = group.transform.then(accumulated);
        if !group.transform.is_identity() {
            write!(
                attributes,
                " transform=\"matrix({} {} {} {} {} {})\"",
                number(group.transform.a),
                number(group.transform.b),
                number(group.transform.c),
                number(group.transform.d),
                number(group.transform.e),
                number(group.transform.f)
            )
            .unwrap();
        }
        let clip_id = if let Some(clip) = &group.clip {
            let id = self.definition_id();
            write!(
                self.defs,
                "<clipPath id=\"{id}\" clipPathUnits=\"userSpaceOnUse\"><path d=\"{}\" clip-rule=\"{}\"/></clipPath>",
                path_data(clip),
                fill_rule(clip.fill_rule)
            )
            .unwrap();
            Some(id)
        } else {
            None
        };
        if group.opacity < 1.0 {
            write!(
                attributes,
                " opacity=\"{}\"",
                number(group.opacity.clamp(0.0, 1.0))
            )
            .unwrap();
        }
        if !group.effects.is_empty() {
            let id = self.definition_id();
            let filter_region = group
                .effects
                .iter()
                .any(|effect| matches!(effect, Effect::OuterShadow { .. }))
                .then(|| self.filter_region(group, group_to_page));
            let mut filter = String::new();
            let mut shadows = Vec::new();
            for (index, effect) in group.effects.iter().enumerate() {
                match effect {
                    Effect::OuterShadow {
                        dx,
                        dy,
                        blur,
                        color,
                    } => {
                        let blur_id = format!("{id}-blur-{index}");
                        let offset_id = format!("{id}-offset-{index}");
                        let flood_id = format!("{id}-flood-{index}");
                        let shadow_id = format!("{id}-shadow-{index}");
                        write!(
                            filter,
                            "<feGaussianBlur in=\"SourceAlpha\" stdDeviation=\"{}\" result=\"{blur_id}\"/><feOffset in=\"{blur_id}\" dx=\"{}\" dy=\"{}\" result=\"{offset_id}\"/><feFlood flood-color=\"{}\" flood-opacity=\"{}\" result=\"{flood_id}\"/><feComposite in=\"{flood_id}\" in2=\"{offset_id}\" operator=\"in\" result=\"{shadow_id}\"/>",
                            number((*blur).max(0.0) / 2.0),
                            number(*dx),
                            number(*dy),
                            color_hex(*color),
                            number(color.a.clamp(0.0, 1.0))
                        )
                        .unwrap();
                        shadows.push(shadow_id);
                        if *blur > 0.0
                            && !preserves_isotropic_blur(group_to_page)
                            && matches!(filter_region, Some(Ok(Some(_))))
                        {
                            self.diagnose(
                                &format!("{path}.effects[{index}]"),
                                "non-uniform or skewed transform makes SVG shadow blur anisotropic instead of the raster backend's average-scale isotropic blur",
                            );
                        }
                    }
                    _ => self.diagnose(
                        &format!("{path}.effects[{index}]"),
                        "unsupported group effect was omitted while its children were preserved",
                    ),
                }
            }
            if !filter.is_empty() {
                filter.push_str("<feMerge>");
                for shadow in shadows {
                    write!(filter, "<feMergeNode in=\"{shadow}\"/>").unwrap();
                }
                filter.push_str("<feMergeNode in=\"SourceGraphic\"/></feMerge>");
                match filter_region {
                    Some(Ok(Some(region))) => {
                        write!(
                            self.defs,
                            "<filter id=\"{id}\" filterUnits=\"userSpaceOnUse\" primitiveUnits=\"userSpaceOnUse\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" color-interpolation-filters=\"sRGB\">{filter}</filter>",
                            number(region.x),
                            number(region.y),
                            number(region.width),
                            number(region.height)
                        )
                        .unwrap();
                        write!(attributes, " filter=\"url(#{id})\"").unwrap();
                    }
                    Some(Ok(None)) | None => {}
                    Some(Err(())) => self.diagnose(
                        path,
                        "group effects were omitted because singular-transform source bounds could not be proven",
                    ),
                }
            }
        }

        write!(output, "<g{attributes}>").unwrap();
        if let Some(clip_id) = clip_id {
            write!(output, "<g clip-path=\"url(#{clip_id})\">").unwrap();
        }
        self.emit_elements(
            &group.children,
            &format!("{path}.children"),
            output,
            group_to_page,
        );
        if group.clip.is_some() {
            output.push_str("</g>");
        }
        output.push_str("</g>");
    }

    fn filter_region(
        &self,
        group: &GroupElement,
        group_to_page: Transform,
    ) -> Result<Option<Rect>, ()> {
        if let Some(region) = inverse_transform(group_to_page)
            .map(|page_to_group| {
                page_to_group.transform_rect_bbox(Rect {
                    x: 0.0,
                    y: 0.0,
                    width: self.page_width,
                    height: self.page_height,
                })
            })
            .filter(rect_is_finite)
        {
            return Ok(Some(region));
        }
        group_effect_bounds(group)
    }

    fn paint_attributes(&mut self, paint: &Paint, property: &str, path: &str) -> Option<String> {
        match paint {
            Paint::Solid(color) => Some(format!(
                "{property}=\"{}\" {property}-opacity=\"{}\"",
                color_hex(*color),
                number(color.a.clamp(0.0, 1.0))
            )),
            Paint::Linear {
                start,
                end,
                stops,
                extend,
            } => {
                self.diagnose_gradient_extension(*extend, path);
                let id = self.definition_id();
                let mut definition = format!(
                    "<linearGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\">",
                    number(start.x),
                    number(start.y),
                    number(end.x),
                    number(end.y)
                );
                emit_stops(stops, &mut definition);
                definition.push_str("</linearGradient>");
                self.defs.push_str(&definition);
                Some(format!("{property}=\"url(#{id})\""))
            }
            Paint::Radial {
                center,
                radius,
                focal,
                stops,
                extend,
            } => {
                self.diagnose_gradient_extension(*extend, path);
                let id = self.definition_id();
                let mut definition = format!(
                    "<radialGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" cx=\"{}\" cy=\"{}\" r=\"{}\" fx=\"{}\" fy=\"{}\">",
                    number(center.x),
                    number(center.y),
                    number(*radius),
                    number(focal.x),
                    number(focal.y)
                );
                emit_stops(stops, &mut definition);
                definition.push_str("</radialGradient>");
                self.defs.push_str(&definition);
                Some(format!("{property}=\"url(#{id})\""))
            }
            Paint::Tile { .. } => {
                self.diagnose(
                    path,
                    "tile paint was omitted because its carrier has no media bytes",
                );
                None
            }
        }
    }

    fn diagnose_gradient_extension(&mut self, extend: (bool, bool), path: &str) {
        if extend != (true, true) {
            self.diagnose(
                path,
                "asymmetric or disabled gradient extension uses SVG pad extension",
            );
        }
    }
}

fn collect_used_fonts(
    elements: &[PositionedElement],
    seen: &mut HashSet<FontId>,
    ordered: &mut Vec<FontId>,
) {
    for element in elements {
        match element {
            PositionedElement::Text(run) if !run.text.is_empty() => {
                if seen.insert(run.font_id) {
                    ordered.push(run.font_id);
                }
            }
            PositionedElement::MultilingualText(run) if !run.logical_text.is_empty() => {
                if seen.insert(run.font_id) {
                    ordered.push(run.font_id);
                }
            }
            PositionedElement::Group(group) => collect_used_fonts(&group.children, seen, ordered),
            PositionedElement::MarkedContent { children, .. } => {
                collect_used_fonts(children, seen, ordered)
            }
            _ => {}
        }
    }
}

fn inverse_transform(transform: Transform) -> Option<Transform> {
    let determinant = transform.a * transform.d - transform.b * transform.c;
    if !determinant.is_finite() || determinant == 0.0 {
        return None;
    }
    let inverse = Transform {
        a: transform.d / determinant,
        b: -transform.b / determinant,
        c: -transform.c / determinant,
        d: transform.a / determinant,
        e: (transform.c * transform.f - transform.d * transform.e) / determinant,
        f: (transform.b * transform.e - transform.a * transform.f) / determinant,
    };
    [
        inverse.a, inverse.b, inverse.c, inverse.d, inverse.e, inverse.f,
    ]
    .iter()
    .all(|value| value.is_finite())
    .then_some(inverse)
}

fn preserves_isotropic_blur(transform: Transform) -> bool {
    let x_scale_squared = transform.a * transform.a + transform.b * transform.b;
    let y_scale_squared = transform.c * transform.c + transform.d * transform.d;
    let dot = transform.a * transform.c + transform.b * transform.d;
    if !x_scale_squared.is_finite() || !y_scale_squared.is_finite() || !dot.is_finite() {
        return false;
    }
    let tolerance = x_scale_squared.max(y_scale_squared).max(1.0) * 1e-12;
    (x_scale_squared - y_scale_squared).abs() <= tolerance && dot.abs() <= tolerance
}

fn group_effect_bounds(group: &GroupElement) -> Result<Option<Rect>, ()> {
    let Some(mut source) = elements_bounds(&group.children)? else {
        return Ok(None);
    };
    if let Some(clip) = &group.clip {
        let Some(clip_bounds) = clip.bounds() else {
            return Ok(None);
        };
        let Some(clipped) = intersect_rect(source, clip_bounds) else {
            return Ok(None);
        };
        source = clipped;
    }
    let mut bounds = source;
    for effect in &group.effects {
        if let Effect::OuterShadow { dx, dy, blur, .. } = effect {
            let margin = blur.max(0.0) * 2.0 + 1.0;
            let shadow = Rect {
                x: source.x + dx - margin,
                y: source.y + dy - margin,
                width: source.width + margin * 2.0,
                height: source.height + margin * 2.0,
            };
            bounds = union_rect(bounds, shadow);
        }
    }
    rect_is_finite(&bounds).then_some(Some(bounds)).ok_or(())
}

fn elements_bounds(elements: &[PositionedElement]) -> Result<Option<Rect>, ()> {
    let mut bounds = None;
    for element in elements {
        if let Some(element) = element_bounds(element)? {
            bounds = Some(match bounds {
                Some(bounds) => union_rect(bounds, element),
                None => element,
            });
        }
    }
    Ok(bounds)
}

fn element_bounds(element: &PositionedElement) -> Result<Option<Rect>, ()> {
    match element {
        PositionedElement::Text(run) if !run.text.is_empty() => Err(()),
        PositionedElement::MultilingualText(run) if !run.logical_text.is_empty() => Err(()),
        PositionedElement::Line {
            start, end, width, ..
        } => {
            let margin = width.abs() / 2.0 + 1.0;
            Ok(Some(Rect {
                x: start.x.min(end.x) - margin,
                y: start.y.min(end.y) - margin,
                width: (start.x - end.x).abs() + margin * 2.0,
                height: (start.y - end.y).abs() + margin * 2.0,
            }))
        }
        PositionedElement::FilledRect { rect, .. } | PositionedElement::Image { rect, .. } => {
            Ok(Some(normal_rect(*rect)))
        }
        PositionedElement::Path(element) => Ok(element.path.bounds().map(|bounds| {
            let margin = element
                .stroke
                .as_ref()
                .map(|stroke| match stroke.join {
                    LineJoin::Miter => stroke.width.abs() * 2.0 + 1.0,
                    LineJoin::Round | LineJoin::Bevel => stroke.width.abs() / 2.0 + 1.0,
                })
                .unwrap_or(0.0);
            Rect {
                x: bounds.x - margin,
                y: bounds.y - margin,
                width: bounds.width + margin * 2.0,
                height: bounds.height + margin * 2.0,
            }
        })),
        PositionedElement::Group(group) => {
            Ok(group_effect_bounds(group)?
                .map(|bounds| group.transform.transform_rect_bbox(bounds)))
        }
        PositionedElement::MarkedContent { children, .. } => elements_bounds(children),
        PositionedElement::LinkAnnotation { .. }
        | PositionedElement::Text(_)
        | PositionedElement::MultilingualText(_) => Ok(None),
        _ => Err(()),
    }
}

fn normal_rect(rect: Rect) -> Rect {
    Rect {
        x: rect.x.min(rect.x + rect.width),
        y: rect.y.min(rect.y + rect.height),
        width: rect.width.abs(),
        height: rect.height.abs(),
    }
}

fn intersect_rect(left: Rect, right: Rect) -> Option<Rect> {
    let left = normal_rect(left);
    let right = normal_rect(right);
    let x = left.x.max(right.x);
    let y = left.y.max(right.y);
    let right_edge = (left.x + left.width).min(right.x + right.width);
    let bottom = (left.y + left.height).min(right.y + right.height);
    (right_edge >= x && bottom >= y).then_some(Rect {
        x,
        y,
        width: right_edge - x,
        height: bottom - y,
    })
}

fn union_rect(left: Rect, right: Rect) -> Rect {
    let left = normal_rect(left);
    let right = normal_rect(right);
    let x = left.x.min(right.x);
    let y = left.y.min(right.y);
    let right_edge = (left.x + left.width).max(right.x + right.width);
    let bottom = (left.y + left.height).max(right.y + right.height);
    Rect {
        x,
        y,
        width: right_edge - x,
        height: bottom - y,
    }
}

fn rect_is_finite(rect: &Rect) -> bool {
    [rect.x, rect.y, rect.width, rect.height]
        .iter()
        .all(|value| value.is_finite())
}

fn emit_stops(stops: &[oxml_layout::GradientStop], output: &mut String) {
    let mut normalized = stops
        .iter()
        .map(|stop| oxml_layout::GradientStop {
            offset: if stop.offset.is_nan() {
                0.0
            } else {
                stop.offset.clamp(0.0, 1.0)
            },
            color: stop.color,
        })
        .collect::<Vec<_>>();
    normalized.sort_by(|left, right| left.offset.total_cmp(&right.offset));
    let mut deduplicated: Vec<oxml_layout::GradientStop> = Vec::with_capacity(normalized.len());
    for stop in normalized {
        if let Some(previous) = deduplicated.last_mut()
            && previous.offset == stop.offset
        {
            *previous = stop;
        } else {
            deduplicated.push(stop);
        }
    }

    for stop in &deduplicated {
        write!(
            output,
            "<stop offset=\"{}\" stop-color=\"{}\" stop-opacity=\"{}\"/>",
            number(stop.offset),
            color_hex(stop.color),
            number(stop.color.a.clamp(0.0, 1.0))
        )
        .unwrap();
    }
}

fn path_data(path: &Path) -> String {
    let mut data = String::new();
    for command in &path.commands {
        if !data.is_empty() {
            data.push(' ');
        }
        match command {
            PathCommand::MoveTo(point) => {
                write!(data, "M{} {}", number(point.x), number(point.y)).unwrap()
            }
            PathCommand::LineTo(point) => {
                write!(data, "L{} {}", number(point.x), number(point.y)).unwrap()
            }
            PathCommand::CurveTo { c1, c2, to } => write!(
                data,
                "C{} {} {} {} {} {}",
                number(c1.x),
                number(c1.y),
                number(c2.x),
                number(c2.y),
                number(to.x),
                number(to.y)
            )
            .unwrap(),
            PathCommand::Close => data.push('Z'),
        }
    }
    data
}

fn number(value: f64) -> String {
    if value == 0.0 {
        return "0".to_owned();
    }
    let mut value = format!("{value:.6}");
    while value.contains('.') && value.ends_with('0') {
        value.pop();
    }
    if value.ends_with('.') {
        value.pop();
    }
    value
}

fn color_hex(color: Color) -> String {
    let channel = |value: f64| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!(
        "#{:02X}{:02X}{:02X}",
        channel(color.r),
        channel(color.g),
        channel(color.b)
    )
}

fn line_cap(cap: LineCap) -> &'static str {
    match cap {
        LineCap::Butt => "butt",
        LineCap::Round => "round",
        LineCap::Square => "square",
    }
}

fn line_join(join: LineJoin) -> &'static str {
    match join {
        LineJoin::Miter => "miter",
        LineJoin::Round => "round",
        LineJoin::Bevel => "bevel",
    }
}

fn fill_rule(rule: FillRule) -> &'static str {
    match rule {
        FillRule::NonZero => "nonzero",
        FillRule::EvenOdd => "evenodd",
    }
}

fn font_content_type(font: &FontData) -> (&'static str, &'static str) {
    if font.data.starts_with(b"ttcf") {
        ("font/collection", "collection")
    } else if font.data.starts_with(b"OTTO") {
        ("font/otf", "opentype")
    } else if font.data.starts_with(b"wOFF") {
        ("font/woff", "woff")
    } else if font.data.starts_with(b"wOF2") {
        ("font/woff2", "woff2")
    } else {
        ("font/ttf", "truetype")
    }
}

fn image_mime(data: &[u8]) -> Option<&'static str> {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else {
        None
    }
}

fn is_safe_link(target: &str) -> bool {
    if target.is_empty()
        || target
            .chars()
            .any(|character| character.is_control() || !is_xml_character(character))
    {
        return false;
    }
    let target = target.trim();
    if target.is_empty() {
        return false;
    }
    let Some(colon) = target.find(':') else {
        return true;
    };
    if target[..colon]
        .chars()
        .any(|character| matches!(character, '/' | '?' | '#'))
    {
        return true;
    }
    let scheme = &target[..colon];
    if scheme.is_empty()
        || !scheme.chars().enumerate().all(|(index, character)| {
            character.is_ascii_alphabetic()
                || (index > 0
                    && (character.is_ascii_digit()
                        || character == '+'
                        || character == '-'
                        || character == '.'))
        })
    {
        return false;
    }
    matches!(
        scheme.to_ascii_lowercase().as_str(),
        "http" | "https" | "mailto"
    )
}

fn escape_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn sanitize_xml_text(value: &str) -> (String, bool) {
    let mut replaced = false;
    let text = value
        .chars()
        .map(|character| {
            if is_xml_character(character) {
                character
            } else {
                replaced = true;
                '\u{FFFD}'
            }
        })
        .collect();
    (text, replaced)
}

fn is_xml_character(character: char) -> bool {
    matches!(character, '\u{9}' | '\u{A}' | '\u{D}')
        || matches!(character as u32, 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF)
}

fn escape_attribute(value: &str) -> String {
    escape_text(value)
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

