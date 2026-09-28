//! tcspike: drive rdocx 0.14.0 over the spike inputs.
//!
//!   tcspike compare <original.docx> <edited.docx> <out.docx> <granularity>
//!       original.compare_with_options(edited, AUTHOR, DATE) -> out.docx
//!   tcspike check <tracked.docx> <prefix>
//!       re-open, list revisions, save/re-open, accept_all / reject_all,
//!       tracked-view PDF. Writes <prefix>.accepted.docx, <prefix>.rejected.docx,
//!       <prefix>.resaved.docx, <prefix>.tracked.pdf.
//!
//! Every command prints one JSON object on stdout.

use rdocx::{ComparisonGranularity, ComparisonOptions, Document, RenderOptions, RevisionKind, RevisionView};
use serde_json::{json, Value};

const AUTHOR: &str = "hanji (model edit)";
const DATE: &str = "2026-09-28T00:00:00Z";

fn kind(k: RevisionKind) -> &'static str {
    match k {
        RevisionKind::Insertion => "ins",
        RevisionKind::Deletion => "del",
        RevisionKind::MoveFrom => "moveFrom",
        RevisionKind::MoveTo => "moveTo",
        RevisionKind::RunPropertyChange => "rPrChange",
        RevisionKind::ParagraphPropertyChange => "pPrChange",
        RevisionKind::TablePropertyChange => "tblPrChange",
        RevisionKind::SectionPropertyChange => "sectPrChange",
    }
}

fn revisions(doc: &Document) -> Value {
    let revs = doc.revisions();
    let mut by_kind = serde_json::Map::new();
    let mut authors = std::collections::BTreeSet::new();
    let mut dates = std::collections::BTreeSet::new();
    for r in &revs {
        let e = by_kind.entry(kind(r.kind())).or_insert(json!(0));
        *e = json!(e.as_u64().unwrap() + 1);
        authors.insert(r.author().to_owned());
        dates.insert(r.timestamp().unwrap_or("").to_owned());
    }
    json!({"count": revs.len(), "by_kind": by_kind, "authors": authors, "dates": dates})
}

fn err(stage: &str, e: impl std::fmt::Display) -> Value {
    json!({"ok": false, "stage": stage, "error": e.to_string()})
}

fn compare(orig: &str, edited: &str, out: &str, gran: &str) -> Value {
    let mut a = match Document::open(orig) {
        Ok(d) => d,
        Err(e) => return err("open original", e),
    };
    let b = match Document::open(edited) {
        Ok(d) => d,
        Err(e) => return err("open edited", e),
    };
    let options = ComparisonOptions {
        granularity: match gran {
            "run" => ComparisonGranularity::Run,
            "char" => ComparisonGranularity::Character,
            _ => ComparisonGranularity::Word,
        },
        ..Default::default()
    };
    let diags = match a.compare_with_options(&b, AUTHOR, DATE, &options) {
        Ok(d) => d,
        Err(e) => return err("compare", e),
    };
    if let Err(e) = a.save(out) {
        return err("save", e);
    }
    json!({
        "ok": true,
        "diagnostics": diags.iter().map(|d| format!("{}: {}", d.location, d.message)).collect::<Vec<_>>(),
        "revisions": revisions(&a),
    })
}

fn check(path: &str, prefix: &str) -> Value {
    let doc = match Document::open(path) {
        Ok(d) => d,
        Err(e) => return err("reopen", e),
    };
    let revs = revisions(&doc);
    // save without change, re-open: revisions must survive
    let mut resaved = Document::open(path).expect("opened above");
    let resaved_path = format!("{prefix}.resaved.docx");
    let resave = match resaved.save(&resaved_path).and_then(|_| Document::open(&resaved_path)) {
        Ok(d) => json!({"ok": true, "revisions": revisions(&d)}),
        Err(e) => err("resave", e),
    };
    let mut acc = Document::open(path).expect("opened above");
    let accepted = match acc.accept_all() {
        Ok(n) => match acc.save(format!("{prefix}.accepted.docx")) {
            Ok(()) => json!({"ok": true, "resolved": n, "left": acc.revisions().len()}),
            Err(e) => err("save accepted", e),
        },
        Err(e) => err("accept_all", e),
    };
    let mut rej = Document::open(path).expect("opened above");
    let rejected = match rej.reject_all() {
        Ok(n) => match rej.save(format!("{prefix}.rejected.docx")) {
            Ok(()) => json!({"ok": true, "resolved": n, "left": rej.revisions().len()}),
            Err(e) => err("save rejected", e),
        },
        Err(e) => err("reject_all", e),
    };
    let pdf = match doc.to_pdf_deterministic_with_options(RenderOptions { revision_view: RevisionView::Tracked }) {
        Ok(bytes) => match std::fs::write(format!("{prefix}.tracked.pdf"), &bytes) {
            Ok(()) => json!({"ok": true, "bytes": bytes.len()}),
            Err(e) => err("write pdf", e),
        },
        Err(e) => err("tracked pdf", e),
    };
    json!({"ok": true, "revisions": revs, "resave": resave, "accept_all": accepted,
           "reject_all": rejected, "tracked_pdf": pdf})
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = match args.get(1).map(String::as_str) {
        Some("compare") if args.len() == 6 => compare(&args[2], &args[3], &args[4], &args[5]),
        Some("check") if args.len() == 4 => check(&args[2], &args[3]),
        _ => {
            eprintln!("usage: tcspike compare <orig> <edited> <out> <run|word|char> | check <docx> <prefix>");
            std::process::exit(2);
        }
    };
    println!("{out}");
}
