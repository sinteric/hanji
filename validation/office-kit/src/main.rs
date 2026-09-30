//! The Office check kit (DESIGN.md §9 validity): exports for a person to open
//! in Word, PowerPoint, Excel and Hancom Office, which this environment
//! cannot run, with a checklist.
//!
//! ```sh
//! cargo run --release --manifest-path validation/office-kit/Cargo.toml [OUT_DIR]
//! ```
//!
//! Per source: the untouched original (`NN-…-original.ext`), then its exports,
//! each edited one with `NN-…-edits.md` beside it (the edits in plain words,
//! a deck's slide order, and the before/after model text diff).
//!
//! A deck's -edits.md gives each box an edit names in cm too, in PowerPoint's
//! Format Shape → Size & Properties fields. With LibreOffice, PyMuPDF and
//! Pillow installed, `render.py` draws each edited slide of a `pset` file
//! next to its original slide (`NN-…-pset-slideK.png`, the changed objects
//! outlined); `OFFICE_KIT_RENDER=0` skips it.
//!
//! Writes `target/office-kit/` (the files, `CHECKLIST.md`, `checklist.csv`)
//! and `target/office-kit.zip` at the workspace root. Every file is an export
//! of a corpus file whose licence allows redistribution (the corpora's
//! SOURCES.md), or of hanji-store's blank package.
//!
//! - docx: per source, GetPut (no edit), E10 (every scripted edit in one
//!   revision, with the formatting edits F1–F4: a first-line indent on body
//!   paragraphs, a heading style restyled, a new style given to a
//!   paragraph, a table's header row filled), and tracked changes: the scripted edits one at a time, as
//!   many as the tracked export writes (it refuses some, with the reason);
//!   and two new documents from hanji-store's blank docx.
//! - pptx: per deck, GetPut and P9 (title, bullet, notes and shape text
//!   edits, a slide added from a layout, one deleted, one moved, …, and the
//!   geometry edits: a shape moved, a picture resized, a text box added
//!   under a title, two objects aligned; the picture edits: a picture
//!   cropped, one cut to an ellipse with new alternative text; text
//!   formatting: a word made 24 pt coral, a shape's font changed; and fills:
//!   a shape filled, one's fill cleared; and a group's shape's text edited).
//! - xlsx: per workbook, GetPut and range operations that change the inputs
//!   of formulas, whose cached values the export recomputes, and `format`
//!   operations on named ranges (a header row filled and bold, a column in
//!   a theme colour, a range filled, an outline).
//! - hwpx: per file, GetPut and E10.

mod formats;
#[path = "../../../crates/hanji-pptx/tests/edits/mod.rs"]
mod pptx_edits;

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use hanji_core::{Block, Engine, ImportOptions, Kind, Remainder};
use hanji_docx::{DocxEngine, ExportOptions, History, Reviewer};
use hanji_format::sheet::WindowOf;
use hanji_hwpx::HwpxEngine;
use hanji_pptx::PptxEngine;
use hanji_testkit::{edit_jobs, Cx, Doc, Edit, Format, CAPS};
use hanji_xlsx::XlsxEngine;

use formats::{Docx, Hwpx, Pptx};

const AUTHOR: &str = "hanji (model edit)";
const OPEN: &str = "Opens with no repair or recovery prompt";

/// One file of the kit.
struct Item {
    file: String,
    format: &'static str,
    source: String,
    licence: String,
    variant: String,
    about: String,
    checks: Vec<String>,
    /// What was edited, and what to look for.
    notes: Vec<String>,
    /// `NN-…-edits.md`: the edits in plain words and the model text diff.
    md: Option<String>,
    /// A deck's slide order: which original slide each slide of the file is.
    order: Option<String>,
}

struct Kit {
    dir: PathBuf,
    items: Vec<Item>,
    /// Decks whose edited slides `render.py` draws before and after.
    renders: Vec<serde_json::Value>,
}

fn slug(name: &str) -> String {
    let stem = name.rsplit_once('.').map_or(name, |x| x.0);
    let mut s: String =
        stem.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
    while s.contains("--") {
        s = s.replace("--", "-");
    }
    s.trim_matches('-').chars().take(32).collect()
}

impl Kit {
    #[allow(clippy::too_many_arguments)]
    fn add(
        &mut self,
        format: &'static str,
        source: &str,
        licence: &str,
        variant: &str,
        about: &str,
        bytes: &[u8],
        checks: Vec<String>,
        notes: Vec<String>,
    ) {
        let ext = match source.rsplit_once('.') {
            Some((_, "pptm")) => "pptx",
            _ => format,
        };
        let ext = if variant == "original" { source.rsplit_once('.').map_or(ext, |x| x.1) } else { ext };
        let file = format!("{:02}-{format}-{}-{variant}.{ext}", self.items.len() + 1, slug(source));
        std::fs::write(self.dir.join(&file), bytes).unwrap();
        println!("  {file} ({} KB)", bytes.len() / 1024);
        let mut checks = checks;
        if variant != "original" {
            if let Some(o) = self.items.iter().find(|i| i.source == source && i.variant == "original") {
                let c = if variant == "getput" {
                    format!("Compare with {}: should look identical", o.file)
                } else {
                    format!("Compare with {}: only the changes in the -edits.md file beside this one differ", o.file)
                };
                checks.insert(1.min(checks.len()), c);
            }
        }
        self.items.push(Item {
            file,
            format,
            source: source.to_string(),
            licence: licence.to_string(),
            variant: variant.to_string(),
            about: about.to_string(),
            checks,
            notes,
            md: None,
            order: None,
        });
    }

    /// The source file as it came, to compare the exports with.
    fn original(&mut self, format: &'static str, source: &str, licence: &str, bytes: &[u8]) {
        self.add(
            format,
            source,
            licence,
            "original",
            "the source file, untouched, for comparison",
            bytes,
            vec![],
            vec![],
        );
    }

    /// `NN-…-edits.md` for the file just added: what changed in plain
    /// words and the model text diff (`diff`, unified; empty when the text
    /// did not change).
    fn edits(&mut self, diff: &str, diff_what: &str) {
        let it = self.items.last().unwrap();
        let md_name = format!("{}-edits.md", it.file.rsplit_once('.').unwrap().0);
        let mut md = format!("# What {} changed\n\n", it.file);
        let orig = self.items.iter().find(|i| i.source == it.source && i.variant == "original").map(|i| i.file.clone());
        let it = self.items.last_mut().unwrap();
        let _ = write!(md, "{}. Source: `{}`", cap(&it.about), it.source);
        match &orig {
            Some(o) => {
                let _ = writeln!(md, "; compare with `{o}`.\n");
            }
            None => md.push_str(".\n\n"),
        }
        if let Some(order) = &it.order {
            let _ = writeln!(md, "{order}\n");
        }
        md.push_str("## The edits\n\n");
        for n in &it.notes {
            if n.starts_with("```") {
                let _ = writeln!(md, "\n{n}\n");
            } else {
                let _ = writeln!(md, "- {}", placeholders(n));
            }
        }
        let _ = writeln!(md, "\n## {diff_what}\n");
        if diff.is_empty() {
            md.push_str("(no change)\n");
        } else {
            const MAX: usize = 1500;
            let lines: Vec<&str> = diff.lines().collect();
            let _ = writeln!(md, "```diff\n{}\n```", lines[..lines.len().min(MAX)].join("\n"));
            if lines.len() > MAX {
                let _ = writeln!(md, "\n({} more lines of diff left out)", lines.len() - MAX);
            }
        }
        std::fs::write(self.dir.join(&md_name), md).unwrap();
        it.md = Some(md_name);
    }

    fn file_of(&self, source: &str, variant: &str) -> String {
        self.items.iter().find(|i| i.source == source && i.variant == variant).map_or("?".into(), |i| i.file.clone())
    }
}

/// An edit description quotes text with Rust escapes for the placeholder characters (`\u{f0000}`): shown as `◆`.
fn placeholders(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(k) = rest.find("\\u{") {
        out.push_str(&rest[..k]);
        match rest[k..].find('}') {
            Some(e) => {
                out.push('◆');
                rest = &rest[k + e + 1..];
            }
            None => {
                out.push_str(&rest[k..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().chain(c).collect())
}

/// The model text diff, before (the original's text) and after (the edited revision's).
fn text_diff(before: &str, after: &str) -> String {
    hanji_store::merge::unified(before, after, "original (model text)", "edited (model text)")
}

const TEXT_DIFF: &str = "Model text, before and after (unified diff; `-` original, `+` edited)";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

fn read(dir: &str, name: &str) -> Vec<u8> {
    std::fs::read(root().join(dir).join(name)).unwrap_or_else(|e| panic!("{dir}/{name}: {e}"))
}

fn opened_in(app: &str) -> String {
    format!("{OPEN} in {app}")
}

// ---------------------------------------------------------------- docx

const DOCX_DIR: &str = "prototype/remainder/corpus";

const DOCX: &[(&str, &str)] = &[
    ("korean-report.docx", "CC0-1.0, synthetic (written for this project)"),
    ("sample-docx.docx", "Apache-2.0 (docx4j test data)"),
    ("docx4j-tables.docx", "Apache-2.0 (docx4j test data)"),
    ("testWORD_various.docx", "Apache-2.0 (Apache Tika test data)"),
    ("fdo76098.docx", "MPL-2.0 (LibreOffice test data)"),
    ("loadAndSave.docx", "Apache-2.0 (docx4j test data)"),
];

/// An exact edit for a local change, a design C rewrite for a move. The
/// span is the edit's own when it knows one (a whole slide or paragraph
/// deleted spans whole lines, as in the corpus harness): the smallest
/// differing span starts and ends inside lines when neighbours begin
/// alike, and cut that way the edit keeps the wrong slide's or paragraph's
/// content. Text edits within a paragraph keep the smallest span, which a
/// tracked export shows as the smallest change ("5" inserted, not "10"
/// replaced by "105").
fn step(fmt: &dyn Format, rem: &Remainder, text: &str, new_text: &str, ed: &Edit) -> Option<hanji_core::Reanchored> {
    let r = if ed.local {
        let (a, b, repl) = ed.span.clone().unwrap_or_else(|| hanji_testkit::edit_span(text, new_text));
        hanji_core::reanchor_span_in(fmt.model(), rem, text, a, b, &repl, CAPS).ok()?
    } else {
        hanji_core::reanchor_rewrite_in(fmt.model(), rem, text, new_text, CAPS).ok()?
    };
    r.report.refused.is_empty().then_some(r)
}

fn docx(kit: &mut Kit) {
    let word = opened_in("Word");
    let tracked = ExportOptions {
        tracked_changes: Some(Reviewer { author: AUTHOR.into(), date: "2026-09-29T00:00:00Z".into() }),
    };
    for (name, licence) in DOCX {
        let bytes = read(DOCX_DIR, name);
        let opts = ImportOptions::default();
        let imp = DocxEngine.import(&bytes, &opts).unwrap();
        let (text, rem) = (imp.text, imp.remainder);
        let getput = DocxEngine.export(&text, &rem).unwrap();
        kit.original("docx", name, licence, &bytes);
        kit.add(
            "docx",
            name,
            licence,
            "getput",
            "imported and exported with no edit",
            &getput,
            vec![word.clone()],
            vec![],
        );
        let (blocks, _, _, _) = DocxEngine::split(&bytes, &opts).unwrap();
        let d = Doc { blocks, entries: rem.entries.clone() };
        let cx = Cx { fmt: &Docx, rem: &rem };
        let jobs = edit_jobs(&Docx, &d, &cx, &text, &hanji_testkit::FORMATTED_EDITS, "E10", None);
        let e10 = jobs.last().unwrap();
        match hanji_core::rewrite(&rem, &text, &e10.new_text, CAPS) {
            Ok(r) => {
                let out = DocxEngine.export(&r.text, &r.remainder).unwrap();
                let notes = e10.what.split("; ").map(String::from).collect();
                kit.add(
                    "docx",
                    name,
                    licence,
                    "e10",
                    "every scripted edit (E1–E9, and the formatting edits F1–F4) in one revision, as direct changes",
                    &out,
                    vec![word.clone(), "Shows the edits listed below".into()],
                    notes,
                );
                kit.edits(&text_diff(&text, &r.text), TEXT_DIFF);
            }
            Err(e) => println!("  {name} E10: refused: {}", e.to_string().lines().next().unwrap_or("")),
        }
        // Tracked changes: the edits one at a time, each kept when the tracked export writes it.
        let mut h = History::new(&text, &rem).unwrap();
        let (mut cur_text, mut cur_rem) = (text.clone(), rem.clone());
        let mut kept: Vec<String> = vec![];
        let mut last = None;
        for (_, f) in hanji_testkit::EDITS {
            let blocks = hanji_testkit::written(&Docx, &cur_text, &cur_rem);
            let d = Doc { blocks, entries: cur_rem.entries.clone() };
            let Some(ed) = f(&d, &Cx { fmt: &Docx, rem: &cur_rem }) else { continue };
            let new_text = Docx.text_of(&ed.blocks, &cur_rem);
            let Some(r) = step(&Docx, &cur_rem, &cur_text, &new_text, &ed) else { continue };
            let mut h2 = h.clone();
            let out = h2.push(&r).and_then(|_| DocxEngine.export_with(&r.text, &r.remainder, &tracked, Some(&h2)));
            match out {
                Ok(pkg) => {
                    kept.push(format!("{} ({})", ed.name, ed.what));
                    (h, cur_text, cur_rem, last) = (h2, r.text, r.remainder, Some(pkg));
                }
                Err(e) => println!("  {name} tracked, left out {}: {e}", ed.name),
            }
        }
        if let Some(pkg) = last {
            let getput_file = kit.file_of(name, "original");
            kit.add(
                "docx",
                name,
                licence,
                "tracked",
                "the scripted edits as tracked changes by \"hanji (model edit)\"",
                &pkg,
                vec![
                    word.clone(),
                    "Review > Track Changes lists the changes by \"hanji (model edit)\", dated 2026-09-29".into(),
                    "Review, then Accept All, shows the edited text (the edits listed below applied)".into(),
                    format!("Reject All shows the original (it reads like {getput_file})"),
                ],
                kept,
            );
            kit.edits(&text_diff(&text, &cur_text), TEXT_DIFF);
        }
    }
    // Two new documents from a blank template.
    let template = hanji_store::blank::package(hanji_store::Format::Docx);
    let imp = DocxEngine.import(&template, &ImportOptions::default()).unwrap();
    let front = &imp.text[..imp.text.find("\n---\n").unwrap() + 5];
    for (source, body, about) in [
        (
            "blank-report-en.docx",
            NEW_EN,
            "a new report from hanji's blank docx: headings, lists, emphasis, a merged table",
        ),
        ("blank-report-ko.docx", NEW_KO, "a new Korean report from hanji's blank docx (DESIGN.md §5.2's example)"),
    ] {
        // In canonical form, so the export reads back as the same text (PutGet).
        let text = hanji_format::serialize(&hanji_format::parse(&format!("{front}{body}")).unwrap());
        let r = hanji_core::rewrite(&imp.remainder, &imp.text, &text, CAPS).unwrap_or_else(|e| panic!("{source}: {e}"));
        let out = DocxEngine.export(&r.text, &r.remainder).unwrap_or_else(|e| panic!("{source}: {e}"));
        let again = DocxEngine.import(&out, &ImportOptions::default()).unwrap();
        // The canonical text adds the style lines of the styles it uses (§5.2).
        assert_eq!(again.text, DocxEngine::text_of(&r.new, &r.remainder, None), "{source}: PutGet");
        kit.add(
            "docx",
            source,
            "written for this project (hanji-store's blank package and this tool)",
            "new",
            about,
            &out,
            vec![
                word.clone(),
                "Headings use Heading 1 / Heading 2; bullets and numbers show; the table has a merged cell".into(),
            ],
            vec!["Model text written:".into(), format!("```\n{}```", body)],
        );
        kit.edits(&text_diff(&imp.text, &r.text), "Model text, blank package to new document (unified diff)");
    }
}

const NEW_EN: &str = "# Quarterly sales report
Sales grew **12%** over the same quarter last year, and *34* new customers signed.

## Highlights
- New customers: 34
  - Capital region: 21
- Returning customers: 112

Next steps:

1. Keep the regional teams
1. Open two branches in Busan

## Figures
| Region | Branch | Sales |
|---|---|---|
| Seoul | Gangnam | 120 |
| ^^ | Jongno | 95 |
| Total || 215 |

<div style=\"Quote\">Figures are preliminary.</div>
";

const NEW_KO: &str = "# 3분기 영업 보고
매출은 전년 대비 **12%** 증가했다.

<div style=\"Quote\">신규 고객 34곳 중 21곳이 수도권.</div>

- 신규 고객 34곳
  - 수도권 21곳

다음 분기:

1. 목표 매출 1,300억 원

<p/>

| 지역 | 지점 | 매출 |
|---|---|---|
| 서울 | 강남 | 120 |
| ^^ | 종로 | 95 |
| 합계 || 215 |

| 지역 | 매출 | 증감 |
|---|---|---|
| 수도권 | 1,204 | +15% |
| 지방 | 812 | +4%<p/>부산 신규 2곳 |
";

// ---------------------------------------------------------------- pptx

const PPTX_DIR: &str = "crates/hanji-pptx/corpus";

const PPTX: &[(&str, &str)] = &[
    ("korean-deck.pptx", "CC0-1.0, synthetic (written for this project with python-pptx)"),
    ("SampleShow.pptx", "Apache-2.0 (Apache POI test data)"),
    ("layouts.pptx", "Apache-2.0 (Apache POI test data)"),
    ("shapes.pptx", "Apache-2.0 (Apache POI test data)"),
    ("txt-font-props.pptx", "MIT (python-pptx test data)"),
    ("with_japanese.pptx", "Apache-2.0 (Apache POI test data)"),
    ("audit/synth-modern-pitch.pptx", "CC0-1.0, synthetic (written for this project's canvas audit with python-pptx)"),
    ("audit/onlyoffice-sample.pptx", "Apache-2.0 (ONLYOFFICE document-templates)"),
];

fn pptx(kit: &mut Kit) {
    let ppt = opened_in("PowerPoint");
    for (name, licence) in PPTX {
        let bytes = read(PPTX_DIR, name);
        let imp = PptxEngine.import(&bytes, &ImportOptions::default()).unwrap();
        let (text, rem) = (imp.text, imp.remainder);
        let getput = PptxEngine.export(&text, &rem).unwrap();
        kit.original("pptx", name, licence, &bytes);
        kit.add(
            "pptx",
            name,
            licence,
            "getput",
            "imported and exported with no edit",
            &getput,
            vec![ppt.clone()],
            vec![],
        );
        // The edit set one edit at a time (an exact span where it is local), each kept when it places everything.
        let blocks0 = hanji_testkit::written(&Pptx, &text, &rem);
        let orig = sld_ids(&blocks0, &rem);
        let (mut cur_text, mut cur_rem) = (text.clone(), rem.clone());
        let mut done: Vec<Done> = vec![];
        for (_, f) in pptx_edits::EDITS {
            let blocks = hanji_testkit::written(&Pptx, &cur_text, &cur_rem);
            let d = Doc { blocks, entries: cur_rem.entries.clone() };
            let Some(ed) = f(&d, &Cx { fmt: &Pptx, rem: &cur_rem }) else { continue };
            let new_text = Pptx.text_of(&ed.blocks, &cur_rem);
            match step(&Pptx, &cur_rem, &cur_text, &new_text, &ed) {
                Some(r) if PptxEngine.export(&r.text, &r.remainder).is_ok() => {
                    let before = slide_ids(&d.blocks, &cur_rem, &orig);
                    let heads = slide_heads(&d.blocks);
                    let named = ed
                        .named
                        .iter()
                        .filter_map(|h| heads.iter().position(|x| x == h))
                        .map(|n| (n + 1, heads.len(), before[n]))
                        .collect();
                    let after = slide_ids(&hanji_testkit::written(&Pptx, &r.text, &r.remainder), &r.remainder, &orig);
                    let adds =
                        after.iter().filter(|x| x.is_none()).count() > before.iter().filter(|x| x.is_none()).count();
                    done.push(Done { name: ed.name, what: ed.what, named, adds, after });
                    (cur_text, cur_rem) = (r.text, r.remainder);
                }
                _ => println!("  {name}: left out {} (it would lose content the text does not show)", ed.name),
            }
        }
        if !done.is_empty() {
            let out = PptxEngine.export(&cur_text, &cur_rem).unwrap();
            let fin = slide_ids(&hanji_testkit::written(&Pptx, &cur_text, &cur_rem), &cur_rem, &orig);
            let notes = (0..done.len()).map(|k| in_cm(described(&done, k, &fin))).collect();
            kit.add(
                "pptx",
                name,
                licence,
                "pset",
                "the pptx edit set, one edit after another: text edits, a slide added from a layout, one deleted, one moved, geometry edits (a shape moved, a picture resized, a text box added, two objects aligned), picture edits (a picture cropped, one cut to an ellipse with new alternative text), text formatting (a word made 24 pt coral, a shape's font changed) fills (a shape filled accent2, one's fill cleared) and a group's shape's text edited",
                &out,
                vec![
                    ppt.clone(),
                    "Shows the edits listed below (slides added, deleted and moved; text changed; objects moved, resized, added and aligned where the listed boxes say, in PowerPoint's cm as each line gives them; a picture cropped as its crop says, the percent cut off its left, top, right and bottom; a picture cut to an ellipse, its alternative text (Format Picture > Alt Text) as listed; the listed word 24 pt coral, the listed shape in Noto Sans KR; the listed shapes filled accent 2 and without fill; the listed text in a group changed, its formatting kept)".into(),
                ],
                notes,
            );
            kit.items.last_mut().unwrap().order = Some(slide_order(&blocks0, orig.len(), &fin));
            kit.edits(&text_diff(&text, &cur_text), TEXT_DIFF);
            kit.renders.push(serde_json::json!({
                "file": kit.items.last().unwrap().file,
                "md": kit.items.last().unwrap().md,
                "original": kit.file_of(name, "original"),
                "before": text,
                "after": cur_text,
                "order": fin,
            }));
        }
    }
}

/// Where PowerPoint shows a box: Format Shape → Size & Properties (도형 서식 → 크기 및 속성), in cm
/// from the slide's top-left corner, rounded to 0.01 cm.
const PPT_FIELDS: [&str; 4] =
    ["가로 위치 (Horizontal position)", "세로 위치 (Vertical position)", "너비 (Width)", "높이 (Height)"];

/// A box in pt as PowerPoint's four fields in cm (1 pt = 12700 EMU, 1 cm = 360000 EMU), rounded
/// half up to 0.01 cm.
fn box_cm(b: [f64; 4]) -> String {
    let cm = |pt: f64| {
        let emu = (pt * 12700.0).round() as i64;
        let h = (emu * 100 + emu.signum() * 180_000) / 360_000;
        format!("{}{}.{:02}", if h < 0 { "-" } else { "" }, h.abs() / 100, h.abs() % 100)
    };
    let f: Vec<String> = PPT_FIELDS.iter().zip(b).map(|(n, v)| format!("{n} {} cm", cm(v))).collect();
    format!("{} (box {} {} {} {} pt)", f.join(", "), b[0], b[1], b[2], b[3])
}

/// An edit line that names a box (P12 move, P13 resize, P14 add, P15 align), with the box
/// before and after as PowerPoint shows it, in cm.
fn in_cm(line: String) -> String {
    fn four(s: &str) -> Option<[f64; 4]> {
        let v: Vec<f64> = s
            .split_whitespace()
            .take(4)
            .map(|t| t.trim_end_matches(|c: char| !c.is_ascii_digit()).parse().ok())
            .collect::<Option<_>>()?;
        (v.len() == 4).then(|| [v[0], v[1], v[2], v[3]])
    }
    let after_word = |w: &str| line.find(w).map(|k| &line[k + w.len()..]);
    let (before, after) = if let Some(rest) = after_word("from box ") {
        (four(rest), rest.find(" to ").and_then(|k| four(&rest[k + 4..])))
    } else if let Some(rest) = after_word(" at box ") {
        (None, four(rest))
    } else if let Some(rest) = after_word("(box ") {
        let b = four(rest);
        let x = after_word("(x ").and_then(|r| r.split(')').next()?.trim().parse::<f64>().ok());
        (b, b.zip(x).map(|(b, x)| [x, b[1], b[2], b[3]]))
    } else {
        return line;
    };
    let Some(after) = after else { return line };
    let before = before.map_or("none (a new object)".to_string(), box_cm);
    format!(
        "{line}. **In PowerPoint** (도형 서식 → 크기 및 속성 / Format Shape → Size & Properties; for a picture 그림 서식 / Format Picture): before {before}; after {}. PowerPoint rounds to 0.01 cm",
        box_cm(after)
    )
}

/// One edit of a deck's edit set, as the kit made it.
struct Done {
    name: &'static str,
    what: String,
    /// The slides its description names: number, the deck's slide count, and
    /// which original slide it is (`None`: one the edits added).
    named: Vec<(usize, usize, Option<usize>)>,
    /// Whether it added a slide.
    adds: bool,
    /// Which original slide each slide is after it.
    after: Vec<Option<usize>>,
}

/// Level-0 heads: where each slide starts.
fn slide_heads(blocks: &[Block]) -> Vec<usize> {
    (0..blocks.len()).filter(|&k| blocks[k].head().is_some_and(|h| h.level == 0)).collect()
}

/// Each slide's `p:sldId`, by its skeleton entry (none for a slide the edits added).
fn sld_ids(blocks: &[Block], rem: &Remainder) -> Vec<Option<String>> {
    slide_heads(blocks)
        .into_iter()
        .map(|h| {
            let e = rem.entries.iter().find(|e| e.kind == Kind::Slide && e.meta.tag == "slide" && e.path == [h])?;
            let info: hanji_pptx::import::SlideInfo = serde_json::from_str(&e.meta.aux[0]).ok()?;
            Some(info.sld_id)
        })
        .collect()
}

/// Which original slide (from 1) each slide is, by its `p:sldId`; `None` for a new one.
fn slide_ids(blocks: &[Block], rem: &Remainder, orig: &[Option<String>]) -> Vec<Option<usize>> {
    sld_ids(blocks, rem)
        .into_iter()
        .map(|id| id.and_then(|id| orig.iter().position(|o| o.as_ref() == Some(&id))))
        .map(|n| n.map(|n| n + 1))
        .collect()
}

/// An edit's line: its description, then where the slides it names are in
/// the file, and what a later edit hides of it.
fn described(done: &[Done], k: usize, fin: &[Option<usize>]) -> String {
    let d = &done[k];
    let tag = |x: &Done| x.name.split_whitespace().next().unwrap_or(x.name).to_string();
    let at = |id: Option<usize>| fin.iter().position(|x| *x == id).map(|q| q + 1);
    let who = |id: Option<usize>| id.map_or("new".to_string(), |o| format!("original slide {o}"));
    // One slide's parts are split by "; ", several slides' by ", " within one and "; " between.
    let sep = if d.named.len() + d.adds as usize > 1 { ", " } else { "; " };
    let mut hidden = vec![];
    let mut parts = vec![];
    for &(n, total, id) in &d.named {
        let mut p = who(id);
        match at(id) {
            Some(q) => {
                let _ = write!(p, "{sep}slide {q} in this file");
            }
            // Gone after this edit: this edit deleted it.
            None if !d.after.contains(&id) => {}
            None => {
                let by = done[k + 1..].iter().find(|x| !x.after.contains(&id)).map_or("a later edit".into(), tag);
                let _ = write!(p, "{sep}not in this file");
                hidden.push(format!("{by} deleted this slide later, so this change does not show in this file"));
            }
        }
        parts.push((n, total, p));
    }
    if d.adds {
        let mut p = who(None);
        if let Some(q) = at(None) {
            let _ = write!(p, "{sep}slide {q} in this file");
            if q < fin.len() {
                let last = |x: &Done| x.after.last() == Some(&None);
                let by = done[k + 1..].iter().find(|x| !last(x)).map_or("a later edit".into(), tag);
                hidden.push(format!("{by} moved it later, so it is not at the end in this file"));
            }
        }
        parts.push((0, 0, p));
    }
    let mut line = format!("{} ({})", d.name, d.what);
    match parts.as_slice() {
        [] => {}
        [(_, _, p)] => {
            let _ = write!(line, " ({p})");
        }
        _ => {
            let each: Vec<String> = parts.iter().map(|(n, total, p)| format!("slide {n} of {total}: {p}")).collect();
            let _ = write!(line, " ({})", each.join("; "));
        }
    }
    for h in hidden {
        let _ = write!(line, ". **Hidden by a later edit:** {h}");
    }
    line
}

/// `Slide order` for a deck's -edits.md and checklist: which original slide
/// each slide of the file is, and the original slides no longer in it.
fn slide_order(blocks0: &[Block], n: usize, fin: &[Option<usize>]) -> String {
    let each: Vec<String> = fin
        .iter()
        .enumerate()
        .map(|(q, id)| format!("{} ← {}", q + 1, id.map_or("new".to_string(), |o| o.to_string())))
        .collect();
    let mut line = format!("**Slide order** (slide in this file ← original slide): {}.", each.join(", "));
    let heads = slide_heads(blocks0);
    let gone: Vec<String> = (1..=n)
        .filter(|o| !fin.contains(&Some(*o)))
        .map(|o| match pptx_edits::slide_at(blocks0, heads[o - 1]).3 {
            t if t.is_empty() => format!("{o}"),
            t => format!("{o} ('{t}')"),
        })
        .collect();
    if !gone.is_empty() {
        let _ = write!(line, " Deleted: original slide {}.", gone.join(", "));
    }
    line
}

// ---------------------------------------------------------------- xlsx

const XLSX_DIR: &str = "crates/hanji-xlsx/corpus";

/// Workbook, licence, operations, and the windows that show their results.
/// Workbook, licence, operations (JSON), windows to show as (sheet, range).
type XlsxCase = (&'static str, &'static str, &'static str, &'static [(&'static str, &'static str)]);

const XLSX: &[XlsxCase] = &[
    (
        "korean-sales.xlsx",
        "CC0-1.0, synthetic (written for this project with openpyxl)",
        r##"[{"op": "set", "range": "매출!D5", "values": [[18420000]]},
             {"op": "append_rows", "table": "Sales", "rows": [{"월": "2026-04", "지점": "강남", "제품군": "가전", "매출": 12400000, "원가": 9000000}]},
             {"op": "format", "range": "매출!A3:F3", "set": {"fill": "#D9D9D9", "bold": true, "border-bottom": "0.75pt solid #000000"}}]"##,
        &[("요약", "A1:B8"), ("매출", "A3:F6")],
    ),
    (
        "table-sample.xlsx",
        "Apache-2.0 (Apache POI test data)",
        r##"[{"op": "set", "range": "Tabelle1!D5", "values": [[10]]}]"##,
        &[("Tabelle1", "C4:G9")],
    ),
    (
        "ExcelTables.xlsx",
        "Apache-2.0 (Apache POI test data)",
        r##"[{"op": "set", "range": "ExcelTable!H3", "values": [[58]]},
             {"op": "format", "range": "ExcelTable!I2:I4", "set": {"color": "accent6", "italic": true}}]"##,
        &[("ExcelTable", "G1:I4")],
    ),
    (
        "simple-monthly-budget.xlsx",
        "Apache-2.0 (Apache POI test data)",
        r##"[{"op": "set", "range": "Simple Monthly Budget!C5", "values": [[3200]]},
             {"op": "set", "range": "Simple Monthly Budget!C11", "values": [[950]]},
             {"op": "format", "range": "Simple Monthly Budget!B5:C7", "set": {"fill": "accent2+80%"}},
             {"op": "format", "range": "Simple Monthly Budget!E4:H5", "set": {"outline": "2pt solid #1F3864"}}]"##,
        &[("Simple Monthly Budget", "A1:H8")],
    ),
    (
        "WithThreeCharts.xlsx",
        "Apache-2.0 (Apache POI test data)",
        r##"[{"op": "set", "range": "Sheet1!B2", "values": [[30]]}, {"op": "set", "range": "Sheet1!D1", "values": [[11]]}]"##,
        &[("Sheet1", "A1:E6")],
    ),
    (
        "xlookup.xlsx",
        "Apache-2.0 (Apache POI test data)",
        r##"[{"op": "set", "range": "Sheet2!E2", "values": [[250000]]}, {"op": "set", "range": "Sheet1!B2", "values": [[4390]]}]"##,
        &[("Sheet2", "B1:F7"), ("Sheet1", "B1:D2")],
    ),
];

fn xlsx(kit: &mut Kit) {
    let excel = opened_in("Excel");
    for (name, licence, ops, shows) in XLSX {
        let bytes = read(XLSX_DIR, name);
        let imp = XlsxEngine.import(&bytes, &ImportOptions::default()).unwrap();
        let getput = XlsxEngine.export(&imp.text, &imp.remainder).unwrap();
        kit.original("xlsx", name, licence, &bytes);
        kit.add(
            "xlsx",
            name,
            licence,
            "getput",
            "imported and exported with no edit",
            &getput,
            vec![excel.clone()],
            vec![],
        );
        let a = match XlsxEngine::apply(&imp.text, &imp.remainder, ops) {
            Ok(a) => a,
            Err(e) => {
                println!("  {name} ops: refused: {e}");
                continue;
            }
        };
        let out = XlsxEngine.export(&a.text, &a.remainder).unwrap();
        let left = left_cells(&a.report.recalc.left);
        let mut notes = vec![format!("Operations: `{}`", ops.split_whitespace().collect::<Vec<_>>().join(" "))];
        notes.push(
            "Values the export holds: formula results as computed by hanji, except the cells hanji left to Excel \
             (listed below, if any). A formula cell left blank here had no cached value in the source; Excel \
             computes it when it opens the file."
                .into(),
        );
        if !left.is_empty() {
            let each: Vec<String> = left.iter().map(|(sheet, r, why)| format!("{sheet}!{r} ({why})")).collect();
            notes.push(format!(
                "Left to Excel ({LEFT}): {}. hanji did not compute these; the export keeps their old cached values \
                 and asks Excel to recalculate the workbook when it opens it. The windows show them as `{STALE}`, \
                 not as results.",
                each.join("; ")
            ));
        }
        // The values before and after, window by window, and the structure text.
        let (mut before, mut after) = (String::new(), String::new());
        for (sheet, range) in *shows {
            let of = WindowOf::Range { sheet: sheet.to_string(), range: range.to_string() };
            match XlsxEngine::window(&a.remainder, &of) {
                Ok(w) => notes.push(format!("```\n{}\n```", mark_left(&w, sheet, &left).trim_end())),
                Err(e) => notes.push(format!("({sheet}!{range}: {e})")),
            }
            before.push_str(&XlsxEngine::window(&imp.remainder, &of).unwrap_or_default());
            before.push('\n');
            after.push_str(&mark_left(&XlsxEngine::window(&a.remainder, &of).unwrap_or_default(), sheet, &left));
            after.push('\n');
        }
        before.push_str(&imp.text);
        after.push_str(&a.text);
        kit.add(
            "xlsx",
            name,
            licence,
            "ops",
            if ops.contains("\"format\"") {
                "range operations that change the inputs of formulas, and format operations (fill, borders, font) on the ranges they name"
            } else {
                "range operations that change the inputs of formulas"
            },
            &out,
            vec![
                excel.clone(),
                if ops.contains("\"format\"") {
                    "Each format operation's range (in the operations below) shows what it sets: the fill, bold, italic, colour or border it names, and nothing else of the cells changes".into()
                } else {
                    "No cell's formatting changes".into()
                },
                if left.is_empty() {
                    "Formula results show the new values without pressing F9 (compare the values listed below)".into()
                } else {
                    format!(
                        "Formula results show the new values without pressing F9 (compare the values listed below; \
                         Excel computes the cells marked `{STALE}` when it opens the file)"
                    )
                },
            ],
            notes,
        );
        kit.edits(
            &text_diff(&before, &after),
            "Cell values and model text, before and after (unified diff; the row windows above, then the structure)",
        );
    }
}

/// How the kit describes a cell the recalculator left to Excel.
const LEFT: &str = "stale here; Excel recomputes on open";

/// What a window shows in place of such a cell's stale cached value.
const STALE: &str = "(Excel recomputes)";

/// The cells the recalculator left (`Sheet!A1 (why)`, `Sheet!C2:D2 (why)`):
/// sheet, cells, why.
fn left_cells(left: &[String]) -> Vec<(String, hanji_core::cells::CellRange, String)> {
    let mut out = vec![];
    for l in left {
        // The sheet name may hold `!` and ` (`: the cells are the first
        // `!A1 (` or `!A1:B2 (` after it.
        let found = l.match_indices('!').find_map(|(k, _)| {
            let (cells, why) = l[k + 1..].split_once(" (")?;
            let r = hanji_core::cells::CellRange::parse(cells)?;
            Some((l[..k].to_string(), r, why.strip_suffix(')').unwrap_or(why).to_string()))
        });
        if let Some(f) = found {
            out.push(f);
        }
    }
    out
}

/// A window (`<data>` pipe table) with the cells left to Excel on `sheet`
/// shown as `STALE`, not as their stale cached values.
fn mark_left(window: &str, sheet: &str, left: &[(String, hanji_core::cells::CellRange, String)]) -> String {
    use hanji_core::cells::{col_index, CellRef};
    use hanji_format::sheet::split_pipe_row;
    let areas: Vec<_> = left.iter().filter(|(s, _, _)| s == sheet).map(|(_, r, _)| *r).collect();
    if areas.is_empty() {
        return window.to_string();
    }
    let mut cols: Vec<Option<u32>> = vec![];
    let mut out = String::new();
    for line in window.lines() {
        let mut line = line.to_string();
        if let Some(cells) = split_pipe_row(&line) {
            if cells.first().is_some_and(|c| c == "row") {
                cols = cells.iter().map(|c| col_index(c)).collect();
            } else if let Ok(row) = cells.first().map_or("", |c| c.as_str()).parse::<u32>() {
                let mut cells = cells;
                let mut hit = false;
                for (k, c) in cells.iter_mut().enumerate().skip(1) {
                    let Some(Some(col)) = cols.get(k) else { continue };
                    if areas.iter().any(|a| a.contains(CellRef::new(*col, row))) {
                        *c = STALE.to_string();
                        hit = true;
                    }
                }
                if hit {
                    let each: Vec<String> = cells.iter().map(|c| c.replace('|', "\\|")).collect();
                    line = format!("| {} |", each.join(" | "));
                }
            }
        }
        out.push_str(&line);
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------- hwpx

const HWPX_DIR: &str = "crates/hanji-hwpx/corpus";

const HWPX: &[(&str, &str)] = &[
    ("fdi-2025q2.hwpx", "KOGL Type 1 (기획재정부 press release), via rhwp's samples (MIT)"),
    ("hy-002.hwpx", "KOGL Type 1 (해양수산부 press release), via rhwp's samples (MIT)"),
    ("hcar-001.hwpx", "government form, via rhwp's samples (MIT)"),
    ("footnote-01.hwpx", "MIT (rhwp test data)"),
    ("para-001.hwpx", "MIT (rhwp test data)"),
    ("basic-table-01.hwpx", "MIT (rhwp test data)"),
];

/// No layout cache past its paragraph's end: Hancom asks to repair such a file, and rhwp does not see it.
fn hwpx_layout_ok(name: &str, variant: &str, pkg: &[u8]) {
    let problems = hanji_hwpx::layout_problems(pkg).unwrap();
    assert!(problems.is_empty(), "{name} {variant}: {problems:#?}");
}

fn hwpx(kit: &mut Kit) {
    let hancom = "Opens in Hancom Office without an error".to_string();
    for (name, licence) in HWPX {
        let bytes = read(HWPX_DIR, name);
        let imp = HwpxEngine.import(&bytes, &ImportOptions::default()).unwrap();
        let (text, rem) = (imp.text, imp.remainder);
        let getput = HwpxEngine.export(&text, &rem).unwrap();
        hwpx_layout_ok(name, "getput", &getput);
        kit.original("hwpx", name, licence, &bytes);
        kit.add(
            "hwpx",
            name,
            licence,
            "getput",
            "imported and exported with no edit",
            &getput,
            vec![hancom.clone()],
            vec![],
        );
        let (blocks, _, _, _) = HwpxEngine::split(&bytes, &ImportOptions::default()).unwrap();
        let d = Doc { blocks, entries: rem.entries.clone() };
        let cx = Cx { fmt: &Hwpx, rem: &rem };
        let jobs = edit_jobs(&Hwpx, &d, &cx, &text, &hanji_testkit::EDITS, "E10", None);
        let e10 = jobs.last().unwrap();
        match hanji_core::rewrite(&rem, &text, &e10.new_text, CAPS) {
            Ok(r) => {
                let out = HwpxEngine.export(&r.text, &r.remainder).unwrap();
                hwpx_layout_ok(name, "e10", &out);
                kit.add(
                    "hwpx",
                    name,
                    licence,
                    "e10",
                    "every scripted edit (E1–E9) in one revision",
                    &out,
                    vec![hancom.clone(), "Shows the edits listed below".into()],
                    e10.what.split("; ").map(String::from).collect(),
                );
                kit.edits(&text_diff(&text, &r.text), TEXT_DIFF);
            }
            Err(e) => println!("  {name} E10: refused: {}", e.to_string().lines().next().unwrap_or("")),
        }
    }
}

// ---------------------------------------------------------------- checklist and zip

fn checklist(kit: &Kit, commit: &str) -> (String, String) {
    let mut md = String::new();
    let _ = writeln!(
        md,
        "# hanji Office check kit\n\nBuilt from commit `{commit}` by `validation/office-kit`. DESIGN.md §9: every export must open in Word, PowerPoint, Excel or Hancom Office with no repair prompt. This environment cannot run them, so these files are checked by hand.\n\nFor each file: open it, tick pass or fail for each line, and write anything odd under Notes (a screenshot helps). `checklist.csv` has the same lines for a spreadsheet. Files are named `NN-format-source-variant`: `getput` is imported and exported with no edit, `original` is the source file, untouched, `e10` / `pset` / `ops` hold edits (each with an `-edits.md` beside it: the edits in plain words and the before/after diff), `tracked` holds tracked changes, `new` was written from hanji's blank docx.\n"
    );
    let mut csv = String::from("file,format,source,variant,check,pass,fail,notes\n");
    let q = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
    for (k, it) in kit.items.iter().enumerate() {
        if it.format == "pptx" && kit.items[..k].iter().all(|i| i.format != "pptx") {
            let _ = writeln!(md, "## How to check a deck\n\n{HOW_TO_CHECK_A_DECK}\n");
        }
        let _ = writeln!(md, "## {}\n", it.file);
        if it.variant == "original" {
            let _ = writeln!(
                md,
                "Source: `{}`, {}. The file as it came, for comparing the exports below with; nothing to check.\n",
                it.source, it.licence
            );
            continue;
        }
        let _ = writeln!(md, "Source: `{}`, {}. {}.\n", it.source, it.licence, cap(&it.about));
        if let Some(order) = &it.order {
            let _ = writeln!(md, "{order}\n");
        }
        let named = |c: &String| match &it.md {
            Some(e) => c.replace("the -edits.md file beside this one", e),
            None => c.clone(),
        };
        for c in it.checks.iter().map(named) {
            let _ = writeln!(md, "- Pass [ ] Fail [ ] {c}");
            let _ = writeln!(csv, "{},{},{},{},{},,,", q(&it.file), it.format, q(&it.source), it.variant, q(&c));
        }
        if let Some(e) = &it.md {
            let _ = writeln!(md, "\nWhat changed, and the before/after diff: `{e}`.");
        }
        let stem = format!("{}-slide", it.file.rsplit_once('.').unwrap().0);
        let imgs: Vec<String> =
            pngs(&kit.dir).into_iter().filter(|n| n.starts_with(&stem)).map(|n| format!("`{n}`")).collect();
        if !imgs.is_empty() {
            let _ = writeln!(
                md,
                "\nEach edited slide before (left, the original slide) and after (right), the changed objects outlined in red: {}. These are LibreOffice renders, which can differ slightly from PowerPoint.",
                imgs.join(", ")
            );
        }
        if !it.notes.is_empty() {
            let _ = writeln!(md, "\n<details><summary>What changed</summary>\n");
            for n in &it.notes {
                if n.starts_with("```") {
                    let _ = writeln!(md, "{n}\n");
                } else {
                    let _ = writeln!(md, "- {n}");
                }
            }
            let _ = writeln!(md, "\n</details>");
        }
        let _ = writeln!(md, "\nNotes: ______________________________________________\n");
    }
    let _ = writeln!(
        md,
        "## Sources and licences\n\nThe corpus files come from `prototype/remainder/corpus` (docx), `crates/hanji-pptx/corpus`, `crates/hanji-xlsx/corpus` and `crates/hanji-hwpx/corpus`; each one's SOURCES.md gives the upstream repository and pinned commit. Each file is redistributed under the licence of the repository it is test data in; the Korean press releases are published under 공공누리 제1유형 (KOGL Type 1), which allows redistribution with the source named. The two `new` documents and the synthetic Korean files were written for this project."
    );
    (md, csv)
}

const HOW_TO_CHECK_A_DECK: &str = "hanji's text of a deck holds each slide's layout, and every object on it in z-order with its position and size in points (`box=\"x y w h\"` from the slide's top-left corner, 72 pt = 1 inch = 2.54 cm; each -edits.md line that names a box also gives it in cm as PowerPoint shows it under 도형 서식 → 크기 및 속성 (Format Shape → Size & Properties): 가로 위치 (Horizontal position) and 세로 위치 (Vertical position), from the top-left corner, 너비 (Width) and 높이 (Height). PowerPoint rounds to 0.01 cm, so a field can differ from the listed value by 0.01 cm): the slots (title, body, …) and shapes with their text, lines and connectors by their two ends, groups with their objects, pictures as `<picture/>` lines with their image, crop (percent cut off each edge), mask (the shape they are cut to) and alternative text, and a `<keep/>` line for each object it does not model (a table or chart). The design (fonts, colours, fills, the theme, masters and layouts, transitions, animations) stays in the remainder, which the export writes back unchanged. So, comparing a `pset` export with its `original`: a slide no edit touched must look exactly like its original slide (the **Slide order** line says which original slide each one is); an edited slide may differ only in the text, order, layout, positions and sizes its edits list, and an object an edit moved or resized must sit at the box it names (a new text box under a title: just below it, as wide as it). Beside each `pset` file's -edits.md, `NN-…-pset-slideK.png` shows slide K of the file after its edits (right) next to the original slide it came from (left), the objects the edits changed outlined in red. These are LibreOffice renders, which can differ slightly from PowerPoint (fonts, text wrapping, effects): they show where to look, and PowerPoint is what the check is about. Anything else, such as an object moved that no edit names, a lost picture, a connector come loose or a changed font, is a fail: note the slide.";

/// Before/after PNGs of the decks' edited slides (`render.py`: LibreOffice, then PyMuPDF), when
/// LibreOffice is installed; `OFFICE_KIT_RENDER=0` skips them.
fn render(kit: &Kit) {
    if kit.renders.is_empty() || std::env::var("OFFICE_KIT_RENDER").is_ok_and(|v| v == "0") {
        return;
    }
    let manifest = kit.dir.with_extension("renders.json");
    std::fs::write(&manifest, serde_json::to_string(&kit.renders).unwrap()).unwrap();
    let script = root().join("validation/office-kit/render.py");
    println!("renders");
    match std::process::Command::new("python3").arg(&script).arg(&manifest).arg(&kit.dir).status() {
        Ok(s) if s.success() => {}
        r => println!("  no renders: {} failed ({r:?})", script.display()),
    }
    let _ = std::fs::remove_file(&manifest);
}

fn pngs(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.ends_with(".png"))
        .collect();
    v.sort();
    v
}

fn zip_dir(dir: &Path, names: &[String], out: &Path) {
    let f = std::fs::File::create(out).unwrap();
    let mut z = zip::ZipWriter::new(f);
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for n in names {
        z.start_file(format!("office-kit/{n}"), opts).unwrap();
        z.write_all(&std::fs::read(dir.join(n)).unwrap()).unwrap();
    }
    z.finish().unwrap();
}

fn main() {
    let dir = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| root().join("target/office-kit"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut kit = Kit { dir: dir.clone(), items: vec![], renders: vec![] };
    println!("docx");
    docx(&mut kit);
    println!("pptx");
    pptx(&mut kit);
    println!("xlsx");
    xlsx(&mut kit);
    println!("hwpx");
    hwpx(&mut kit);
    let commit = std::process::Command::new("git")
        .args(["-C", &root().to_string_lossy(), "rev-parse", "--short", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    render(&kit);
    let (md, csv) = checklist(&kit, &commit);
    std::fs::write(dir.join("CHECKLIST.md"), md).unwrap();
    std::fs::write(dir.join("checklist.csv"), csv).unwrap();
    let mut names: Vec<String> =
        kit.items.iter().flat_map(|i| std::iter::once(i.file.clone()).chain(i.md.clone())).collect();
    names.sort();
    names.extend(pngs(&dir));
    names.sort();
    names.extend(["CHECKLIST.md".to_string(), "checklist.csv".to_string()]);
    let zip = dir.with_extension("zip");
    zip_dir(&dir, &names, &zip);
    let per = |f: &str| kit.items.iter().filter(|i| i.format == f).count();
    println!(
        "{}: {} files ({} exports and originals: docx {}, pptx {}, xlsx {}, hwpx {}), {} KB",
        zip.display(),
        names.len(),
        kit.items.len(),
        per("docx"),
        per("pptx"),
        per("xlsx"),
        per("hwpx"),
        std::fs::metadata(&zip).unwrap().len() / 1024
    );
}
