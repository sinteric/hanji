//! The `hanji` binary end to end, per format: open → read → edit → export →
//! reopen with PutGet holding, and the refusals as exit status 1 with the
//! error in JSON.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::Value;

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
    assert_eq!(env.read("document"), text);
    env.ok(&["export", "document", &env.path("new.hwpx")]);
    let back = env.ok(&["open", &env.path("new.hwpx")]);
    assert_eq!(env.read(back["doc_id"].as_str().unwrap()), text);
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
    assert_eq!(e["lines"], serde_json::json!([8, 10]));
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
    assert_eq!((e["diagnostics"][0]["line"].as_u64(), e["diagnostics"][0]["col"].as_u64()), (Some(10), Some(6)));
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
    let r = env.ok(&["reimport", "korean-report", &out]);
    assert_eq!((r["parent"].as_u64(), r["revision"].as_u64()), (Some(1), Some(2)));
    assert!(r["diff"].as_str().unwrap().contains("+4분기부터는 부산과 대구에 지점을 연다."), "{r:#}");
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
    let out = env.cmd().args(["read", "document", "--lines", "6:8"]).output().unwrap();
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "# 장 1\n\n장 1, 문단 1: 지점별 매출과 원가를 정리한다.\n");
    assert!(String::from_utf8(out.stderr).unwrap().contains("lines 6–8 of"));
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
