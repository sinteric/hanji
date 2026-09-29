//! Cross-checks the docx corpus test's tracked-change exports with rdocx
//! (DESIGN.md §10.2; prototype/tracked-changes/SPIKE.md, route B):
//!
//! ```sh
//! cargo test --release -p hanji-docx --test corpus corpus_tracked   # writes target/tmp/docx-tracked-out
//! cargo run --release --manifest-path crates/hanji-docx/validate/Cargo.toml [DIR]
//! ```
//!
//! For every tracked export `E…-{C,exact}.docx`: rdocx opens it and lists
//! the author's revisions; rdocx's `accept_revisions_by_author` gives the
//! same text as the test harness's Accept All (`….accepted.docx`, opened
//! with rdocx), and `reject_revisions_by_author` the same as its Reject All.
//! rdocx is not on the export path; it is a second opinion on the markup.

use std::path::{Path, PathBuf};

use rdocx::Document;

const AUTHOR: &str = "hanji (model edit)";

fn text(p: &Path) -> Result<String, String> {
    Document::open(p).map(|d| d.text()).map_err(|e| e.to_string())
}

fn main() {
    let dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../target/tmp/docx-tracked-out"));
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e} (run the docx corpus test first)", dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    let (mut total, mut opened, mut listed, mut acc, mut rej) = (0, 0, 0, 0, 0);
    let mut problems = vec![];
    for d in &dirs {
        let mut files: Vec<PathBuf> = std::fs::read_dir(d)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| {
                let n = p.file_name().unwrap().to_string_lossy();
                n.starts_with('E') && n.ends_with(".docx") && n.matches('.').count() == 1
            })
            .collect();
        files.sort();
        for f in files {
            total += 1;
            let name = format!("{}/{}", d.file_name().unwrap().to_string_lossy(), f.file_name().unwrap().to_string_lossy());
            let doc = match Document::open(&f) {
                Ok(doc) => doc,
                Err(e) => {
                    problems.push(format!("{name}: rdocx does not open it: {e}"));
                    continue;
                }
            };
            opened += 1;
            let mine = doc.revisions().iter().filter(|r| r.author() == AUTHOR).count();
            if mine > 0 {
                listed += 1;
            } else {
                problems.push(format!("{name}: rdocx lists none of the revisions"));
            }
            for (accept, suffix) in [(true, "accepted"), (false, "rejected")] {
                let mut d2 = Document::open(&f).unwrap();
                let r = if accept { d2.accept_revisions_by_author(AUTHOR) } else { d2.reject_revisions_by_author(AUTHOR) };
                let want = text(&f.with_extension(format!("{suffix}.docx")));
                match (r, want) {
                    (Ok(_), Ok(w)) if d2.text() == w => {
                        if accept {
                            acc += 1
                        } else {
                            rej += 1
                        }
                    }
                    (Ok(_), Ok(w)) => {
                        let got = d2.text();
                        let at = got.lines().zip(w.lines()).position(|(a, b)| a != b);
                        let line = at.map(|k| format!("line {}: rdocx {:?} / harness {:?}", k + 1, got.lines().nth(k).unwrap(), w.lines().nth(k).unwrap()));
                        problems.push(format!("{name}: {suffix} text differs: {}", line.unwrap_or_else(|| format!("{} vs {} lines", got.lines().count(), w.lines().count()))));
                    }
                    (Err(e), _) => problems.push(format!("{name}: rdocx {suffix}: {e}")),
                    (_, Err(e)) => problems.push(format!("{name}: harness {suffix} file: {e}")),
                }
            }
        }
    }
    println!(
        "rdocx 0.14.0 over {total} tracked exports: opened {opened}, lists the revisions {listed}, accept by author = harness Accept All {acc}/{opened}, reject by author = harness Reject All {rej}/{opened}"
    );
    for p in problems {
        println!("  {p}");
    }
}
