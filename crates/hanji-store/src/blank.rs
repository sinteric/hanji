//! The blank packages a new file starts from when no template is given
//! (DESIGN.md §2 rule 3: a new file is an existing file that starts from a
//! blank or template package). Their parts are in `blank/<format>/`, zipped
//! here with a fixed timestamp so the same blank gives the same bytes (§8).
//!
//! - docx: A4, Normal (Korean East Asian font and `ko-KR`), Heading 1–6,
//!   Title, Subtitle, Quote, caption, List Paragraph, Table Grid (the default
//!   table style) and a bullet and a decimal list; no metadata.
//! - hwpx: the header (Hancom's default styles 바탕글, 본문, 개요 1–10, …),
//!   settings and page setup of rhwp's `basic-table-01.hwpx` sample (MIT, in
//!   `hanji-hwpx/corpus/`), with a bullet added; one empty paragraph.
//! - pptx: 16:9, one master with Title Slide, Title and Content, Two Content,
//!   Title Only and Blank, a notes master, and one empty Title Slide.
//! - xlsx: one empty sheet, Sheet1.

use hanji_core::Part;
use hanji_package::package;

use crate::formats::Format;

/// 1980-01-01 00:00 as a DOS date and time.
const DOS_TIME: u32 = 0x0021_0000;

macro_rules! parts {
    ($dir:literal: $($name:literal),* $(,)?) => {
        &[$(($name, include_bytes!(concat!("../blank/", $dir, "/", $name)) as &[u8])),*]
    };
}

const DOCX: &[(&str, &[u8])] = parts!("docx":
    "[Content_Types].xml",
    "_rels/.rels",
    "word/document.xml",
    "word/_rels/document.xml.rels",
    "word/styles.xml",
    "word/numbering.xml",
    "word/settings.xml",
);

const HWPX: &[(&str, &[u8])] = parts!("hwpx":
    "mimetype",
    "version.xml",
    "Contents/header.xml",
    "Contents/section0.xml",
    "settings.xml",
    "META-INF/container.xml",
    "Contents/content.hpf",
    "META-INF/manifest.xml",
);

const PPTX: &[(&str, &[u8])] = parts!("pptx":
    "[Content_Types].xml",
    "_rels/.rels",
    "ppt/presentation.xml",
    "ppt/_rels/presentation.xml.rels",
    "ppt/slideMasters/slideMaster1.xml",
    "ppt/slideMasters/_rels/slideMaster1.xml.rels",
    "ppt/slideLayouts/slideLayout1.xml",
    "ppt/slideLayouts/_rels/slideLayout1.xml.rels",
    "ppt/slideLayouts/slideLayout2.xml",
    "ppt/slideLayouts/_rels/slideLayout2.xml.rels",
    "ppt/slideLayouts/slideLayout3.xml",
    "ppt/slideLayouts/_rels/slideLayout3.xml.rels",
    "ppt/slideLayouts/slideLayout4.xml",
    "ppt/slideLayouts/_rels/slideLayout4.xml.rels",
    "ppt/slideLayouts/slideLayout5.xml",
    "ppt/slideLayouts/_rels/slideLayout5.xml.rels",
    "ppt/slides/slide1.xml",
    "ppt/slides/_rels/slide1.xml.rels",
    "ppt/notesMasters/notesMaster1.xml",
    "ppt/notesMasters/_rels/notesMaster1.xml.rels",
    "ppt/theme/theme1.xml",
    "ppt/theme/theme2.xml",
    "ppt/presProps.xml",
    "ppt/tableStyles.xml",
);

const XLSX: &[(&str, &[u8])] = parts!("xlsx":
    "[Content_Types].xml",
    "_rels/.rels",
    "xl/workbook.xml",
    "xl/_rels/workbook.xml.rels",
    "xl/worksheets/sheet1.xml",
    "xl/styles.xml",
    "xl/sharedStrings.xml",
);

/// The blank package of `format`.
pub fn package(format: Format) -> Vec<u8> {
    let files = match format {
        Format::Docx => DOCX,
        Format::Hwpx => HWPX,
        Format::Pptx => PPTX,
        Format::Xlsx => XLSX,
    };
    let parts: Vec<Part> = files
        .iter()
        .map(|(name, data)| Part {
            name: name.to_string(),
            data: data.to_vec(),
            dos_time: DOS_TIME,
            external_attr: 0,
            // hwpx: the mimetype part comes first, stored.
            deflate: *name != "mimetype",
        })
        .collect();
    package::write(&parts).expect("the blank packages zip")
}
