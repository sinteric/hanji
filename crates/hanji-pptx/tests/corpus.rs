//! §9 on the pptx corpus (`corpus/`, see its SOURCES.md) through the shared
//! harness (hanji-testkit): GetPut, the pptx edit set P1–P9 with PutGet and
//! remainder outcomes against the oracle, under design C and exact spans, and
//! validity. Set HANJI_REPORT=1 to print per-file numbers, HANJI_SOFFICE=1 to
//! also convert every export to PDF with LibreOffice and compare slide counts.

use std::path::PathBuf;

use hanji_core::{Block, Engine, EngineError, Entry, ImportOptions, ImportReport, Kind, Remainder, TextModel};
use hanji_pptx::{export_parts, write_package, xml, PptxEngine, PptxModel};
use hanji_testkit::{Format, Out};

mod edits;

pub struct Pptx;

/// `a:rPr` attributes that change nothing a person sees.
const QUIET_RPR: &[&str] =
    &["lang", "altLang", "dirty", "err", "noProof", "smtClean", "smtId", "b", "i", "u", "strike", "bmk"];

impl Format for Pptx {
    fn engine(&self) -> &dyn Engine {
        &PptxEngine
    }
    fn ext(&self) -> &'static str {
        "pptx"
    }
    fn split(
        &self,
        pkg: &[u8],
        opts: &ImportOptions,
    ) -> Result<(Vec<Block>, Remainder, ImportReport, String), EngineError> {
        let (b, r, rep, s) = PptxEngine::split(pkg, opts)?;
        let line = format!(
            "slides {} slots {} (objects {}) shapes {} latent {} notes {} kept {} list items {}",
            s.slides, s.slots, s.keep_slots, s.shapes, s.latent, s.notes, s.kept, s.list_items
        );
        Ok((b, r, rep, line))
    }
    fn text_of(&self, blocks: &[Block], rem: &Remainder) -> String {
        PptxEngine::text_of(blocks, rem, None)
    }
    fn export_blocks(&self, blocks: &[Block], rem: &Remainder) -> Result<(Vec<u8>, bool), EngineError> {
        let parts = export_parts(blocks, rem)?;
        let well_formed = parts
            .iter()
            .filter(|p| p.name.ends_with(".xml") || p.name.ends_with(".rels"))
            .all(|p| xml::parse(&p.data).is_ok());
        Ok((write_package(&parts)?, well_formed))
    }
    fn is_split_part(&self, name: &str) -> bool {
        name == "ppt/presentation.xml"
            || numbered(name, "ppt/slides/slide")
            || numbered(name, "ppt/notesSlides/notesSlide")
    }
    fn visible_run(&self, e: &Entry) -> bool {
        e.kind == Kind::Run
            && e.xml.get(1).is_some_and(|x| {
                let r = xml::fragment(x);
                r.has_elements() || r.attrs.iter().any(|a| !QUIET_RPR.contains(&a.0.as_str()))
            })
    }
    fn holds_section(&self, _: &[&Entry], _: usize) -> bool {
        false
    }
    fn preferred_styles(&self) -> &'static [&'static str] {
        &[]
    }
    fn is_drawing(&self, xml: &str) -> bool {
        xml.contains("<p:pic") || xml.contains("<p:graphicFrame")
    }
    fn model(&self) -> &dyn TextModel {
        &PptxModel
    }
}

fn numbered(name: &str, base: &str) -> bool {
    name.strip_prefix(base).and_then(|r| r.strip_suffix(".xml")).is_some_and(|n| n.parse::<u32>().is_ok())
}

pub fn corpus() -> Vec<(String, Vec<u8>)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus");
    let mut files = hanji_testkit::corpus(&dir, "pptx");
    files.extend(hanji_testkit::corpus(&dir, "pptm"));
    files
}

fn out_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("pptx-corpus-out")
}

#[test]
fn corpus_getput_putget_remainder() {
    let files = corpus();
    assert!(files.len() >= 10, "the corpus has {} files", files.len());
    let sum = hanji_testkit::run_corpus_with(&Pptx, &files, &Out { dir: out_dir(), ext: "pptx" }, &edits::EDITS, "P9");
    #[cfg(not(target_os = "wasi"))]
    if std::env::var("HANJI_SOFFICE").is_ok() {
        soffice();
    }
    assert!(sum.failures.is_empty(), "{}", sum.failures.join("\n"));
    assert_eq!(sum.getput_ok, files.len());
    // Exact spans lose nothing.
    assert_eq!(sum.touched.get("exact").map_or(0, |t| t[2]), 0, "exact spans lost entries");
}

/// Every export converts to PDF with LibreOffice, one page per slide it
/// shows (its slides not hidden).
#[cfg(not(target_os = "wasi"))]
fn soffice() {
    use std::process::Command;
    let Ok(v) = Command::new("soffice").arg("--version").output() else {
        println!("soffice: not available");
        return;
    };
    // `/Type /Page` objects (not `/Pages`).
    let pages = |p: &std::path::Path| -> Option<usize> {
        let d = std::fs::read(p).ok().filter(|d| !d.is_empty())?;
        let mut n = 0;
        for k in 0..d.len().saturating_sub(5) {
            if &d[k..k + 5] == b"/Type" {
                let mut x = k + 5;
                while d.get(x).is_some_and(|c| c.is_ascii_whitespace()) {
                    x += 1;
                }
                if d[x..].starts_with(b"/Page") && d.get(x + 5) != Some(&b's') {
                    n += 1;
                }
            }
        }
        Some(n)
    };
    let slides = |p: &std::path::Path| -> Option<usize> {
        let parts = hanji_pptx::package::read(&std::fs::read(p).ok()?).ok()?;
        let pres = parts.iter().find(|x| x.name == "ppt/presentation.xml")?;
        let d = xml::parse(&pres.data).ok()?;
        let ids: Vec<String> = d
            .root
            .child("p:sldIdLst")
            .map(|l| l.elements().filter_map(|e| e.get("r:id")).collect())
            .unwrap_or_default();
        // LibreOffice leaves hidden slides (`show="0"`) out of the PDF.
        let shown = |id: &String| {
            let part = hanji_package::opc::target_of(&parts, &pres.name, id)?;
            let d = xml::parse(hanji_pptx::package::get(&parts, &part)?).ok()?;
            Some(d.root.get("show").as_deref() != Some("0"))
        };
        let mut n = 0;
        for id in &ids {
            n += usize::from(shown(id)?);
        }
        Some(n)
    };
    let (mut ok, mut total, mut differ) = (0, 0, vec![]);
    for dir in std::fs::read_dir(out_dir()).unwrap() {
        let dir = dir.unwrap().path();
        let files: Vec<PathBuf> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "pptx"))
            .collect();
        let pdf = dir.join("pdf");
        let _ = std::fs::create_dir_all(&pdf);
        let _ = Command::new("soffice")
            .args(["--headless", "--convert-to", "pdf", "--outdir"])
            .arg(&pdf)
            .args(&files)
            .output();
        for f in &files {
            total += 1;
            let p = pdf.join(f.with_extension("pdf").file_name().unwrap());
            match (pages(&p), slides(f)) {
                (Some(n), Some(s)) => {
                    ok += 1;
                    if n != s {
                        differ.push(format!("{}: {n} pages, {s} slides", f.display()));
                    }
                }
                _ => println!("soffice: failed {}", f.display()),
            }
        }
    }
    println!("soffice: exports whose page count differs from their slide count: {differ:?}");
    println!("soffice ({}): {ok}/{total} exports converted to PDF", String::from_utf8_lossy(&v.stdout).trim());
    assert_eq!(ok, total, "LibreOffice (with Impress) converts every export");
    assert!(differ.is_empty(), "{differ:?}");
}

#[test]
fn neutralised_import_keeps_getput_for_the_rest() {
    hanji_testkit::neutralised_import_keeps_getput_for_the_rest(&Pptx, &corpus());
}

#[test]
fn a_serialized_remainder_exports_the_same_package() {
    hanji_testkit::a_serialized_remainder_exports_the_same_package(&Pptx, &corpus());
}
