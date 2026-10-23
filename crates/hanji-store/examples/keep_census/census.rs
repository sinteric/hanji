//! The census itself: open every file of the GetPut/PutGet corpora, count
//! the `<keep/>` placeholders in its text, and write the report. The example
//! (`main.rs`) runs it, and `tests/keep_census.rs` checks the committed report
//! against it.

use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use hanji_core::ImportOptions;
use hanji_store::Format;
use serde::Serialize;

/// The command that regenerates the committed report.
pub const COMMAND: &str = "cargo run --locked -p hanji-store --example keep_census -- --write";

/// The committed report, from the repository root.
pub const MARKDOWN: &str = "docs/keep-census.md";
/// The same numbers for tools.
pub const JSON: &str = "docs/keep-census.json";

/// The corpora, as the corpus tests read them: a directory from the
/// repository root and the file extensions taken from it.
const CORPORA: &[(&str, &[&str])] = &[
    // hanji-docx/tests/corpus.rs
    ("prototype/remainder/corpus", &["docx"]),
    // hanji-hwpx/tests/corpus.rs
    ("crates/hanji-hwpx/corpus", &["hwpx"]),
    // hanji-pptx/tests/corpus.rs: the corpus and the audit decks
    ("crates/hanji-pptx/corpus", &["pptx", "pptm"]),
    ("crates/hanji-pptx/corpus/audit", &["pptx"]),
    // hanji-xlsx/tests/common/mod.rs
    ("crates/hanji-xlsx/corpus", &["xlsx", "xlsm"]),
];

#[derive(Serialize)]
pub struct Census {
    /// How to regenerate it.
    command: &'static str,
    corpora: Vec<CorpusCount>,
    total: Count,
    formats: Vec<FormatCount>,
    /// Most first.
    kinds: Vec<KindCount>,
    files: Vec<FileCount>,
}

#[derive(Serialize)]
struct CorpusCount {
    /// `dir/*.ext`: one entry per extension.
    corpus: String,
    files: usize,
}

#[derive(Default, Serialize)]
struct Count {
    files: usize,
    failed: usize,
    with_keeps: usize,
    keeps: usize,
}

#[derive(Serialize)]
struct FormatCount {
    format: &'static str,
    #[serde(flatten)]
    count: Count,
}

#[derive(Serialize)]
struct KindCount {
    format: &'static str,
    kind: String,
    keeps: usize,
    /// The files that show the kind at least once.
    files: usize,
}

#[derive(Serialize)]
struct FileCount {
    /// From the repository root.
    path: String,
    format: &'static str,
    #[serde(flatten)]
    outcome: Outcome,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Outcome {
    Opened {
        keeps: usize,
        kinds: BTreeMap<String, usize>,
    },
    Failed {
        error: String,
    },
}

/// The repository root.
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("hanji-store sits in crates/ under the repository root")
        .to_path_buf()
}

/// Open every corpus file under `root` and count its placeholders. A file
/// that cannot be read or opened is recorded with the reason; it does not
/// stop the census.
pub fn take(root: &Path) -> Census {
    let mut corpora = vec![];
    let mut files = vec![];
    for (dir, exts) in CORPORA {
        for ext in *exts {
            let names = files_in(&root.join(dir), ext);
            corpora.push(CorpusCount {
                corpus: format!("{dir}/*.{ext}"),
                files: names.len(),
            });
            for name in names {
                let path = format!("{dir}/{name}");
                let format =
                    Format::of_name(&name).expect("the corpora hold only extensions hanji opens");
                let outcome = open(&root.join(&path), format)
                    .unwrap_or_else(|error| Outcome::Failed { error });
                files.push(FileCount {
                    path,
                    format: format.name(),
                    outcome,
                });
            }
        }
    }
    files.sort_by(|a, b| (a.format, &a.path).cmp(&(b.format, &b.path)));
    let mut total = Count::default();
    let mut formats: Vec<FormatCount> = Format::ALL
        .iter()
        .map(|f| FormatCount {
            format: f.name(),
            count: Count::default(),
        })
        .collect();
    let mut kinds: BTreeMap<(&'static str, String), (usize, usize)> = BTreeMap::new();
    for file in &files {
        let at = formats
            .iter_mut()
            .find(|f| f.format == file.format)
            .expect("every format is in Format::ALL");
        for count in [&mut total, &mut at.count] {
            count.files += 1;
            match &file.outcome {
                Outcome::Opened { keeps, .. } => {
                    count.keeps += keeps;
                    count.with_keeps += usize::from(*keeps > 0);
                }
                Outcome::Failed { .. } => count.failed += 1,
            }
        }
        if let Outcome::Opened { kinds: of_file, .. } = &file.outcome {
            for (kind, n) in of_file {
                let k = kinds.entry((file.format, kind.clone())).or_default();
                k.0 += n;
                k.1 += 1;
            }
        }
    }
    let mut kinds: Vec<KindCount> = kinds
        .into_iter()
        .map(|((format, kind), (keeps, files))| KindCount {
            format,
            kind,
            keeps,
            files,
        })
        .collect();
    // The map gave (format, kind) order; a stable sort keeps it for ties.
    kinds.sort_by_key(|k| Reverse(k.keeps));
    Census {
        command: COMMAND,
        corpora,
        total,
        formats,
        kinds,
        files,
    }
}

/// The names of the files with extension `ext` in `dir`, sorted.
fn files_in(dir: &Path, ext: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("corpus directory {}: {e}", dir.display()))
        .map(|e| e.expect("a corpus directory entry").path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == ext))
        .map(|p| {
            p.file_name()
                .expect("a file has a name")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

/// Open the file as `hanji open` does (the store's import: the engine of the
/// format its extension names, §8 neutralisation on) and count the
/// placeholders its text shows, by kind.
fn open(path: &Path, format: Format) -> Result<Outcome, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read: {e}"))?;
    // A panic in an engine is a bug in hanji: the census reports it as this
    // file's failure instead of stopping, as a host may at its own boundary
    // (STYLE.md, "Don't call catch_unwind").
    let imported = panic::catch_unwind(AssertUnwindSafe(|| {
        format.engine().import(&bytes, &ImportOptions::default())
    }))
    .map_err(|payload| {
        let message = payload
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_default();
        format!("panic: {message}")
    })?
    .map_err(|e| format!("open: {e}"))?;
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    for kind in keep_kinds(&imported.text)? {
        *kinds.entry(kind).or_default() += 1;
    }
    Ok(Outcome::Opened {
        keeps: kinds.values().sum(),
        kinds,
    })
}

/// The kind of every placeholder in a model text, in order. A placeholder is
/// a `<keep …/>` tag the text does not escape (text writes a literal one as
/// `\<keep`); its values are quoted and hold no raw `"` or `<` (`&quot;`,
/// `&lt;`). This reads every type's text alike: a Document's block and inline
/// placeholders, a Presentation's object lines (inside groups too, which
/// carry their shape id and have no remainder entry of their own), and a
/// Spreadsheet's structure.
pub fn keep_kinds(text: &str) -> Result<Vec<String>, String> {
    let mut kinds = vec![];
    for (at, _) in text.match_indices("<keep") {
        let escapes = text[..at].bytes().rev().take_while(|&b| b == b'\\').count();
        let rest = &text[at + "<keep".len()..];
        if escapes % 2 == 1 || rest.starts_with(|c: char| c.is_alphanumeric() || c == '-') {
            continue;
        }
        let line = text[..at].matches('\n').count() + 1;
        let attrs = tag_attrs(rest).map_err(|e| format!("line {line}: a <keep> tag {e}"))?;
        match (attrs.get("id"), attrs.get("kind")) {
            (Some(_), Some(kind)) => kinds.push(kind.clone()),
            _ => {
                return Err(format!(
                    "line {line}: a <keep> tag without an id and a kind"
                ))
            }
        }
    }
    Ok(kinds)
}

/// The attributes of a tag, from just after its name to its `/>`.
fn tag_attrs(mut s: &str) -> Result<BTreeMap<String, String>, &'static str> {
    let mut attrs = BTreeMap::new();
    loop {
        s = s.trim_start();
        if s.starts_with("/>") {
            return Ok(attrs);
        }
        let (name, after) = s.split_once("=\"").ok_or("is not closed by />")?;
        if name.is_empty() || name.contains(|c: char| c.is_whitespace() || c == '/' || c == '>') {
            return Err("has an attribute without a quoted value");
        }
        let (value, after) = after.split_once('"').ok_or("has an unclosed value")?;
        let value = value
            .replace("&quot;", "\"")
            .replace("&lt;", "<")
            .replace("&amp;", "&");
        attrs.insert(name.to_string(), value);
        s = after;
    }
}

/// The report as JSON.
pub fn json(census: &Census) -> String {
    let mut s = serde_json::to_string_pretty(census).expect("a census serializes");
    s.push('\n');
    s
}

/// The report as Markdown, its tables padded so it reads as plain text too.
pub fn markdown(census: &Census) -> String {
    let mut out = format!(
        "# `<keep/>` census

<!-- Generated by `{COMMAND}`. Do not edit. -->

How many `<keep/>` placeholders hanji's text shows for the files of the
GetPut/PutGet corpora (README.md, \"What is verified\"): the content hanji
cannot edit yet. Fewer is better.

Each file is opened as `hanji open` opens it: by its extension, with §8
neutralisation, so removed content (macros, OLE objects, ActiveX) is not
counted. Every `<keep …/>` tag in its text counts once under its `kind`:
Document placeholders block and inline, Presentation object lines (objects
inside groups too), Spreadsheet structure lines. A file that fails to open
is listed under Failures and not counted.

After a change that adds or removes placeholders, regenerate this file and
`keep-census.json` (the same numbers, for tools) from the repository root:

```sh
{COMMAND}
```

`cargo test -p hanji-store --test keep_census` fails while they are out of
date.

## Corpora

"
    );
    out += &table(
        &[("corpus", false), ("files", true)],
        census
            .corpora
            .iter()
            .map(|c| vec![format!("`{}`", c.corpus), c.files.to_string()]),
    );
    out += "\n## By format\n\n";
    let count_row = |name: &str, c: &Count| {
        vec![
            name.to_string(),
            c.files.to_string(),
            c.failed.to_string(),
            c.with_keeps.to_string(),
            c.keeps.to_string(),
        ]
    };
    out += &table(
        &[
            ("format", false),
            ("files", true),
            ("failed", true),
            ("with keeps", true),
            ("keeps", true),
        ],
        census
            .formats
            .iter()
            .map(|f| count_row(f.format, &f.count))
            .chain([count_row("all", &census.total)]),
    );
    out += "\n## By kind, most first\n\n";
    out += &table(
        &[
            ("format", false),
            ("kind", false),
            ("keeps", true),
            ("files", true),
        ],
        census.kinds.iter().map(|k| {
            vec![
                k.format.to_string(),
                k.kind.clone(),
                k.keeps.to_string(),
                k.files.to_string(),
            ]
        }),
    );
    out += "\n## By file\n\n";
    out += &table(
        &[
            ("file", false),
            ("format", false),
            ("keeps", true),
            ("kinds", false),
        ],
        census.files.iter().map(|f| {
            let (keeps, kinds) = match &f.outcome {
                Outcome::Opened { keeps, kinds } => (
                    keeps.to_string(),
                    kinds
                        .iter()
                        .map(|(kind, n)| format!("{kind} {n}"))
                        .collect::<Vec<_>>()
                        .join(", "),
                ),
                Outcome::Failed { .. } => ("—".to_string(), "failed to open".to_string()),
            };
            vec![format!("`{}`", f.path), f.format.to_string(), keeps, kinds]
        }),
    );
    out += "\n## Failures\n\n";
    let failures: Vec<String> = census
        .files
        .iter()
        .filter_map(|f| match &f.outcome {
            Outcome::Failed { error } => {
                Some(format!("- `{}`: {}\n", f.path, error.replace('\n', " ")))
            }
            Outcome::Opened { .. } => None,
        })
        .collect();
    if failures.is_empty() {
        out += "None: every file opened.\n";
    } else {
        out += &failures.concat();
    }
    out
}

/// A Markdown table, each column padded to its widest cell; a `true` column
/// is numbers, aligned right. A last column aligned left is not padded: it
/// would only end each line in spaces.
fn table(head: &[(&str, bool)], rows: impl Iterator<Item = Vec<String>>) -> String {
    let rows: Vec<Vec<String>> = rows.collect();
    let width = |i: usize| {
        let cells = rows.iter().map(|r| r[i].chars().count());
        let last_left = i + 1 == head.len() && !head[i].1;
        (if last_left { None } else { cells.max() })
            .unwrap_or(0)
            .max(head[i].0.chars().count())
            .max(3)
    };
    let widths: Vec<usize> = (0..head.len()).map(width).collect();
    let line = |cells: Vec<String>| {
        let cells: Vec<String> = cells
            .iter()
            .zip(head)
            .zip(&widths)
            .map(|((c, (_, right)), &w)| {
                let pad = " ".repeat(w.saturating_sub(c.chars().count()));
                if *right {
                    format!("{pad}{c}")
                } else {
                    format!("{c}{pad}")
                }
            })
            .collect();
        format!("| {} |\n", cells.join(" | ").trim_end())
    };
    let mut out = line(head.iter().map(|(h, _)| h.to_string()).collect());
    out += &line(
        head.iter()
            .zip(&widths)
            .map(|((_, right), &w)| {
                if *right {
                    format!("{}:", "-".repeat(w - 1))
                } else {
                    "-".repeat(w)
                }
            })
            .collect(),
    );
    for r in rows {
        out += &line(r);
    }
    out
}
