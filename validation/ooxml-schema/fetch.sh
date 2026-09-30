#!/bin/sh
# Downloads the ISO/IEC 29500 transitional schemas (Part 4) and the OPC schemas (Part 2) into
# xsd/ (git-ignored), from python-pptx's copy at a pinned commit, and checks each file's SHA-256.
# ISO publishes these schemas freely; python-pptx keeps them unchanged under spec/.
set -e
cd "$(dirname "$0")"
REV=278b47b1dedd5b46ee84c286e77cdfb0bf4594be
BASE=https://raw.githubusercontent.com/scanny/python-pptx/$REV/spec
mkdir -p xsd
while read -r sha path; do
  f=xsd/$(basename "$path")
  if [ -f "$f" ] && echo "$sha  $f" | sha256sum -c --quiet - 2>/dev/null; then continue; fi
  curl -fsSL -o "$f.tmp" "$BASE/$path"
  echo "$sha  $f.tmp" | sha256sum -c --quiet - && mv "$f.tmp" "$f"
done < SHA256SUMS
echo "ok  $(ls xsd | wc -l) schemas in $(pwd)/xsd"
