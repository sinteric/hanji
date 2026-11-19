//! Measures binary HWP through hanji (DESIGN.md §9), file by file:
//!
//! ```sh
//! cargo run --release --manifest-path crates/hanji-hwpx/hwp-roundtrip/Cargo.toml -- DIR [OUT.tsv]
//! ```
//!
//! For every `.hwp` under DIR:
//!
//! 1. **open**: `hanji_hwpx::hwp::to_hwpx` (rhwp reads the HWP and writes
//!    HWPX), with rhwp's content-loss entries and model differences;
//! 2. **import**: hanji's HWPX engine imports that package;
//! 3. **GetPut**: the unchanged text exports the same package (section
//!    parts canonically equal, every other part byte-equal);
//! 4. **edit**: a paragraph appended to the end exports, and re-imports as
//!    the edited text (PutGet);
//! 5. **save .hwp**: `hanji_hwpx::hwp::to_hwp` on the GetPut and the edited
//!    exports (its own check: rhwp's content loss and a re-read). For the
//!    unedited one, rhwp's model of the written .hwp against its model of the
//!    original .hwp; for the edited one, hanji's text of the written .hwp
//!    against the edited text, placeholder ids aside (the check
//!    `hanji-store` makes before it writes an .hwp);
//! 6. **rhwp alone**: rhwp reads the original HWP, writes HWP and reads it
//!    again, the baseline rhwp gives without hanji.
//!
//! Writes one TSV row per file and prints the totals. `HWP_MAX_BYTES` leaves
//! out larger files. A stage that panics
//! counts as a failure of that stage.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use hanji_core::{reanchor_rewrite_in, DocumentModel, Engine, ImportOptions};
use hanji_hwpx::owpml::section_number;
use hanji_hwpx::{hwp, xml, HwpxEngine};
use hanji_package::package;

const EDIT: &str = "hanji 편집 확인 문단";

#[derive(Default, Clone)]
struct Row {
    name: String,
    bytes: usize,
    open: String,
    open_losses: usize,
    import: String,
    lines: usize,
    getput: String,
    edit: String,
    save_unedited: String,
    save_unedited_losses: usize,
    hwp_vs_original: String,
    save_edited: String,
    save_edited_losses: usize,
    edited_text_back: String,
    rhwp_alone: String,
    first_loss: String,
    ms: u128,
}

fn stage<T>(f: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(r) => r,
        Err(p) => Err(format!(
            "panic: {}",
            p.downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default()
        )),
    }
}

fn short(s: &str) -> String {
    let s = s.replace(['\t', '\n'], " ");
    s.chars().take(160).collect()
}

/// The text with each placeholder's id blank (an id hashes the preserved
/// XML, which rhwp writes again with other bytes; hanji-store compares so).
fn without_keep_ids(text: &str) -> String {
    const ID: &str = "<keep id=\"";
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(k) = rest.find(ID) {
        out.push_str(&rest[..k + ID.len()]);
        rest = &rest[k + ID.len()..];
        rest = rest.find('"').map_or("", |e| &rest[e..]);
    }
    out.push_str(rest);
    out
}

fn getput_equal(original: &[u8], out: &[u8]) -> bool {
    let (Ok(a), Ok(b)) = (package::read(original), package::read(out)) else {
        return false;
    };
    let canon = |d: &[u8]| xml::canon_part(d).ok();
    a.len() == b.len()
        && a.iter().zip(&b).all(|(x, y)| {
            x.name == y.name
                && if section_number(&x.name).is_some() {
                    canon(&x.data) == canon(&y.data)
                } else {
                    x.data == y.data
                }
        })
}

fn measure(path: &Path) -> Row {
    let t0 = Instant::now();
    let bytes = std::fs::read(path).unwrap();
    let mut row = Row {
        name: path.file_name().unwrap().to_string_lossy().into_owned(),
        bytes: bytes.len(),
        ..Default::default()
    };
    // 6 first: it needs only rhwp.
    row.rhwp_alone = match stage(|| {
        let a = rhwp::parser::parse_document(&bytes).map_err(|e| e.to_string())?;
        let w = rhwp::serializer::serialize_hwp_with_report(&a).map_err(|e| e.to_string())?;
        let (out, rep) = w.into_parts();
        let b = rhwp::parser::parse_document(&out).map_err(|e| e.to_string())?;
        let d = rhwp::serializer::hwpx::roundtrip::diff_documents(&a, &b);
        Ok((rep.len(), d.differences.len()))
    }) {
        Ok((0, 0)) => "clean".into(),
        Ok((l, d)) => format!("loss:{l}/diff:{d}"),
        Err(e) => format!("fail:{}", short(&e)),
    };
    let conv = match stage(|| hwp::to_hwpx(&bytes).map_err(|e| e.to_string())) {
        Ok(c) => c,
        Err(e) => {
            row.open = format!("fail:{}", short(&e));
            row.ms = t0.elapsed().as_millis();
            return row;
        }
    };
    row.open_losses = conv.losses.len();
    row.open = if conv.losses.is_empty() {
        "clean".into()
    } else {
        "lossy".into()
    };
    if let Some(l) = conv.losses.first() {
        row.first_loss = short(&format!("open: {l}"));
    }
    let hwpx = conv.bytes;
    let imp = match stage(|| {
        HwpxEngine
            .import(&hwpx, &ImportOptions::RAW)
            .map_err(|e| e.to_string())
    }) {
        Ok(i) => i,
        Err(e) => {
            row.import = format!("fail:{}", short(&e));
            row.ms = t0.elapsed().as_millis();
            return row;
        }
    };
    row.import = "ok".into();
    row.lines = imp.text.lines().count();
    let caps = HwpxEngine.capabilities();
    let unedited = stage(|| {
        let re = reanchor_rewrite_in(&DocumentModel, &imp.remainder, &imp.text, &imp.text, caps)
            .map_err(|e| e.to_string())?;
        if !re.report.refused.is_empty() || !re.report.removed.is_empty() {
            return Err(format!("GetPut moved entries: {:?}", re.report));
        }
        HwpxEngine
            .export(&imp.text, &re.remainder)
            .map_err(|e| e.to_string())
    });
    match &unedited {
        Ok(out) if getput_equal(&hwpx, out) => row.getput = "ok".into(),
        Ok(_) => row.getput = "differs".into(),
        Err(e) => row.getput = format!("fail:{}", short(e)),
    }
    let new_text = format!("{}\n\n{EDIT}\n", imp.text.trim_end_matches('\n'));
    let edited = stage(|| {
        let re = reanchor_rewrite_in(&DocumentModel, &imp.remainder, &imp.text, &new_text, caps)
            .map_err(|e| e.to_string())?;
        let out = HwpxEngine
            .export(&new_text, &re.remainder)
            .map_err(|e| e.to_string())?;
        let back = HwpxEngine
            .import(&out, &ImportOptions::RAW)
            .map_err(|e| e.to_string())?;
        if !back.text.contains(EDIT) {
            return Err("the edited export reads back without the edit".into());
        }
        if let Some((a, b)) = new_text
            .lines()
            .zip(back.text.lines())
            .find(|(a, b)| a != b)
        {
            return Err(format!("reads back as other text: {a:?} -> {b:?}"));
        }
        Ok((out, back.text))
    });
    row.edit = match &edited {
        Ok(_) => "ok".into(),
        Err(e) => format!("fail:{}", short(e)),
    };
    if let Ok(out) = &unedited {
        match stage(|| hwp::to_hwp(out).map_err(|e| e.to_string())) {
            Ok(c) => {
                row.save_unedited_losses = c.losses.len();
                row.save_unedited = if c.losses.is_empty() {
                    "clean".into()
                } else {
                    "lossy".into()
                };
                if row.first_loss.is_empty() {
                    if let Some(l) = c.losses.first() {
                        row.first_loss = short(&format!("save: {l}"));
                    }
                }
                row.hwp_vs_original = match stage(|| {
                    let a = rhwp::parser::parse_document(&bytes).map_err(|e| e.to_string())?;
                    let b = rhwp::parser::parse_document(&c.bytes).map_err(|e| e.to_string())?;
                    let d = rhwp::serializer::hwpx::roundtrip::diff_documents(&a, &b);
                    Ok(rhwp::serializer::hwpx::roundtrip::strip_hwpx_to_hwp_noise(d).differences)
                }) {
                    Ok(d) if d.is_empty() => "same".into(),
                    Ok(d) => format!("diff:{}:{}", d.len(), short(&d[0].to_string())),
                    Err(e) => format!("fail:{}", short(&e)),
                };
            }
            Err(e) => row.save_unedited = format!("fail:{}", short(&e)),
        }
    }
    if let Ok((out, edited_text)) = &edited {
        match stage(|| hwp::to_hwp(out).map_err(|e| e.to_string())) {
            Ok(c) => {
                row.save_edited_losses = c.losses.len();
                row.save_edited = if c.losses.is_empty() {
                    "clean".into()
                } else {
                    "lossy".into()
                };
                row.edited_text_back = match stage(|| {
                    let x = hwp::to_hwpx(&c.bytes).map_err(|e| e.to_string())?;
                    let back = HwpxEngine
                        .import(&x.bytes, &ImportOptions::RAW)
                        .map_err(|e| e.to_string())?;
                    Ok(without_keep_ids(&back.text) == without_keep_ids(edited_text))
                }) {
                    Ok(true) => "same".into(),
                    Ok(false) => "differs".into(),
                    Err(e) => format!("fail:{}", short(&e)),
                };
            }
            Err(e) => row.save_edited = format!("fail:{}", short(&e)),
        }
    }
    row.ms = t0.elapsed().as_millis();
    row
}

/// `HWP_MAX_BYTES`: leave out larger files (rhwp lays a document out on every
/// read, so the largest samples take minutes each).
fn max_bytes() -> u64 {
    std::env::var("HWP_MAX_BYTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(u64::MAX)
}

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            files(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("hwp"))
            && p.metadata().is_ok_and(|m| m.len() <= max_bytes())
        {
            out.push(p);
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = PathBuf::from(args.next().expect("usage: hwp-roundtrip DIR [OUT.tsv]"));
    let tsv = args.next().map(PathBuf::from);
    let mut paths = vec![];
    files(&dir, &mut paths);
    paths.sort();
    std::panic::set_hook(Box::new(|_| {}));
    let next = AtomicUsize::new(0);
    let rows = Mutex::new(vec![]);
    let workers = std::thread::available_parallelism().map_or(2, |n| n.get());
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| loop {
                let k = next.fetch_add(1, Ordering::Relaxed);
                let Some(p) = paths.get(k) else { break };
                let r = measure(p);
                eprintln!("{}/{} {} {}ms", k + 1, paths.len(), r.name, r.ms);
                rows.lock().unwrap().push(r);
            });
        }
    });
    let mut rows = rows.into_inner().unwrap();
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    let mut t = String::from("file\tbytes\topen\topen_losses\timport\tlines\tgetput\tedit\tsave_unedited\tsave_unedited_losses\thwp_vs_original\tsave_edited\tsave_edited_losses\tedited_text_back\trhwp_alone\tms\tfirst_loss\n");
    for r in &rows {
        t.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            r.name,
            r.bytes,
            r.open,
            r.open_losses,
            r.import,
            r.lines,
            r.getput,
            r.edit,
            r.save_unedited,
            r.save_unedited_losses,
            r.hwp_vs_original,
            r.save_edited,
            r.save_edited_losses,
            r.edited_text_back,
            r.rhwp_alone,
            r.ms,
            r.first_loss
        ));
    }
    if let Some(p) = &tsv {
        std::fs::write(p, &t).unwrap();
    }
    let n = rows.len();
    let count = |f: &dyn Fn(&Row) -> bool| rows.iter().filter(|r| f(r)).count();
    println!("files: {n}");
    println!(
        "open (rhwp HWP -> HWPX): ok {} (clean {}), failed {}",
        count(&|r| !r.open.starts_with("fail") && !r.open.is_empty()),
        count(&|r| r.open == "clean"),
        count(&|r| r.open.starts_with("fail"))
    );
    println!(
        "hanji import: ok {}, failed {}",
        count(&|r| r.import == "ok"),
        count(&|r| r.import.starts_with("fail"))
    );
    println!(
        "GetPut: ok {}, differs {}, failed {}",
        count(&|r| r.getput == "ok"),
        count(&|r| r.getput == "differs"),
        count(&|r| r.getput.starts_with("fail"))
    );
    println!(
        "edit + PutGet: ok {}, failed {}",
        count(&|r| r.edit == "ok"),
        count(&|r| r.edit.starts_with("fail"))
    );
    println!(
        "save .hwp unedited: clean {}, lossy {}, failed {}",
        count(&|r| r.save_unedited == "clean"),
        count(&|r| r.save_unedited == "lossy"),
        count(&|r| r.save_unedited.starts_with("fail"))
    );
    println!(
        "  rhwp model of saved .hwp = of original .hwp: {}",
        count(&|r| r.hwp_vs_original == "same")
    );
    println!(
        "save .hwp edited: clean {}, lossy {}, failed {}",
        count(&|r| r.save_edited == "clean"),
        count(&|r| r.save_edited == "lossy"),
        count(&|r| r.save_edited.starts_with("fail"))
    );
    println!(
        "  edited .hwp reads back as the edited text: {}",
        count(&|r| r.edited_text_back == "same")
    );
    println!(
        "rhwp alone HWP -> HWP: clean {}, lossy {}, failed {}",
        count(&|r| r.rhwp_alone == "clean"),
        count(&|r| r.rhwp_alone.starts_with("loss")),
        count(&|r| r.rhwp_alone.starts_with("fail"))
    );
}
