#!/bin/sh
# Regenerates korean-report.docx from korean-report.fodt (synthetic, CC0) with LibreOffice.
set -e
cd "$(dirname "$0")"
out=$(mktemp -d)
soffice --headless --convert-to 'docx:MS Word 2007 XML' --outdir "$out" korean-report.fodt >/dev/null
mv "$out/korean-report.docx" korean-report.docx
rmdir "$out"
