#!/bin/sh
# Re-downloads the third-party corpus files at pinned commits and checks them against
# the committed copies' SHA-256. (The files are committed too; this script documents
# provenance and lets anyone verify it.) korean-deck.pptx is made by make_korean.py, turns-deck.pptx by make_turns.py.
set -e
cd "$(dirname "$0")"
PPTX=https://raw.githubusercontent.com/scanny/python-pptx/278b47b1dedd5b46ee84c286e77cdfb0bf4594be/features/steps/test_files
POI=https://raw.githubusercontent.com/apache/poi/93b6a820f4aaefd7b062615f843799947872cb24/test-data/slideshow
fetch() { # base name sha256
  curl -fsSL -o "$2.tmp" "$1/$2"
  echo "$3  $2.tmp" | sha256sum -c --quiet - && mv "$2.tmp" "$2" && echo "ok  $2"
}
fetch "$PPTX" act-props.pptm                     14af8bc6fa9ed94fbc895d8744993c682d29b028d232d2a25c29ddfe668682f5
fetch "$PPTX" ext-rels.pptx                      c959a97d2bf687cd142c1a4e8eefe00758162ce78210fc1a608a4a75594a9d27
fetch "$PPTX" ph-populated-placeholders.pptx     b08599276b4d708757951f2d45ac74cb4e81e94463d575e4d857690d0d231a60
fetch "$PPTX" ph-unpopulated-placeholders.pptx   4a73c52d57e944abfc3367d7521f0ae8740030309af5644f6702ded0508cca53
fetch "$PPTX" prs-notes.pptx                     664dc13965c7ea24688256d1ecd869936ca51a2e94d2aefe0f614bb8554976ea
fetch "$PPTX" sld-notes.pptx                     9a83da2e319161c1e2993de878f65b57ec10e14e3ef418b46354c48350866856
fetch "$PPTX" shp-access-ole-object.pptx         a448dc27c4d1212ad23f680c040f8a918ef7cbb6c41c9ad57f33d1d59a8f9bb4
fetch "$PPTX" shp-groupshape.pptx                37c9a0bd919f9fc4e89600939cbaf798e79c6aaf96406aee5bba2828cea3e3a6
fetch "$PPTX" shp-shapes.pptx                    302dc642b791f49da6057b69a7ef2a0804f61d9469e7e49e9e734ae3305951cc
fetch "$PPTX" tbl-cell.pptx                      02df45aad4927c4c471d76d4f638e82e14c42d4db773103cf8f950f73b0ae976
fetch "$PPTX" txt-text.pptx                      cef8bba88769e3799cab29299860bffdf598bab178d3efb8666ac83686614c68
fetch "$PPTX" txt-font-props.pptx                38899c8b02e1925d3a69c7803c7cf17899938ab240ad739abf14a722f5675f15
fetch "$POI"  SampleShow.pptx                    bfb4b2f07c9233afd2f32aa5781d849d0c7d225bf03721a10becf487b481d828
fetch "$POI"  layouts.pptx                       9c3d53afa3115de2ba72ec77637a616aeed505a6ff3b30a46db871c4edd32be5
fetch "$POI"  with_japanese.pptx                 f0b1efbfbd70f7ee2395dc2505639f99449971ba404a6ec8cf1b97d82cf45c10
fetch "$POI"  shapes.pptx                        19fde9b87e33dd1a95fdbba0cf6abc2278bf03874f4665c7f8b88b6afe4a2571
fetch "$POI"  45545_Comment.pptx                 0295a51f63150e3ee680154d07dab88062ba2306030b790561f85436f3ded7c8
fetch "$POI"  SimpleMacro.pptm                   8a3573c82fd07a301d7f175b8bee646c0b58eb8a945bd0ff408225b6e2b89b15
fetch "$POI"  EmbeddedVideo.pptx                 7940e3b1a339db11f00b65399a2fe77e0e85a5da3a30ac8d6c8a0a77527b2ab2
fetch "$POI"  60810.pptx                         61afceb0365523ceba6fc00a525157148c2ddbdb3b5d843992ac3c107e9921bf
echo "97b1c566bdfbdb5cfc10abdf47d1cc763a03c0774de2beadd1136f0a9d481c85  korean-deck.pptx" | sha256sum -c --quiet - && echo "ok  korean-deck.pptx (committed copy)"
