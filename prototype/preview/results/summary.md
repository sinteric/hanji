| Candidate | Format | Scope | Files | Layout | T | P | L | W | W_rel | Content-SSIM | Raw SSIM | Page mismatch | No render | Rule 5 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| libreoffice | docx | all | 9 | **0.822** | 0.988 | 0.989 | 0.983 | 0.535 | 0.701 | 0.537 | 0.865 | 1 | 0 | fail (≥.85: 44%, pages exact 89%, pages ≥.80: 58%) |
| libreoffice | docx | clean | 4 | **0.781** | 1.000 | 0.991 | 0.961 | 0.472 | 0.682 | 0.363 | 0.901 | 0 | 0 | fail (≥.85: 50%, pages exact 100%, pages ≥.80: 43%) |
| libreoffice | pptx | all | 8 | **0.924** | 0.998 | 1.000 | 0.982 | 0.760 | 0.796 | 0.535 | 0.949 | 0 | 0 | fail (≥.85: 75%, pages exact 100%, pages ≥.80: 81%) |
| libreoffice | xlsx | all | 5 | **0.585** | 0.932 | 0.725 | 0.941 | 0.333 | 0.488 | 0.235 | 0.832 | 1 | 0 | fail (≥.85: 40%, pages exact 80%, pages ≥.80: 9%) |
| libreoffice-h2orestart | hwpx | all | 8 | **0.285** | 0.927 | 0.551 | 0.643 | 0.071 | 0.165 | 0.157 | 0.586 | 5 | 0 | fail (≥.85: 0%, pages exact 38%, pages ≥.80: 0%) |
| rdocx | docx | all | 9 | **0.619** | 0.967 | 0.961 | 0.936 | 0.222 | 0.504 | 0.462 | 0.801 | 2 | 0 | fail (≥.85: 0%, pages exact 78%, pages ≥.80: 7%) |
| rdocx | docx | clean | 4 | **0.607** | 0.971 | 0.947 | 0.905 | 0.229 | 0.432 | 0.168 | 0.738 | 1 | 0 | fail (≥.85: 0%, pages exact 75%, pages ≥.80: 10%) |
| rpptx | pptx | all | 8 | **0.791** | 0.871 | 0.875 | 0.866 | 0.611 | 0.660 | 0.469 | 0.837 | 1 | 1 | fail (≥.85: 62%, pages exact 88%, pages ≥.80: 60%) |
| rhwp | hwpx | all | 8 | **0.836** | 0.990 | 0.851 | 0.951 | 0.713 | 0.854 | 0.396 | 0.801 | 1 | 0 | fail (≥.85: 62%, pages exact 88%, pages ≥.80: 60%) |

| Candidate | File | Pages (native→render) | T | P | L | W | W_rel | Layout | Content-SSIM | Raw SSIM | Note |
|---|---|---|---|---|---|---|---|---|---|---|---|
| libreoffice | 01-docx-untouched-korean-report | 2→2 | 0.992 | 1.000 | 1.000 | 0.338 | 0.575 | **0.761** | 0.804 | 0.932 | Word markup view on 2/2 pages: rescaled |
| libreoffice | 02-docx-untouched-sample-docx | 5→5 | 0.996 | 0.985 | 1.000 | 0.843 | 0.959 | **0.954** | 0.709 | 0.878 | Word markup view on 5/5 pages: rescaled |
| libreoffice | 03-docx-untouched-docx4j-tables | 3→3 | 1.000 | 0.971 | 1.000 | 0.144 | 0.705 | **0.612** | 0.330 | 0.771 |  |
| libreoffice | 04-docx-untouched-testword-various | 2→2 | 1.000 | 0.993 | 1.000 | 0.658 | 0.837 | **0.899** | 0.365 | 0.956 |  |
| libreoffice | 05-docx-untouched-loadandsave | 3→4 | 0.913 | 0.952 | 1.000 | 0.493 | 0.407 | **0.809** | 0.429 | 0.595 | Word markup view on 3/3 pages: rescaled |
| libreoffice | 06-docx-edited-korean-report-e10 | 2→2 | 0.992 | 1.000 | 1.000 | 0.369 | 0.667 | **0.778** | 0.791 | 0.931 | Word markup view on 2/2 pages: rescaled |
| libreoffice | 07-docx-edited-sample-docx-e10 | 4→4 | 0.996 | 1.000 | 1.000 | 0.886 | 0.967 | **0.969** | 0.650 | 0.846 | Word markup view on 4/4 pages: rescaled |
| libreoffice | 08-docx-new-report-en | 1→1 | 1.000 | 1.000 | 0.923 | 0.873 | 0.874 | **0.947** | 0.603 | 0.972 |  |
| libreoffice | 09-docx-new-report-ko | 1→1 | 1.000 | 1.000 | 0.920 | 0.214 | 0.314 | **0.666** | 0.154 | 0.906 |  |
| libreoffice | 10-pptx-untouched-korean-deck | 7→7 | 1.000 | 1.000 | 1.000 | 0.798 | 0.814 | **0.945** | 0.216 | 0.946 |  |
| libreoffice | 11-pptx-untouched-shapes | 6→6 | 1.000 | 1.000 | 1.000 | 0.962 | 0.974 | **0.990** | 0.864 | 0.985 |  |
| libreoffice | 12-pptx-untouched-onlyoffice-sample | 7→7 | 0.987 | 1.000 | 1.000 | 0.898 | 0.934 | **0.970** | 0.740 | 0.903 |  |
| libreoffice | 13-pptx-untouched-modern-pitch | 8→8 | 0.999 | 1.000 | 0.936 | 0.509 | 0.595 | **0.831** | 0.776 | 0.951 |  |
| libreoffice | 14-pptx-untouched-korean-report-deck | 6→6 | 1.000 | 1.000 | 1.000 | 0.829 | 0.846 | **0.954** | 0.681 | 0.961 |  |
| libreoffice | 15-pptx-edited-korean-deck-pset | 7→7 | 1.000 | 1.000 | 1.000 | 0.782 | 0.790 | **0.940** | 0.158 | 0.952 |  |
| libreoffice | 16-pptx-edited-modern-pitch-pset | 8→8 | 0.999 | 1.000 | 0.919 | 0.444 | 0.546 | **0.799** | 0.678 | 0.950 |  |
| libreoffice | 17-pptx-new-deck-ko | 4→4 | 1.000 | 1.000 | 1.000 | 0.855 | 0.868 | **0.962** | 0.168 | 0.947 |  |
| libreoffice | 18-xlsx-untouched-korean-sales | 5→5 | 0.905 | 0.769 | 0.934 | 0.054 | 0.157 | **0.433** | 0.151 | 0.859 |  |
| libreoffice | 19-xlsx-untouched-monthly-budget | 2→2 | 0.890 | 1.000 | 0.992 | 0.848 | 0.854 | **0.930** | 0.577 | 0.945 |  |
| libreoffice | 20-xlsx-untouched-tables-forms | 21→15 | 0.968 | 0.084 | 0.968 | 0.051 | 0.441 | **0.252** | 0.108 | 0.548 |  |
| libreoffice | 21-xlsx-edited-korean-sales-ops | 5→5 | 0.904 | 0.773 | 0.933 | 0.058 | 0.163 | **0.441** | 0.150 | 0.857 |  |
| libreoffice | 22-xlsx-new-sales-ko | 2→2 | 0.992 | 1.000 | 0.880 | 0.655 | 0.824 | **0.870** | 0.187 | 0.949 |  |
| libreoffice-h2orestart | 23-hwpx-untouched-fdi-2025q2 | 9→14 | 0.634 | 0.055 | 0.363 | 0.007 | 0.162 | **0.097** | 0.113 | 0.351 |  |
| libreoffice-h2orestart | 24-hwpx-untouched-hy-002 | 2→3 | 0.970 | 0.709 | 0.738 | 0.005 | 0.146 | **0.224** | 0.090 | 0.369 |  |
| libreoffice-h2orestart | 25-hwpx-untouched-hcar-001 | 6→10 | 0.981 | 0.014 | 0.806 | 0.195 | 0.359 | **0.216** | 0.094 | 0.386 |  |
| libreoffice-h2orestart | 26-hwpx-untouched-footnote-01 | 6→7 | 0.944 | 0.230 | 0.682 | 0.007 | 0.120 | **0.179** | 0.271 | 0.691 |  |
| libreoffice-h2orestart | 27-hwpx-untouched-para-001 | 3→4 | 1.000 | 0.675 | 0.229 | 0.002 | 0.100 | **0.127** | 0.112 | 0.458 |  |
| libreoffice-h2orestart | 28-hwpx-edited-footnote-01-e10 | 7→7 | 0.942 | 0.853 | 0.682 | 0.007 | 0.192 | **0.250** | 0.274 | 0.843 |  |
| libreoffice-h2orestart | 29-hwpx-edited-hy-002-e10 | 3→3 | 0.974 | 0.881 | 0.675 | 0.113 | 0.020 | **0.506** | 0.152 | 0.701 |  |
| libreoffice-h2orestart | 30-hwpx-new-plan-ko | 2→2 | 0.973 | 0.993 | 0.967 | 0.231 | 0.216 | **0.681** | 0.146 | 0.889 |  |
| rdocx | 01-docx-untouched-korean-report | 2→2 | 0.983 | 1.000 | 1.000 | 0.339 | 0.541 | **0.760** | 0.803 | 0.931 | Word markup view on 2/2 pages: rescaled |
| rdocx | 02-docx-untouched-sample-docx | 5→5 | 1.000 | 0.912 | 0.932 | 0.053 | 0.630 | **0.461** | 0.728 | 0.892 | Word markup view on 5/5 pages: rescaled |
| rdocx | 03-docx-untouched-docx4j-tables | 3→3 | 1.000 | 0.971 | 1.000 | 0.122 | 0.705 | **0.587** | 0.268 | 0.750 |  |
| rdocx | 04-docx-untouched-testword-various | 2→5 | 0.884 | 0.817 | 0.933 | 0.361 | 0.373 | **0.702** | 0.120 | 0.375 |  |
| rdocx | 05-docx-untouched-loadandsave | 3→4 | 0.848 | 0.948 | 0.938 | 0.257 | 0.393 | **0.663** | 0.495 | 0.640 | Word markup view on 3/3 pages: rescaled |
| rdocx | 06-docx-edited-korean-report-e10 | 2→2 | 0.992 | 1.000 | 1.000 | 0.374 | 0.589 | **0.780** | 0.790 | 0.930 | Word markup view on 2/2 pages: rescaled |
| rdocx | 07-docx-edited-sample-docx-e10 | 4→4 | 1.000 | 1.000 | 0.932 | 0.058 | 0.652 | **0.482** | 0.667 | 0.863 | Word markup view on 4/4 pages: rescaled |
| rdocx | 08-docx-new-report-en | 1→1 | 1.000 | 1.000 | 0.818 | 0.408 | 0.477 | **0.760** | 0.143 | 0.923 |  |
| rdocx | 09-docx-new-report-ko | 1→1 | 1.000 | 1.000 | 0.868 | 0.023 | 0.172 | **0.377** | 0.142 | 0.902 |  |
| rpptx | 10-pptx-untouched-korean-deck | 7→7 | 1.000 | 1.000 | 1.000 | 0.494 | 0.617 | **0.838** | 0.195 | 0.929 |  |
| rpptx | 11-pptx-untouched-shapes | 6→6 | 1.000 | 1.000 | 1.000 | 0.971 | 0.982 | **0.993** | 0.894 | 0.988 |  |
| rpptx | 12-pptx-untouched-onlyoffice-sample | 7→0 | 0.000 | 0.000 | 0.000 | 0.000 | – | **0.000** | 0.000 | 0.000 |  |
| rpptx | 13-pptx-untouched-modern-pitch | 8→8 | 0.998 | 1.000 | 0.984 | 0.923 | 0.926 | **0.976** | 0.887 | 0.976 |  |
| rpptx | 14-pptx-untouched-korean-report-deck | 6→6 | 0.976 | 1.000 | 1.000 | 0.679 | 0.731 | **0.902** | 0.670 | 0.952 |  |
| rpptx | 15-pptx-edited-korean-deck-pset | 7→7 | 1.000 | 1.000 | 1.000 | 0.429 | 0.573 | **0.809** | 0.178 | 0.944 |  |
| rpptx | 16-pptx-edited-modern-pitch-pset | 8→8 | 0.997 | 1.000 | 0.943 | 0.816 | 0.822 | **0.936** | 0.779 | 0.974 |  |
| rpptx | 17-pptx-new-deck-ko | 4→4 | 1.000 | 1.000 | 1.000 | 0.577 | 0.626 | **0.872** | 0.145 | 0.932 |  |
| rhwp | 23-hwpx-untouched-fdi-2025q2 | 9→9 | 0.993 | 0.994 | 0.987 | 0.918 | 0.957 | **0.973** | 0.567 | 0.797 |  |
| rhwp | 24-hwpx-untouched-hy-002 | 2→2 | 0.970 | 1.000 | 0.932 | 0.979 | 0.980 | **0.970** | 0.578 | 0.827 |  |
| rhwp | 25-hwpx-untouched-hcar-001 | 6→6 | 1.000 | 0.998 | 1.000 | 0.810 | 0.952 | **0.948** | 0.366 | 0.750 |  |
| rhwp | 26-hwpx-untouched-footnote-01 | 6→6 | 0.993 | 0.977 | 0.923 | 0.298 | 0.686 | **0.719** | 0.347 | 0.841 |  |
| rhwp | 27-hwpx-untouched-para-001 | 3→3 | 1.000 | 1.000 | 0.943 | 0.888 | 0.889 | **0.957** | 0.535 | 0.845 |  |
| rhwp | 28-hwpx-edited-footnote-01-e10 | 7→6 | 0.991 | 0.267 | 0.880 | 0.050 | 0.573 | **0.329** | 0.221 | 0.690 |  |
| rhwp | 29-hwpx-edited-hy-002-e10 | 3→3 | 0.975 | 0.576 | 0.943 | 0.883 | 0.920 | **0.827** | 0.297 | 0.732 |  |
| rhwp | 30-hwpx-new-plan-ko | 2→2 | 0.994 | 0.993 | 1.000 | 0.874 | 0.876 | **0.964** | 0.261 | 0.923 |  |
