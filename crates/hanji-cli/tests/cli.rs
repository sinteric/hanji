//! The `hanji` binary end to end, per format: open → read → edit → export →
//! reopen with PutGet holding, and the refusals as exit status 1 with the
//! error in JSON.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::Value;

#[path = "../../hanji-preview/tests/fixtures/xlsx_grid.rs"]
mod xlsx_grid;
#[path = "../../hanji-preview/tests/fixtures/xlsx_inherited.rs"]
mod xlsx_inherited;
#[path = "../../hanji-preview/tests/fixtures/xlsx_numeric.rs"]
mod xlsx_numeric;

struct Env {
    dir: PathBuf,
}

impl Env {
    fn new(name: &str) -> Env {
        let dir = std::env::temp_dir().join(format!("hanji-cli-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Env { dir }
    }

    fn cmd(&self) -> Command {
        let mut c = Command::cargo_bin("hanji").unwrap();
        c.current_dir(&self.dir).env_remove("HANJI_STORE").arg("--store").arg(self.dir.join("store"));
        c
    }

    /// `hanji --json …`: the JSON it prints, and whether it succeeded.
    fn json(&self, args: &[&str]) -> (Value, bool) {
        let out = self.cmd().arg("--json").args(args).output().unwrap();
        let v: Value = serde_json::from_slice(&out.stdout)
            .unwrap_or_else(|e| panic!("{args:?}: {e}: {}", String::from_utf8_lossy(&out.stdout)));
        (v, out.status.success())
    }

    fn ok(&self, args: &[&str]) -> Value {
        let (v, ok) = self.json(args);
        assert!(ok, "{args:?}: {v:#}");
        v
    }

    fn err(&self, args: &[&str]) -> Value {
        let (v, ok) = self.json(args);
        assert!(!ok, "{args:?} succeeded: {v:#}");
        v["error"].clone()
    }

    /// `hanji read`'s stdout: the text alone.
    fn read(&self, doc: &str) -> String {
        let out = self.cmd().args(["read", doc]).output().unwrap();
        assert!(out.status.success());
        String::from_utf8(out.stdout).unwrap()
    }

    fn path(&self, name: &str) -> String {
        self.dir.join(name).display().to_string()
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn corpus(path: &str) -> String {
    format!("{}/../../{path}", env!("CARGO_MANIFEST_DIR"))
}

/// Open, edit, export, reopen: the reopened text is the edited text.
fn round_trip(env: &Env, file: &str, old: &str, new: &str) {
    let o = env.ok(&["open", &corpus(file)]);
    let id = o["doc_id"].as_str().unwrap().to_string();
    let r = env.ok(&["read", &id]);
    assert_eq!(r["revision"], 1);
    assert!(r["text"].as_str().unwrap().contains(old));
    let c = env.ok(&["edit", &id, "--rev", "1", "--old", old, "--new", new]);
    assert_eq!((c["parent"].as_u64(), c["revision"].as_u64()), (Some(1), Some(2)));
    let written = env.read(&id);
    assert!(written.contains(new));
    let ext = file.rsplit('.').next().unwrap();
    let out = env.path(&format!("out.{ext}"));
    env.ok(&["export", &id, &out, "--acknowledge-surfaced"]);
    let back = env.ok(&["open", &out]);
    assert_eq!(env.read(back["doc_id"].as_str().unwrap()), written, "PutGet");
}

#[test]
fn docx() {
    round_trip(
        &Env::new("docx"),
        "prototype/remainder/corpus/korean-report.docx",
        "| ^^ | 종로 | 95 |",
        "| ^^ | 종로 | 97 |",
    );
}

#[test]
fn hwpx() {
    round_trip(&Env::new("hwpx"), "crates/hanji-hwpx/corpus/basic-table-01.hwpx", "| 5 | 6 |", "| 5 | 6.5 |");
}

#[test]
fn pptx() {
    round_trip(&Env::new("pptx"), "crates/hanji-pptx/corpus/korean-deck.pptx", "핵심 지표", "핵심 성과 지표");
}

#[test]
fn preview_of_a_document_and_of_a_file() {
    let env = Env::new("preview");
    env.ok(&["open", &corpus("crates/hanji-pptx/corpus/korean-deck.pptx")]);
    let out = env.path("out");
    let v = env.ok(&["preview", "korean-deck", "--out", &out, "--format", "png"]);
    assert_eq!(v["files"].as_array().unwrap().len(), 7);
    assert!(Path::new(v["files"][0].as_str().unwrap()).is_file());
    assert!(v["fonts"]["substituted"].is_array());
    // A file is previewed as hanji would export it, and not stored.
    let deck = env.path("deck.pptx");
    std::fs::copy(corpus("crates/hanji-pptx/corpus/shapes.pptx"), &deck).unwrap();
    let text = env.cmd().args(["preview", &deck]).output().unwrap();
    assert!(text.status.success());
    let text = String::from_utf8(text.stdout).unwrap();
    assert!(text.starts_with("wrote ") && text.contains("deck-r1-preview.html"), "{text}");
    assert!(text.lines().nth(1).unwrap().starts_with("6 slides from revision 1 of deck; fonts: "), "{text}");
    assert!(Path::new(&env.path("deck-r1-preview.html")).is_file());
    assert_eq!(env.ok(&["list"]).as_array().unwrap().len(), 1, "the file was not stored");
    let v = env.ok(&["preview", &deck, "--out", &out, "--format", "svg"]);
    assert!(v["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["message"].as_str().unwrap().contains("unsupported slide hyperlink action")));
    assert!(text.contains("warning: ") && text.contains("unsupported slide hyperlink action"));
    let e = env.err(&["preview", &deck, "--range", "A1:B2"]);
    assert_eq!(e["code"], "bad_request");
    assert!(e["message"].as_str().unwrap().contains("only to XLSX"));
    let e = env.err(&["preview", &deck, "--rev", "2"]);
    assert_eq!(e["code"], "bad_request");
    env.ok(&["open", &corpus("prototype/remainder/corpus/korean-report.docx")]);
    let p = env.ok(&["preview", "korean-report", "--out", &out]);
    assert_eq!(p["format"], "docx");
    assert!(p["pages"].as_u64().unwrap() >= 1);
    assert!(p["slides"].is_null());
    assert!(p["warnings"].as_array().unwrap().iter().any(|w| w.as_str().unwrap().contains("experimental")));
}

#[test]
fn docx_file_and_stored_revision_preview_keep_content_and_source_read_only() {
    let env = Env::new("docx-preview");
    let file = env.path("report.docx");
    let original = std::fs::read(corpus("prototype/preview/baseline/01-docx-untouched-korean-report.docx")).unwrap();
    std::fs::write(&file, &original).unwrap();
    let out = env.path("pages");
    let p = env.ok(&["preview", &file, "--format", "svg", "--out", &out]);
    assert_eq!(p["pages"], 2);
    assert_eq!(p["files"].as_array().unwrap().len(), 2);
    let svg = std::fs::read_to_string(p["files"][0].as_str().unwrap()).unwrap();
    let content = hanji_package::xml::parse(svg.as_bytes()).unwrap().root.text_of(&["text", "tspan"]);
    assert!(content.contains("3분기") && svg.contains("data:font/") && svg.contains("<image"), "{p:#}");
    assert!(p["diagnostics"].is_array());
    assert_eq!(std::fs::read(&file).unwrap(), original);
    assert!(env.ok(&["list"]).as_array().unwrap().is_empty());
    env.ok(&["open", &file]);
    env.ok(&["edit", "report", "--rev", "1", "--old", "3분기 영업 보고", "--new", "수정된 영업 보고"]);
    let history = env.ok(&["history", "report"]);
    std::fs::remove_file(&file).unwrap();
    let p = env.ok(&["preview", "report", "--rev", "1", "--format", "svg", "--out", &out]);
    let svg = std::fs::read_to_string(p["files"][0].as_str().unwrap()).unwrap();
    let content = hanji_package::xml::parse(svg.as_bytes()).unwrap().root.text_of(&["text", "tspan"]);
    assert!(content.contains("3분기") && !content.contains("수정된"));
    assert_eq!(env.ok(&["history", "report"]), history);
    let p = env.ok(&["preview", "report", "--format", "svg", "--out", &out]);
    let svg = std::fs::read_to_string(p["files"][0].as_str().unwrap()).unwrap();
    let content = hanji_package::xml::parse(svg.as_bytes()).unwrap().root.text_of(&["text", "tspan"]);
    assert!(content.contains("수정된"));
}

#[test]
fn hwpx_file_outputs_and_stored_revisions_are_read_only() {
    let env = Env::new("hwpx-preview");
    let file = env.path("plan.hwpx");
    let original = std::fs::read(corpus("prototype/preview/baseline/30-hwpx-new-plan-ko.hwpx")).unwrap();
    std::fs::write(&file, &original).unwrap();
    let out = env.path("pages");
    for format in ["svg", "png", "html"] {
        let p = env.ok(&["preview", &file, "--format", format, "--out", &out]);
        assert_eq!(p["format"], "hwpx");
        assert_eq!(p["pages"], 2);
        assert!(p["slides"].is_null());
        assert_eq!(p["files"].as_array().unwrap().len(), if format == "html" { 1 } else { 2 });
        assert!(p["warnings"].as_array().unwrap().iter().any(|w| w.as_str().unwrap().contains("experimental")));
        assert_eq!(std::fs::read(&file).unwrap(), original);
        assert!(env.ok(&["list"]).as_array().unwrap().is_empty());
    }
    env.ok(&["open", &file]);
    let text = env.read("plan");
    let heading = text.lines().find_map(|line| line.strip_prefix("# ")).unwrap();
    env.ok(&["edit", "plan", "--rev", "1", "--old", heading, "--new", "HANJI_PREVIEW_REVISION_TWO"]);
    let history = env.ok(&["history", "plan"]);
    std::fs::remove_file(&file).unwrap();
    for (rev, edited) in [("1", false), ("2", true)] {
        let p = env.ok(&["preview", "plan", "--rev", rev, "--format", "svg", "--out", &out]);
        let content: String = p["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|file| {
                let svg = std::fs::read(file.as_str().unwrap()).unwrap();
                hanji_package::xml::parse(&svg).unwrap().root.text_of(&["text", "tspan"])
            })
            .collect();
        assert_eq!(content.contains("HANJI_PREVIEW_REVISION_TWO"), edited, "{content}");
        assert_eq!(env.ok(&["history", "plan"]), history);
    }
}

fn xlsx_fixture(env: &Env, name: &str, sheet: &str) -> String {
    let path = env.path(name);
    std::fs::write(&path, xlsx_grid::build(sheet, "", xlsx_grid::STYLES, &[])).unwrap();
    path
}

fn cell<'a>(preview: &'a Value, address: &str) -> &'a Value {
    preview["cells"].as_array().unwrap().iter().find(|c| c["address"] == address).unwrap()
}

#[test]
fn xlsx_file_preview_formats_are_read_only_and_report_cached_results() {
    let env = Env::new("xlsx-preview-formats");
    let path = xlsx_fixture(&env, "Quarter <one>.xlsx", xlsx_grid::GRID);
    let before = std::fs::read(&path).unwrap();
    for format in ["svg", "png", "html"] {
        let v = env.ok(&["preview", &path, "--sheet", "매출 & Sales", "--range", "a1:b4", "--format", format]);
        assert_eq!(v["doc_id"], "quarter-one");
        assert_eq!(v["source_kind"], "file");
        assert_eq!(v["revision"], 1);
        assert_eq!(v["sheet"]["index"], 0);
        assert_eq!(v["range"], "A1:B4");
        assert_eq!(cell(&v, "B2")["display"], "1,234.50");
        assert_eq!(cell(&v, "A3")["formula_result"], "cached-unverified");
        assert_eq!(cell(&v, "B3")["formula_result"], "missing");
        assert_eq!(cell(&v, "B3")["display"], "#UNEVALUATED");
        assert_eq!(cell(&v, "A4")["display"], "");
        assert_eq!(cell(&v, "B4")["formula_result"], "cached-unverified");
        assert!(v["fonts"]["substituted"].is_array());
        assert!(v["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w.as_str().unwrap().contains("conditionalFormatting")));
        assert!(v["diagnostics"].as_array().unwrap().iter().any(|d| d["path"] == "sheets[0].drawing"));
        assert!(v["diagnostics"].as_array().unwrap().iter().any(|d| d["path"] == "sheets[0].cells[B3].clipping"));
        let files = v["files"].as_array().unwrap();
        assert_eq!(files.len(), 1);
        let file = files[0].as_str().unwrap();
        assert!(file.ends_with(&format!("quarter-one-r1-sheet-1-A1-B4.{format}")), "{file}");
        let bytes = std::fs::read(file).unwrap();
        if format == "png" {
            assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
            let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
            let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
            assert_eq!(width, (v["page"]["width"].as_f64().unwrap() * 96.0 / 72.0).round() as u32);
            assert_eq!(height, (v["page"]["height"].as_f64().unwrap() * 96.0 / 72.0).round() as u32);
        } else {
            let text = String::from_utf8(bytes).unwrap();
            assert!(text.contains("<svg"));
            assert!(!text.contains("<script>"));
            if format == "html" {
                assert!(text.contains("매출 &amp; Sales"));
                assert!(text.contains("cached formula results only"));
            }
        }
    }
    assert_eq!(std::fs::read(path).unwrap(), before);
    assert!(env.ok(&["list"]).as_array().unwrap().is_empty());
}

#[test]
fn xlsx_sheet_selection_defaults_and_window_refusals() {
    let env = Env::new("xlsx-preview-boundaries");
    let path = xlsx_fixture(&env, "big.xlsx", xlsx_grid::GRID);
    let out = env.path("good");
    let v = env.ok(&["preview", &path, "--out", &out]);
    assert_eq!(v["range"], "A1:L40", "whole-sheet dimension must not select a huge window");
    assert_eq!(v["sheet"]["name"], "매출 & Sales");
    let v = env.ok(&["preview", &path, "--sheet-index", "2", "--out", &out, "--range", "A1"]);
    assert_eq!(v["sheet"]["state"], "hidden");
    assert_eq!(v["sheet"]["index"], 1);
    let v = env.ok(&["preview", &path, "--range", "XFD1048576", "--out", &out, "--format", "svg"]);
    assert_eq!(cell(&v, "XFD1048576")["display"], "LAST");
    assert!(v["page"]["width"].as_f64().unwrap() < 200.0);
    let refuse = env.path("refused");
    for args in [
        vec!["--sheet-index", "0"],
        vec!["--sheet-index", "3"],
        vec!["--sheet", "missing"],
        vec!["--range", "A0"],
        vec!["--range", "XFE1"],
        vec!["--range", "A1:A1048577"],
        vec!["--range", "A1:XFD1048576"],
        vec!["--range", "A1:A513"],
        vec!["--range", "A1:DY2"],
        vec!["--range", "A1:DX257"],
    ] {
        let mut command = vec!["preview", &path, "--out", &refuse];
        command.extend(args);
        assert_eq!(env.err(&command)["code"], "bad_request");
        assert!(!Path::new(&refuse).exists());
    }
    let e = env.err(&["preview", &path, "--range", "A1:A2", "--out", &refuse]);
    assert!(e["message"].as_str().unwrap().contains("cuts merged range A1:B1"));
    assert!(!Path::new(&refuse).exists());
    let e = env.err(&["preview", &path, "--range", "A1:DX256", "--format", "png", "--out", &refuse]);
    assert!(e["message"].as_str().unwrap().contains("pixel budget"));
    assert!(!Path::new(&refuse).exists());
    assert_eq!(env.err(&["preview", &path, "--rev", "1"])["code"], "bad_request");
    let conflict = env.cmd().args(["preview", &path, "--sheet", "Hidden", "--sheet-index", "2"]).output().unwrap();
    assert!(!conflict.status.success());
    assert!(String::from_utf8(conflict.stderr).unwrap().contains("cannot be used with"));
}

#[test]
fn xlsx_default_skips_hidden_sheets_and_non_worksheets() {
    use hanji_package::package;
    let env = Env::new("xlsx-preview-visible");
    let path = xlsx_fixture(&env, "hidden.xlsx", xlsx_grid::GRID);
    let mut parts = package::read(&std::fs::read(&path).unwrap()).unwrap();
    let workbook = parts.iter_mut().find(|p| p.name == "xl/workbook.xml").unwrap();
    workbook.data = String::from_utf8(workbook.data.clone())
        .unwrap()
        .replace("sheetId=\"1\"", "state=\"hidden\" sheetId=\"1\"")
        .replace("state=\"hidden\"/></sheets>", "state=\"visible\"/></sheets>")
        .into_bytes();
    std::fs::write(&path, package::write(&parts).unwrap()).unwrap();
    assert_eq!(env.ok(&["preview", &path])["sheet"]["index"], 1);
    let rels = parts.iter_mut().find(|p| p.name == "xl/_rels/workbook.xml.rels").unwrap();
    rels.data = String::from_utf8(rels.data.clone())
        .unwrap()
        .replace(
            "relationships/worksheet\" Target=\"worksheets/sheet2.xml",
            "relationships/chartsheet\" Target=\"worksheets/sheet2.xml",
        )
        .into_bytes();
    std::fs::write(&path, package::write(&parts).unwrap()).unwrap();
    assert!(env.err(&["preview", &path])["message"].as_str().unwrap().contains("no visible worksheet"));
    assert_eq!(env.err(&["preview", &path, "--sheet-index", "2"])["code"], "unsupported");
    assert_eq!(env.ok(&["preview", &path, "--sheet", "매출 & Sales"])["sheet"]["state"], "hidden");
}

#[test]
fn xlsx_stored_preview_uses_exported_revision_and_does_not_change_history() {
    let env = Env::new("xlsx-preview-revisions");
    let path = xlsx_fixture(&env, "book.xlsx", xlsx_grid::GRID);
    let id = env.ok(&["open", &path])["doc_id"].as_str().unwrap().to_string();
    // The source file is removed; a stored preview must use the revision bytes.
    std::fs::remove_file(&path).unwrap();
    env.ok(&["ops", &id, "--rev", "1", r#"[{"op":"set","range":"'매출 & Sales'!B2","values":[[42]]}]"#]);
    let history = env.ok(&["history", &id]);
    let out = env.path("revisions");
    let current = env.ok(&["preview", &id, "--range", "A1:B4", "--format", "svg", "--out", &out]);
    assert_eq!(current["revision"], 2);
    assert_eq!(current["source_kind"], "revision");
    assert_eq!(cell(&current, "B2")["display"], "42.00");
    let old = env.ok(&["preview", &id, "--rev", "1", "--range", "A1:B4", "--format", "html", "--out", &out]);
    assert_eq!(old["revision"], 1);
    assert_eq!(cell(&old, "B2")["display"], "1,234.50");
    assert_eq!(env.ok(&["history", &id]), history);
}

#[test]
fn xlsx_preview_keeps_macros_and_external_formulas_inert() {
    let env = Env::new("xlsx-preview-inert");
    let sheet = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="str"><f>WEBSERVICE("https://invalid.example/&lt;script&gt;")</f><v>stored result</v></c></row></sheetData></worksheet>"#;
    let rels = br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="external" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://invalid.example" TargetMode="External"/></Relationships>"#;
    let path = env.path("inert.xlsm");
    std::fs::write(
        &path,
        xlsx_grid::build(
            sheet,
            r#"<calcPr forceFullCalc="1"/>"#,
            xlsx_grid::STYLES,
            &[("xl/vbaProject.bin", b"inert test bytes"), ("xl/worksheets/_rels/sheet1.xml.rels", rels)],
        ),
    )
    .unwrap();
    let v = env.ok(&["preview", &path, "--range", "A1", "--format", "html"]);
    assert_eq!(cell(&v, "A1")["display"], "stored result");
    assert_eq!(cell(&v, "A1")["formula_result"], "cached-possibly-stale");
    for feature in ["workbook.macros", "workbook.externalLinks"] {
        assert!(v["diagnostics"].as_array().unwrap().iter().any(|d| d["path"] == feature));
    }
    let html = std::fs::read_to_string(v["files"][0].as_str().unwrap()).unwrap();
    assert!(!html.contains("https://invalid.example"));
    assert!(!html.contains("<script>"));
}

#[test]
fn xlsx_preview_json_and_svg_apply_inherited_number_formats() {
    let env = Env::new("xlsx-preview-inherited-formats");
    let path = env.path("inherited.xlsx");
    std::fs::write(&path, xlsx_grid::build(xlsx_inherited::INHERITED_GRID, "", xlsx_inherited::INHERITED_STYLES, &[]))
        .unwrap();
    let v = env.ok(&["preview", &path, "--range", "A1:E6", "--format", "svg"]);
    for (address, display) in [
        ("A1", "50%"),
        ("B1", "1900-01-02"),
        ("C1", "$1,234.50"),
        ("C3", "50%"),
        ("E3", "0.500"),
        ("B4", "1900-01-02"),
        ("B5", "$1,234.50"),
    ] {
        assert_eq!(cell(&v, address)["display"], display, "{address}");
    }
    assert_eq!(cell(&v, "D3")["display"], "0.5", "explicit General overrides inherited percentage");
    assert_eq!(cell(&v, "C3")["formula_result"], "cached-unverified");
    let svg = std::fs::read_to_string(v["files"][0].as_str().unwrap()).unwrap();
    for display in ["50%", "1900-01-02", "$1,234.50", "0.500"] {
        assert!(svg.contains(&format!(">{display}</text>")), "{display}");
    }
}

#[test]
fn numeric_overflow_and_quality_contract_are_consistent_across_cli_outputs() {
    let env = Env::new("numeric-quality");
    let path = env.path("numbers.xlsx");
    let original = xlsx_grid::build(xlsx_numeric::GRID, "", xlsx_numeric::STYLES, &[]);
    std::fs::write(&path, &original).unwrap();
    for format in ["svg", "png", "html"] {
        let v = env.ok(&["preview", &path, "--range", "A1:D11", "--format", format]);
        assert_eq!(cell(&v, "A1")["display"], "-123456789");
        assert_eq!(cell(&v, "A10")["display"], "#UNEVALUATED");
        assert_eq!(cell(&v, "A10")["formula_result"], "missing");
        assert_eq!(cell(&v, "B10")["display"], "0");
        assert_ne!(cell(&v, "B10")["formula_result"], "missing");
        assert_eq!(v["quality"]["version"], 1);
        assert_eq!(v["quality"]["source"], "xlsx");
        assert_eq!(v["quality"]["rendering"], "rendered");
        assert_eq!(v["quality"]["status"], "degraded");
        assert_eq!(v["quality"]["coverage"]["complete"], false);
        let diagnostics = v["quality"]["diagnostics"].as_array().unwrap();
        let d =
            diagnostics.iter().find(|d| d["code"] == "xlsx.numeric-overflow" && d["location"]["cell"] == "A1").unwrap();
        assert_eq!(d["severity"], "warning");
        assert_eq!(d["location"]["sheet_index"], 0);
        assert_eq!(d["consequence"], "overflow-indicator");
        assert!(v["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|old| old["path"] == d["path"] && old["message"] == d["message"]));
        assert!(diagnostics.iter().any(|d| d["code"] == "xlsx.formula-cache-missing"
            && d["location"]["cell"] == "A10"
            && d["severity"] == "error"));
        let file = v["files"][0].as_str().unwrap();
        let output = std::fs::read(file).unwrap();
        if format != "png" {
            let text = String::from_utf8(output).unwrap();
            assert!(text.contains("A1: -123456789"));
            assert!(!text.contains(">2026-10-03T13:00:00Z</text>"));
            if format == "html" {
                assert!(text.contains("Complete cell values"));
            }
        } else {
            assert!(output.starts_with(b"\x89PNG\r\n\x1a\n"));
        }
    }
    assert_eq!(std::fs::read(&path).unwrap(), original);
}

#[test]
fn preview_help_documents_worksheet_defaults_and_limits() {
    let env = Env::new("preview-help");
    let output = env.cmd().args(["preview", "--help"]).output().unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    for text in ["--sheet", "--sheet-index", "--range", "A1:L40", "1-based", "Cached formula results only", "512 rows"]
    {
        assert!(help.contains(text), "missing {text}: {help}");
    }
}

#[test]
fn xlsx() {
    let env = Env::new("xlsx");
    round_trip(
        &env,
        "crates/hanji-xlsx/corpus/korean-sales.xlsx",
        "| 원가 | number | #,##0 |",
        "| 원가 | number | #,##0.0 |",
    );
    let c = env.ok(&[
        "ops",
        "korean-sales",
        "--rev",
        "2",
        r#"[{"op": "set", "range": "매출!D4", "values": [[11111111]]}]"#,
    ]);
    assert_eq!(c["applied"], 1);
    let r = env.ok(&["read", "korean-sales", "--table", "Sales", "--rows", "4:4"]);
    assert!(r["data"].as_str().unwrap().contains("| 4 | 2026-01 | 강남 | 가전 | 11,111,111 |"), "{r:#}");
    let e = env.err(&["ops", "korean-sales", "--rev", "3", r#"[{"op": "sett"}]"#]);
    assert_eq!(e["code"], "invalid");
    assert!(e["message"].as_str().unwrap().contains("set, append_rows"), "{e:#}");
}

#[test]
fn a_new_document_from_the_blank_package() {
    let env = Env::new("new");
    let o = env.ok(&["new", "document", "--format", "hwpx"]);
    assert_eq!(o["format"], "hwpx");
    let text = "---\ntype: document\nformat: hwpx\nschema: 1\n---\n# 제목\n\n본문.\n";
    std::fs::write(env.dir.join("t.md"), text).unwrap();
    env.ok(&["write", "document", "--rev", "1", "t.md"]);
    // The stored text shows the styles it uses (§5.2): the default's line, then 개요 1's.
    let styled = text.replace(
        "---\n# 제목",
        "---\n<style name=\"바탕글\" align=justify line-spacing=160% font=함초롬바탕 size=10pt color=#000000/>\n<style name=\"개요 1\" indent-left=10pt/>\n\n# 제목",
    );
    assert_eq!(env.read("document"), styled);
    env.ok(&["export", "document", &env.path("new.hwpx")]);
    let back = env.ok(&["open", &env.path("new.hwpx")]);
    assert_eq!(env.read(back["doc_id"].as_str().unwrap()), styled);
    let e = env.err(&["new", "presentation", "--format", "docx"]);
    assert_eq!(e["code"], "bad_request");
}

#[test]
fn refusals_are_exit_status_1_with_the_reason() {
    let env = Env::new("refusals");
    env.ok(&["open", &corpus("prototype/remainder/corpus/korean-report.docx")]);
    env.ok(&["edit", "korean-report", "--rev", "1", "--old", "4분기에는", "--new", "4분기부터"]);
    // Stale revision.
    let e = env.err(&["edit", "korean-report", "--rev", "1", "--old", "대구", "--new", "광주"]);
    assert_eq!((e["code"].as_str(), e["head"].as_u64()), (Some("stale_revision"), Some(2)));
    // Ambiguous old.
    let e = env.err(&["edit", "korean-report", "--rev", "2", "--old", "수도권", "--new", "x"]);
    assert_eq!((e["code"].as_str(), e["matches"].as_u64()), (Some("ambiguous_match"), Some(2)));
    assert_eq!(e["lines"], serde_json::json!([14, 16]));
    // Validator errors: line, column, allowed names.
    let e = env.err(&[
        "edit",
        "korean-report",
        "--rev",
        "2",
        "--old",
        "<div style=\"Note\">",
        "--new",
        "<div style=\"Callout\">",
    ]);
    assert_eq!(e["code"], "invalid");
    assert_eq!((e["diagnostics"][0]["line"].as_u64(), e["diagnostics"][0]["col"].as_u64()), (Some(16), Some(6)));
    assert!(e["diagnostics"][0]["message"].as_str().unwrap().contains("\"Block Quotation\""));
    std::fs::write(
        env.dir.join("bad.md"),
        "---\ntype: document\nformat: docx\nschema: 1\n---\n| a | b |\n|---|---|\n| 1 |\n",
    )
    .unwrap();
    let (v, ok) = env.json(&["validate", "bad.md"]);
    assert!(!ok && v["valid"] == false && v["diagnostics"][0]["line"] == 8, "{v:#}");
    // Surfaced before export.
    let e = env.err(&["export", "korean-report", &env.path("x.docx")]);
    assert_eq!(e["code"], "surfaced_not_acknowledged");
    assert_eq!(e["surfaced"][0]["kind"], "comment");
    assert!(!env.dir.join("x.docx").exists(), "nothing is written");
    // Tracked changes (§10.2): revision 2's edit ("4분기에는" → "4분기부터") as w:del / w:ins.
    env.ok(&["export", "korean-report", &env.path("t.docx"), "--acknowledge-surfaced", "--tracked-changes"]);
    let parts = hanji_package::package::read(&std::fs::read(env.dir.join("t.docx")).unwrap()).unwrap();
    let doc = String::from_utf8_lossy(hanji_package::package::get(&parts, "word/document.xml").unwrap()).into_owned();
    let rev = |tag: &str| doc.matches(&format!("<w:{tag} w:id=")).count();
    assert!(rev("del") >= 1 && rev("ins") >= 1, "{doc}");
    assert!(doc.contains("w:author=\"hanji (model edit)\""));
    assert!(doc.contains("분기에는</w:delText>") || doc.contains("에는</w:delText>"), "{doc}");
    let e = env.err(&["export", "korean-report", &env.path("x.docm"), "--acknowledge-surfaced"]);
    assert_eq!(e["code"], "bad_request", "an export carries no macros: not a .docm");
    assert_eq!(env.err(&["read", "nope"])["code"], "not_found");
    // People get the message on stderr.
    let out = env.cmd().args(["edit", "korean-report", "--rev", "1", "--old", "a", "--new", "b"]).output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr)
        .starts_with("error (stale_revision): revision 1 is not the current revision"));
    let h = env.ok(&["history", "korean-report"]);
    assert_eq!(h["head"], 2, "refusals make no revision");
}

/// A person opens the export in Word, changes a word and saves; the file comes back in.
#[test]
fn reimport_of_a_file_a_person_changed() {
    let env = Env::new("reimport");
    env.ok(&["open", &corpus("prototype/remainder/corpus/korean-report.docx")]);
    let out = env.path("report.docx");
    env.ok(&["export", "korean-report", &out, "--acknowledge-surfaced"]);
    let mut parts = hanji_package::package::read(&std::fs::read(&out).unwrap()).unwrap();
    let doc = parts.iter_mut().find(|p| p.name == "word/document.xml").unwrap();
    doc.data = String::from_utf8(doc.data.clone()).unwrap().replacen(">분기에는 <", ">분기부터는 <", 1).into_bytes();
    std::fs::write(&out, hanji_package::package::write(&parts).unwrap()).unwrap();
    let r = env.ok(&["reimport", "korean-report", &out, "--base-rev", "1"]);
    assert_eq!((r["parent"].as_u64(), r["revision"].as_u64()), (Some(1), Some(2)));
    assert!(
        r["diff"]
            .as_str()
            .unwrap()
            .contains("+4분기부터는 [부산]{color=#C00000}과 [대구]{color=#C00000}에 지점을 연다."),
        "{r:#}"
    );
    // An edit the model made against revision 1 is merged over it.
    let c = env.ok(&["edit", "korean-report", "--rev", "1", "--old", "| 합계 || 215 |", "--new", "| 합계 || 217 |"]);
    assert_eq!(c["rebased_over"], serde_json::json!([2]));
    let text = env.read("korean-report");
    assert!(text.contains("4분기부터는") && text.contains("| 합계 || 217 |"));
    let d = env.cmd().args(["diff", "korean-report", "1", "3"]).output().unwrap();
    let d = String::from_utf8(d.stdout).unwrap();
    assert!(d.contains("-| 합계 || 215 |") && d.contains("+4분기부터는"), "{d}");
    let h = env.cmd().args(["history", "korean-report"]).output().unwrap();
    let h = String::from_utf8(h.stdout).unwrap();
    assert!(h.contains("reimport") && h.lines().nth(2).unwrap().starts_with('*'), "{h}");
}

#[test]
fn reimport_requires_an_export_base_or_deliberate_replacement() {
    let env = Env::new("reimport-guard");
    env.ok(&["open", &corpus("prototype/remainder/corpus/korean-report.docx")]);
    let path = env.path("report.docx");
    env.ok(&["export", "korean-report", &path, "--acknowledge-surfaced"]);
    env.ok(&["edit", "korean-report", "--rev", "1", "--old", "| 합계 || 215 |", "--new", "| 합계 || 217 |"]);
    let before = env.read("korean-report");
    let history = env.ok(&["history", "korean-report"]);

    let missing = env.cmd().args(["reimport", "korean-report", &path]).output().unwrap();
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("--base-rev"));
    let help = env.cmd().args(["reimport", "--help"]).output().unwrap();
    let help = String::from_utf8(help.stdout).unwrap();
    assert!(help.contains("--base-rev") && help.contains("--replace-head"), "{help}");
    let conflicting =
        env.cmd().args(["reimport", "korean-report", &path, "--base-rev", "1", "--replace-head"]).output().unwrap();
    assert!(!conflicting.status.success());

    let e = env.err(&["reimport", "korean-report", &path, "--base-rev", "1"]);
    assert_eq!(e["code"], "stale_revision");
    assert_eq!(e["head"], 2);
    assert!(e["message"].as_str().unwrap().contains("--replace-head"), "{e:#}");
    assert_eq!(env.read("korean-report"), before);
    assert_eq!(env.ok(&["history", "korean-report"]), history);

    // The explicit override intentionally restores the old exported file.
    let r = env.ok(&["reimport", "korean-report", &path, "--replace-head"]);
    assert_eq!((r["parent"].as_u64(), r["revision"].as_u64()), (Some(2), Some(3)));
    assert!(env.read("korean-report").contains("| 합계 || 215 |"));
}

#[test]
fn a_part_of_a_large_file() {
    let env = Env::new("large");
    env.ok(&["new", "document"]);
    let mut text = String::from("---\ntype: document\nformat: docx\nschema: 1\n---\n");
    for s in 1..=20 {
        text.push_str(&format!("# 장 {s}\n\n"));
        for p in 1..=120 {
            text.push_str(&format!("장 {s}, 문단 {p}: 지점별 매출과 원가를 정리한다.\n\n"));
        }
    }
    std::fs::write(env.dir.join("big.md"), &text).unwrap();
    env.ok(&["write", "document", "--rev", "1", "big.md"]);
    let r = env.ok(&["read", "document"]);
    assert_eq!(r["partial"], true);
    assert_eq!(r["outline"].as_array().unwrap().len(), 20);
    assert!(r["next"].as_str().unwrap().starts_with("lines "));
    let r = env.ok(&["read", "document", "--section", "장 12"]);
    assert!(r["text"].as_str().unwrap().starts_with("# 장 12\n\n장 12, 문단 1:"));
    let out = env.cmd().args(["read", "document", "--lines", "9:11"]).output().unwrap();
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "# 장 1\n\n장 1, 문단 1: 지점별 매출과 원가를 정리한다.\n");
    assert!(String::from_utf8(out.stderr).unwrap().contains("lines 9–11 of"));
}

#[test]
fn edits_from_files_and_lists() {
    let env = Env::new("files");
    env.ok(&["open", &corpus("prototype/remainder/corpus/korean-report.docx")]);
    let dir = Path::new(&env.dir);
    std::fs::write(dir.join("old.txt"), "| 서울 | 강남 | 120 |\n| ^^ | 종로 | 95 |").unwrap();
    std::fs::write(dir.join("new.txt"), "| 서울 | 강남 | 125 |\n| ^^ | 종로 | 95 |").unwrap();
    env.ok(&["edit", "korean-report", "--rev", "1", "--old-file", "old.txt", "--new-file", "new.txt"]);
    std::fs::write(dir.join("edits.json"), r#"[{"old": "| 서울 | 강남 | 125 |", "new": "| 서울 | 강남 | 126 |"}, {"old": "4분기에는", "new": "4분기부터"}]"#).unwrap();
    let c = env.ok(&["edit", "korean-report", "--rev", "2", "--edits", "edits.json"]);
    assert_eq!(c["revision"], 3);
    assert!(env.read("korean-report").contains("| 서울 | 강남 | 126 |"));
    let g = env.cmd().arg("guide").output().unwrap();
    assert!(String::from_utf8(g.stdout).unwrap().contains("range operations"));
}

/// A Document in its other format, with the list of what did not cross (§3).
#[test]
fn a_document_exported_in_its_other_format_lists_what_did_not_cross() {
    let env = Env::new("convert");
    env.ok(&["open", &corpus("prototype/remainder/corpus/korean-report.docx")]);
    let out = env.path("out.hwpx");
    let e = env.ok(&["export", "korean-report", &out, "--acknowledge-surfaced", "--format", "hwpx"]);
    assert_eq!(e["format"], "hwpx");
    let c = &e["conversion"];
    assert_eq!((c["from"].as_str(), c["to"].as_str()), (Some("docx"), Some("hwpx")));
    assert_eq!(c["placeholder_kinds"]["footnote"], 1);
    assert_eq!(c["placeholders"][0]["kind"], "footnote");
    assert!(c["placeholders"][0]["summary"].as_str().unwrap().contains("내부 집계 기준"));
    assert_eq!(c["properties"][0]["property"], "table-align=left table-indent=0pt");
    assert!(c["properties"][0]["reason"].as_str().unwrap().contains("hwpx has no table position"));
    let normal = c["styles"].as_array().unwrap().iter().find(|s| s["name"] == "Normal").unwrap();
    assert_eq!((normal["fate"].as_str(), normal["target"].as_str()), (Some("replaced"), Some("바탕글")));
    // The file is an hwpx, and its text is the document's.
    let back = env.ok(&["open", &out]);
    assert_eq!((back["format"].as_str(), back["doc_type"].as_str()), (Some("hwpx"), Some("document")));
    let text = env.read(back["doc_id"].as_str().unwrap());
    assert!(text.contains("# 3분기 영업 보고") && !text.contains("<keep "), "{text}");
    // People read the list on stdout, after the line that says what was written.
    let o = env
        .cmd()
        .args(["export", "korean-report", &env.path("again.hwpx"), "--acknowledge-surfaced", "--format", "hwpx"])
        .output()
        .unwrap();
    let said = String::from_utf8(o.stdout).unwrap();
    assert!(said.starts_with("wrote "), "{said}");
    assert!(said.contains("converted docx → hwpx. Not carried across:"), "{said}");
    assert!(
        said.contains("placeholders dropped, 4 (comment ×1, drawing ×1, footnote ×1, tracked-insert ×1)"),
        "{said}"
    );
    // Naming the document's own format is an ordinary export: no list.
    let e = env.ok(&["export", "korean-report", &env.path("same.docx"), "--acknowledge-surfaced", "--format", "docx"]);
    assert!(e.get("conversion").is_none(), "{e:#}");
    // Another type is refused, and so is a file name of another format than the one asked for.
    let e = env.err(&["export", "korean-report", &env.path("x.pptx"), "--format", "pptx"]);
    assert_eq!(e["code"], "unsupported");
    assert!(e["message"].as_str().unwrap().contains("a document is exported as docx or hwpx"), "{e:#}");
    let e =
        env.err(&["export", "korean-report", &env.path("wrong.docx"), "--acknowledge-surfaced", "--format", "hwpx"]);
    assert_eq!(e["code"], "bad_request");
    assert!(!env.dir.join("wrong.docx").exists(), "nothing is written");
    env.ok(&["open", &corpus("crates/hanji-pptx/corpus/60810.pptx")]);
    let e = env.err(&["export", "60810", &env.path("deck.docx"), "--format", "docx"]);
    assert_eq!(e["code"], "unsupported");
    assert!(e["message"].as_str().unwrap().contains("a presentation is exported as pptx"), "{e:#}");
}
