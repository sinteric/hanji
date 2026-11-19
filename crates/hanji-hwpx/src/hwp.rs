//! Binary HWP through rhwp: HWP and HWPX are one document model in two
//! containers (HWP 5.0 is a CFB file of binary records, HWPX a zip of
//! OWPML), so hanji edits an .hwp as the HWPX rhwp writes for it, and can
//! write the result back as .hwp.
//!
//! [`to_hwpx`] reads an HWP 5.0 (or HWP 3.0) file with rhwp and writes it as
//! HWPX. [`to_hwp`] is the way back. Each direction is rhwp's own export
//! with its own `--verify` check: rhwp reads what it wrote and compares the
//! two document models, leaving out only the differences that are the two
//! containers' spelling of one thing. What did not cross is in
//! [`Converted::losses`]: rhwp's content-loss report, then the model
//! differences. A caller writes an .hwp only when that list is empty, so a
//! save that would lose content is never written silently.
//!
//! Only the containers are here. Neither direction touches the engine:
//! the engine imports and exports HWPX as it does for any .hwpx file.
//! [`is_hwp`] needs no rhwp; the conversions need the `hwp` feature.

#[cfg(feature = "hwp")]
use hanji_core::EngineError;

/// The first bytes of a CFB (OLE compound) file, which HWP 5.0 is.
const CFB_MAGIC: [u8; 8] = [0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1];
/// The first bytes of an HWP 3.0 file.
const HWP3_MAGIC: &[u8] = b"HWP Document File";
/// How many model differences a loss list names before it says how many more.
#[cfg(feature = "hwp")]
const LISTED: usize = 20;

/// Whether `bytes` look like binary HWP by their header: a CFB container
/// (HWP 5.0, but also legacy .doc, .xls and .ppt, which rhwp then refuses)
/// or the HWP 3.0 signature.
pub fn is_hwp(bytes: &[u8]) -> bool {
    bytes.starts_with(&CFB_MAGIC) || bytes.starts_with(HWP3_MAGIC)
}

#[cfg(feature = "hwp")]
/// One direction's output, and what did not cross.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct Converted {
    pub bytes: Vec<u8>,
    /// rhwp's content-loss entries, then the differences between the two
    /// models, each one line. Empty when nothing was lost.
    pub losses: Vec<String>,
}

#[cfg(feature = "hwp")]
fn package(m: String) -> EngineError {
    EngineError::Package(m)
}

/// rhwp's verification differences as loss lines, at most [`LISTED`] of
/// them and a count of the rest.
#[cfg(feature = "hwp")]
fn differences(diff: &rhwp::serializer::hwpx::roundtrip::IrDiff) -> Vec<String> {
    listed(diff, "the document model changed")
}

#[cfg(feature = "hwp")]
fn listed(diff: &rhwp::serializer::hwpx::roundtrip::IrDiff, what: &str) -> Vec<String> {
    let mut out: Vec<String> = diff
        .differences
        .iter()
        .take(LISTED)
        .map(|d| format!("{what}: {d}"))
        .collect();
    if diff.differences.len() > LISTED {
        out.push(format!(
            "and {} more model differences",
            diff.differences.len() - LISTED
        ));
    }
    out
}

#[cfg(feature = "hwp")]
fn reported(report: &rhwp::serializer::ContentLossReport) -> Vec<String> {
    report.losses().iter().map(|l| l.to_string()).collect()
}

/// An HWP file as HWPX (see the module documentation). This is rhwp's
/// `export-hwpx --verify`: its HWPX export, and the comparison of the file's
/// model with the model of the HWPX it wrote, without the differences that
/// are only the two containers' spelling of one thing.
#[cfg(feature = "hwp")]
pub fn to_hwpx(hwp: &[u8]) -> Result<Converted, EngineError> {
    use rhwp::document_core::DocumentCore;
    use rhwp::parser::FileFormat;
    use rhwp::serializer::hwpx::roundtrip as rt;
    let source = rhwp::parser::detect_format(hwp);
    match source {
        FileFormat::Hwp | FileFormat::Hwp3 => {}
        FileFormat::DrmProtected => {
            return Err(package(
                "the file is DRM-protected (an encrypted security container): open it in the program that protects it and save an unprotected copy".into(),
            ))
        }
        _ => return Err(package("not an HWP file: it has neither the HWP 5.0 (CFB) nor the HWP 3.0 header".into())),
    }
    let core = DocumentCore::from_bytes(hwp).map_err(|e| {
        package(format!(
            "rhwp cannot read it as HWP: {e} (a .doc, .xls or .ppt is a CFB file too, and hanji does not open those)"
        ))
    })?;
    let written = core
        .export_hwpx_native_with_report()
        .map_err(|e| package(format!("rhwp cannot write it as HWPX: {e}")))?;
    let (bytes, report) = written.into_parts();
    let mut losses = reported(&report);
    let back = DocumentCore::from_bytes(&bytes)
        .map_err(|e| package(format!("rhwp cannot read back the HWPX it wrote: {e}")))?;
    let expected = rt::hwpx_export_expected_document(&core.hwpx_export_expected_document());
    let reloaded = rt::hwpx_export_expected_document(back.document());
    let diff = rt::diff_exported_hwpx_documents(&expected, &reloaded);
    let diff = match source {
        FileFormat::Hwp3 => rt::strip_hwp3_to_hwpx_noise(&expected, &reloaded, diff),
        _ => rt::strip_hwp_to_hwpx_noise(diff),
    };
    losses.extend(differences(&diff));
    Ok(Converted { bytes, losses })
}

/// What writing `hwpx` as .hwp would change in `hwp`, the file it was read
/// from: [`to_hwp`]'s losses, then the differences between rhwp's model of
/// `hwp` and of the .hwp written, as rhwp parses the two files (without the
/// differences that are only the two containers' spelling of one thing).
/// Empty when the unedited file survives HWP to HWPX and back.
///
/// [`to_hwpx`] and [`to_hwp`] each check one direction against rhwp's
/// model of its input, and rhwp's model of an .hwp is not always the file:
/// opening it settles what Hancom would show (a section without a column
/// definition takes the default A4 page, an empty cell paragraph gets a line,
/// duplicate page-number controls collapse to one). rhwp's own .hwp save
/// writes the original records back; the way through HWPX cannot, so this
/// compares the two files themselves.
#[cfg(feature = "hwp")]
pub fn round_trip(hwp: &[u8], hwpx: &[u8]) -> Result<Vec<String>, EngineError> {
    use rhwp::serializer::hwpx::roundtrip as rt;
    let back = to_hwp(hwpx)?;
    let mut changes = back.losses;
    let parse = |b: &[u8]| {
        rhwp::parser::parse_document(b)
            .map_err(|e| package(format!("rhwp cannot parse the .hwp: {e}")))
    };
    let diff = rt::strip_hwpx_to_hwp_noise(rt::diff_documents(&parse(hwp)?, &parse(&back.bytes)?));
    changes.extend(listed(&diff, "the .hwp would come back changed"));
    Ok(changes)
}

/// An HWPX package as HWP 5.0, checked (see the module documentation). This
/// is rhwp's `convert --verify`: the HWPX-to-HWP adapter on a copy of the
/// model, the HWP write, and the comparison of the adapted model with the
/// model of the file it wrote, without the differences that are only the two
/// containers' spelling of one thing.
#[cfg(feature = "hwp")]
pub fn to_hwp(hwpx: &[u8]) -> Result<Converted, EngineError> {
    use rhwp::document_core::DocumentCore;
    use rhwp::serializer::hwpx::roundtrip as rt;
    let core = DocumentCore::from_bytes(hwpx)
        .map_err(|e| package(format!("rhwp cannot read the HWPX: {e}")))?;
    let snapshot = core.prepare_hwp_export_snapshot();
    let written = rhwp::serializer::serialize_document_with_report(snapshot.document())
        .map_err(|e| package(format!("rhwp cannot write it as HWP: {e}")))?;
    let (bytes, report) = written.into_parts();
    let mut losses = reported(&report);
    let back = DocumentCore::from_bytes(&bytes)
        .map_err(|e| package(format!("rhwp cannot read back the HWP it wrote: {e}")))?;
    let diff = rt::strip_hwpx_to_hwp_noise(rt::diff_exported_hwp_documents(
        snapshot.document(),
        back.document(),
    ));
    losses.extend(differences(&diff));
    Ok(Converted { bytes, losses })
}
