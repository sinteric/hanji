use hanji_preview::{
    quality::{
        Consequence, DiagnosticCode, PreviewSource, QualityReport, QualityStatus, RenderingStatus, Severity,
        Strictness, WithQuality,
    },
    Diagnostic, FontsReport, MissingGlyph,
};

#[test]
fn empty_diagnostics_never_assert_complete_detection_or_native_fidelity() {
    for source in [PreviewSource::Pptx, PreviewSource::Docx, PreviewSource::Hwpx, PreviewSource::Xlsx] {
        let report = QualityReport::inspect(source, &[], &FontsReport::default(), &[]);
        assert_eq!(report.version, 1);
        assert_eq!(report.rendering, RenderingStatus::Ready);
        assert_eq!(report.status, QualityStatus::NoKnownIssues);
        assert!(!report.coverage.complete);
        assert!(report.coverage.unchecked.contains(&"native-application-fidelity"));
        assert!(report.enforce(Strictness::CriticalLosses).is_ok());
        assert_eq!(report.rendered().rendering, RenderingStatus::Rendered);
    }
}

#[test]
fn known_omission_has_stable_code_source_object_page_and_opt_in_refusal() {
    let legacy = Diagnostic {
        path: "pages[2].elements[4].children[1]".into(),
        message: "image bytes are neither PNG nor JPEG and were omitted".into(),
    };
    let original = serde_json::to_value(&legacy).unwrap();
    let report =
        QualityReport::inspect(PreviewSource::Pptx, std::slice::from_ref(&legacy), &FontsReport::default(), &[]);
    assert_eq!(report.status, QualityStatus::UnsupportedContent);
    let d = &report.diagnostics[0];
    assert_eq!(d.code, DiagnosticCode::ImageOmitted);
    assert_eq!(d.source, PreviewSource::Pptx);
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(d.consequence, Consequence::UnsupportedOmission);
    assert_eq!(d.location.page_index, Some(2));
    assert_eq!(d.location.object_path.as_deref(), Some(legacy.path.as_str()));
    let serialized = serde_json::to_value(d).unwrap();
    assert_eq!(serialized["code"], "svg.image-omitted");
    assert_eq!(serialized["path"], original["path"]);
    assert_eq!(serialized["message"], original["message"]);
    assert!(report.enforce(Strictness::AllowKnownLosses).is_ok());
    let refusal = report.enforce(Strictness::CriticalLosses).unwrap_err();
    assert_eq!(refusal.code, "preview.critical-loss-refused");
    assert_eq!(refusal.quality.rendering, RenderingStatus::Refused);
    assert_eq!(refusal.quality.diagnostics, report.diagnostics);
    assert_eq!(serde_json::to_value(&legacy).unwrap(), original);
}

#[test]
fn opaque_upstream_and_future_diagnostics_never_become_critical_by_keyword_guess() {
    let legacy = [
        Diagnostic {
            path: "layout.diagnostics[4]".into(),
            message: "critical image omission; numeric clipping; failed font".into(),
        },
        Diagnostic { path: "future.feature".into(), message: "critical unsupported omission".into() },
    ];
    let report = QualityReport::inspect(PreviewSource::Docx, &legacy, &FontsReport::default(), &[]);
    assert_eq!(report.diagnostics[0].code, DiagnosticCode::LayoutReport);
    assert_eq!(report.diagnostics[1].code, DiagnosticCode::UnclassifiedDiagnostic);
    assert!(report.diagnostics.iter().all(|d| d.severity == Severity::Warning));
    assert!(report.diagnostics.iter().all(|d| d.location.object_path.is_none() && d.location.page_index.is_none()));
    assert!(report.enforce(Strictness::CriticalLosses).is_ok());
    assert!(!report.coverage.complete);
}

#[test]
fn hancom_aggregate_counters_have_page_locations_without_invented_table_or_cell_objects() {
    let legacy = [
        Diagnostic {
            path: "pages[2].tables".into(),
            message: "rhwp reports 3 overflowing cell lines; table content may be clipped".into(),
        },
        Diagnostic { path: "pages[2].tables".into(), message: "rhwp reports 2 table overlaps".into() },
    ];
    let report = QualityReport::inspect(PreviewSource::Hwpx, &legacy, &FontsReport::default(), &[]);
    assert_eq!(report.diagnostics[0].code, DiagnosticCode::TableOverflow);
    assert_eq!(report.diagnostics[1].code, DiagnosticCode::TableOverlap);
    for d in &report.diagnostics {
        assert_eq!(d.location.page_index, Some(2));
        assert_eq!(d.location.object_path, None);
        assert_eq!(d.location.cell, None);
    }
    assert!(report.enforce(Strictness::CriticalLosses).is_ok()); // Heuristic counters alone do not prove a critical loss.
}

#[test]
fn font_loss_does_not_depend_on_warning_strings_and_preserves_page_provenance() {
    let fonts = FontsReport {
        missing_glyphs: vec![MissingGlyph {
            char: "U+274F".into(),
            requested: "missing symbol font".into(),
            pages: vec![1, 3],
        }],
        missing_glyphs_total: 101,
        ..Default::default()
    };
    let report = QualityReport::inspect(PreviewSource::Hwpx, &[], &fonts, &[]);
    assert_eq!(report.status, QualityStatus::Degraded);
    assert_eq!(report.diagnostics[0].code, DiagnosticCode::MissingGlyphs);
    assert_eq!(report.diagnostics[0].location.page_index, None);
    assert_eq!(report.diagnostics[0].location.page_indices, vec![0, 2]);
    assert!(report.diagnostics.iter().any(|d| d.path == "fonts.missing_glyphs_total" && d.message.contains("101")));
    assert!(report.enforce(Strictness::CriticalLosses).is_err());
    assert!(report.coverage.unchecked.contains(&"exhaustive-hancom-control-detection"));
}

#[test]
fn worksheet_missing_cache_is_distinct_from_overflow_and_zero() {
    let legacy = [
        Diagnostic {
            path: "sheets[3].cells[B12].formula".into(),
            message: "formula has no cached result; shown as #UNEVALUATED; no evaluation was performed".into(),
        },
        Diagnostic {
            path: "sheets[3].cells[C12].formula".into(),
            message: "stored formula result shown; cache freshness is unverified; no evaluation was performed".into(),
        },
    ];
    let report = QualityReport::inspect(PreviewSource::Xlsx, &legacy, &FontsReport::default(), &[]);
    let missing = &report.diagnostics[0];
    assert_eq!(missing.code, DiagnosticCode::FormulaCacheMissing);
    assert_eq!(missing.severity, Severity::Error);
    assert_eq!(missing.consequence, Consequence::ValueUnavailable);
    assert_eq!(missing.location.sheet_index, Some(3));
    assert_eq!(missing.location.cell.as_deref(), Some("B12"));
    assert_eq!(missing.location.page_index, None); // Worksheet address, not a printed page.
    assert_eq!(report.diagnostics[1].code, DiagnosticCode::FormulaCacheUnverified);
    assert_eq!(report.diagnostics[1].severity, Severity::Warning);
}

#[test]
fn serialization_wrapper_is_additive_to_existing_machine_fields() {
    let legacy = serde_json::json!({"doc_id":"d", "cells":[{"address":"A1","display":"0"}], "diagnostics":[]});
    let quality = QualityReport::inspect(PreviewSource::Xlsx, &[], &FontsReport::default(), &[]).rendered();
    let output = serde_json::to_value(WithQuality { result: &legacy, quality }).unwrap();
    for (name, value) in legacy.as_object().unwrap() {
        assert_eq!(&output[name], value);
    }
    assert_eq!(output["quality"]["version"], 1);
    assert_eq!(output["quality"]["rendering"], "rendered");
    assert_eq!(output["quality"]["coverage"]["complete"], false);
}
