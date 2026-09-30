#!/bin/sh
# Re-downloads the audit's third-party decks at pinned commits and checks them against the
# committed copies' SHA-256 (see ../SOURCES.md). The synthetic decks are made by make_synth.py.
set -e
cd "$(dirname "$0")"
fetch() { # url name sha256
  curl -fsSL -o "$2.tmp" "$1"
  echo "$3  $2.tmp" | sha256sum -c --quiet - && mv "$2.tmp" "$2" && echo "ok  $2"
}
fetch https://raw.githubusercontent.com/ONLYOFFICE/document-templates/0a52fb76fa53fddf7d70da28878537cde3d29421/sample/sample.pptx \
  onlyoffice-sample.pptx 524d1144395e5889f217ee9f84c7331433e2ac99735d933f82390ca9a49dcb5d
fetch https://raw.githubusercontent.com/dotnet/Open-XML-SDK/431ab05cf160248cc3885a4a766026d4f8243792/test/DocumentFormat.OpenXml.Tests.Assets/assets/TestFiles/o09_Performance_typical.pptx \
  o09_Performance_typical.pptx 7b24556c2bc71b59f2e25574a25e416a99f20f99e0a62b676ea5bf061def805f
fetch https://raw.githubusercontent.com/LibreOffice/core/7642fc49b32eaac1e1ab8639df991902fd9a48ff/sd/qa/unit/data/pptx/slide-section-test.pptx \
  slide-section-test.pptx aebee5a724a9b5b8020c41fe789df9b3805265f20bf9a87dd28d8b0ed556deb0
