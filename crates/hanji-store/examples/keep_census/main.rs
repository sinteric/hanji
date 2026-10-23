//! The `<keep/>` census: how many placeholders hanji's text shows for the
//! files of the GetPut/PutGet corpora, by format, by kind and by file. From
//! the repository root:
//!
//! - `cargo run --locked -p hanji-store --example keep_census` prints the
//!   report (Markdown, its tables padded to read as plain text);
//! - `-- --json` prints it as JSON;
//! - `-- --write` rewrites the committed report, `docs/keep-census.md` and
//!   `docs/keep-census.json`.
//!
//! The report is deterministic: the files are sorted by path, which is
//! relative to the repository root, and it holds no times.

use std::process::ExitCode;

mod census;

const USAGE: &str = "usage: cargo run -p hanji-store --example keep_census [-- --json | --write]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = match args.as_slice() {
        [] => "",
        [a] if a == "--json" || a == "--write" => a.as_str(),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    let root = census::root();
    let report = census::take(&root);
    match mode {
        "--json" => print!("{}", census::json(&report)),
        "--write" => {
            for (path, text) in [
                (census::MARKDOWN, census::markdown(&report)),
                (census::JSON, census::json(&report)),
            ] {
                let path = root.join(path);
                let written = path
                    .parent()
                    .map_or(Ok(()), std::fs::create_dir_all)
                    .and_then(|()| std::fs::write(&path, text));
                if let Err(e) = written {
                    eprintln!("{}: {e}", path.display());
                    return ExitCode::FAILURE;
                }
            }
            println!("wrote {} and {}", census::MARKDOWN, census::JSON);
        }
        _ => print!("{}", census::markdown(&report)),
    }
    ExitCode::SUCCESS
}
