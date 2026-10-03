use hanji_package::package;

/// Full-width vertical body table between authored 120/96 pt stories.
/// Existing package relationships are retained from the actual Korean corpus.
pub fn build(alignment: &str, vertical_alignment: &str) -> Vec<u8> {
    let bytes = include_bytes!("../../../../prototype/preview/baseline/01-docx-untouched-korean-report.docx");
    let mut parts = package::read(bytes).unwrap();
    let document = format!(
        r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body><w:p><w:pPr><w:jc w:val="{alignment}"/><w:spacing w:before="0" w:after="0"/></w:pPr><w:r><w:rPr><w:rFonts w:ascii="Arial" w:hAnsi="Arial"/></w:rPr><w:t>Body paragraph aligned against the reserved vertical measure</w:t></w:r></w:p><w:tbl><w:tblPr><w:tblW w:w="5000" w:type="pct"/></w:tblPr><w:tblGrid/><w:tr><w:tc><w:tcPr><w:shd w:fill="CCEEFF"/></w:tcPr><w:p><w:r><w:rPr><w:rFonts w:ascii="Arial" w:hAnsi="Arial"/></w:rPr><w:t>Body table</w:t></w:r></w:p></w:tc></w:tr></w:tbl><w:sectPr><w:headerReference w:type="default" r:id="rId3"/><w:footerReference w:type="default" r:id="rId4"/><w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720"/><w:textDirection w:val="tbRl"/><w:vAlign w:val="{vertical_alignment}"/></w:sectPr></w:body></w:document>"#
    );
    for (name, content) in [
        ("word/document.xml", document),
        ("word/header1.xml", story("hdr", "Tall header", 2400)),
        ("word/footer1.xml", story("ftr", "Tall footer", 1920)),
    ] {
        let part = parts.iter_mut().find(|p| p.name == name).unwrap();
        part.data = content.into_bytes();
    }
    package::write(&parts).unwrap()
}

fn story(root: &str, text: &str, line: u32) -> String {
    format!(
        r#"<w:{root} xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:p><w:pPr><w:spacing w:before="0" w:after="0" w:line="{line}" w:lineRule="exact"/></w:pPr><w:r><w:rPr><w:rFonts w:ascii="Arial" w:hAnsi="Arial"/></w:rPr><w:t>{text}</w:t></w:r></w:p></w:{root}>"#
    )
}
