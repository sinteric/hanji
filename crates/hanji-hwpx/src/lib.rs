//! The hwpx engine (OWPML, KS X 6101): splits `Contents/section*.xml` into
//! model text and remainder entries at the XML level, and puts them back.
//! Every other part is copied through byte for byte, except `header.xml`
//! when the text needs a shape, border fill, font or style the file does
//! not have yet, or its style section changed a style (§5.2). No I/O:
//! package bytes in, package bytes out.
//!
//! rhwp is not on the import/export path: it parses into its own document
//! model and writes the package from it, which drops what that model does
//! not hold (tracked changes, without a loss report). The remainder needs
//! the section XML as written. rhwp validates exports in the tests.

pub mod export;
pub mod format;
pub mod header;
pub mod import;
pub mod owpml;
pub mod safety;

use std::collections::{BTreeSet, HashMap};

use hanji_core::{Block, Capabilities, Engine, EngineError, ImportOptions, ImportReport, Imported, Kind, Remainder};
use hanji_format::{self as fmt, FrontMatter};
use serde::{Deserialize, Serialize};

pub use hanji_package::{package, xml};

use crate::header::Header;
use crate::owpml::{section_number, HEADER_PART};

pub struct HwpxEngine;

/// One section part around its paragraphs, stored once.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SectionShell {
    pub part: String,
    pub prolog: String,
    /// `<hs:sec …>` start tag.
    pub root_open: String,
    pub root_name: String,
    pub epilog: String,
    /// The run holding the section's `hp:secPr`, which leads its first paragraph.
    pub start_run: Option<String>,
    /// The start run was the head of the paragraph's first run: its children
    /// go back into that run.
    #[serde(default)]
    pub start_merged: bool,
}

/// What export needs besides the entries: the sections, and the tracked
/// changes the sections refer to (id, what it is).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PackageShell {
    pub sections: Vec<SectionShell>,
    pub tracked: Vec<(String, String)>,
}

/// The package's section parts, in section order.
fn section_parts(parts: &[hanji_core::Part]) -> Vec<String> {
    let mut v: Vec<(u32, String)> =
        parts.iter().filter_map(|p| section_number(&p.name).map(|n| (n, p.name.clone()))).collect();
    v.sort();
    v.into_iter().map(|x| x.1).collect()
}

impl HwpxEngine {
    /// Model text for resolved blocks and a remainder.
    pub fn text_of(blocks: &[Block], rem: &Remainder, template: Option<&str>) -> String {
        let front = FrontMatter::document("hwpx", template);
        let d = hanji_core::model::unresolve(blocks, &rem.styles, front, &|id| {
            rem.keep(id).cloned().expect("keep in remainder")
        });
        fmt::serialize(&d)
    }

    /// Model blocks of an import (for tests and tools).
    pub fn split(
        package: &[u8],
        opts: &ImportOptions,
    ) -> Result<(Vec<Block>, Remainder, ImportReport, import::Stats), EngineError> {
        let pkg = |m: String| EngineError::Package(m);
        let mut parts = package::read(package).map_err(pkg)?;
        if package::get(&parts, "mimetype").is_none_or(|m| !m.starts_with(b"application/hwp+zip")) {
            return Err(pkg("not an hwpx package: it has no mimetype part saying application/hwp+zip".into()));
        }
        let names = section_parts(&parts);
        if names.is_empty() {
            return Err(pkg("the package has no Contents/section0.xml".into()));
        }
        let mut sections = vec![];
        for n in &names {
            let d = xml::parse(package::get(&parts, n).unwrap()).map_err(|e| pkg(format!("{n}: {e}")))?;
            if d.root.name != "hs:sec" || !d.root.attrs.iter().any(|a| a.0 == "xmlns:hp") {
                return Err(pkg(format!(
                    "{n}: the section must be <hs:sec> with the paragraph namespace as hp: (other prefixes are not supported yet)"
                )));
            }
            sections.push((n.clone(), d));
        }
        let mut report = ImportReport::default();
        if opts.neutralise {
            safety::neutralise(&mut parts, &mut sections, &mut report).map_err(pkg)?;
        }
        safety::surface(&parts, &sections, &mut report);
        let header = Header::read(package::get(&parts, HEADER_PART)).map_err(pkg)?;
        let mut imp = import::Importer::new(&header);
        let mut blocks = vec![];
        let mut shells = vec![];
        for (k, (name, d)) in sections.iter().enumerate() {
            let start = imp.section(k, &d.root, &mut blocks).map_err(|e| pkg(format!("{name}: {e}")))?;
            shells.push(SectionShell {
                part: name.clone(),
                prolog: d.prolog.clone(),
                root_open: d.root.open_tag(),
                root_name: d.root.name.clone(),
                epilog: d.epilog.clone(),
                start_merged: start.as_ref().is_some_and(|s| s.1),
                start_run: start.map(|s| s.0.to_xml()),
            });
        }
        let mut split = imp.finish(blocks);
        hanji_core::model::mark_shown(&mut split.styles, &split.blocks);
        let namespaces = sections[0]
            .1
            .root
            .attrs
            .iter()
            .filter_map(|(k, v)| xml::ns_prefix(k).map(|p| (p.to_string(), v.clone())))
            .collect();
        for p in &mut parts {
            if names.contains(&p.name) {
                p.data.clear();
            }
        }
        let shell = PackageShell { sections: shells, tracked: split.tracked };
        let rem = Remainder {
            format: "hwpx".into(),
            namespaces,
            shell: vec![serde_json::to_string(&shell).unwrap()],
            styles: split.styles,
            entries: split.entries,
            parts,
            next_id: split.next_id,
        };
        Ok((split.blocks, rem, report, split.stats))
    }
}

impl Engine for HwpxEngine {
    fn format(&self) -> &'static str {
        "hwpx"
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities { formatting: true, ..Default::default() }
    }

    fn import(&self, package: &[u8], opts: &ImportOptions) -> Result<Imported, EngineError> {
        let (blocks, remainder, report, _) = Self::split(package, opts)?;
        let text = Self::text_of(&blocks, &remainder, opts.template.as_deref());
        Ok(Imported { text, remainder, report })
    }

    fn export(&self, text: &str, rem: &Remainder) -> Result<Vec<u8>, EngineError> {
        let (_, blocks) = hanji_core::model_of(text, rem, self.capabilities()).map_err(EngineError::Invalid)?;
        // The style section as the text has it.
        let mut rem = rem.clone();
        hanji_core::TextModel::restyle(&hanji_core::DocumentModel, text, &mut rem, self.capabilities());
        write_package(&rem, export_sections(&blocks, &rem)?)
    }
}

impl HwpxEngine {
    /// A new revision's blocks with what its text could not show (§5.2): a
    /// paragraph that had no text, and has now without writing its own
    /// `{…}`, keeps the formatting it had, and the text shows it from now
    /// on; a new table whose text gives its cells no box takes the look of
    /// the file's first table cell (Hancom's, solid 0.12 mm lines and
    /// centred, in a file without one); and values as hwpx holds them
    /// (border widths snapped to Hancom's, line spacing in whole percents).
    /// `rem`'s entries stop marking a paragraph whose text shows it.
    pub fn complete(blocks: &mut [Block], rem: &mut Remainder) {
        use hanji_format::{styled, Cell, Inline, Props};
        if !rem.styles.formatting {
            return;
        }
        for s in &mut rem.styles.paragraph {
            format::snap_props(&mut s.props);
        }
        let Ok(header) = Header::read(package::get(&rem.parts, HEADER_PART)) else { return };
        let f = export::Fmt::new(&rem.styles);
        // A new table's cells, as export draws them.
        let look = match rem.entries.iter().find(|e| e.kind == Kind::Tc) {
            Some(e) => header.cell_box(&xml::fragment(&e.xml[0]), &xml::fragment(&e.xml[1])),
            None => {
                let mut p = Props::new();
                let line = hanji_format::Key::BorderTop.parse(Some("0.34pt solid #000000")).unwrap();
                for k in hanji_format::Key::SIDES {
                    p.set(k, line.clone());
                }
                p.set(hanji_format::Key::Valign, hanji_format::Value::Choice("middle".into()));
                p
            }
        }
        .diff(&styled::box_default());
        for (bi, b) in blocks.iter_mut().enumerate() {
            let Block::Table(t) = b else { continue };
            let new = !rem.entries.iter().any(|e| e.kind == Kind::Tbl && e.path == [bi] && e.meta.tag.is_empty());
            if new && t.boxes.iter().flatten().all(Props::is_empty) {
                t.boxes = t
                    .rows
                    .iter()
                    .map(|r| {
                        r.iter().map(|c| if matches!(c, Cell::Text(_)) { look.clone() } else { Props::new() }).collect()
                    })
                    .collect();
            }
        }
        let mut fill = |path: Vec<usize>, style: &str, content: &mut Inline, props: &mut Props| {
            format::snap_props(props);
            for u in &mut content.units {
                format::snap_props(&mut u.props);
            }
            if !styled::shows(content) {
                return;
            }
            let Some(e) = rem.entries.iter_mut().find(|e| e.kind == Kind::Ppr && e.path == path && e.meta.unshown)
            else {
                return;
            };
            e.meta.unshown = false;
            if !props.is_empty() {
                return;
            }
            let x = xml::fragment(&e.xml[0]);
            let sid = e.meta.style.clone().unwrap_or_else(|| "0".into());
            let style = if style.is_empty() {
                rem.styles.paragraph_name(&sid).unwrap_or(&rem.styles.default_paragraph).to_string()
            } else {
                style.to_string()
            };
            let ppr = x
                .get("paraPrIDRef")
                .and_then(|v| v.parse().ok())
                .or_else(|| sid.parse().ok().and_then(|id| header.style(id)).map(|st| st.para_pr));
            if let Some(ppr) = ppr {
                let base = f.of(&style);
                *props = header.para_values(ppr).only(&styled::PARA_OWN).diff(base);
            }
        };
        for (bi, b) in blocks.iter_mut().enumerate() {
            match b {
                Block::Para(p) => fill(vec![bi], &p.style, &mut p.content, &mut p.props),
                Block::Table(t) => {
                    for bx in t.boxes.iter_mut().flatten() {
                        format::snap_props(bx);
                    }
                    for (ri, row) in t.rows.iter_mut().enumerate() {
                        for (ci, c) in row.iter_mut().enumerate() {
                            if let Cell::Text(ps) = c {
                                for (k, p) in ps.iter_mut().enumerate() {
                                    let style = p.style.clone().unwrap_or_default();
                                    fill(vec![bi, ri, ci, k], &style, &mut p.content, &mut p.props);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Self::level_indents(blocks, rem, &header, &f);
    }

    /// A new list item, or one the text moved to another level, takes its
    /// level's indent (§5.2) from the nearest item at that level, of its
    /// kind, that is where it was (in its own list first), as export takes
    /// its shape; unless its text writes an indent of its own.
    fn level_indents(blocks: &mut [Block], rem: &Remainder, header: &Header, f: &export::Fmt) {
        let ind = import::LIST_INDENT;
        let own = |bi: usize| rem.entries.iter().find(|e| e.kind == Kind::Ppr && e.path == [bi]);
        fn item_of(b: &Block) -> Option<(hanji_core::ListItem, &hanji_core::Para)> {
            match b {
                Block::Para(p) => p.item.map(|i| (i, p)),
                _ => None,
            }
        }
        let style_of = |p: &hanji_core::Para| {
            if p.style.is_empty() {
                rem.styles.item_style().to_string()
            } else {
                p.style.clone()
            }
        };
        let stable = |k: usize| {
            item_of(&blocks[k])
                .is_some_and(|(i, _)| own(k).and_then(|e| e.meta.item).is_some_and(|w| w.level == i.level))
        };
        let mut fixes = vec![];
        for bi in 0..blocks.len() {
            let Some((it, p)) = item_of(&blocks[bi]) else { continue };
            let e = own(bi);
            let moved = e.and_then(|e| e.meta.item).is_some_and(|w| w.level != it.level);
            if e.is_some() && !moved {
                continue;
            }
            let base = f.of(&style_of(p)).only(&ind);
            if let Some(e) = e {
                // Moved: an indent it had is carried, not written.
                let x = xml::fragment(&e.xml[0]);
                let ppr = x.get("paraPrIDRef").and_then(|v| v.parse().ok());
                let had = ppr.map(|id| header.para_values(id).only(&ind).diff(&base)).unwrap_or_default();
                if p.props.only(&ind) != had {
                    continue;
                }
            } else if !p.props.only(&ind).is_empty() {
                continue;
            }
            let lo = (0..=bi).rev().take_while(|&k| item_of(&blocks[k]).is_some()).last().unwrap_or(bi);
            let hi = (bi..blocks.len()).take_while(|&k| item_of(&blocks[k]).is_some()).last().unwrap_or(bi);
            let mut near: Vec<usize> = (0..blocks.len())
                .filter(|&k| {
                    k != bi
                        && stable(k)
                        && item_of(&blocks[k]).is_some_and(|(i, _)| i.level == it.level && i.ordered == it.ordered)
                })
                .collect();
            near.sort_by_key(|&k| (!(lo..=hi).contains(&k), k.abs_diff(bi), k));
            let Some((_, q)) = near.first().and_then(|&k| item_of(&blocks[k])) else { continue };
            let eff = f.of(&style_of(q)).only(&ind).overlay(&q.props.only(&ind));
            fixes.push((bi, eff.diff(&base)));
        }
        for (bi, want) in fixes {
            if let Block::Para(p) = &mut blocks[bi] {
                p.props = p.props.without(&ind).overlay(&want);
            }
        }
    }
}

/// What export writes: the section parts, and `header.xml` when it gained shapes.
pub struct Exported {
    pub sections: Vec<(String, Vec<u8>)>,
    pub header: Option<Vec<u8>>,
}

/// The package: `rem`'s parts, with the exported ones in place.
pub fn write_package(rem: &Remainder, out: Exported) -> Result<Vec<u8>, EngineError> {
    let mut parts = rem.parts.clone();
    let by: HashMap<&str, &Vec<u8>> = out.sections.iter().map(|(n, d)| (n.as_str(), d)).collect();
    for p in &mut parts {
        if let Some(d) = by.get(p.name.as_str()) {
            p.data = (*d).clone();
        } else if p.name == HEADER_PART {
            if let Some(h) = &out.header {
                p.data = h.clone();
            }
        }
    }
    package::write(&parts).map_err(EngineError::Package)
}

/// Every paragraph of a package whose layout cache points past its end
/// ([`owpml::stale_layout`]), by section part: the check the corpus test
/// and the Office kit run on every hwpx export (rhwp lays the paragraphs
/// out again and does not see it; Hancom asks to repair the file).
pub fn layout_problems(package: &[u8]) -> Result<Vec<String>, String> {
    let parts = package::read(package)?;
    let mut out = vec![];
    for name in section_parts(&parts) {
        let d = xml::parse(package::get(&parts, &name).unwrap()).map_err(|e| format!("{name}: {e}"))?;
        out.extend(owpml::layout_problems(&d.root).into_iter().map(|p| format!("{name}: {p}")));
    }
    Ok(out)
}

/// The section parts (and header) for resolved blocks placed against `rem`.
pub fn export_sections(blocks: &[Block], rem: &Remainder) -> Result<Exported, EngineError> {
    let refused = EngineError::Refused;
    let shell: PackageShell = serde_json::from_str(
        rem.shell.first().ok_or_else(|| EngineError::Package("remainder has no package shell".into()))?,
    )
    .map_err(|e| EngineError::Package(e.to_string()))?;
    let mut header = Header::read(package::get(&rem.parts, HEADER_PART)).map_err(EngineError::Package)?;
    let lists = hanji_core::plan_lists(blocks, &rem.entries, &mut header).map_err(refused)?;
    tracked_groups_intact(blocks, rem).map_err(refused)?;
    // The style section (§5.2): changed styles point at new shapes, new ones are written.
    let mut styles = rem.styles.clone();
    if styles.formatting {
        header.write_styles(&mut styles).map_err(refused)?;
    }
    let mut ex = export::Exporter::new(&styles, header, &shell, &rem.entries, lists);
    if styles.formatting {
        ex = ex.formatted(export::Fmt::new(&styles));
    }
    let body = ex.body(blocks).map_err(refused)?;
    // Every tracked change the file had is still referred to (§10.2: never dropped).
    let mut seen = BTreeSet::new();
    for (_, paras) in &body {
        for p in paras {
            owpml::tracked_ids(p, &mut |v| {
                seen.insert(v);
            });
        }
    }
    if let Some((_, what)) = shell.tracked.iter().find(|(id, _)| !seen.contains(id)) {
        return Err(refused(format!(
            "the edit removes a {what}; tracked changes cannot be dropped on export. Keep its placeholder, or accept or reject the change in Hancom first"
        )));
    }
    let mut names: Vec<&str> = shell.sections.iter().map(|s| s.part.as_str()).collect();
    names.sort_by_key(|n| section_number(n));
    let mut sections = vec![];
    for ((s, paras), name) in body.into_iter().zip(names) {
        let mut x = s.prolog.clone();
        x.push_str(&s.root_open);
        for mut p in paras {
            // Never a layout cache that points past its paragraph: Hancom
            // asks to repair such a file. A cache counts the section's start
            // run (`hp:secPr` and its controls) in the paragraph that held
            // it; when an edit puts another paragraph first, the run moves
            // there and the old first paragraph's lines can start past its
            // end. Without its cache Hancom lays the paragraph out again. A
            // cache still inside its paragraph stays: Hancom opens those.
            owpml::drop_stale_layouts(&mut p);
            xml::write_element(&p, &mut x);
        }
        x.push_str(&format!("</{}>", s.root_name));
        x.push_str(&s.epilog);
        sections.push((name.to_string(), x.into_bytes()));
    }
    Ok(Exported { sections, header: ex.header.part() })
}

/// The paragraphs of one tracked change stay together, all of them and in
/// their order: a change that spans paragraphs cannot be cut, shortened or
/// reordered. (A paragraph inside the change holds no mark of it, so the
/// census of tracked-change ids alone would not see it go.)
fn tracked_groups_intact(blocks: &[Block], rem: &Remainder) -> Result<(), String> {
    let mut last: HashMap<&str, (usize, u64)> = HashMap::new();
    let mut count: HashMap<&str, (usize, Option<usize>, &str)> = HashMap::new();
    // Page breaks are paragraph properties in hwpx: they do not part a change.
    let is_break = |b: &Block| matches!(b, Block::Para(p) if export::is_page_break(&p.content));
    for (bi, b) in blocks.iter().filter(|b| !is_break(b)).enumerate() {
        let Block::Keep(id) = b else { continue };
        let Some(e) =
            rem.entries.iter().find(|e| e.kind == Kind::Bkeep && e.meta.keep.as_ref().is_some_and(|k| &k.id == id))
        else {
            continue;
        };
        if e.meta.keep.as_ref().is_none_or(|k| k.kind != "tracked-change") {
            continue;
        }
        let g = e.meta.aux[0].as_str();
        let s = e.meta.keep.as_ref().unwrap().summary.as_str();
        if let Some((pb, pseq)) = last.get(g) {
            if *pb + 1 != bi || *pseq > e.seq {
                return Err(format!("a tracked change that spans paragraphs was cut or reordered ({s}); keep its placeholders together and in order"));
            }
        }
        last.insert(g, (bi, e.seq));
        let total = e.meta.aux.get(1).and_then(|n| n.parse().ok());
        count.entry(g).or_insert((0, total, s)).0 += 1;
    }
    if let Some((_, _, s)) = count.values().find(|(n, total, _)| total.is_some_and(|t| *n != t)) {
        return Err(format!("the edit removes a paragraph of a tracked change that spans paragraphs ({s}); tracked changes cannot be dropped on export. Keep its placeholders, or accept or reject the change in Hancom first"));
    }
    Ok(())
}
