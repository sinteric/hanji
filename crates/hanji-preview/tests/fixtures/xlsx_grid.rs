use hanji_core::Part;
use hanji_package::package;

pub const STYLES: &str = r##"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><numFmts count="1"><numFmt numFmtId="164" formatCode="#,##0.00"/></numFmts><fonts count="2"><font><name val="Calibri"/><sz val="11"/><color rgb="FF000000"/></font><font><name val="Calibri"/><sz val="14"/><b/><color rgb="FF123456"/></font></fonts><fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="solid"><fgColor rgb="FFCCEEDD"/></patternFill></fill></fills><borders count="2"><border/><border><bottom style="medium"><color rgb="FF005500"/></bottom></border></borders><cellXfs count="3"><xf fontId="0" fillId="0" borderId="0" numFmtId="0"/><xf fontId="1" fillId="1" borderId="1" numFmtId="0"><alignment horizontal="center" vertical="center" wrapText="1"/></xf><xf fontId="0" fillId="0" borderId="0" numFmtId="164"/></cellXfs></styleSheet>"##;
pub const GRID: &str = r##"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><dimension ref="A1:XFD1048576"/><sheetFormatPr defaultRowHeight="15" defaultColWidth="8.43"/><cols><col min="1" max="1" width="20"/><col min="2" max="2" width="10"/><col min="3" max="3" hidden="1" width="10"/></cols><sheetData><row r="1" ht="30" customHeight="1"><c r="A1" t="s" s="1"><v>0</v></c></row><row r="2"><c r="A2" t="s"><v>1</v></c><c r="B2" s="2"><v>1234.5</v></c><c r="C2" t="inlineStr"><is><t>HIDDEN</t></is></c></row><row r="3"><c r="A3"><f>SUM(B2,1)</f><v>1235.5</v></c><c r="B3"><f>1+2</f><v/></c></row><row r="4"><c r="A4" t="str"><f>IF(1,"","")</f><v/></c><c r="B4"><f t="shared" si="0"/><v>8</v></c></row><row r="5" hidden="1"><c r="A5"><v>999</v></c></row><row r="1048576"><c r="XFD1048576" t="inlineStr"><is><t>LAST</t></is></c></row></sheetData><mergeCells count="1"><mergeCell ref="A1:B1"/></mergeCells><conditionalFormatting sqref="B2"><cfRule type="expression" priority="1"><formula>B2&gt;0</formula></cfRule></conditionalFormatting><drawing/><hyperlinks/><pageSetup orientation="landscape"/></worksheet>"##;

pub fn build(sheet: &str, calc: &str, styles: &str, extra: &[(&str, &[u8])]) -> Vec<u8> {
    let workbook = format!(
        r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="매출 &amp; Sales" sheetId="1" r:id="s1"/><sheet name="Hidden" sheetId="2" r:id="s2" state="hidden"/></sheets>{calc}</workbook>"#
    );
    let mut parts = vec![
        ("_rels/.rels", r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="main" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#.to_string()),
        ("xl/workbook.xml", workbook),
        ("xl/_rels/workbook.xml.rels", r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="s1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="s2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet2.xml"/><Relationship Id="st" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="ss" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/></Relationships>"#.to_string()),
        ("xl/styles.xml", styles.to_string()),
        ("xl/sharedStrings.xml", r#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><si><t>매출 &amp; 売上</t></si><si><t>Ω 😀 &lt;script&gt;alert(1)&lt;/script&gt;</t></si></sst>"#.to_string()),
        ("xl/worksheets/sheet1.xml", sheet.to_string()),
        ("xl/worksheets/sheet2.xml", r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData/></worksheet>"#.to_string()),
    ].into_iter().map(|(name, s)| Part { name: name.into(), data: s.into_bytes(), dos_time: 0, external_attr: 0, deflate: true }).collect::<Vec<_>>();
    parts.extend(extra.iter().map(|(name, data)| Part {
        name: (*name).into(),
        data: data.to_vec(),
        dos_time: 0,
        external_attr: 0,
        deflate: true,
    }));
    package::write(&parts).unwrap()
}
