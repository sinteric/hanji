#!/bin/sh
# Re-downloads the corpus files from rhwp's samples at a pinned commit and checks them
# against the committed copies' SHA-256. (The files are committed too; this script
# documents provenance and lets anyone verify it.)
set -e
cd "$(dirname "$0")"
RHWP=https://raw.githubusercontent.com/edwardkim/rhwp/680111ec7bea2fe11110de18c3676ba5a1cf7847/samples/hwpx
fetch() { # name upstream-path sha256
  curl -fsSL -o "$1.tmp" "$RHWP/$2"
  echo "$3  $1.tmp" | sha256sum -c --quiet - && mv "$1.tmp" "$1" && echo "ok  $1"
}
fetch basic-table-01.hwpx                        basic-table-01.hwpx                                8d9df52bb09803fa1754317360e27709621555b3e1163a0d7eb4ad9abab5b26b
fetch eq-002.hwpx                                eq-002.hwpx                                        ecb229b47bfa221bdd3cba06fd72887b1536fb944549e789969ea19cde87a8bb
fetch fdi-2025q2.hwpx                            2025%EB%85%84%202%EB%B6%84%EA%B8%B0%20%ED%95%B4%EC%99%B8%EC%A7%81%EC%A0%91%ED%88%AC%EC%9E%90%20%28%EC%B5%9C%EC%A2%85%29.hwpx e49c69c090fa7abe9d33971f2983839f30c3efd77068d6d24b99db93a3c2872f
fetch field-multipara-clickhere.hwpx             field-multipara-clickhere.hwpx                     9c297bb8a3c15ae95e4446dbbe8a5f3bc0e03e532b34a2ab5db0fb5966c15955
fetch footnote-01.hwpx                           footnote-01.hwpx                                   2b59b7248af275a5fa6e108f95997ceacbec9f3bb4fc9b1a30502373a8f672cb
fetch footnote-tbox-01.hwpx                      footnote-tbox-01.hwpx                              e7e6d109beee222f01f089b014c42d8d0f540564b57c7dd47384c1b5ae19e2f7
fetch form-01.hwpx                               form-01.hwpx                                       3bbd207b88fe61e802706de3ccf98abdb8b450493164eec657c9ee88a5aba87e
fetch hcar-001.hwpx                              hcar-001.hwpx                                      31c3c5a5758638bf7c653fc4c60e7df430395f6b78d6a04effcd53274a5948f0
fetch hy-002.hwpx                                hy-002.hwpx                                        1bfe5179efa7c57254651aede428c601e312418e5f923ec921c741d32452c496
fetch issue1535_coanchored_float_exclusion.hwpx  issue1535_coanchored_float_exclusion.hwpx          8f23ccab265dc768fc470b168ed4e5f47ce2e302fdf9be37cb81e6c5355a2b1a
fetch issue1948_cross_para_fieldend.hwpx         issue1948_cross_para_fieldend.hwpx                 38244a9ce101e1b24f012bf856bdb5bdc2e006632542b66813a615488cbf0066
fetch landscape-001.hwpx                         landscape-001.hwpx                                 8d21e2050fb1727c6bfd5a123def0a2591da790a9e2370e6fb7492a833a2062e
fetch mel-001.hwpx                               mel-001.hwpx                                       ec75dc24ade52e055c54ea0529af100a623027c465571fda7edaa97f6e736d8b
fetch para-001.hwpx                              para-001.hwpx                                      3feded11906a573dda366b4299428bce2de573a153d11059fa8e1e26b6fc354d
fetch table-text.hwpx                            table-text.hwpx                                    c31c5267c4c686cefd5b5df1056e0e4bc64d13c8c67dcdaeb2933a28e9df6c8f
fetch tb-img-03.hwpx                             tb-img-03.hwpx                                     072faaadf15bc0786f22383d8231d3c5cdfe76e0181f3d7915c8bfead4f07d93

# The binary HWP 5.0 fixtures (hwp/, DESIGN.md §5.6), from a later commit.
RHWP=https://raw.githubusercontent.com/edwardkim/rhwp/981ce66b4183b13c1a3b5907e04a8969c8c42034/samples
fetch hwp/eq-002.hwp                             eq-002.hwp                                         e3306c23cf1d991debb3fdf113e5f27c5a28e614e6c95c246afb18ce8a549ea9
fetch hwp/footnote-01.hwp                        footnote-01.hwp                                    5bbc8a8fd23415aad59dd91d1eb261050946d9c64635933fcd49609ec2cc94e5
fetch hwp/hcar-001.hwp                           hcar-001.hwp                                       1ba79b18b75745faad58da4844603f9f1e094fc48aa0d27330d806188f19ab6e
fetch hwp/stroke.hwp                             issue8403/stroke.hwp                               1059be20e316c7b7342269cf4cd49637df1c313b0c505bab980758a9d6ead429
fetch hwp/page-activated.hwp                     issue6133/156483831_poster_title_above_offset_float.hwp 1d2ecde2a4b34bf9abc5cfab8551f9b6bfc417d9013504dcdbbb07a41a62e4e8
