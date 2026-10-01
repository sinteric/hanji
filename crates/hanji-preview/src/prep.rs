//! Engine-compat transforms on the preview's copy of the package. They work
//! around rpptx 0.12.1 and do not change what PowerPoint draws; the copy is
//! rendered and dropped, never exported or stored (DESIGN.md §2 rule 4: the
//! input is the exact package export writes).
//!
//! 1. **The theme's empty East Asian font is filled from its Hangul face.**
//!    A theme may leave `a:ea typeface=""` and name the Hangul face in
//!    `a:font script="Hang"`; PowerPoint draws Korean text in that face, and
//!    rpptx reads only `a:ea`. The face is the one hanji-pptx reads
//!    (`ThemeFonts`, the same logic as the text model's `font=`).
//! 2. **Runs are split where the script changes** between East Asian and
//!    other characters (rpptx's own test for its typeface choice). PowerPoint
//!    picks the Latin or East Asian font per character; rpptx picks one per
//!    run, so a run mixing Hangul and Latin was drawn all in one face. Both
//!    halves keep the run's properties; a space stays with the text before it.
//! 3. **Animations (`p:timing`) are dropped.** They do not change the static
//!    slide, and rpptx refuses some schema-valid ones (`duplicate
//!    p:attrName`, deck 12 of the preview spike).

use hanji_core::Part;
use hanji_package::xml::{self, Element, Node};
use hanji_pptx::text::ThemeFonts;

/// Whether rpptx draws `c` with a run's East Asian typeface
/// (`rpptx-render` 0.12.1, `is_east_asian_character`).
pub fn is_east_asian(c: char) -> bool {
    matches!(
        c as u32,
        0x1100..=0x11FF
            | 0x2E80..=0xA4CF
            | 0xAC00..=0xD7AF
            | 0xF900..=0xFAFF
            | 0xFE30..=0xFE4F
            | 0xFF00..=0xFFEF
            | 0x20000..=0x323AF
    )
}

/// The package with the transforms applied.
pub fn prepare(package: &[u8]) -> Result<Vec<u8>, String> {
    let mut parts = hanji_package::package::read(package)?;
    for p in &mut parts {
        if let Some(new) = prepare_part(p)? {
            p.data = new;
        }
    }
    hanji_package::package::write(&parts)
}

fn is_xml_in(name: &str, dir: &str) -> bool {
    name.strip_prefix(dir).is_some_and(|rest| !rest.contains('/') && rest.ends_with(".xml"))
}

fn prepare_part(p: &Part) -> Result<Option<Vec<u8>>, String> {
    let theme = is_xml_in(&p.name, "ppt/theme/");
    let drawn = ["ppt/slides/", "ppt/slideLayouts/", "ppt/slideMasters/"].iter().any(|d| is_xml_in(&p.name, d));
    if !theme && !drawn {
        return Ok(None);
    }
    let mut doc = xml::parse(&p.data).map_err(|e| format!("{}: {e}", p.name))?;
    let changed =
        if theme { fill_theme_ea(&mut doc.root) } else { split_runs(&mut doc.root) | drop_timing(&mut doc.root) };
    Ok(changed.then(|| xml::write_doc(&doc)))
}

/// Transform 1: an empty `a:ea` of the major or minor font takes that
/// group's `a:font script="Hang"`.
pub fn fill_theme_ea(theme: &mut Element) -> bool {
    let fonts = ThemeFonts::of(theme);
    let mut changed = false;
    theme.walk_mut(&mut |e| {
        if !e.is("a:fontScheme") {
            return;
        }
        for (group, hang) in [("a:majorFont", &fonts.major_hang), ("a:minorFont", &fonts.minor_hang)] {
            let (Some(hang), Some(g)) = (hang, e.child_mut(group)) else { continue };
            if let Some(ea) = g.child_mut("a:ea").filter(|ea| ea.get("typeface").unwrap_or_default().is_empty()) {
                ea.set("typeface", hang);
                changed = true;
            }
        }
    });
    changed
}

/// Transform 3: the slide's (layout's, master's) `p:timing`.
pub fn drop_timing(root: &mut Element) -> bool {
    xml::remove_child(root, "p:timing").is_some()
}

/// Transform 2: every `a:r` whose text mixes East Asian and other
/// characters becomes one run per stretch.
pub fn split_runs(root: &mut Element) -> bool {
    let mut changed = false;
    root.walk_mut(&mut |e| {
        if !e.children.iter().any(|n| matches!(n, Node::El(r) if r.is("a:r"))) {
            return;
        }
        let mut out = Vec::with_capacity(e.children.len());
        for n in std::mem::take(&mut e.children) {
            match n {
                Node::El(r) if r.is("a:r") => match split_run(&r) {
                    Some(runs) => {
                        changed = true;
                        out.extend(runs.into_iter().map(Node::El));
                    }
                    None => out.push(Node::El(r)),
                },
                n => out.push(n),
            }
        }
        e.children = out;
    });
    changed
}

/// The pieces of one run, or `None` when it is in one script.
fn split_run(r: &Element) -> Option<Vec<Element>> {
    let t = r.child("a:t")?;
    let text = xml::unescape(
        &t.children.iter().map(|n| if let Node::Text(s) = n { s.as_str() } else { "" }).collect::<String>(),
    );
    let pieces = script_pieces(&text);
    if pieces.len() < 2 {
        return None;
    }
    Some(
        pieces
            .into_iter()
            .map(|piece| {
                let mut run = r.clone();
                for n in &mut run.children {
                    if let Node::El(c) = n {
                        if c.is("a:t") {
                            c.children = vec![Node::Text(xml::escape_text(piece))];
                        }
                    }
                }
                run
            })
            .collect(),
    )
}

/// `text` cut where it changes between East Asian and other characters; a
/// space joins the stretch before it.
pub fn script_pieces(text: &str) -> Vec<&str> {
    let mut pieces = vec![];
    let (mut start, mut current) = (0, None);
    for (i, c) in text.char_indices() {
        let ea = if c == ' ' { current.unwrap_or(false) } else { is_east_asian(c) };
        match current {
            Some(k) if k != ea => {
                pieces.push(&text[start..i]);
                start = i;
            }
            _ => {}
        }
        current = Some(ea);
    }
    if start < text.len() {
        pieces.push(&text[start..]);
    }
    pieces
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(s: &str) -> Element {
        xml::parse(s.as_bytes()).unwrap().root
    }

    #[test]
    fn mixed_runs_are_split_at_the_script_boundary() {
        assert_eq!(script_pieces("매출 Q3 실적"), vec!["매출 ", "Q3 ", "실적"]);
        assert_eq!(script_pieces("KPI 달성"), vec!["KPI ", "달성"]);
        assert_eq!(script_pieces("전부 한글"), vec!["전부 한글"]);
        assert_eq!(script_pieces(""), Vec::<&str>::new());
        let mut p = root(
            r#"<a:p xmlns:a="a"><a:r><a:rPr lang="ko-KR" b="1"/><a:t>매출 &amp; Q3</a:t></a:r><a:r><a:t>plain</a:t></a:r></a:p>"#,
        );
        assert!(split_runs(&mut p));
        assert_eq!(
            p.to_xml(),
            r#"<a:p xmlns:a="a"><a:r><a:rPr lang="ko-KR" b="1"/><a:t>매출 </a:t></a:r><a:r><a:rPr lang="ko-KR" b="1"/><a:t>&amp; Q3</a:t></a:r><a:r><a:t>plain</a:t></a:r></a:p>"#
        );
        assert!(!split_runs(&mut p));
    }

    #[test]
    fn the_empty_theme_ea_takes_the_hangul_face() {
        let mut t = root(
            r#"<a:theme xmlns:a="a"><a:themeElements><a:fontScheme name="x"><a:majorFont><a:latin typeface="Calibri Light"/><a:ea typeface=""/><a:cs typeface=""/><a:font script="Hang" typeface="맑은 고딕"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/><a:ea typeface="굴림"/><a:font script="Hang" typeface="맑은 고딕"/></a:minorFont></a:fontScheme></a:themeElements></a:theme>"#,
        );
        assert!(fill_theme_ea(&mut t));
        let s = t.to_xml();
        assert!(s.contains(r#"<a:ea typeface="맑은 고딕"/><a:cs typeface=""/>"#), "{s}");
        assert!(s.contains(r#"<a:ea typeface="굴림"/>"#), "an author's East Asian font stays: {s}");
        assert!(!fill_theme_ea(&mut t));
    }

    #[test]
    fn timing_is_dropped() {
        let mut s = root(r#"<p:sld xmlns:p="p"><p:cSld/><p:timing><p:tnLst/></p:timing></p:sld>"#);
        assert!(drop_timing(&mut s));
        assert_eq!(s.to_xml(), r#"<p:sld xmlns:p="p"><p:cSld/></p:sld>"#);
        assert!(!drop_timing(&mut s));
    }

    #[test]
    fn a_package_is_prepared_and_the_original_is_untouched() {
        let deck =
            std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../hanji-pptx/corpus/korean-deck.pptx")).unwrap();
        let out = prepare(&deck).unwrap();
        let parts = hanji_package::package::read(&out).unwrap();
        let before = hanji_package::package::read(&deck).unwrap();
        assert_eq!(parts.len(), before.len());
        for p in &parts {
            if p.name.starts_with("ppt/slides/slide") {
                assert!(!String::from_utf8_lossy(&p.data).contains("<p:timing"), "{}", p.name);
            }
        }
    }
}
