//! The MCP server end to end: spawn `hanji-mcp`, speak JSON-RPC over its
//! stdio, and run open → read → edit → export → reopen per format with
//! PutGet holding, and the refusals as `isError` results with the reason.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};

struct Server {
    child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
    next: u64,
    dir: PathBuf,
}

/// A tool call's result: its text blocks, and whether it is an error.
struct Reply {
    blocks: Vec<String>,
    error: bool,
}

impl Reply {
    fn json(&self) -> Value {
        let k = if self.error { 1 } else { 0 };
        serde_json::from_str(&self.blocks[k]).unwrap_or_else(|e| panic!("{e}: {}", self.blocks[k]))
    }
}

impl Server {
    fn start(name: &str) -> Server {
        let dir = std::env::temp_dir().join(format!("hanji-mcp-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_hanji-mcp"))
            .arg("--store")
            .arg(dir.join("store"))
            .current_dir(&dir)
            .env_remove("HANJI_STORE")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut s = Server { child, stdin, out, next: 1, dir };
        let init = s.request(
            "initialize",
            json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "1"}}),
        );
        assert_eq!(init["result"]["serverInfo"]["name"], "hanji");
        assert!(init["result"]["instructions"].as_str().unwrap().contains("Read before every edit"));
        s.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
        s
    }

    fn send(&mut self, v: &Value) {
        writeln!(self.stdin, "{v}").unwrap();
        self.stdin.flush().unwrap();
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        loop {
            let mut line = String::new();
            assert!(self.out.read_line(&mut line).unwrap() > 0, "the server closed stdout");
            let v: Value = serde_json::from_str(&line).unwrap();
            if v["id"] == id {
                return v;
            }
        }
    }

    fn call(&mut self, tool: &str, args: Value) -> Reply {
        let v = self.request("tools/call", json!({"name": tool, "arguments": args}));
        let r = &v["result"];
        assert!(r.is_object(), "{tool}: {v}");
        let blocks = r["content"].as_array().unwrap().iter().map(|b| b["text"].as_str().unwrap().to_string()).collect();
        Reply { blocks, error: r["isError"] == true }
    }

    fn ok(&mut self, tool: &str, args: Value) -> Reply {
        let r = self.call(tool, args.clone());
        assert!(!r.error, "{tool} {args}: {}", r.blocks.join("\n"));
        r
    }

    fn err(&mut self, tool: &str, args: Value) -> Value {
        let r = self.call(tool, args.clone());
        assert!(r.error, "{tool} {args} succeeded: {}", r.blocks.join("\n"));
        r.json()["error"].clone()
    }

    /// (revision, text) of a read.
    fn read(&mut self, doc: &str) -> (u64, String) {
        let r = self.ok("hanji_read", json!({"doc_id": doc}));
        (r.json()["revision"].as_u64().unwrap(), r.blocks[1].clone())
    }

    fn path(&self, name: &str) -> String {
        self.dir.join(name).display().to_string()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn corpus(path: &str) -> String {
    format!("{}/../../{path}", env!("CARGO_MANIFEST_DIR"))
}

fn round_trip(s: &mut Server, file: &str, old: &str, new: &str) -> String {
    let o = s.ok("hanji_open", json!({"path": corpus(file)})).json();
    let id = o["doc_id"].as_str().unwrap().to_string();
    let (rev, text) = s.read(&id);
    assert!(text.contains(old), "{text}");
    let c = s.ok("hanji_edit", json!({"doc_id": id, "revision": rev, "old": old, "new": new})).json();
    assert_eq!(c["revision"], rev + 1);
    let (_, written) = s.read(&id);
    assert!(written.contains(new));
    let out = s.path(&format!("out.{}", file.rsplit('.').next().unwrap()));
    s.ok("hanji_export", json!({"doc_id": id, "path": out, "acknowledge_surfaced": true}));
    let back = s.ok("hanji_open", json!({"path": out})).json();
    let (_, reread) = s.read(back["doc_id"].as_str().unwrap());
    assert_eq!(reread, written, "PutGet");
    id
}

#[test]
fn tools_are_listed_with_descriptions_and_schemas() {
    let mut s = Server::start("list");
    let v = s.request("tools/list", json!({}));
    let tools = v["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(
        names.iter().copied().collect::<std::collections::BTreeSet<_>>(),
        [
            "hanji_open",
            "hanji_new",
            "hanji_read",
            "hanji_edit",
            "hanji_write",
            "hanji_ops",
            "hanji_validate",
            "hanji_export",
            "hanji_reimport",
            "hanji_history"
        ]
        .into_iter()
        .collect()
    );
    let edit = tools.iter().find(|t| t["name"] == "hanji_edit").unwrap();
    assert!(edit["description"].as_str().unwrap().contains("exactly once"));
    assert_eq!(edit["inputSchema"]["required"], json!(["doc_id", "revision"]));
    let new = tools.iter().find(|t| t["name"] == "hanji_new").unwrap();
    assert!(new["inputSchema"].to_string().contains("presentation"));
}

#[test]
fn docx() {
    round_trip(
        &mut Server::start("docx"),
        "prototype/remainder/corpus/korean-report.docx",
        "| ^^ | 종로 | 95 |",
        "| ^^ | 종로 | 97 |",
    );
}

#[test]
fn hwpx() {
    round_trip(&mut Server::start("hwpx"), "crates/hanji-hwpx/corpus/basic-table-01.hwpx", "| 5 | 6 |", "| 5 | 6.5 |");
}

#[test]
fn pptx() {
    round_trip(&mut Server::start("pptx"), "crates/hanji-pptx/corpus/korean-deck.pptx", "핵심 지표", "핵심 성과 지표");
}

#[test]
fn xlsx() {
    let mut s = Server::start("xlsx");
    let id = round_trip(
        &mut s,
        "crates/hanji-xlsx/corpus/korean-sales.xlsx",
        "| 원가 | number | #,##0 |",
        "| 원가 | number | #,##0.0 |",
    );
    let ops = json!([{"op": "set", "range": "매출!D4", "values": [[11111111]]}]);
    let c = s.ok("hanji_ops", json!({"doc_id": id, "revision": 2, "ops": ops})).json();
    assert_eq!(c["revision"], 3);
    let r = s.ok("hanji_read", json!({"doc_id": id, "table": "Sales", "rows": "4:4"}));
    assert!(r.blocks[2].contains("| 4 | 2026-01 | 강남 | 가전 | 11,111,111 |"), "{}", r.blocks[2]);
    // ops as a JSON string works too.
    let c = s.ok("hanji_ops", json!({"doc_id": id, "revision": 3, "ops": "[{\"op\": \"set\", \"range\": \"매출!D4\", \"values\": [[1]]}]"})).json();
    assert_eq!(c["revision"], 4);
}

#[test]
fn a_new_presentation_written_whole() {
    let mut s = Server::start("new");
    let o = s.ok("hanji_new", json!({"type": "presentation"})).json();
    let id = o["doc_id"].as_str().unwrap().to_string();
    let (rev, _) = s.read(&id);
    let text = "---\ntype: presentation\nformat: pptx\nschema: 1\n---\n\nlayout: Title Slide\n::title::\n3분기 보고\n\n---\n\nlayout: Title and Content\n::title::\n핵심 지표\n::body::\n- 매출 12% 증가\n::notes::\n강조\n";
    s.ok("hanji_write", json!({"doc_id": id, "revision": rev, "text": text}));
    // Stored in canonical form (§5.3): the slide size, and each slot's box from its layout.
    let canonical = "---\ntype: presentation\nformat: pptx\nschema: 1\nsize: 960 x 540 pt\n---\n\nlayout: Title Slide\n::title box=\"120 88 720 188\"::\n3분기 보고\n\n---\n\nlayout: Title and Content\n::title box=\"66 29 828 104\"::\n핵심 지표\n::body box=\"66 144 828 343\"::\n- 매출 12% 증가\n::notes::\n강조\n";
    assert_eq!(s.read(&id).1, canonical);
    let out = s.path("deck.pptx");
    s.ok("hanji_export", json!({"doc_id": id, "path": out}));
    let back = s.ok("hanji_open", json!({"path": out})).json();
    assert_eq!(s.read(back["doc_id"].as_str().unwrap()).1, canonical);
    let v = s.ok("hanji_validate", json!({"text": text.replace("Title Slide", "Title Slid"), "doc_id": id})).json();
    assert_eq!(v["valid"], false);
    assert!(v["diagnostics"][0]["message"].as_str().unwrap().contains("\"Title Slide\""), "{v}");
}

#[test]
fn refusals_are_errors_with_the_reason() {
    let mut s = Server::start("refusals");
    s.ok("hanji_open", json!({"path": corpus("prototype/remainder/corpus/korean-report.docx")}));
    let d = json!("korean-report");
    s.ok("hanji_edit", json!({"doc_id": d, "revision": 1, "old": "4분기에는", "new": "4분기부터"}));
    let e = s.err("hanji_edit", json!({"doc_id": d, "revision": 1, "old": "대구", "new": "광주"}));
    assert_eq!((e["code"].as_str(), e["head"].as_u64()), (Some("stale_revision"), Some(2)));
    let r = s.call("hanji_edit", json!({"doc_id": d, "revision": 2, "old": "수도권", "new": "x"}));
    assert!(
        r.error && r.blocks[0].starts_with("ambiguous_match: the old text occurs 2 times (lines 14, 16)"),
        "{}",
        r.blocks[0]
    );
    let e = s.err("hanji_edit", json!({"doc_id": d, "revision": 2, "edits": [{"old": "4분기부터", "new": "4분기부터는"}, {"old": "<div style=\"Note\">", "new": "<div style=\"Callout\">"}]}));
    assert_eq!((e["code"].as_str(), e["edit"].as_u64()), (Some("invalid"), Some(2)));
    assert_eq!((e["diagnostics"][0]["line"].as_u64(), e["diagnostics"][0]["col"].as_u64()), (Some(16), Some(6)));
    let e = s.err("hanji_export", json!({"doc_id": d, "path": s.path("x.docx")}));
    assert_eq!(e["code"], "surfaced_not_acknowledged");
    assert_eq!(e["surfaced"][0]["kind"], "comment");
    let e = s.err("hanji_edit", json!({"doc_id": d, "revision": 2, "old": "a"}));
    assert_eq!(e["code"], "bad_request");
    let e = s.err("hanji_read", json!({"doc_id": "nope"}));
    assert!(e["message"].as_str().unwrap().contains("korean-report"), "{e}");
    let h = s.ok("hanji_history", json!({"doc_id": d})).json();
    assert_eq!(h["head"], 2);
}

#[test]
fn reimport_and_a_merged_edit() {
    let mut s = Server::start("reimport");
    s.ok("hanji_open", json!({"path": corpus("prototype/remainder/corpus/korean-report.docx")}));
    let d = json!("korean-report");
    let out = s.path("report.docx");
    s.ok("hanji_export", json!({"doc_id": d, "path": out, "acknowledge_surfaced": true}));
    let mut parts = hanji_package::package::read(&std::fs::read(&out).unwrap()).unwrap();
    let doc = parts.iter_mut().find(|p| p.name == "word/document.xml").unwrap();
    doc.data = String::from_utf8(doc.data.clone()).unwrap().replacen(">분기에는 <", ">분기부터는 <", 1).into_bytes();
    std::fs::write(&out, hanji_package::package::write(&parts).unwrap()).unwrap();
    let r = s.ok("hanji_reimport", json!({"doc_id": d, "path": out})).json();
    assert_eq!(r["revision"], 2);
    assert!(r["diff"].as_str().unwrap().contains("+4분기부터는"));
    let c = s
        .ok("hanji_edit", json!({"doc_id": d, "revision": 1, "old": "| 합계 || 215 |", "new": "| 합계 || 217 |"}))
        .json();
    assert_eq!(c["rebased_over"], json!([2]));
    let diff = s.ok("hanji_history", json!({"doc_id": d, "from": 1, "to": 3})).json();
    let diff = diff["diff"].as_str().unwrap();
    assert!(diff.contains("+| 합계 || 217 |") && diff.contains("+4분기부터는"), "{diff}");
}

#[test]
fn a_large_document_is_read_in_parts() {
    let mut s = Server::start("large");
    s.ok("hanji_new", json!({"type": "document"}));
    let mut text = String::from("---\ntype: document\nformat: docx\nschema: 1\n---\n");
    for c in 1..=20 {
        text.push_str(&format!("# 장 {c}\n\n"));
        for p in 1..=120 {
            text.push_str(&format!("장 {c}, 문단 {p}: 지점별 매출과 원가를 정리한다.\n\n"));
        }
    }
    s.ok("hanji_write", json!({"doc_id": "document", "revision": 1, "text": text}));
    let r = s.ok("hanji_read", json!({"doc_id": "document"}));
    let head = r.json();
    assert_eq!(head["partial"], true);
    assert!(r.blocks[1].len() <= 60_000 && head["next"].is_string());
    assert_eq!(head["outline"].as_array().unwrap().len(), 20);
    let r = s.ok("hanji_read", json!({"doc_id": "document", "section": "장 20"}));
    assert!(
        r.blocks[1].starts_with("# 장 20\n\n") && r.blocks[1].ends_with("문단 120: 지점별 매출과 원가를 정리한다.\n")
    );
}
