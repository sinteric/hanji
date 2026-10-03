//! Experimental DOCX pages through the shared layout, font and output pipeline.

use std::collections::{BTreeMap, HashMap};

use hanji_package::{package, xml};
use oxml_layout::{FontFile, FontId};

use crate::fonts::{FontResolver, Fonts, Metrics, Script, Source};
use crate::{finish_preview, sfnt, FaceInfo, FontOptions, FontsReport, Preview};

/// DOCX preview with caller font bytes and deterministic bundled fallbacks.
/// Layout uses the accepted revision view. No document is edited or saved.
pub fn render_with_fonts(bytes: &[u8], options: &FontOptions) -> Result<Preview, String> {
    let document = rdocx::Document::from_bytes(bytes).map_err(|e| format!("rdocx cannot open the document: {e}"))?;
    let embedded = embedded_fonts(&document);
    let fonts = Fonts::from_bytes(&options.fonts, &embedded, options.aliases.clone())?;
    render(&document, bytes, &fonts)
}

/// DOCX preview using the optional native font-directory/system adapter.
#[cfg(all(feature = "host-fonts", not(target_family = "wasm")))]
pub fn render_native(bytes: &[u8], options: &crate::Options) -> Result<Preview, String> {
    let document = rdocx::Document::from_bytes(bytes).map_err(|e| format!("rdocx cannot open the document: {e}"))?;
    let embedded = embedded_fonts(&document);
    let fonts = Fonts::load(&options.font_dirs, &embedded, options.system_fonts);
    render(&document, bytes, &fonts)
}

fn embedded_fonts(document: &rdocx::Document) -> Vec<FontFile> {
    document
        .fonts()
        .into_iter()
        .flat_map(|font| {
            font.embedded_fonts
                .into_iter()
                .filter(|f| !f.data.is_empty())
                .map(move |f| FontFile { family: font.name.clone(), data: f.data })
        })
        .collect()
}

fn requested_fonts(bytes: &[u8]) -> Result<BTreeMap<String, Script>, String> {
    let mut requested = BTreeMap::from([
        ("Arial".to_string(), Script::Latin),
        ("Calibri".to_string(), Script::Latin),
        ("Times New Roman".to_string(), Script::Latin),
    ]);
    let mut observed = BTreeMap::new();
    let mut has_hangul = false;
    for part in package::read(bytes)? {
        if !part.name.starts_with("word/") || !part.name.ends_with(".xml") {
            continue;
        }
        let tree = xml::parse(&part.data).map_err(|e| format!("{}: {e}", part.name))?;
        tree.root.walk(&mut |element| {
            if element.local() == "r" {
                let mut text = String::new();
                element.walk(&mut |child| {
                    if child.local() == "t" {
                        for node in &child.children {
                            if let xml::Node::Text(value) = node {
                                text.push_str(&xml::unescape(value));
                            }
                        }
                    }
                });
                has_hangul |= text.chars().any(crate::fonts::is_hangul);
                if !text.is_empty() {
                    let script = Script::of(&text);
                    element.walk(&mut |child| {
                        if child.local() != "rFonts" {
                            return;
                        }
                        for (name, value) in &child.attrs {
                            let active = match xml::local_name(name) {
                                "eastAsia" => script != Script::Latin,
                                _ => false,
                            };
                            let family = xml::unescape(value);
                            if active && !family.is_empty() {
                                observed
                                    .entry(family)
                                    .and_modify(|old| *old = prefer_script(*old, script))
                                    .or_insert(script);
                            }
                        }
                    });
                }
            }
            if element.local() == "rFonts" {
                for (name, value) in &element.attrs {
                    let script = match xml::local_name(name) {
                        "ascii" | "hAnsi" | "cs" => Script::Latin,
                        "eastAsia" => Script::Hangul,
                        _ => continue,
                    };
                    let family = xml::unescape(value);
                    if !family.is_empty() {
                        requested
                            .entry(family)
                            .and_modify(|s| {
                                if script == Script::Hangul {
                                    *s = script;
                                }
                            })
                            .or_insert(script);
                    }
                }
            } else if matches!(element.local(), "latin" | "ea" | "cs") {
                if let Some(family) = element.get("typeface").filter(|f| !f.is_empty()) {
                    requested.entry(family).or_insert(if element.local() == "ea" {
                        Script::Hangul
                    } else {
                        Script::Latin
                    });
                }
            }
        });
    }
    // East Asian style declarations do not imply Korean text. Prefer the
    // actual run script when a family is explicit, then the document script
    // for inherited East Asian styles.
    if !has_hangul {
        for script in requested.values_mut() {
            if *script == Script::Hangul {
                *script = Script::Cjk;
            }
        }
    }
    requested.extend(observed);
    Ok(requested)
}

fn prefer_script(left: Script, right: Script) -> Script {
    match (left, right) {
        (Script::Hangul, _) | (_, Script::Hangul) => Script::Hangul,
        (Script::Cjk, _) | (_, Script::Cjk) => Script::Cjk,
        _ => Script::Latin,
    }
}

fn render(document: &rdocx::Document, bytes: &[u8], resolver: &dyn FontResolver) -> Result<Preview, String> {
    let mut files = Vec::new();
    let mut infos = Vec::new();
    let mut warnings = resolver.warnings().to_vec();
    warnings.push("DOCX preview is experimental: pagination and Word-specific layout may differ from Word; inspect rendering diagnostics.".into());
    for (requested, script) in requested_fonts(bytes)? {
        let mut physical_styles = std::collections::BTreeSet::new();
        for (bold, italic) in [(false, false), (true, false), (false, true), (true, true)] {
            let face = resolver.resolve_font(&requested, script, bold, italic);
            let draws_script = face.is_some();
            let Some(face) = face.or_else(|| resolver.resolve_font(&requested, Script::Latin, bold, italic)) else {
                continue;
            };
            let parsed = ttf_parser::Face::parse(&face.font.data, face.font.face_index)
                .map_err(|e| format!("font {requested}: {e}"))?;
            let style = (parsed.is_bold(), parsed.is_italic());
            if !physical_styles.insert(style) {
                continue;
            }
            let data = sfnt::build(
                &face.font.data,
                face.font.face_index,
                &sfnt::Adjust {
                    family: requested.clone(),
                    bold: style.0,
                    italic: style.1,
                    line_em: None,
                    ea_advance: face.ea_advance.filter(|_| draws_script),
                },
            )?;
            files.push(FontFile { family: requested.clone(), data });
            infos.push(FaceInfo {
                requested: requested.clone(),
                script,
                drawn: draws_script.then_some(face.family),
                source: draws_script.then_some(face.source),
                metrics: if draws_script { face.metrics } else { Metrics::Substitute },
            });
        }
    }
    let refs: Vec<_> = files.iter().map(|f| (f.family.as_str(), f.data.as_slice())).collect();
    let layout = document
        .layout_with_fonts_and_bundled_fallback(&refs)
        .map_err(|e| format!("rdocx cannot lay out the document: {e}"))?
        .into_layout_result();
    let faces: HashMap<FontId, FaceInfo> = layout
        .fonts
        .iter()
        .map(|font| {
            let info = files
                .iter()
                .zip(&infos)
                .find(|(file, _)| file.family == font.family && file.data.as_slice() == font.data.as_ref())
                .map(|(_, info)| info.clone())
                .unwrap_or_else(|| FaceInfo {
                    requested: font.family.clone(),
                    script: Script::Latin,
                    drawn: Some(font.family.clone()),
                    source: Some(Source::Bundled),
                    metrics: Metrics::Substitute,
                });
            (font.id, info)
        })
        .collect();
    if faces.values().any(|f| f.script == Script::Hangul && f.drawn.is_none()) {
        warnings.push(crate::NO_KOREAN_FONT.into());
    }
    finish_preview(Preview {
        layout,
        faces,
        chars: vec![],
        page_subsets: vec![],
        document_subsets: vec![],
        fonts: FontsReport::default(),
        warnings,
        diagnostics: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn japanese_runs_do_not_request_a_hangul_font() {
        let bytes = include_bytes!("../../../prototype/preview/baseline/04-docx-untouched-testword-various.docx");
        let requested = requested_fonts(bytes).unwrap();
        assert_eq!(requested.get("MS Mincho"), Some(&Script::Cjk));
        assert_eq!(requested.get("Arial"), Some(&Script::Latin));
    }
}
