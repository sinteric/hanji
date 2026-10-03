//! A deterministic byte-only worksheet job, suitable for native/WASI comparison.
#[path = "../tests/fixtures/xlsx_grid.rs"]
mod fixture;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use hanji_preview::{
    fonts,
    xlsx::{open_xlsx, XlsxOptions},
    FontData, FontOptions, PageData, PageFormat,
};

pub fn windows_json() -> String {
    let data = hanji_preview::sfnt::build(
        oxml_layout::bundled_fonts::bundled_font_data()[0].1,
        0,
        &hanji_preview::sfnt::Adjust {
            family: "Caller Sans".into(),
            bold: false,
            italic: false,
            line_em: None,
            ea_advance: None,
        },
    )
    .unwrap();
    let opts = FontOptions {
        fonts: vec![FontData::new(data)],
        aliases: Some(
            fonts::Aliases::parse("[[family]]\nnames = [\"Calibri\"]\nmetric = [\"Caller Sans\"]\n").unwrap(),
        ),
    };
    let bytes = fixture::build(fixture::GRID, r#"<calcPr fullCalcOnLoad="1"/>"#, fixture::STYLES, &[]);
    let mut workbook = open_xlsx(&bytes, XlsxOptions::default()).unwrap();
    let mut out = vec![];
    for (sheet, range) in [(0, "A1:C5"), (0, "XFD1048576"), (1, "A1")] {
        let window = workbook.render_window_with_fonts(sheet, range, &opts).unwrap();
        let PageData::Svg(svg) = window.render(PageFormat::Svg).unwrap().data else { unreachable!() };
        let PageData::Png(png) = window.render(PageFormat::Png { dpi: 72.0 }).unwrap().data else { unreachable!() };
        out.push(serde_json::json!({ "sheet": window.sheet, "range": window.range, "page": window.page_info(),
            "cells": window.cells, "fonts": window.fonts(), "diagnostics": window.diagnostics(),
            "svg": svg, "pngBase64": STANDARD.encode(&png), "html": window.html("XLSX example") }));
    }
    serde_json::to_string(&out).unwrap()
}

fn main() {
    println!("{}", windows_json());
}
