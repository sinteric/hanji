# Transitional-schema check

Validates .pptx, .docx and .xlsx packages (and the macro and template variants) against the
ISO/IEC 29500 transitional schemas, with the rules outside the schemas that Office enforces
when it opens a file. A file that fails it can make Word, PowerPoint or Excel offer to
repair it. CI runs it on every corpus source and on every file of the Office check kit
(`validation/office-kit`).

```sh
validation/ooxml-schema/fetch.sh     # the schemas, into xsd/ (git-ignored), checked by SHA-256
pip install lxml
python3 validation/ooxml-schema/validate.py crates/*/corpus prototype/remainder/corpus
cargo run --release --manifest-path validation/office-kit/Cargo.toml
python3 validation/ooxml-schema/validate.py target/office-kit
```

What it checks is in `validate.py`'s docstring. In short:

- every XML part in a transitional namespace, against the schemas, after markup
  compatibility processing (what an application that knows only those namespaces sees);
- content types for every part; relationship targets that exist; `r:id`-style references
  that name a relationship of their part; declared `mc:Ignorable` and `Requires` prefixes;
- in decks: unique slide, master and layout ids, unique shape ids per slide, and
  animation targets and build entries that name a shape of their slide;
- embedded packages (a chart's workbook) the same way.

Parts in other namespaces (Office 2010+ extensions, custom XML, VML drawings) are listed
as not checked.

**The schemas.** ISO/IEC 29500-4:2016 (transitional) and the OPC schemas of Part 2, as
python-pptx keeps them under `spec/` at a pinned commit (`fetch.sh`, `SHA256SUMS`). One
erratum is patched when they are loaded (`ERRATA` in `validate.py`): `a:buSzPct` accepts
the thousandths-of-a-percent form every Office version writes.

**The allowlist.** `allowlist.txt` names known deviations in third-party files (and in
files LibreOffice wrote for the corpus) that Office accepts, each with its reason, as
`file-glob | part-glob | message-regex | reason`. Anything else fails. Files hanji
generates or exports have no entries.
