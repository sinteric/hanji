//! The committed `<keep/>` census (`docs/keep-census.md` and `.json`) is the
//! one the corpora give today: a change that adds or removes placeholders
//! regenerates it with
//! `cargo run --locked -p hanji-store --example keep_census -- --write`.
//!
//! Native only: the census reads the same text on wasm32, so a second run
//! there would only repeat this one.
#![cfg(not(target_os = "wasi"))]

#[path = "../examples/keep_census/census.rs"]
mod census;

#[test]
fn the_committed_census_is_current() {
    let root = census::root();
    let report = census::take(&root);
    for (path, want) in [
        (census::MARKDOWN, census::markdown(&report)),
        (census::JSON, census::json(&report)),
    ] {
        // A checkout may have turned the line ends into CRLF.
        let have = std::fs::read_to_string(root.join(path))
            .unwrap_or_default()
            .replace("\r\n", "\n");
        let first = have
            .lines()
            .zip(want.lines())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| have.lines().count().min(want.lines().count()));
        assert!(
            have == want,
            "{path} is out of date from line {}: it has {:?} where the census gives {:?}. Run `{}`.",
            first + 1,
            have.lines().nth(first).unwrap_or("(end of file)"),
            want.lines().nth(first).unwrap_or("(end of file)"),
            census::COMMAND
        );
    }
}

#[test]
fn the_census_counts_each_unescaped_keep_tag_by_its_kind() {
    let text = concat!(
        "Before<keep id=\"k1\" kind=\"footnote\" summary=\"a &quot;/&gt; b\"/> after.\n",
        "<keep id=\"s3\" kind=\"diagram\" summary=\"x/>y &lt;keep\" box=\"18 52 675 240\"/>\n",
        "\\<keep id=\"k2\" kind=\"text\" summary=\"\"/> is text, and so is <keeper>.\n",
        "\\\\<keep id=\"k3\" kind=\"tracked&amp;insert\" summary=\"\"/>\n",
    );
    assert_eq!(
        census::keep_kinds(text).unwrap(),
        ["footnote", "diagram", "tracked&insert"],
        "an escaped `\\<keep` is text; an escaped backslash before one is not an escape"
    );
    for bad in [
        "<keep id=\"k1\" kind=\"field\"",
        "<keep id=\"k1\" kind=field/>",
        "<keep id=\"k1\" summary=\"\"/>",
    ] {
        assert!(
            census::keep_kinds(bad).is_err(),
            "{bad:?} is not a placeholder"
        );
    }
}
