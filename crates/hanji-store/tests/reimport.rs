//! Re-imports must not silently replace changes committed after an export.

use std::cell::RefCell;
use std::rc::Rc;

use hanji_core::Part;
use hanji_package::package;
use hanji_store::*;

#[derive(Clone, Default)]
struct SharedStorage(Rc<RefCell<MemStorage>>);

impl Storage for SharedStorage {
    fn get(&self, key: &str) -> std::result::Result<Option<Vec<u8>>, String> {
        self.0.borrow().get(key)
    }

    fn put(&mut self, key: &str, data: &[u8]) -> std::result::Result<(), String> {
        self.0.borrow_mut().put(key, data)
    }

    fn keys(&self, prefix: &str) -> std::result::Result<Vec<String>, String> {
        self.0.borrow().keys(prefix)
    }
}

impl SharedStorage {
    fn snapshot(&self) -> Vec<(String, Vec<u8>)> {
        self.keys("").unwrap().into_iter().map(|k| (k.clone(), self.get(&k).unwrap().unwrap())).collect()
    }
}

const ACK: ExportOptions = ExportOptions { acknowledge_surfaced: true, tracked_changes: false, format: None };

fn person_edits(bytes: &[u8], part: &str, from: &str, to: &str) -> Vec<u8> {
    let mut parts: Vec<Part> = package::read(bytes).unwrap();
    let part = parts.iter_mut().find(|p| p.name == part).unwrap();
    let xml = String::from_utf8(part.data.clone()).unwrap();
    assert!(xml.contains(from), "missing {from:?} in {}", part.name);
    part.data = xml.replacen(from, to, 1).into_bytes();
    package::write(&parts).unwrap()
}

#[test]
fn docx_refuses_a_stale_export_after_a_committed_edit_without_changing_the_store() {
    let storage = SharedStorage::default();
    let mut ws = Workspace::new(storage.clone());
    let source = include_bytes!("../../../prototype/remainder/corpus/korean-report.docx");
    let o = ws.open_bytes("report.docx", source, None).unwrap();
    let (export, bytes) = ws.export_bytes(&o.doc_id, None, &ACK).unwrap();
    // The person and model edit different lines, but the model's edit is
    // already committed: this is not the pending-edit rebase flow.
    let polished = person_edits(&bytes, "word/document.xml", ">분기에는 <", ">분기부터는 <");
    let c = ws
        .edit(&o.doc_id, 1, &[TextEdit { old: "| ^^ | 종로 | 95 |".into(), new: "| ^^ | 종로 | 97 |".into() }])
        .unwrap();
    let before = storage.snapshot();
    let current = ws.export_bytes(&o.doc_id, None, &ACK).unwrap().1;

    let err = ws.reimport_bytes_at_revision(&o.doc_id, export.revision, "polished.docx", &polished).unwrap_err();
    assert_eq!(err.code, Code::StaleRevision);
    assert_eq!(err.detail.head, Some(c.revision));
    assert!(err.message.contains("already committed") && err.message.contains("--replace-head"), "{err}");
    assert_eq!(storage.snapshot(), before, "no record, text or remainder may be written on refusal");
    assert_eq!(ws.export_bytes(&o.doc_id, None, &ACK).unwrap().1, current);

    // Deliberate authoritative replacement remains available to library
    // callers, just as --replace-head exposes it in the CLI.
    let r = ws.reimport_bytes(&o.doc_id, "polished.docx", &polished).unwrap();
    assert_eq!((r.parent, r.revision), (2, 3));
    let text = ws.read(&o.doc_id, None, &Window::default()).unwrap().text;
    assert!(text.contains("| ^^ | 종로 | 95 |") && text.contains("4분기부터는"), "{text}");
}

#[test]
fn xlsx_refuses_a_stale_export_even_when_only_cells_changed() {
    let storage = SharedStorage::default();
    let mut ws = Workspace::new(storage.clone());
    let source = include_bytes!("../../hanji-xlsx/corpus/korean-sales.xlsx");
    let o = ws.open_bytes("sales.xlsx", source, None).unwrap();
    let (export, bytes) = ws.export_bytes(&o.doc_id, None, &ACK).unwrap();
    let before_text = ws.read(&o.doc_id, None, &Window::default()).unwrap().text;
    let c = ws.ops(&o.doc_id, 1, r#"[{"op":"set","range":"매출!D4","values":[[11111111]]}]"#).unwrap();
    assert_eq!(ws.read(&o.doc_id, None, &Window::default()).unwrap().text, before_text);
    let before = storage.snapshot();
    let current = ws.export_bytes(&o.doc_id, None, &ACK).unwrap().1;

    let err = ws.reimport_bytes_at_revision(&o.doc_id, export.revision, "sales.xlsx", &bytes).unwrap_err();
    assert_eq!(err.code, Code::StaleRevision);
    assert_eq!(err.detail.head, Some(c.revision));
    assert_eq!(storage.snapshot(), before);
    assert_eq!(ws.export_bytes(&o.doc_id, None, &ACK).unwrap().1, current);

    // A fresh export carries the committed value. Editing that file and
    // naming its base is safe, including values held only in the remainder.
    let polished = person_edits(&current, "xl/worksheets/sheet1.xml", "<v>11111111</v>", "<v>22222222</v>");
    let r = ws.reimport_bytes_at_revision(&o.doc_id, c.revision, "sales.xlsx", &polished).unwrap();
    assert_eq!((r.parent, r.revision), (2, 3));
    let w = Window { table: Some("Sales".into()), rows: Some("4:4".into()), ..Default::default() };
    assert!(ws.read(&o.doc_id, None, &w).unwrap().data.unwrap().contains("22,222,222"));

    // Re-imported edits are committed changes too; an older base cannot
    // undo them, even when no model edit has happened since that base.
    let before = storage.snapshot();
    assert_eq!(
        ws.reimport_bytes_at_revision(&o.doc_id, 2, "sales.xlsx", &current).unwrap_err().code,
        Code::StaleRevision
    );
    assert_eq!(storage.snapshot(), before);
}

#[test]
fn checked_reimport_checks_the_revision_before_parsing_the_package() {
    let storage = SharedStorage::default();
    let mut ws = Workspace::new(storage.clone());
    let o = ws.create(DocType::Spreadsheet, None, None).unwrap();
    ws.ops(&o.doc_id, 1, r#"[{"op":"set","range":"Sheet1!A1","values":[[1]]}]"#).unwrap();
    let before = storage.snapshot();
    let err = ws.reimport_bytes_at_revision(&o.doc_id, 1, "invalid.xlsx", b"invalid package").unwrap_err();
    assert_eq!((err.code, err.detail.head), (Code::StaleRevision, Some(2)));
    let err = ws.reimport_bytes_at_revision(&o.doc_id, 99, "invalid.xlsx", b"invalid package").unwrap_err();
    assert_eq!((err.code, err.detail.head), (Code::NotFound, Some(2)));
    assert_eq!(storage.snapshot(), before);
}
