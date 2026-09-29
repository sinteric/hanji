#!/bin/sh
# Re-downloads the third-party corpus files at pinned commits and checks them against
# the committed copies' SHA-256. (The files are committed too; this script documents
# provenance and lets anyone verify it.) The korean-sales files are made by make_korean.py.
set -e
cd "$(dirname "$0")"
POI=https://raw.githubusercontent.com/apache/poi/93b6a820f4aaefd7b062615f843799947872cb24/test-data/spreadsheet
RX=https://raw.githubusercontent.com/jmcnamara/rust_xlsxwriter/72dae65fc232bd0c5c66ccb03a2b0359f2cf2ea5/tests/input
fetch() { # base remote-name local-name sha256
  curl -fsSL -o "$3.tmp" "$1/$2"
  echo "$4  $3.tmp" | sha256sum -c --quiet - && mv "$3.tmp" "$3" && echo "ok  $3"
}
fetch "$POI" "50867_with_table.xlsx" 50867_with_table.xlsx            142c5e3e48bde3adb5378f7c143b7374f24177d949b6117e4cbfd7ab1e7bae1a
fetch "$POI" "54084%20-%20Greek%20-%20beyond%20BMP.xlsx" 54084-Greek-beyond-BMP.xlsx      ba91f677a35478dcdb5452a9ec65235ea506d352f74d624f8c96bfb319cd83b0
fetch "$POI" "57893-many-merges.xlsx" 57893-many-merges.xlsx           f1b0cc07459a2e00f1e930ff54330e45b9d0626f45468b1ffa7eb3dc21d6f46b
fetch "$POI" "DataValidations-49244.xlsx" DataValidations-49244.xlsx       8066b12c878d74aa35b55b22dc9daba9a56e91332a35cf96c4fc0f6af6bd3bd1
fetch "$POI" "DateFormatTests.xlsx" DateFormatTests.xlsx             16445dc0deba00671ee4315015a970d415ce9886da59c974532d946e99276c66
fetch "$POI" "ExcelPivotTableSample.xlsx" ExcelPivotTableSample.xlsx       f0ef1d61c9f3d8b27b18a876b8e24676314c6450ed64a725a7bfaa603fdd67e3
fetch "$POI" "ExcelTables.xlsx" ExcelTables.xlsx                 1e43011440eb39000d1751abaf9ded42d37b26f91b4f586a94bf91ce8ecbe9e5
fetch "$POI" "ExcelWithAttachments.xlsm" ExcelWithAttachments.xlsm        34779a3208c1625d22058ae1f9dbe5afbde9c815785b77b1d883b4ef03ff42ee
fetch "$POI" "GeneralFormatTests.xlsx" GeneralFormatTests.xlsx          607ff06ff0295d0920d81b27e09b91e86301b9f3862d1d22b093f8f8c5a4de28
fetch "$POI" "InlineStrings.xlsx" InlineStrings.xlsx               bdfd54e82e7eec3a16bc12708dd2fbb8ebee12921215f6ed78fd32a84bfbfd74
fetch "$POI" "SampleSS.xlsx" SampleSS.xlsx                    44f1b6ef310c370d4902ee3452e6174da25a67bdb09f47850d51b3287cb3db71
fetch "$POI" "SimpleMacro.xlsm" SimpleMacro.xlsm                 f76c986f4ebc25c2cc57c088b2511a1269f4bd61d6223a2ab58db351da348ba6
fetch "$POI" "SimpleWithComments.xlsx" SimpleWithComments.xlsx          d7d337b3436e863487d26439a57ef749950c599862325d7bce7e830acf1df0ba
fetch "$POI" "StructuredReferences.xlsx" StructuredReferences.xlsx        a746e336176f8157c50a0a3b017c4633b7b1d546bd1b8af0f8165a48d1bacc56
fetch "$POI" "Tables.xlsx" Tables.xlsx                      3402bd6406a6ef32c3b88c45decb820f0ca797a8d971007f869b5ad3743bcbc8
fetch "$POI" "WithChart.xlsx" WithChart.xlsx                   51f27aabf7417f5aaa0542857b6bb56a92d4afbabdc5c63f084c06f92b67429d
fetch "$POI" "WithConditionalFormatting.xlsx" WithConditionalFormatting.xlsx   ead4aad13d2b975f795b50e7c8b7da443fb1525a8f9fa7347a5f09041a3ed426
fetch "$POI" "WithDrawing.xlsx" WithDrawing.xlsx                 97b1ab359d6aecbe37864f30c85106e386f0a19bea8f290103927362a2701066
fetch "$POI" "WithTable.xlsx" WithTable.xlsx                   624b33056b0d2686f4264eb21281fc0245167490463d2fdeb556984c239f7fa8
fetch "$POI" "WithThreeCharts.xlsx" WithThreeCharts.xlsx             629b69bd250d9b4e173eb2088eb80b656b7748b0287cd7b0df0c2dc22c6af7b0
fetch "$POI" "WithVariousData.xlsx" WithVariousData.xlsx             1440a832bb810c09687d3498b841782964b0b697e9242428fed1072ab22d078a
fetch "$POI" "link-external-workbook-b.xlsx" link-external-workbook-b.xlsx    6e0e5c2aa3bf870cfeba03cb2350c312c639f2875b6501029d0bc134916b2361
fetch "$RX"  array_formula01.xlsx         rx_array_formula01.xlsx          86e378e08a38771235ffb54fdc4a919d1870f904e1502f340f462d75bf39b01c
fetch "$RX"  autofilter01.xlsx            rx_autofilter01.xlsx             4cb7740635dc265bb84aa0b0ae16ee522b6f82241c40993aec369f912a18af34
fetch "$RX"  chart_bar01.xlsx             rx_chart_bar01.xlsx              4efc350a4ca69bcf07040c6f47cc2c96b86d4a6f64fdcdb195113f6f70802ba0
fetch "$RX"  comment01.xlsx               rx_comment01.xlsx                26bd6802f9a12c476915372c3beec7d0c52e73fe071441ec9ff88cea11bf30aa
fetch "$RX"  cond_format01.xlsx           rx_cond_format01.xlsx            9c6f43ab29e688d6acf9c71b7b8b9c60130cdf9c6b9584a27e3d2aac7fbf909e
fetch "$RX"  data_validation01.xlsx       rx_data_validation01.xlsx        0f44788ecc5cea8eaaee2067ed44212f149f7b2d35da054be73f17585924e368
fetch "$RX"  defined_name01.xlsx          rx_defined_name01.xlsx           73ec99c0a5d2c43f9cc67119de63f819f57505c20ee63abb3eac6e28ea7a52a7
fetch "$RX"  dynamic_array01.xlsx         rx_dynamic_array01.xlsx          c94d6be35d4b4e9ef2fb92757a6d67bde4308fc57b4372f803efb24b84a6a5f8
fetch "$RX"  hyperlink01.xlsx             rx_hyperlink01.xlsx              cbaa9070eb0ff0f580474eeef96e69881e1a02e5d71a4c51364f28c812c7f5c7
fetch "$RX"  image01.xlsx                 rx_image01.xlsx                  27d8cadaeb843934d3cf02a63543f1dada79e6c7cc3ec4ab8a9bfc940246663f
fetch "$RX"  macro01.xlsm                 rx_macro01.xlsm                  09c35d1580eb6d7e678ba8249cdd1cbc0bd245fbb0eed8794981728715944736
fetch "$RX"  merge_range01.xlsx           rx_merge_range01.xlsx            6b52019ce3bc3f9803104b58c621684110d895549047c66132743ec45425bdc0
fetch "$RX"  rich_string01.xlsx           rx_rich_string01.xlsx            b054eb9f739cde3d78111d7ca424fcbd7e1bffcc4d33236e4791f2c306f2828f
fetch "$RX"  table01.xlsx                 rx_table01.xlsx                  b05857766a6eb25b2856b7aa7f5b6a678f5a0b68cf829ec344fe49242e71be3e
fetch "$RX"  table05.xlsx                 rx_table05.xlsx                  0f75c74a1e9ca82ff58c8adbec3a43dab1451614809d64978d2825fb1402f34e
fetch "$RX"  table14.xlsx                 rx_table14.xlsx                  e601b013ec123c4a37ff0ca566e54aceeee063acd0dd17dba61b7c79fdc431ec
fetch "$POI" "shared_formulas.xlsx" shared_formulas.xlsx             31612d513b5ea5aaa69764779f87ac52588687770f3ccbf4b420ec21bc70561f
fetch "$POI" "sharedhyperlink.xlsx" sharedhyperlink.xlsx             91329edc5d293926c0294b6443449d90b3ac9d53a174c2f5c4d9e59ecea8fc73
fetch "$POI" "simple-monthly-budget.xlsx" simple-monthly-budget.xlsx       cae00c6894b95743017aeded4186f15a12c2d8382d71760029df490d58197db6
fetch "$POI" "table-sample.xlsx" table-sample.xlsx                d43a2c85e8f91c100a62f0ed0006789604ff818f017dd3be90e10a1af2093462
fetch "$POI" "unicodeSheetName.xlsx" unicodeSheetName.xlsx            2d7414dd71e5b3e33602c19ad454f072771ea164d7477a3b3b4cddf6c622dc67
fetch "$POI" "xlookup.xlsx" xlookup.xlsx                     4d8fd3db6469df14f280964b3a447e0be027eda0e2195ca62693e92d8d353910
echo "3835e656521e8fc72f4342bbd9fabafd75740bad16c3a16b34d999eff9a140c7  korean-sales-lo.xlsx" | sha256sum -c --quiet - && echo "ok  korean-sales-lo.xlsx (committed copy)"
echo "4e6e34617ecb7696c3d61a1f9583f66764070dd30a50cc4aec6af490ffc86ce3  korean-sales.xlsx" | sha256sum -c --quiet - && echo "ok  korean-sales.xlsx (committed copy)"
