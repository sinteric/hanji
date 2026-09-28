#!/bin/sh
# Fetch the Hancom-authored sample (Hancom Office 2021, 11.0.0.2129) from rhwp's
# test samples at a pinned commit. Not committed here: it is rhwp test data (268 KB).
set -e
cd "$(dirname "$0")"
REV=680111ec7bea2fe11110de18c3676ba5a1cf7847
curl -fsSL -o mel-001.hwpx "https://raw.githubusercontent.com/edwardkim/rhwp/$REV/samples/hwpx/mel-001.hwpx"
echo "ec75dc24ade52e055c54ea0529af100a623027c465571fda7edaa97f6e736d8b  mel-001.hwpx" | sha256sum -c -
