#!/bin/sh
# render.sh [lo|rdocx|rpptx|rhwp]...: render baseline/ with LibreOffice and/or the engine CLIs (README.md).
# Needs fonts/files (fonts/FONTS.md); for lo: soffice with Writer, Calc, Impress and libreoffice-h2orestart;
# for the engines: cargo. Output: lo-pdf/, lo-pdf-hwpx/, renders/<candidate>/NN-page-K.{svg,png}.
set -eu
cd "$(dirname "$0")"
export FONTCONFIG_FILE="$PWD/fonts/fonts.conf"
PY=${PY:-python3}
R=${RENDERS:-renders}
[ $# -gt 0 ] || set -- lo rdocx rpptx rhwp
for what in "$@"; do
  case $what in
  lo)  # reference only; LibreOffice is not a preview engine
    mkdir -p out/lo-profile lo-pdf lo-pdf-hwpx renders/libreoffice renders/libreoffice-h2orestart
    soffice -env:UserInstallation="file://$PWD/out/lo-profile" --headless --convert-to pdf --outdir lo-pdf \
      baseline/*.docx baseline/*.pptx baseline/*.xlsx
    soffice -env:UserInstallation="file://$PWD/out/lo-profile" --headless --convert-to pdf --outdir lo-pdf-hwpx baseline/*.hwpx
    for f in lo-pdf/*.pdf; do "$PY" raster.py "$f" renders/libreoffice "$(basename "$f" | cut -c1-2)"; done
    for f in lo-pdf-hwpx/*.pdf; do "$PY" raster.py "$f" renders/libreoffice-h2orestart "$(basename "$f" | cut -c1-2)"; done
    ;;
  rdocx|rpptx|rhwp)
    case $what in rdocx) ext=docx; extra="--mode caller --view accepted";; rpptx) ext=pptx; extra="--mode caller";; rhwp) ext=hwpx; extra="--path layer";; esac
    extra=$(echo "$extra" | tr ' ' '\n')
    td=${TARGET_DIR:-engines/$what/target}
    cargo build --release --manifest-path "engines/$what/Cargo.toml" --target-dir "$td"
    mkdir -p "$R/$what"
    aliases=$(sed 's/.*/--alias\n&/' engines/aliases.txt)
    IFS='
'
    # shellcheck disable=SC2086
    "$td/release/render-$what" --out "$R/$what" --font-dir fonts/files $aliases $extra baseline/*.$ext \
      2>"$R/$what/stderr.log" || echo "$what: some files failed, see $R/$what/results.jsonl" >&2
    unset IFS
    ;;
  *) echo "unknown: $what" >&2; exit 2;;
  esac
done
