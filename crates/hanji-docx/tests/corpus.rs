//! §9 on the prototype's 13-file corpus through the shared harness
//! (hanji-testkit): GetPut, the scripted edits E1–E10 with PutGet and
//! remainder outcomes against the oracle, and validity. Set HANJI_REPORT=1
//! to print per-file numbers, HANJI_SOFFICE=1 to also convert every export
//! to PDF with LibreOffice.

use std::path::PathBuf;

use hanji_core::{Block, Engine, EngineError, Entry, ImportOptions, ImportReport, Kind, Remainder};
use hanji_docx::{export_document, write_package, xml, DocxEngine};
use hanji_testkit::{Format, Out};

struct Docx;

const VISIBLE_RPR: &[&str] = &["color", "sz", "highlight", "u", "shd", "strike", "caps", "vertAlign"];

impl Format for Docx {
    fn engine(&self) -> &dyn Engine {
        &DocxEngine
    }
    fn ext(&self) -> &'static str {
        "docx"
    }
    fn split(
        &self,
        pkg: &[u8],
        opts: &ImportOptions,
    ) -> Result<(Vec<Block>, Remainder, ImportReport, String), EngineError> {
        let (b, r, rep, s) = DocxEngine::split(pkg, opts)?;
        Ok((b, r, rep, format!("tables {}/{} kept {:?}", s.tables_modelled, s.tables_kept, s.kept_reasons)))
    }
    fn text_of(&self, blocks: &[Block], rem: &Remainder) -> String {
        DocxEngine::text_of(blocks, rem, None)
    }
    fn export_blocks(&self, blocks: &[Block], rem: &Remainder) -> Result<(Vec<u8>, bool), EngineError> {
        let doc = export_document(blocks, rem)?;
        let well_formed = xml::parse(&doc.document).is_ok();
        Ok((write_package(rem, doc)?, well_formed))
    }
    fn is_split_part(&self, name: &str) -> bool {
        name == "word/document.xml"
    }
    fn visible_run(&self, e: &Entry) -> bool {
        e.kind == Kind::Run
            && e.xml.len() >= 2
            && xml::fragment(&e.xml[1]).elements().any(|c| VISIBLE_RPR.contains(&c.local()))
    }
    fn holds_section(&self, entries: &[&Entry], _: usize) -> bool {
        entries.iter().any(|e| e.kind == Kind::Ppr && e.xml.len() > 1 && e.xml[1].contains("sectPr"))
    }
    fn preferred_styles(&self) -> &'static [&'static str] {
        &["Quote", "Intense Quote", "List Paragraph", "Body Text", "Subtitle", "Title", "Note", "Quotations"]
    }
    fn is_drawing(&self, xml: &str) -> bool {
        xml.contains("drawing") || xml.contains("pict")
    }
}

fn corpus() -> Vec<(String, Vec<u8>)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap().to_path_buf();
    hanji_testkit::corpus(&root.join("prototype/remainder/corpus"), "docx")
}

fn out_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("corpus-out")
}

#[test]
fn corpus_getput_putget_remainder() {
    let files = corpus();
    let sum = hanji_testkit::run_corpus(&Docx, &files, &Out { dir: out_dir(), ext: "docx" });
    #[cfg(not(target_os = "wasi"))]
    if std::env::var("HANJI_SOFFICE").is_ok() {
        soffice();
    }
    assert!(sum.failures.is_empty(), "{}", sum.failures.join("\n"));
    assert_eq!(sum.getput_ok, files.len());
}

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
    let (mut ok, mut total, mut differ) = (0, 0, vec![]);
    for dir in std::fs::read_dir(out_dir()).unwrap() {
        let dir = dir.unwrap().path();
        let files: Vec<PathBuf> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "docx"))
            .collect();
        let pdf = dir.join("pdf");
        let _ = std::fs::create_dir_all(&pdf);
        let _ = Command::new("soffice")
            .args(["--headless", "--convert-to", "pdf", "--outdir"])
            .arg(&pdf)
            .args(&files)
            .output();
        let original = pages(&pdf.join("ORIGINAL.pdf"));
        for f in files.iter().filter(|f| !f.ends_with("ORIGINAL.docx")) {
            total += 1;
            let p = pdf.join(f.with_extension("pdf").file_name().unwrap());
            match pages(&p) {
                Some(n) => {
                    ok += 1;
                    if f.ends_with("getput.docx") && Some(n) != original {
                        differ.push(format!("{}: {n} pages, original {original:?}", dir.display()));
                    }
                }
                None => println!("soffice: failed {}", f.display()),
            }
        }
    }
    println!("soffice: GetPut exports whose page count differs from the original: {differ:?}");
    println!("soffice ({}): {ok}/{total} exports converted to PDF", String::from_utf8_lossy(&v.stdout).trim());
}

#[test]
fn neutralised_import_keeps_getput_for_the_rest() {
    hanji_testkit::neutralised_import_keeps_getput_for_the_rest(&Docx, &corpus());
}

#[test]
fn a_serialized_remainder_exports_the_same_package() {
    hanji_testkit::a_serialized_remainder_exports_the_same_package(&Docx, &corpus());
}
