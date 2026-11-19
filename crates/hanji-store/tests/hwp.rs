//! Binary HWP through rhwp (DESIGN.md §5.6), through the library API: an .hwp
//! opens as an hwpx document by its header whatever its name, with rhwp's
//! loss report; an edited revision exports as .hwpx by default and as .hwp
//! when asked, and the .hwp reopens as the text written; an export that would
//! lose content is refused with what it would lose, and so is an .hwp export
//! of a file whose unedited round trip, measured at open, changes it; a CFB file that is not HWP
//! and an .hwp export of another format are refused; a binary .hwp
//! re-imports into its document.

use hanji_store::*;

const CFB: [u8; 8] = [0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1];

fn corpus(path: &str) -> (String, Vec<u8>) {
    let full = format!("{}/../../{path}", env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(&full).unwrap_or_else(|e| panic!("{full}: {e}"));
    (path.rsplit('/').next().unwrap().to_string(), bytes)
}

fn hwp(name: &str) -> (String, Vec<u8>) {
    corpus(&format!("crates/hanji-hwpx/corpus/hwp/{name}"))
}

fn ws() -> Workspace<MemStorage> {
    Workspace::new(MemStorage::new())
}

fn text(ws: &Workspace<MemStorage>, id: &str) -> String {
    let r = ws.read(id, None, &Window::default()).unwrap();
    assert!(!r.partial);
    r.text
}

/// The text with each placeholder's id blank: rhwp writes a preserved object
/// back with its own bytes, so the id that hashes them changes.
fn without_keep_ids(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find("<keep id=\"") {
        let (head, tail) = rest.split_at(i + "<keep id=\"".len());
        out.push_str(head);
        rest = &tail[tail.find('"').unwrap()..];
    }
    out.push_str(rest);
    out
}

const ACK: ExportOptions = ExportOptions::ACKNOWLEDGE_SURFACED;

/// Open an .hwp, append a paragraph, and return the document id and the
/// text written.
fn open_and_edit(ws: &mut Workspace<MemStorage>, name: &str) -> (String, String) {
    let (name, bytes) = hwp(name);
    let o = ws
        .open_bytes(&name, &bytes, None)
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    assert_eq!(o.format, Format::Hwpx, "{name}");
    let from = o.from_hwp.as_ref().expect("opened through rhwp");
    assert!(from.losses.is_empty(), "{name}: {:?}", from.losses);
    assert!(
        from.hwp_changes.is_empty(),
        "{name}: {:?}",
        from.hwp_changes
    );
    let mut written = text(ws, &o.doc_id).trim_end().to_string();
    written.push_str("\n\n한지에서 덧붙인 문단.\n");
    let c = ws
        .write(&o.doc_id, o.revision, &written)
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(!c.unchanged);
    let now = text(ws, &o.doc_id);
    (o.doc_id, now)
}

#[test]
fn an_edited_hwp_exports_as_hwpx_by_default_and_as_hwp_when_asked() {
    for name in ["eq-002.hwp", "footnote-01.hwp", "hcar-001.hwp"] {
        let mut ws = ws();
        let (id, written) = open_and_edit(&mut ws, name);

        let (out, hwpx) = ws.export_bytes(&id, None, &ACK).unwrap();
        assert!(!out.hwp, "{name}");
        assert!(hwpx.starts_with(b"PK"), "{name}: hwpx is the default");
        let back = ws.open_bytes("back.hwpx", &hwpx, None).unwrap();
        assert!(back.from_hwp.is_none());
        assert_eq!(text(&ws, &back.doc_id), written, "{name}: PutGet");

        let (out, bin) = ws
            .export_bytes(&id, None, &ACK.with_hwp())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(out.hwp, "{name}");
        assert_eq!(out.format, Format::Hwpx);
        assert!(bin.starts_with(&CFB), "{name}: binary HWP 5.0");
        let (again, same) = ws.export_bytes(&id, None, &ACK.with_hwp()).unwrap();
        assert_eq!(out.digest, again.digest, "{name}: §8 determinism");
        assert_eq!(bin, same);
        let back = ws.open_bytes("back.hwp", &bin, None).unwrap();
        assert!(back.from_hwp.unwrap().losses.is_empty(), "{name}");
        assert_eq!(
            without_keep_ids(&text(&ws, &back.doc_id)),
            without_keep_ids(&written),
            "{name}: the .hwp reopens as the text written"
        );
    }
}

#[test]
fn an_hwp_opens_by_its_header_whatever_its_name() {
    let (_, bytes) = hwp("hcar-001.hwp");
    let mut ws = ws();
    let named = ws.open_bytes("hcar-001.hwp", &bytes, None).unwrap();
    for name in ["hcar-001.hwpx", "hcar-001.docx", "hcar-001"] {
        let o = ws
            .open_bytes(name, &bytes, None)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(o.format, Format::Hwpx, "{name}");
        assert!(o.from_hwp.is_some(), "{name}");
        assert_eq!(text(&ws, &o.doc_id), text(&ws, &named.doc_id), "{name}");
    }
}

#[test]
fn an_hwpx_named_hwp_opens_as_hwpx() {
    let mut ws = ws();
    let (_, bin) = hwp("hcar-001.hwp");
    let o = ws.open_bytes("hcar-001.hwp", &bin, None).unwrap();
    let (_, hwpx) = ws.export_bytes(&o.doc_id, None, &ACK).unwrap();
    let again = ws.open_bytes("misnamed.hwp", &hwpx, None).unwrap();
    assert_eq!(again.format, Format::Hwpx);
    assert!(again.from_hwp.is_none(), "it is a zip, not binary HWP");
}

#[test]
fn an_export_that_would_lose_content_is_refused_with_the_losses() {
    // rhwp reads this file's precise character strokes ("정밀 문자 획")
    // but cannot write them back into .hwp, and says so.
    let mut ws = ws();
    let (name, bytes) = hwp("stroke.hwp");
    let o = ws.open_bytes(&name, &bytes, None).unwrap();
    assert!(o.from_hwp.is_some());
    let e = ws
        .export_bytes(&o.doc_id, None, &ACK.with_hwp())
        .unwrap_err();
    assert_eq!(e.code, Code::Refused, "{e}");
    assert!(e.message.contains("without losing content"), "{e}");
    assert!(e.message.contains("\n- "), "the losses are listed: {e}");
    assert!(e.message.contains(".hwpx instead"), "{e}");
    let (out, hwpx) = ws.export_bytes(&o.doc_id, None, &ACK).unwrap();
    assert!(!out.hwp && hwpx.starts_with(b"PK"), "hwpx still exports");
}

#[test]
fn hwp_export_of_another_format_is_a_bad_request() {
    let mut ws = ws();
    let (name, bytes) = corpus("prototype/remainder/corpus/korean-report.docx");
    let o = ws.open_bytes(&name, &bytes, None).unwrap();
    let e = ws
        .export_bytes(&o.doc_id, None, &ACK.with_hwp())
        .unwrap_err();
    assert_eq!(e.code, Code::BadRequest, "{e}");
}

#[test]
fn a_cfb_file_that_is_not_hwp_is_a_package_error() {
    let mut fake = CFB.to_vec();
    fake.resize(4096, 0);
    let e = ws().open_bytes("old.doc", &fake, None).unwrap_err();
    assert_eq!(e.code, Code::Package, "{e}");
}

#[test]
fn a_binary_hwp_reimports_into_its_document() {
    let mut ws = ws();
    let (id, written) = open_and_edit(&mut ws, "hcar-001.hwp");
    let (out, bin) = ws.export_bytes(&id, None, &ACK.with_hwp()).unwrap();
    let r = ws
        .reimport_bytes_at_revision(&id, out.revision, "hcar-001.hwp", &bin)
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(r.from_hwp.unwrap().losses.is_empty());
    assert_eq!(
        without_keep_ids(&text(&ws, &id)),
        without_keep_ids(&written)
    );

    let (name, bytes) = corpus("prototype/remainder/corpus/korean-report.docx");
    let docx = ws.open_bytes(&name, &bytes, None).unwrap();
    let e = ws
        .reimport_bytes(&docx.doc_id, "hcar-001.hwp", &bin)
        .unwrap_err();
    assert_eq!(e.code, Code::BadRequest, "{e}");
}

#[test]
fn an_hwp_that_does_not_come_back_the_same_exports_as_hwpx_only() {
    // rhwp opens this file's first section, which has no column definition,
    // on the default A4 page as Hancom does, so its declared margins do not
    // survive HWP to HWPX and back. Each direction's own check passes; the
    // round trip measured at open does not.
    let mut ws = ws();
    let (name, bytes) = hwp("page-activated.hwp");
    let o = ws.open_bytes(&name, &bytes, None).unwrap();
    let from = o.from_hwp.unwrap();
    assert!(from.losses.is_empty(), "{:?}", from.losses);
    assert!(
        from.hwp_changes.iter().any(|c| c.contains("page_def")),
        "{:?}",
        from.hwp_changes
    );
    let e = ws
        .export_bytes(&o.doc_id, None, &ACK.with_hwp())
        .unwrap_err();
    assert_eq!(e.code, Code::Refused, "{e}");
    assert!(e.message.contains("even unedited"), "{e}");
    assert!(e.message.contains("margin_left"), "{e}");
    let (_, hwpx) = ws.export_bytes(&o.doc_id, None, &ACK).unwrap();
    assert!(hwpx.starts_with(b"PK"), "hwpx still exports");
    // The measurement belongs to the document, not to the open call.
    let mut written = text(&ws, &o.doc_id).trim_end().to_string();
    written.push_str("\n\n덧붙인 문단.\n");
    ws.write(&o.doc_id, o.revision, &written).unwrap();
    let e = ws
        .export_bytes(&o.doc_id, None, &ACK.with_hwp())
        .unwrap_err();
    assert_eq!(e.code, Code::Refused, "{e}");
}
