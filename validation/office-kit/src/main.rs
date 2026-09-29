//! The Office check kit (DESIGN.md §9 validity): exports for a person to open
//! in Word, PowerPoint, Excel and Hancom Office, which this environment
//! cannot run, with a checklist.
//!
//! ```sh
//! cargo run --release --manifest-path validation/office-kit/Cargo.toml [OUT_DIR]
//! ```
//!
//! Per source: the untouched original (`NN-…-original.ext`), then its exports,
//! each edited one with `NN-…-edits.md` beside it (the edits in plain words
//! and the before/after model text diff).
//!
//! Writes `target/office-kit/` (the files, `CHECKLIST.md`, `checklist.csv`)
//! and `target/office-kit.zip` at the workspace root. Every file is an export
//! of a corpus file whose licence allows redistribution (the corpora's
//! SOURCES.md), or of hanji-store's blank package.
//!
//! - docx: per source, GetPut (no edit), E10 (every scripted edit in one
//!   revision), and tracked changes: the scripted edits one at a time, as
//!   many as the tracked export writes (it refuses some, with the reason);
//!   and two new documents from hanji-store's blank docx.
//! - pptx: per deck, GetPut and P9 (title, bullet, notes and shape text
//!   edits, a slide added from a layout, one deleted, one moved, …).
//! - xlsx: per workbook, GetPut and range operations that change the inputs
//!   of formulas, whose cached values the export recomputes.
//! - hwpx: per file, GetPut and E10.

mod formats;
#[path = "../../../crates/hanji-pptx/tests/edits/mod.rs"]
mod pptx_edits;

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use hanji_core::{Engine, ImportOptions, Remainder, TextModel};
use hanji_docx::{DocxEngine, ExportOptions, History, Reviewer};
use hanji_format::sheet::WindowOf;
use hanji_hwpx::HwpxEngine;
use hanji_pptx::PptxEngine;
use hanji_testkit::{edit_jobs, Cx, Doc, Format, CAPS};
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
}

struct Kit {
    dir: PathBuf,
    items: Vec<Item>,
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

/// An exact edit for a local change (the span the texts differ in), a design C rewrite for a move.
fn step(
    model: &dyn TextModel,
    rem: &Remainder,
    text: &str,
    new_text: &str,
    local: bool,
) -> Option<hanji_core::Reanchored> {
    let r = if local {
        let (a, b, repl) = hanji_testkit::differing_span(text, new_text);
        hanji_core::reanchor_span_in(model, rem, text, a, b, &repl, CAPS).ok()?
    } else {
        hanji_core::reanchor_rewrite_in(model, rem, text, new_text, CAPS).ok()?
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
        let jobs = edit_jobs(&Docx, &d, &cx, &text, &hanji_testkit::EDITS, "E10", None);
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
                    "every scripted edit (E1–E9) in one revision, as direct changes",
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
            let Some(r) = step(&hanji_core::DocumentModel, &cur_rem, &cur_text, &new_text, ed.local) else { continue };
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
        assert_eq!(again.text, r.text, "{source}: PutGet");
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
        let model: &dyn TextModel = &hanji_pptx::PptxModel;
        let (mut cur_text, mut cur_rem) = (text.clone(), rem.clone());
        let mut done = vec![];
        for (_, f) in pptx_edits::EDITS {
            let blocks = hanji_testkit::written(&Pptx, &cur_text, &cur_rem);
            let d = Doc { blocks, entries: cur_rem.entries.clone() };
            let Some(ed) = f(&d, &Cx { fmt: &Pptx, rem: &cur_rem }) else { continue };
            let new_text = Pptx.text_of(&ed.blocks, &cur_rem);
            match step(model, &cur_rem, &cur_text, &new_text, ed.local) {
                Some(r) if PptxEngine.export(&r.text, &r.remainder).is_ok() => {
                    done.push(format!("{} ({})", ed.name, ed.what));
                    (cur_text, cur_rem) = (r.text, r.remainder);
                }
                _ => println!("  {name}: left out {} (it would lose content the text does not show)", ed.name),
            }
        }
        if !done.is_empty() {
            let out = PptxEngine.export(&cur_text, &cur_rem).unwrap();
            kit.add(
                "pptx",
                name,
                licence,
                "pset",
                "the pptx edit set, one edit after another: text edits, a slide added from a layout, one deleted, one moved",
                &out,
                vec![ppt.clone(), "Shows the edits listed below (slides added, deleted and moved; text changed)".into()],
                done,
            );
            kit.edits(&text_diff(&text, &cur_text), TEXT_DIFF);
        }
    }
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
             {"op": "append_rows", "table": "Sales", "rows": [{"월": "2026-04", "지점": "강남", "제품군": "가전", "매출": 12400000, "원가": 9000000}]}]"##,
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
        r##"[{"op": "set", "range": "ExcelTable!H3", "values": [[58]]}]"##,
        &[("ExcelTable", "G1:I4")],
    ),
    (
        "simple-monthly-budget.xlsx",
        "Apache-2.0 (Apache POI test data)",
        r##"[{"op": "set", "range": "Simple Monthly Budget!C5", "values": [[3200]]},
             {"op": "set", "range": "Simple Monthly Budget!C11", "values": [[950]]}]"##,
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
        let mut notes = vec![format!("Operations: `{}`", ops.split_whitespace().collect::<Vec<_>>().join(" "))];
        notes.push(
            "Values the export holds (formula results as computed by hanji). A formula cell left blank here had \
             no cached value in the source; Excel computes it when it opens the file."
                .into(),
        );
        // The values before and after, window by window, and the structure text.
        let (mut before, mut after) = (String::new(), String::new());
        for (sheet, range) in *shows {
            let of = WindowOf::Range { sheet: sheet.to_string(), range: range.to_string() };
            match XlsxEngine::window(&a.remainder, &of) {
                Ok(w) => notes.push(format!("```\n{}\n```", w.trim_end())),
                Err(e) => notes.push(format!("({sheet}!{range}: {e})")),
            }
            for (rem, out) in [(&imp.remainder, &mut before), (&a.remainder, &mut after)] {
                out.push_str(&XlsxEngine::window(rem, &of).unwrap_or_default());
                out.push('\n');
            }
        }
        before.push_str(&imp.text);
        after.push_str(&a.text);
        kit.add(
            "xlsx",
            name,
            licence,
            "ops",
            "range operations that change the inputs of formulas",
            &out,
            vec![
                excel.clone(),
                "Formula results show the new values without pressing F9 (compare the values listed below)".into(),
            ],
            notes,
        );
        kit.edits(
            &text_diff(&before, &after),
            "Cell values and model text, before and after (unified diff; the row windows above, then the structure)",
        );
    }
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

fn hwpx(kit: &mut Kit) {
    let hancom = "Opens in Hancom Office without an error".to_string();
    for (name, licence) in HWPX {
        let bytes = read(HWPX_DIR, name);
        let imp = HwpxEngine.import(&bytes, &ImportOptions::default()).unwrap();
        let (text, rem) = (imp.text, imp.remainder);
        let getput = HwpxEngine.export(&text, &rem).unwrap();
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
    for it in &kit.items {
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
    let mut kit = Kit { dir: dir.clone(), items: vec![] };
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
    let (md, csv) = checklist(&kit, &commit);
    std::fs::write(dir.join("CHECKLIST.md"), md).unwrap();
    std::fs::write(dir.join("checklist.csv"), csv).unwrap();
    let mut names: Vec<String> =
        kit.items.iter().flat_map(|i| std::iter::once(i.file.clone()).chain(i.md.clone())).collect();
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
