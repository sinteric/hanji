//! GetPut on files: import, export the text unchanged, compare every part.
//! `cargo run -p hanji-pptx --example getput -- a.pptx b.pptx …`

use hanji_core::{Engine, ImportOptions};
use hanji_package::{package, xml};
use hanji_pptx::PptxEngine;

fn main() {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).unwrap();
        let name = path.rsplit('/').next().unwrap();
        let opts = ImportOptions { neutralise: false, template: None };
        let imp = match PptxEngine.import(&bytes, &opts) {
            Ok(i) => i,
            Err(e) => {
                println!("{name}: IMPORT {e}");
                continue;
            }
        };
        let out = match PptxEngine.export(&imp.text, &imp.remainder) {
            Ok(o) => o,
            Err(e) => {
                println!("{name}: EXPORT {e}");
                continue;
            }
        };
        let (a, b) = (package::read(&bytes).unwrap(), package::read(&out).unwrap());
        let mut bad = vec![];
        if a.len() != b.len() {
            bad.push(format!("{} parts vs {}", a.len(), b.len()));
        }
        for (x, y) in a.iter().zip(&b) {
            if x.name != y.name {
                bad.push(format!("order {} vs {}", x.name, y.name));
                continue;
            }
            if x.data != y.data {
                let (cx, cy) = (xml::canon_part(&x.data).ok(), xml::canon_part(&y.data).ok());
                if cx != cy || cx.is_none() {
                    bad.push(x.name.clone());
                    if std::env::var("DIFF").is_ok() {
                        let (cx, cy) = (cx.unwrap_or_default(), cy.unwrap_or_default());
                        let k = cx.bytes().zip(cy.bytes()).take_while(|(p, q)| p == q).count();
                        println!(
                            "   {}\n   A {}\n   B {}",
                            x.name,
                            &cx[k.saturating_sub(200)..(k + 200).min(cx.len())],
                            &cy[k.saturating_sub(200)..(k + 200).min(cy.len())]
                        );
                    }
                }
            }
        }
        let again = PptxEngine.import(&out, &opts).map(|i| i.text == imp.text);
        println!(
            "{name}: {} slides-text {} {}",
            imp.remainder.entries.len(),
            match again {
                Ok(true) => "same",
                Ok(false) => "DIFFERS",
                Err(_) => "REIMPORT-FAILS",
            },
            if bad.is_empty() { "GetPut ok".to_string() } else { format!("GetPut FAIL {bad:?}") }
        );
    }
}
