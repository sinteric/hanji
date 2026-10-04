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

/// Six source-owned OPC cases. Only the first header differs in each pair.
pub fn selected_story(first_height: u32, vertical: bool, overflow: bool) -> Vec<u8> {
    fn paragraph(text: &str, properties: &str) -> String {
        format!(
            r#"<w:p><w:pPr>{properties}<w:spacing w:before="0" w:after="0"/></w:pPr><w:r><w:rPr><w:rFonts w:ascii="Arial" w:hAnsi="Arial"/></w:rPr><w:t xml:space="preserve">{text}</w:t></w:r></w:p>"#
        )
    }
    fn table(text: &str) -> String {
        format!(
            r#"<w:tbl><w:tblPr><w:tblW w:w="5000" w:type="pct"/></w:tblPr><w:tblGrid/><w:tr><w:tc><w:tcPr><w:shd w:fill="CCEEFF"/></w:tcPr>{}</w:tc></w:tr></w:tbl>"#,
            paragraph(text, "")
        )
    }
    let body = if overflow {
        paragraph("Page one body", "") + &paragraph(&"Word ".repeat(4000), "<w:pageBreakBefore/>")
    } else {
        paragraph("Page one body", "")
            + &table("Page one table")
            + &paragraph("Page two body", "<w:pageBreakBefore/>")
            + &table("Page two table")
    };
    let direction = if vertical { "<w:textDirection w:val=\"tbRl\"/>" } else { "" };
    let w = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    let r = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    let document = format!(
        r#"<w:document xmlns:w="{w}" xmlns:r="{r}"><w:body>{body}<w:sectPr><w:headerReference w:type="default" r:id="defaultHeader"/><w:headerReference w:type="first" r:id="firstHeader"/><w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720"/><w:titlePg/>{direction}</w:sectPr></w:body></w:document>"#
    );
    let parts = [
        ("[Content_Types].xml", r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/><Override PartName="/word/header2.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/></Types>"#.to_owned()),
        ("_rels/.rels", format!(r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="officeDocument" Type="{r}/officeDocument" Target="word/document.xml"/></Relationships>"#)),
        ("word/document.xml", document),
        ("word/_rels/document.xml.rels", format!(r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="defaultHeader" Type="{r}/header" Target="header1.xml"/><Relationship Id="firstHeader" Type="{r}/header" Target="header2.xml"/></Relationships>"#)),
        ("word/header1.xml", story("hdr", "Default header", 240)),
        ("word/header2.xml", story("hdr", "First header", first_height * 20)),
    ].into_iter().map(|(name, xml)| hanji_core::Part {
        name: name.to_owned(), data: xml.into_bytes(), dos_time: 0, external_attr: 0, deflate: true,
    }).collect::<Vec<_>>();
    package::write(&parts).unwrap()
}

/// Owned landscape case whose selected measure rounds during transposition.
pub fn fractional_landscape() -> Vec<u8> {
    let mut parts = package::read(&selected_story(120, true, false)).unwrap();
    let r = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    for part in &mut parts {
        let text = std::str::from_utf8(&part.data).unwrap();
        let replacement = match part.name.as_str() {
            "word/document.xml" => text
                .replace("w:w=\"12240\" w:h=\"15840\"", "w:w=\"15840\" w:h=\"12240\"")
                .replace("<w:pgSz", "<w:footerReference w:type=\"default\" r:id=\"footer\"/><w:footerReference w:type=\"first\" r:id=\"footer\"/><w:pgSz"),
            "word/header1.xml" => story("hdr", "Default header", 8001),
            "word/header2.xml" => story("hdr", "First header", 6001),
            "word/_rels/document.xml.rels" => text.replace(
                "</Relationships>",
                &format!(r#"<Relationship Id="footer" Type="{r}/footer" Target="footer1.xml"/></Relationships>"#),
            ),
            "[Content_Types].xml" => text.replace(
                "</Types>",
                r#"<Override PartName="/word/footer1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"/></Types>"#,
            ),
            _ => continue,
        };
        part.data = replacement.into_bytes();
    }
    parts.push(hanji_core::Part {
        name: "word/footer1.xml".to_owned(),
        data: story("ftr", "Shared footer", 2002).into_bytes(),
        dos_time: 0,
        external_attr: 0,
        deflate: true,
    });
    package::write(&parts).unwrap()
}
