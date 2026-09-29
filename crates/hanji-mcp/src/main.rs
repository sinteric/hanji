//! `hanji-mcp`: the operations of `hanji-store` as MCP tools over stdio
//! (rmcp, the official Rust SDK). The server's instructions carry the
//! format summary (`hanji_store::GUIDE`), and each tool's description
//! teaches read-before-edit and exact spans. A refusal is a tool result with
//! `isError` and the reason, never a silent success (DESIGN.md §2 rule 1).
//!
//! `hanji-mcp [--store DIR]`: documents are kept in DIR (default
//! `$HANJI_STORE`, else `~/.hanji`). Paths in tool
//! arguments are absolute, or relative to the server's working directory.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use hanji_store::{DocType, Error, ExportOptions, Format, FsStorage, TextEdit, Window, Workspace};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig};
use rmcp::{tool, tool_handler, tool_router, ErrorData, ServerHandler, ServiceExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct Hanji {
    ws: Arc<Mutex<Workspace<FsStorage>>>,
    tool_router: ToolRouter<Self>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum TypeArg {
    Document,
    Presentation,
    Spreadsheet,
}

impl From<TypeArg> for DocType {
    fn from(t: TypeArg) -> DocType {
        match t {
            TypeArg::Document => DocType::Document,
            TypeArg::Presentation => DocType::Presentation,
            TypeArg::Spreadsheet => DocType::Spreadsheet,
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum FormatArg {
    Docx,
    Hwpx,
    Pptx,
    Xlsx,
}

impl From<FormatArg> for Format {
    fn from(f: FormatArg) -> Format {
        match f {
            FormatArg::Docx => Format::Docx,
            FormatArg::Hwpx => Format::Hwpx,
            FormatArg::Pptx => Format::Pptx,
            FormatArg::Xlsx => Format::Xlsx,
        }
    }
}

#[derive(Deserialize, JsonSchema)]
struct OpenArgs {
    /// Path of a .docx, .hwpx, .pptx or .xlsx file (absolute, or relative to the server's working directory).
    path: String,
}

#[derive(Deserialize, JsonSchema)]
struct NewArgs {
    /// document (docx or hwpx), presentation (pptx) or spreadsheet (xlsx).
    #[serde(rename = "type")]
    ty: TypeArg,
    /// The home format; by default docx, pptx or xlsx.
    format: Option<FormatArg>,
    /// Path of a template file to start from (its styles, layouts and content); a blank file when left out.
    template: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
struct ReadArgs {
    /// The document, as hanji_open or hanji_new named it.
    doc_id: String,
    /// The revision to read; the current one when left out.
    revision: Option<u32>,
    /// Line range, 1-based and inclusive: "120:180" (or "120").
    lines: Option<String>,
    /// A document's section: its heading's text ("3분기 실적"), up to the next heading of the same or a higher level.
    section: Option<String>,
    /// A presentation's slides, 1-based and inclusive: "3:5" (or "3").
    slides: Option<String>,
    /// Spreadsheet: the table whose rows to show in `data`.
    table: Option<String>,
    /// Spreadsheet: the sheet rows of `table` to show, "2:101" (at most 500).
    rows: Option<String>,
    /// Spreadsheet: the sheet whose `range` to show.
    sheet: Option<String>,
    /// Spreadsheet: cells of `sheet` to show, "A1:F50" (or "Sheet1!A1:F50").
    range: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
struct EditPair {
    /// Text copied exactly from the revision; it must occur exactly once.
    old: String,
    /// What replaces it ("" deletes it).
    new: String,
}

#[derive(Deserialize, JsonSchema)]
struct EditArgs {
    /// The document, as hanji_open or hanji_new named it.
    doc_id: String,
    /// The revision `old` was read from: the current revision.
    revision: u32,
    /// Text copied exactly from the revision; it must occur exactly once in the whole text.
    old: Option<String>,
    /// What replaces `old` ("" deletes it).
    new: Option<String>,
    /// Several edits instead of old/new, applied in order, all or nothing.
    edits: Option<Vec<EditPair>>,
}

#[derive(Deserialize, JsonSchema)]
struct WriteArgs {
    /// The document, as hanji_open or hanji_new named it.
    doc_id: String,
    /// The revision the text was based on: the current revision.
    revision: u32,
    /// The complete new text, front matter included.
    text: String,
}

#[derive(Deserialize, JsonSchema)]
struct OpsArgs {
    /// The document, as hanji_open or hanji_new named it.
    doc_id: String,
    /// The current revision.
    revision: u32,
    /// A JSON array of range operations, e.g. [{"op": "set", "range": "매출!B73", "values": [[18420000]]}].
    ops: serde_json::Value,
}

#[derive(Deserialize, JsonSchema)]
struct ValidateArgs {
    /// The text to check, front matter included.
    text: String,
    /// Its type; by default the document's, or the front matter's.
    #[serde(rename = "type")]
    ty: Option<TypeArg>,
    /// Also check names (styles, layouts, slots, placeholders) against this document.
    doc_id: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
struct ExportArgs {
    /// The document, as hanji_open or hanji_new named it.
    doc_id: String,
    /// The file to write (absolute, or relative to the server's working directory), with the document's extension.
    path: String,
    /// The revision to export; the current one when left out.
    revision: Option<u32>,
    /// Export although the file would carry surfaced content; only after the person has seen the list.
    #[serde(default)]
    acknowledge_surfaced: bool,
    /// docx: write the edits as tracked changes (not available yet).
    #[serde(default)]
    tracked_changes: bool,
}

#[derive(Deserialize, JsonSchema)]
struct ReimportArgs {
    /// The document, as hanji_open or hanji_new named it.
    doc_id: String,
    /// The file the person edited (absolute, or relative to the server's working directory).
    path: String,
}

#[derive(Deserialize, JsonSchema)]
struct HistoryArgs {
    /// The document, as hanji_open or hanji_new named it.
    doc_id: String,
    /// With `to`: the diff from this revision.
    from: Option<u32>,
    /// With `from`: the diff to this revision.
    to: Option<u32>,
}

fn json<T: Serialize>(v: &T) -> String {
    serde_json::to_string_pretty(v).expect("results serialize")
}

fn ok<T: Serialize>(v: &T) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(json(v))])
}

/// A refusal: the message for the model, then the error as JSON.
fn refused(e: Error) -> CallToolResult {
    let code = serde_json::to_value(e.code).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default();
    CallToolResult::error(vec![
        ContentBlock::text(format!("{code}: {}", e.message)),
        ContentBlock::text(json(&serde_json::json!({ "error": e }))),
    ])
}

fn answer<T: Serialize>(r: hanji_store::Result<T>) -> Result<CallToolResult, ErrorData> {
    Ok(r.map_or_else(refused, |v| ok(&v)))
}

#[tool_router]
impl Hanji {
    fn new(store: PathBuf) -> Hanji {
        Hanji { ws: Arc::new(Mutex::new(Workspace::new(FsStorage::new(store)))), tool_router: Self::tool_router() }
    }

    /// Run an operation on the blocking pool, so the server keeps reading
    /// its input (pings, cancellations) while a large export runs.
    async fn with<T: Send + 'static>(
        &self,
        f: impl FnOnce(&mut Workspace<FsStorage>) -> hanji_store::Result<T> + Send + 'static,
    ) -> hanji_store::Result<T> {
        let ws = self.ws.clone();
        tokio::task::spawn_blocking(move || f(&mut ws.lock().unwrap_or_else(|p| p.into_inner())))
            .await
            .unwrap_or_else(|e| Err(Error::io(format!("the operation stopped: {e}"))))
    }

    #[tool(
        description = "Open an office file (.docx, .hwpx, .pptx, .xlsx) as a hanji document. Returns its doc_id, revision 1 and the safety report: `neutralised` lists active or remote content that was removed (macros, remote templates, external links, embedded objects; not kept), `surfaced` lists content that would leave with the file unreviewed (comments, hidden text, tracked deletions, author metadata). Tell the person about both. Then call hanji_read: never edit a document you have not read."
    )]
    async fn hanji_open(&self, Parameters(a): Parameters<OpenArgs>) -> Result<CallToolResult, ErrorData> {
        answer(self.with(move |ws| ws.open(Path::new(&a.path))).await)
    }

    #[tool(
        description = "Make a new document from a blank file, or from a template file (its styles, layouts and content are kept). type: document (docx or hwpx), presentation (pptx) or spreadsheet (xlsx). Returns its doc_id and revision 1. Read it before writing, to see its front matter and what it holds; then hanji_write the whole text (documents, presentations) or use hanji_ops (spreadsheets)."
    )]
    async fn hanji_new(&self, Parameters(a): Parameters<NewArgs>) -> Result<CallToolResult, ErrorData> {
        answer(
            self.with(move |ws| {
                ws.create_from(a.ty.into(), a.format.map(Into::into), a.template.as_deref().map(Path::new))
            })
            .await,
        )
    }

    #[tool(
        description = "Read a revision's text (the current one by default). Read before every edit: the result's `revision` is what hanji_edit, hanji_write and hanji_ops need, and `old` must be copied from this text exactly. The text is the second content block, exactly as stored. A large file comes in parts (`partial`, `outline`, `next`): read on with `lines` (\"120:180\"), `section` (a heading's text) or `slides` (\"3:5\"). For a spreadsheet the text is the workbook's structure, and the third block is a read-only row window (first column: sheet row; values as displayed): ask for `table` with `rows` (\"2:101\"), or `sheet` with `range` (\"A1:F50\")."
    )]
    async fn hanji_read(&self, Parameters(a): Parameters<ReadArgs>) -> Result<CallToolResult, ErrorData> {
        let w = Window {
            lines: a.lines,
            section: a.section,
            slides: a.slides,
            table: a.table,
            rows: a.rows,
            sheet: a.sheet,
            range: a.range,
        };
        let r = match self.with(move |ws| ws.read(&a.doc_id, a.revision, &w)).await {
            Ok(r) => r,
            Err(e) => return Ok(refused(e)),
        };
        let mut head = serde_json::to_value(&r).expect("results serialize");
        if let Some(o) = head.as_object_mut() {
            o.remove("text");
            o.remove("data");
        }
        let mut blocks = vec![ContentBlock::text(json(&head)), ContentBlock::text(r.text)];
        if let Some(d) = r.data {
            blocks.push(ContentBlock::text(d));
        }
        Ok(CallToolResult::success(blocks))
    }

    #[tool(
        description = "Edit a document by exact spans. Each `old` is copied exactly from a hanji_read of `revision` (spaces, line breaks, markers and <keep/> placeholders included) and must occur exactly once in the whole text: add neighbouring text to make it unique. Give `old` and `new`, or `edits`, a list applied in order, all or nothing. `new` is written in the format (see the instructions); keep placeholders unless the person asked to delete that object. `revision` must be the current revision: an edit against an older one is refused as stale, except over a person's re-imported edits, where it is merged if it changes other lines. A refusal changes nothing and says why (no_match, ambiguous_match with the count and lines, invalid with line, column and the allowed names, refused, unplaceable); fix the edit and retry. Returns the new revision; if it says `canonicalized`, read again before editing near the change."
    )]
    async fn hanji_edit(&self, Parameters(a): Parameters<EditArgs>) -> Result<CallToolResult, ErrorData> {
        let edits = match (a.old, a.new, a.edits) {
            (Some(old), Some(new), None) => vec![TextEdit { old, new }],
            (None, None, Some(list)) => list.into_iter().map(|e| TextEdit { old: e.old, new: e.new }).collect(),
            _ => return Ok(refused(Error::bad("give old and new, or edits (a list of {old, new}), not both."))),
        };
        answer(self.with(move |ws| ws.edit(&a.doc_id, a.revision, &edits)).await)
    }

    #[tool(
        description = "Replace a document's whole text: for large changes, or to fill a new document. Send the complete text, front matter included, in canonical form (one paragraph per line, a blank line between blocks). What the text does not show (formatting, pictures, comments) is kept by aligning the old and new text; anything whose place cannot be found is refused, not dropped. Same revision rules as hanji_edit. For small changes prefer hanji_edit."
    )]
    async fn hanji_write(&self, Parameters(a): Parameters<WriteArgs>) -> Result<CallToolResult, ErrorData> {
        answer(self.with(move |ws| ws.write(&a.doc_id, a.revision, &a.text)).await)
    }

    #[tool(
        description = "Write spreadsheet cells with range operations: `ops` is a JSON array applied in order, all or nothing, from set, append_rows, insert_rows, delete_rows, fill_formula, set_type, add_column, sort, add_table, add_sheet (keys in the instructions). Read the structure and a row window first. Numbers are JSON numbers, dates \"YYYY-MM-DD\" text; a value starting with = is stored as text: formulas go only in a column's formula or fill_formula, with structured references ([@매출]). At most 50 new rows per call. `revision` must be the current one. Returns the new revision, ranges that moved, notices and recalculated formulas."
    )]
    async fn hanji_ops(&self, Parameters(a): Parameters<OpsArgs>) -> Result<CallToolResult, ErrorData> {
        let ops = match a.ops {
            serde_json::Value::String(s) => s,
            v => v.to_string(),
        };
        answer(self.with(move |ws| ws.ops(&a.doc_id, a.revision, &ops)).await)
    }

    #[tool(
        description = "Check a text without changing anything: the grammar of its type and, with doc_id, the document's style names, layouts, slots and placeholders. Returns `valid` and `diagnostics` (line, column, and a message with the expected form and the allowed names)."
    )]
    async fn hanji_validate(&self, Parameters(a): Parameters<ValidateArgs>) -> Result<CallToolResult, ErrorData> {
        answer(self.with(move |ws| ws.validate(&a.text, a.ty.map(Into::into), a.doc_id.as_deref())).await)
    }

    #[tool(
        description = "Write a revision (the current one by default) to a file in the document's format. If the file would carry content nobody may have reviewed (comments, hidden text, tracked deletions, author metadata), the export is refused with that list in `surfaced`: show it to the person, and export again with acknowledge_surfaced=true only once they agree. Returns the path, size and a digest (the same revision gives the same bytes)."
    )]
    async fn hanji_export(&self, Parameters(a): Parameters<ExportArgs>) -> Result<CallToolResult, ErrorData> {
        let opts = ExportOptions { acknowledge_surfaced: a.acknowledge_surfaced, tracked_changes: a.tracked_changes };
        answer(self.with(move |ws| ws.export(&a.doc_id, a.revision, Path::new(&a.path), &opts)).await)
    }

    #[tool(
        description = "Bring back a file the person edited in Word, PowerPoint, Excel or Hancom (usually one hanji_export wrote): it becomes the new current revision, and `diff` shows their changes to the text. Read again before editing. An edit sent against the revision before it is merged if it changes other lines, and refused if it changes the same ones."
    )]
    async fn hanji_reimport(&self, Parameters(a): Parameters<ReimportArgs>) -> Result<CallToolResult, ErrorData> {
        answer(self.with(move |ws| ws.reimport(&a.doc_id, Path::new(&a.path))).await)
    }

    #[tool(
        description = "List a document's revisions: id, parent, and what made each (open, new, edit, write, ops, reimport), and the current one (`head`). With `from` and `to`: the text diff between those two revisions instead."
    )]
    async fn hanji_history(&self, Parameters(a): Parameters<HistoryArgs>) -> Result<CallToolResult, ErrorData> {
        match (a.from, a.to) {
            (None, None) => answer(self.with(move |ws| ws.history(&a.doc_id)).await),
            (Some(from), Some(to)) => answer(self.with(move |ws| ws.diff(&a.doc_id, from, to)).await),
            _ => Ok(refused(Error::bad("give both from and to for a diff, or neither for the history."))),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for Hanji {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("hanji", env!("CARGO_PKG_VERSION")))
            .with_instructions(hanji_store::GUIDE)
    }
}

fn store_dir() -> Result<PathBuf, String> {
    let mut args = std::env::args().skip(1);
    let mut store = std::env::var_os("HANJI_STORE").map(PathBuf::from);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--store" => store = Some(args.next().ok_or("--store needs a directory")?.into()),
            "--help" | "-h" => {
                return Err("hanji-mcp [--store DIR]: the hanji MCP server over stdio. Documents are kept in DIR (default $HANJI_STORE, else ~/.hanji).".into())
            }
            other => return Err(format!("unknown argument {other:?}; hanji-mcp [--store DIR]")),
        }
    }
    // Clients start servers in any directory (often `/`): the default is in the home directory.
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from);
    Ok(store.unwrap_or_else(|| home.map_or_else(|| PathBuf::from(".hanji"), |h| h.join(".hanji"))))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
    let store = match store_dir() {
        Ok(s) => s,
        Err(m) => {
            eprintln!("{m}");
            return std::process::ExitCode::from(2);
        }
    };
    let served = match Hanji::new(store).serve(rmcp::transport::stdio()).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("hanji-mcp: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    match served.waiting().await {
        Ok(_) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("hanji-mcp: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
