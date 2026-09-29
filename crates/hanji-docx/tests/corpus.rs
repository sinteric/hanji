//! §9 on the prototype's 13-file corpus through the shared harness
//! (hanji-testkit): GetPut, the scripted edits E1–E10 with PutGet and
//! remainder outcomes against the oracle, and validity. Set HANJI_REPORT=1
//! to print per-file numbers, HANJI_SOFFICE=1 to also convert every export
//! to PDF with LibreOffice.

mod review;

use std::collections::BTreeMap;
use std::path::PathBuf;

use hanji_core::{Block, Engine, EngineError, Entry, ImportOptions, ImportReport, Kind, Remainder};
use hanji_docx::{export_document, package, write_package, xml, DocxEngine, ExportOptions, History, Reviewer};
use hanji_testkit::{Cx, Doc, Format, Job, Out, CAPS, RAW};

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

/// Pages of a PDF: its `/Type /Page` objects (not `/Pages`).
#[cfg(not(target_os = "wasi"))]
fn pdf_pages(p: &std::path::Path) -> Option<usize> {
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
}

/// Every docx in `dir` converted to PDF by LibreOffice: file stem → pages
/// (`None` when it did not convert). `None` when soffice is not installed.
#[cfg(not(target_os = "wasi"))]
fn convert_dir(dir: &std::path::Path, profile: &str) -> Option<BTreeMap<String, Option<usize>>> {
    use std::process::Command;
    let files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "docx"))
        .collect();
    let pdf = dir.join("pdf");
    let _ = std::fs::create_dir_all(&pdf);
    // A profile of its own, so two tests can convert at once.
    let profile = format!("-env:UserInstallation=file://{}", std::env::temp_dir().join(profile).display());
    Command::new("soffice")
        .arg(profile)
        .args(["--headless", "--convert-to", "pdf", "--outdir"])
        .arg(&pdf)
        .args(&files)
        .output()
        .ok()?;
    let stem = |f: &PathBuf| f.file_name().unwrap().to_string_lossy().trim_end_matches(".docx").to_string();
    Some(files.iter().map(|f| (stem(f), pdf_pages(&pdf.join(format!("{}.pdf", stem(f)))))).collect())
}

#[cfg(not(target_os = "wasi"))]
fn soffice_version() -> Option<String> {
    let v = std::process::Command::new("soffice").arg("--version").output().ok()?;
    Some(String::from_utf8_lossy(&v.stdout).trim().to_string())
}

#[cfg(not(target_os = "wasi"))]
fn soffice() {
    let Some(v) = soffice_version() else {
        println!("soffice: not available");
        return;
    };
    let (mut ok, mut total, mut differ) = (0, 0, vec![]);
    for dir in std::fs::read_dir(out_dir()).unwrap() {
        let dir = dir.unwrap().path();
        let Some(pages) = convert_dir(&dir, "hanji-lo-corpus") else { continue };
        let original = pages.get("ORIGINAL").copied().flatten();
        for (f, n) in pages.iter().filter(|f| f.0 != "ORIGINAL") {
            total += 1;
            match n {
                Some(n) => {
                    ok += 1;
                    if f == "getput" && Some(*n) != original {
                        differ.push(format!("{}: {n} pages, original {original:?}", dir.display()));
                    }
                }
                None => println!("soffice: failed {}/{f}", dir.display()),
            }
        }
    }
    println!("soffice: GetPut exports whose page count differs from the original: {differ:?}");
    println!("soffice ({v}): {ok}/{total} exports converted to PDF");
}

/// The tracked exports: each converts, with the page count of the direct
/// export of the same edit; its accepted form has the direct export's page
/// count and its rejected form the unedited export's.
#[cfg(not(target_os = "wasi"))]
fn soffice_tracked() {
    let Some(v) = soffice_version() else {
        println!("soffice: not available");
        return;
    };
    // (tracked converted, tracked = direct pages, accepted = direct, rejected = unedited)
    let mut n = [0usize; 5];
    let mut differ = vec![];
    for dir in std::fs::read_dir(tracked_dir()).unwrap() {
        let dir = dir.unwrap().path();
        let Some(pages) = convert_dir(&dir, "hanji-lo-tracked") else { continue };
        let getput = pages.get("getput").copied().flatten();
        for (f, p) in &pages {
            if f.contains('.') || f == "getput" {
                continue;
            }
            n[0] += 1;
            let get = |x: &str| pages.get(&format!("{f}.{x}")).copied().flatten();
            let Some(p) = *p else {
                differ.push(format!("{}/{f}: did not convert", dir.display()));
                continue;
            };
            n[1] += 1;
            let direct = get("direct");
            let same = |a: Option<usize>, b: Option<usize>, what: &str, differ: &mut Vec<String>| {
                if a.is_some() && a == b {
                    1
                } else {
                    differ.push(format!("{}/{f}: {what} {a:?} pages, want {b:?}", dir.display()));
                    0
                }
            };
            n[2] += same(Some(p), direct, "tracked", &mut differ);
            n[3] += same(get("accepted"), direct, "accepted", &mut differ);
            n[4] += same(get("rejected"), getput, "rejected", &mut differ);
        }
    }
    println!(
        "soffice ({v}), tracked: {}/{} converted; page count = direct export's {}/{}, accepted = direct {}/{}, rejected = unedited {}/{}",
        n[1], n[0], n[2], n[1], n[3], n[1], n[4], n[1]
    );
    for d in differ {
        println!("  {d}");
    }
}

#[test]
fn neutralised_import_keeps_getput_for_the_rest() {
    hanji_testkit::neutralised_import_keeps_getput_for_the_rest(&Docx, &corpus());
}

#[test]
fn a_serialized_remainder_exports_the_same_package() {
    hanji_testkit::a_serialized_remainder_exports_the_same_package(&Docx, &corpus());
}

fn tracked_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("docx-tracked-out")
}

const AUTHOR: &str = "hanji (model edit)";

fn reviewer() -> ExportOptions {
    ExportOptions { tracked_changes: Some(Reviewer { author: AUTHOR.into(), date: "2026-09-29T00:00:00Z".into() }) }
}

fn text_of_package(pkg: &[u8]) -> Result<String, String> {
    let (b, r, _, _) = DocxEngine::split(pkg, &RAW).map_err(|e| e.to_string())?;
    Ok(DocxEngine::text_of(&b, &r, None))
}

/// §10.2's gate, the automatable part: with the option off, export is
/// unchanged (GetPut, and the same bytes for every edit); with it on, every
/// edit E1–E10 under exact spans and design C either exports, and then its
/// Accept All gives the edited text and its Reject All the imported text, or
/// is refused with a reason. HANJI_REPORT=1 prints each refusal,
/// HANJI_SOFFICE=1 also converts the tracked, direct, accepted and rejected
/// exports with LibreOffice and compares page counts.
#[test]
fn corpus_tracked_changes() {
    let files = corpus();
    let report = std::env::var("HANJI_REPORT").is_ok();
    let out = Out { dir: tracked_dir(), ext: "docx" };
    let _ = std::fs::remove_dir_all(tracked_dir());
    let mut failures = vec![];
    // design → (runs, exported, accept ok, reject ok, well-formed)
    let mut totals: BTreeMap<&str, [usize; 5]> = BTreeMap::new();
    let mut refusals: BTreeMap<String, usize> = BTreeMap::new();
    let mut per_edit: BTreeMap<(String, &str), (usize, usize)> = BTreeMap::new();
    for (name, bytes) in &files {
        let (blocks, rem, _, _) = Docx.split(bytes, &RAW).unwrap();
        let cx = Cx { fmt: &Docx, rem: &rem };
        let d = Doc { blocks, entries: rem.entries.clone() };
        let text = Docx.text_of(&d.blocks, &rem);
        // No edit: the tracked export is the direct one, byte for byte.
        let h0 = History::new(&text, &rem).unwrap();
        let direct = DocxEngine.export(&text, &rem).unwrap();
        let tracked0 = DocxEngine.export_with(&text, &rem, &reviewer(), Some(&h0)).unwrap();
        if tracked0 != direct {
            failures.push(format!("{name}: a tracked export with no edit differs from the direct export"));
        }
        out.save(name, "getput", &direct);
        let jobs = hanji_testkit::edit_jobs(&Docx, &d, &cx, &text, &hanji_testkit::EDITS, "E10", None);
        for Job { name: ename, new_text, span, .. } in jobs {
            let short = ename.split_whitespace().next().unwrap().to_string();
            let mut designs = vec![("C", vec![hanji_core::reanchor_rewrite(&rem, &text, &new_text, CAPS).unwrap()])];
            if let Some((s, e, new)) = &span {
                designs.push(("exact", vec![hanji_core::reanchor_span(&rem, &text, *s, *e, new, CAPS).unwrap()]));
            }
            if short == "E10" {
                // The same edits one at a time, each an exact span where it is local.
                let steps = edit_chain(&text, &rem, true);
                if steps.last().is_some_and(|r| r.text == new_text) {
                    designs.push(("steps", steps));
                } else {
                    failures.push(format!("{name}: the edits one at a time do not give E10's text"));
                }
                // And without the move (E4), which a tracked export refuses when it moves a placeholder.
                designs.push(("steps-no-E4", edit_chain(&text, &rem, false)));
            }
            for (design, steps) in designs {
                let r = steps.last().unwrap();
                let tag = format!("{short}-{design}");
                let t = totals.entry(design).or_default();
                t[0] += 1;
                // Option off: the same bytes as export.
                let off = DocxEngine.export_with(&r.text, &r.remainder, &ExportOptions::default(), None);
                if off.ok() != DocxEngine.export(&r.text, &r.remainder).ok() {
                    failures.push(format!("{name} {tag}: export with the option off differs from export"));
                }
                let mut h = History::new(&text, &rem).unwrap();
                let pkg = steps
                    .iter()
                    .try_for_each(|r| h.push(r))
                    .and_then(|_| DocxEngine.export_with(&r.text, &r.remainder, &reviewer(), Some(&h)));
                let pkg = match pkg {
                    Ok(p) => p,
                    Err(e) => {
                        let why = e.to_string();
                        if report {
                            println!("  {name} {tag}: {why}");
                        }
                        *refusals.entry(format!("[{design}] {}", refusal_kind(&why))).or_default() += 1;
                        continue;
                    }
                };
                t[1] += 1;
                per_edit.entry((short.clone(), design)).or_default().1 += 1;
                out.save(name, &tag, &pkg);
                if let Ok(direct) = DocxEngine.export(&r.text, &r.remainder) {
                    out.save(name, &format!("{tag}.direct"), &direct);
                }
                let parts = package::read(&pkg).unwrap();
                if parts
                    .iter()
                    .filter(|p| p.name.ends_with(".xml") || p.name.ends_with(".rels"))
                    .all(|p| xml::parse(&p.data).is_ok())
                {
                    t[4] += 1;
                } else {
                    failures.push(format!("{name} {tag}: not well-formed"));
                }
                for (accept, want, k, what) in [(true, &r.text, 2, "accepted"), (false, &text, 3, "rejected")] {
                    let got = review::resolve(&pkg, AUTHOR, accept).and_then(|p| {
                        out.save(name, &format!("{tag}.{what}"), &p);
                        let left = review::count(&p, AUTHOR);
                        if left > 0 {
                            return Err(format!("{left} revisions left"));
                        }
                        text_of_package(&p)
                    });
                    match got {
                        Ok(g) if &g == want => t[k] += 1,
                        Ok(g) => failures.push(format!("{name} {tag}: {what} text differs\n{}", first_diff(want, &g))),
                        Err(e) => failures.push(format!("{name} {tag}: {what}: {e}")),
                    }
                }
            }
            for design in ["C", "exact", "steps", "steps-no-E4"] {
                if design == "C"
                    || (design == "exact" && span.is_some())
                    || (design.starts_with("steps") && short == "E10")
                {
                    per_edit.entry((short.clone(), design)).or_default().0 += 1;
                }
            }
        }
    }
    for (design, [n, ex, acc, rej, wf]) in &totals {
        println!("tracked [{design}] runs {n}: exported {ex}, Accept All = edited text {acc}/{ex}, Reject All = imported text {rej}/{ex}, well-formed {wf}/{ex}; refused {}", n - ex);
    }
    for ((ed, design), (n, ex)) in &per_edit {
        println!("  {ed:4} [{design:5}] tracked {ex}/{n}");
    }
    for (why, n) in &refusals {
        println!("  refused {n}: {why}");
    }
    #[cfg(not(target_os = "wasi"))]
    if std::env::var("HANJI_SOFFICE").is_ok() {
        soffice_tracked();
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn first_diff(a: &str, b: &str) -> String {
    for (k, (x, y)) in a.lines().zip(b.lines()).enumerate() {
        if x != y {
            return format!("line {}:\n  want {x:?}\n  got  {y:?}", k + 1);
        }
    }
    format!("{} lines, got {}", a.lines().count(), b.lines().count())
}

/// A refusal's reason without the placeholder it names.
fn refusal_kind(why: &str) -> String {
    let why = why.trim_start_matches("refused: ");
    match why.find("placeholder k") {
        Some(k) => {
            let rest = &why[k..];
            let kind = rest.split_once('(').and_then(|x| x.1.split_once(':')).map_or("", |x| x.0);
            format!("{}placeholder ({kind})", &why[..k])
        }
        None => why.split(':').next().unwrap_or(why).to_string(),
    }
}

/// E1–E9 applied one after another, each against the revision before it:
/// an exact span for a local edit (the region the texts differ in), a
/// rewrite for a move.
fn edit_chain(text: &str, rem: &Remainder, with_move: bool) -> Vec<hanji_core::Reanchored> {
    let (mut text, mut rem) = (text.to_string(), rem.clone());
    let mut out = vec![];
    for (fname, f) in hanji_testkit::EDITS {
        if !with_move && fname == "e4_move_section" {
            continue;
        }
        let blocks = hanji_testkit::written(&Docx, &text, &rem);
        let d = Doc { blocks, entries: rem.entries.clone() };
        let Some(ed) = f(&d, &Cx { fmt: &Docx, rem: &rem }) else { continue };
        let new_text = Docx.text_of(&ed.blocks, &rem);
        let r = if ed.local {
            let pre = text.bytes().zip(new_text.bytes()).take_while(|(a, b)| a == b).count();
            let mut pre = pre;
            while !text.is_char_boundary(pre) || !new_text.is_char_boundary(pre) {
                pre -= 1;
            }
            let max = text.len().min(new_text.len()) - pre;
            let mut suf = text.bytes().rev().zip(new_text.bytes().rev()).take_while(|(a, b)| a == b).count().min(max);
            while !text.is_char_boundary(text.len() - suf) || !new_text.is_char_boundary(new_text.len() - suf) {
                suf -= 1;
            }
            let repl = &new_text[pre..new_text.len() - suf];
            hanji_core::reanchor_span(&rem, &text, pre, text.len() - suf, repl, CAPS).unwrap()
        } else {
            hanji_core::reanchor_rewrite(&rem, &text, &new_text, CAPS).unwrap()
        };
        assert_eq!(r.text, new_text);
        (text, rem) = (r.text.clone(), r.remainder.clone());
        out.push(r);
    }
    out
}
