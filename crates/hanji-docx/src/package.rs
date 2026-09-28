//! Zip package read/write. Parts other than the split ones are copied
//! through byte for byte (uncompressed content); timestamps are kept, so the
//! same input gives the same bytes (§8 determinism).

use std::io::{Cursor, Read, Write};

use hanji_core::Part;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipArchive, ZipWriter};

/// Most bytes a package may expand to (§8: a zip bomb is refused, not unpacked).
pub const MAX_UNPACKED: u64 = 1 << 30;

pub fn read(bytes: &[u8]) -> Result<Vec<Part>, String> {
    read_limited(bytes, MAX_UNPACKED)
}

/// [`read`] with at most `limit` unpacked bytes.
pub fn read_limited(bytes: &[u8], limit: u64) -> Result<Vec<Part>, String> {
    let mut z = ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("not a zip package: {e}"))?;
    let mut parts = vec![];
    let mut left = limit;
    for i in 0..z.len() {
        let f = z.by_index(i).map_err(|e| format!("zip entry {i}: {e}"))?;
        if f.is_dir() {
            continue;
        }
        let name = f.name().to_string();
        let mut data = vec![];
        // The declared size can lie; the limit applies to the bytes actually inflated.
        let mut f = f.take(left + 1);
        f.read_to_end(&mut data).map_err(|e| format!("{name}: {e}"))?;
        if data.len() as u64 > left {
            return Err(format!("the package expands to more than {limit} bytes ({name})"));
        }
        left -= data.len() as u64;
        let f = f.into_inner();
        let dt = f.last_modified().unwrap_or_default();
        let (d, t): (u16, u16) = dt.into();
        parts.push(Part {
            name: f.name().to_string(),
            data,
            dos_time: ((d as u32) << 16) | t as u32,
            external_attr: f.unix_mode().unwrap_or(0),
            deflate: f.compression() != CompressionMethod::Stored,
        });
    }
    Ok(parts)
}

pub fn write(parts: &[Part]) -> Result<Vec<u8>, String> {
    let mut z = ZipWriter::new(Cursor::new(Vec::new()));
    for p in parts {
        let dt = DateTime::try_from(((p.dos_time >> 16) as u16, p.dos_time as u16)).unwrap_or_default();
        let mut opts = SimpleFileOptions::default()
            .compression_method(if p.deflate { CompressionMethod::Deflated } else { CompressionMethod::Stored })
            .last_modified_time(dt);
        if p.external_attr != 0 {
            opts = opts.unix_permissions(p.external_attr);
        }
        z.start_file(p.name.as_str(), opts).map_err(|e| format!("{}: {e}", p.name))?;
        z.write_all(&p.data).map_err(|e| format!("{}: {e}", p.name))?;
    }
    Ok(z.finish().map_err(|e| e.to_string())?.into_inner())
}

pub fn get<'a>(parts: &'a [Part], name: &str) -> Option<&'a [u8]> {
    parts.iter().find(|p| p.name == name).map(|p| p.data.as_slice())
}
