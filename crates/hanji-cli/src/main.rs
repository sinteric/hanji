//! `hanji`: the operations of `hanji-store` as subcommands. Output is for
//! people by default and JSON with `--json`; a refusal exits with status 1
//! and says why (DESIGN.md §2 rule 1).

use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use hanji_preview::store::{Output, XlsxSelection};
use hanji_store::{
    Changed, DocType, Error, ExportOptions, Format, FsStorage, ImportReport, MemStorage, Result, TextEdit, Window,
    Workspace,
};
use serde::Serialize;

#[derive(Parser)]
#[command(name = "hanji", version, about = "Read, write and edit office documents (docx, hwpx, pptx, xlsx) as text")]
struct Cli {
    /// Where documents and their revisions are kept.
    #[arg(long, global = true, env = "HANJI_STORE", default_value = ".hanji")]
    store: PathBuf,
    /// Print JSON instead of text for people.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Open a file as a new document (active and remote content is neutralised).
    Open { path: PathBuf },
    /// Make a new document from a blank package or a template.
    New {
        /// document, presentation or spreadsheet
        #[arg(value_parser = parse_type)]
        r#type: DocType,
        /// docx or hwpx (document), pptx, xlsx; by default docx, pptx, xlsx
        #[arg(long, value_parser = parse_format)]
        format: Option<Format>,
        /// A package to start from: its styles, layouts and content.
        #[arg(long)]
        template: Option<PathBuf>,
    },
    /// Print a revision's text (the current one by default), or a part of it.
    Read {
        doc: String,
        #[arg(long)]
        rev: Option<u32>,
        #[command(flatten)]
        window: WindowArgs,
    },
    /// Replace the one occurrence of old with new (an exact span of the revision).
    Edit {
        doc: String,
        /// The revision the edit was made against.
        #[arg(long)]
        rev: u32,
        #[arg(long, requires = "new", conflicts_with_all = ["old_file", "edits"])]
        old: Option<String>,
        #[arg(long, requires = "old", conflicts_with_all = ["new_file", "edits"])]
        new: Option<String>,
        #[arg(long, requires = "new_file", conflicts_with = "edits")]
        old_file: Option<PathBuf>,
        #[arg(long, requires = "old_file", conflicts_with = "edits")]
        new_file: Option<PathBuf>,
        /// A JSON file with a list of {"old": …, "new": …}, applied in order ("-": stdin).
        #[arg(long)]
        edits: Option<PathBuf>,
    },
    /// Replace the whole text (a file, or "-" for stdin).
    Write {
        doc: String,
        #[arg(long)]
        rev: u32,
        file: PathBuf,
    },
    /// Apply range operations to a spreadsheet: a JSON list, from a file, "-" (stdin) or inline.
    Ops {
        doc: String,
        #[arg(long)]
        rev: u32,
        ops: String,
    },
    /// Check a text: its grammar, and a document's names with --doc.
    Validate {
        /// The text file ("-": stdin).
        file: PathBuf,
        #[arg(long = "type", value_parser = parse_type)]
        ty: Option<DocType>,
        #[arg(long)]
        doc: Option<String>,
    },
    /// Export a revision to a file.
    Export {
        doc: String,
        path: PathBuf,
        #[arg(long)]
        rev: Option<u32>,
        /// Export although hidden text, comments, tracked deletions or metadata would leave with the file.
        #[arg(long)]
        acknowledge_surfaced: bool,
        /// Write the edits since the file was opened or re-imported as tracked changes (docx).
        #[arg(long)]
        tracked_changes: bool,
    },
    /// Render PPTX slides, experimental DOCX/HWPX pages or an XLSX window to HTML, SVG or PNG.
    Preview {
        /// A stored document or a .docx/.pptx/.xlsx/.hwpx file (file previews are not stored).
        doc: String,
        #[arg(long)]
        rev: Option<u32>,
        /// The directory to write to; by default the document's file's directory (or this one).
        #[arg(long)]
        out: Option<PathBuf>,
        /// html (one viewer file), svg or png (one file per slide or worksheet window).
        #[arg(long, default_value = "html", value_parser = parse_output)]
        format: Output,
        /// A font directory or file, searched before $HANJI_FONT_DIR and the system fonts (repeatable).
        #[arg(long = "font-dir")]
        font_dirs: Vec<PathBuf>,
        /// XLSX worksheet name (exact); by default the first visible worksheet.
        #[arg(long, conflicts_with = "sheet_index")]
        sheet: Option<String>,
        /// XLSX sheet number, 1-based (including hidden/non-worksheet sheets).
        #[arg(long, conflicts_with = "sheet")]
        sheet_index: Option<usize>,
        /// XLSX A1 window, e.g. A1:H40; default A1:L40. Cached formula results only.
        /// Limits: 512 rows, 128 columns, 32768 cells; merges must fit the window.
        #[arg(long)]
        range: Option<String>,
    },
    /// Bring a person's edits of an exported file back in as a new revision.
    Reimport { doc: String, path: PathBuf },
    /// List a document's revisions.
    History { doc: String },
    /// The text diff between two revisions.
    Diff { doc: String, from: u32, to: u32 },
    /// List the documents in the store.
    List,
    /// How to work with hanji and the format, for a model.
    Guide,
}

#[derive(clap::Args)]
struct WindowArgs {
    /// Line range, 1-based and inclusive: 120:180
    #[arg(long)]
    lines: Option<String>,
    /// A document's section, by its heading text
    #[arg(long)]
    section: Option<String>,
    /// A presentation's slides: 3:5
    #[arg(long)]
    slides: Option<String>,
    /// A spreadsheet table whose rows to show
    #[arg(long)]
    table: Option<String>,
    /// Sheet rows of --table: 2:101
    #[arg(long)]
    rows: Option<String>,
    /// A sheet whose --range to show
    #[arg(long)]
    sheet: Option<String>,
    /// Cells of --sheet: A1:F50
    #[arg(long)]
    range: Option<String>,
}

fn parse_type(s: &str) -> std::result::Result<DocType, String> {
    DocType::parse(s).ok_or_else(|| format!("{s:?} is not a type: document, presentation or spreadsheet"))
}

fn parse_format(s: &str) -> std::result::Result<Format, String> {
    Format::parse(s).ok_or_else(|| format!("{s:?} is not a format: docx, hwpx, pptx or xlsx"))
}

fn parse_output(s: &str) -> std::result::Result<Output, String> {
    Output::parse(s).ok_or_else(|| format!("{s:?} is not a preview format: html, svg or png"))
}

/// Thin native adapter: selection, revision/file routing and output reporting.
fn preview(
    ws: &Workspace<FsStorage>,
    doc: &str,
    rev: Option<u32>,
    out_dir: Option<PathBuf>,
    format: Output,
    font_dirs: Vec<PathBuf>,
    selection: XlsxSelection,
) -> Result<Out> {
    let opts = hanji_preview::Options::with_font_dirs(font_dirs);
    let path = Path::new(doc);
    let file_format = path.is_file().then(|| Format::of_name(doc)).flatten();
    if file_format.is_some() && rev.is_some() {
        return Err(Error::bad("--rev names a revision of a stored document; a file is previewed as it is."));
    }
    let input_format = match file_format {
        Some(format) => format,
        None => ws.doc(doc)?.format,
    };
    hanji_preview::store::check_format(input_format)?;
    if input_format == Format::Xlsx {
        let (mut p, window, dir) = if file_format.is_some() {
            let (p, window) = hanji_preview::store::render_xlsx_file(path, &selection, &opts)?;
            let parent = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
            (p, window, out_dir.unwrap_or(parent))
        } else {
            let (p, window) = hanji_preview::store::render_xlsx(ws, doc, rev, &selection, &opts)?;
            let dir = out_dir.unwrap_or_else(|| hanji_preview::store::default_dir(ws, doc, Path::new(".")));
            (p, window, dir)
        };
        hanji_preview::store::write_xlsx(&window, &mut p, format, &dir)?;
        let mut t = p.files.iter().map(|f| format!("wrote {f}\n")).collect::<String>();
        t.push_str(&format!(
            "worksheet {:?} ({}), range {} from revision {} of {}; {}\n",
            p.sheet.name,
            p.sheet.index + 1,
            p.range,
            p.revision,
            p.doc_id,
            p.summary
        ));
        for w in &p.warnings {
            t.push_str(&format!("warning: {w}\n"));
        }
        return Ok(out(&p, t));
    }
    if selection.is_requested() {
        return Err(Error::bad("--sheet, --sheet-index and --range apply only to XLSX worksheet previews"));
    }
    let (mut p, preview, dir) = if file_format.is_some() {
        let mut mem = Workspace::new(MemStorage::new());
        let bytes = std::fs::read(path).map_err(|e| Error::io(format!("cannot read {}: {e}", path.display())))?;
        let name = path.file_name().map_or_else(|| doc.to_string(), |n| n.to_string_lossy().into_owned());
        let o = mem.open_bytes(&name, &bytes, None)?;
        let (p, preview) = hanji_preview::store::render_document(&mem, &o.doc_id, None, &opts)?;
        let parent =
            path.parent().filter(|p| !p.as_os_str().is_empty()).map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        (p, preview, out_dir.unwrap_or(parent))
    } else {
        let (p, preview) = hanji_preview::store::render_document(ws, doc, rev, &opts)?;
        let dir = out_dir.unwrap_or_else(|| hanji_preview::store::default_dir(ws, doc, Path::new(".")));
        (p, preview, dir)
    };
    hanji_preview::store::write_document(&preview, &mut p, format, &dir)?;
    let mut t = String::new();
    for f in &p.files {
        t.push_str(&format!("wrote {f}\n"));
    }
    let unit = if p.format == Format::Pptx { "slides" } else { "pages" };
    t.push_str(&format!("{} {unit} from revision {} of {}; {}\n", p.pages, p.revision, p.doc_id, p.summary));
    for w in &p.warnings {
        t.push_str(&format!("warning: {w}\n"));
    }
    Ok(out(&p, t))
}

fn read_input(path: &Path) -> Result<String> {
    let mut s = String::new();
    if path == Path::new("-") {
        std::io::stdin().read_to_string(&mut s).map_err(|e| Error::io(format!("cannot read stdin: {e}")))?;
    } else {
        s = std::fs::read_to_string(path).map_err(|e| Error::io(format!("cannot read {}: {e}", path.display())))?;
    }
    Ok(s)
}

/// What a command prints: JSON, or text for people.
struct Out {
    json: String,
    text: String,
    /// Printed to stderr in text mode (so stdout holds only the text read).
    note: String,
    ok: bool,
}

fn out<T: Serialize>(v: &T, text: String) -> Out {
    Out { json: serde_json::to_string_pretty(v).expect("outputs serialize"), text, note: String::new(), ok: true }
}

fn report_lines(r: &ImportReport) -> String {
    let mut s = String::new();
    for (what, list) in [("neutralised", &r.neutralised), ("surface before export", &r.surfaced)] {
        for i in list {
            s.push_str(&format!("{what}: {} at {}: {}\n", i.kind, i.location, i.detail));
        }
    }
    s
}

fn changed_text(c: &Changed) -> String {
    if c.unchanged {
        return format!("{}: unchanged, still revision {}\n", c.doc_id, c.revision);
    }
    let mut s = format!("{}: revision {} (from {})\n", c.doc_id, c.revision, c.parent);
    if !c.rebased_over.is_empty() {
        let over: Vec<String> = c.rebased_over.iter().map(u32::to_string).collect();
        s.push_str(&format!("merged over re-imported revision {}\n", over.join(", ")));
    }
    if c.canonicalized {
        s.push_str("stored in canonical form: read again before the next edit\n");
    }
    for l in &c.removed {
        s.push_str(&format!("removed with its text: {} ({})\n", l.kind, l.reason));
    }
    for m in &c.moved {
        s.push_str(&format!("{} on {}: {} → {}\n", m.tag, m.sheet, m.before, m.after.as_deref().unwrap_or("removed")));
    }
    for n in &c.notices {
        s.push_str(&format!("{}: {} ({})\n", n.kind, n.location, n.detail));
    }
    s
}

fn run(cli: Cli) -> Result<Out> {
    let mut ws = Workspace::new(FsStorage::new(&cli.store));
    Ok(match cli.cmd {
        Cmd::Open { path } => {
            let o = ws.open(&path)?;
            let t = format!(
                "opened {} as {} ({} {}), revision {}, {} lines\n{}",
                path.display(),
                o.doc_id,
                o.format.name(),
                o.doc_type.name(),
                o.revision,
                o.lines,
                report_lines(&o.report)
            );
            out(&o, t)
        }
        Cmd::New { r#type, format, template } => {
            let o = ws.create_from(r#type, format, template.as_deref())?;
            let t = format!("made {} ({} {}), revision {}\n", o.doc_id, o.format.name(), o.doc_type.name(), o.revision);
            out(&o, t)
        }
        Cmd::Read { doc, rev, window: w } => {
            let win = Window {
                lines: w.lines,
                section: w.section,
                slides: w.slides,
                table: w.table,
                rows: w.rows,
                sheet: w.sheet,
                range: w.range,
            };
            let r = ws.read(&doc, rev, &win)?;
            let mut note = format!(
                "{} · revision {}{} · lines {}–{} of {}\n",
                r.doc_id,
                r.revision,
                if r.revision == r.head { " (current)".to_string() } else { format!(" (current: {})", r.head) },
                r.first_line,
                r.last_line,
                r.total_lines
            );
            if !r.outline.is_empty() {
                note.push_str("outline:\n");
                for e in &r.outline {
                    note.push_str(&format!("  {:>6}  {}\n", e.line, e.text));
                }
            }
            if let Some(n) = &r.next {
                note.push_str(&format!("read on: {n}\n"));
            }
            let mut text = r.text.clone();
            if let Some(d) = &r.data {
                text.push('\n');
                text.push_str(d);
            }
            let mut o = out(&r, text);
            o.note = note;
            o
        }
        Cmd::Edit { doc, rev, old, new, old_file, new_file, edits } => {
            let list = match (old, new, old_file, new_file, edits) {
                (Some(old), Some(new), ..) => vec![TextEdit { old, new }],
                (_, _, Some(o), Some(n), _) => vec![TextEdit { old: read_input(&o)?, new: read_input(&n)? }],
                (.., Some(f)) => serde_json::from_str(&read_input(&f)?).map_err(|e| {
                    Error::bad(format!("the edits are not a JSON list of {{\"old\": …, \"new\": …}}: {e}"))
                })?,
                _ => return Err(Error::bad("give --old and --new, --old-file and --new-file, or --edits")),
            };
            let c = ws.edit(&doc, rev, &list)?;
            out(&c, changed_text(&c))
        }
        Cmd::Write { doc, rev, file } => {
            let c = ws.write(&doc, rev, &read_input(&file)?)?;
            out(&c, changed_text(&c))
        }
        Cmd::Ops { doc, rev, ops } => {
            let json = if ops.trim_start().starts_with('[') { ops } else { read_input(Path::new(&ops))? };
            let c = ws.ops(&doc, rev, &json)?;
            out(&c, changed_text(&c))
        }
        Cmd::Validate { file, ty, doc } => {
            let v = ws.validate(&read_input(&file)?, ty, doc.as_deref())?;
            let t = if v.valid {
                format!("valid {}\n", v.doc_type.name())
            } else {
                v.diagnostics.iter().map(|d| format!("line {}, column {}: {}\n", d.line, d.col, d.message)).collect()
            };
            let mut o = out(&v, t);
            o.ok = v.valid;
            o
        }
        Cmd::Export { doc, path, rev, acknowledge_surfaced, tracked_changes } => {
            let e = ws.export(&doc, rev, &path, &ExportOptions { acknowledge_surfaced, tracked_changes })?;
            let mut t = format!(
                "wrote {} ({} bytes) from revision {} of {}; digest {}\n",
                path.display(),
                e.bytes,
                e.revision,
                e.doc_id,
                e.digest
            );
            for s in &e.surfaced {
                t.push_str(&format!("left with the file (acknowledged): {} at {}: {}\n", s.kind, s.location, s.detail));
            }
            out(&e, t)
        }
        Cmd::Preview { doc, rev, out, format, font_dirs, sheet, sheet_index, range } => {
            preview(&ws, &doc, rev, out, format, font_dirs, XlsxSelection { sheet, sheet_index, range })?
        }
        Cmd::Reimport { doc, path } => {
            let r = ws.reimport(&doc, &path)?;
            let t = if r.unchanged {
                format!("{}: no changes in {}; still revision {}\n", r.doc_id, path.display(), r.revision)
            } else {
                format!(
                    "{}: revision {} (re-imported {})\n{}{}",
                    r.doc_id,
                    r.revision,
                    path.display(),
                    report_lines(&r.report),
                    r.diff
                )
            };
            out(&r, t)
        }
        Cmd::History { doc } => {
            let h = ws.history(&doc)?;
            let mut t = String::new();
            for r in &h.revisions {
                let mark = if r.id == h.head { '*' } else { ' ' };
                t.push_str(&format!("{mark} {:>3}  {:<8} {}\n", r.id, format!("{:?}", r.op).to_lowercase(), r.summary));
            }
            out(&h, t)
        }
        Cmd::Diff { doc, from, to } => {
            let d = ws.diff(&doc, from, to)?;
            let t = d.diff.clone();
            out(&d, t)
        }
        Cmd::List => {
            let l = ws.list()?;
            let t = l
                .iter()
                .map(|d| {
                    format!(
                        "{}  {}  revision {}  {}\n",
                        d.doc_id,
                        d.format.name(),
                        d.head,
                        d.source.as_deref().unwrap_or("")
                    )
                })
                .collect();
            out(&l, t)
        }
        Cmd::Guide => out(&hanji_store::GUIDE, hanji_store::GUIDE.to_string()),
    })
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let json = cli.json;
    let (stdout, stderr) = (std::io::stdout(), std::io::stderr());
    match run(cli) {
        Ok(o) => {
            if json {
                let _ = writeln!(stdout.lock(), "{}", o.json);
            } else {
                let _ = write!(stderr.lock(), "{}", o.note);
                let _ = write!(stdout.lock(), "{}", o.text);
            }
            if o.ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(e) => {
            if json {
                let _ = writeln!(stdout.lock(), "{{\"error\": {}}}", serde_json::to_string_pretty(&e).unwrap());
            } else {
                let code = serde_json::to_value(e.code).unwrap();
                let _ = writeln!(stderr.lock(), "error ({}): {}", code.as_str().unwrap_or(""), e.message);
            }
            ExitCode::from(1)
        }
    }
}
