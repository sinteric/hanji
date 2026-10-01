# Fonts used by the preview spike

No font files are committed. Put these into `fonts/files/` (git-ignored); `fonts/fonts.conf` adds that
directory to fontconfig and maps Microsoft and Hancom family names to the free substitutes below.
The spike extracted them from Ubuntu 24.04 (noble) packages, `apt-get download <pkg> && dpkg-deb -x <deb> x`,
then copied every `*.ttf`/`*.ttc` under `x/usr/share/fonts` into `fonts/files/` (73 files, ~270 MB):

| Package (noble) | Families | Licence | Upstream |
|---|---|---|---|
| fonts-noto-cjk 1:20230817+repack1-3 | Noto Sans CJK KR/JP/SC/TC, Noto Serif CJK KR/… (`.ttc`) | OFL-1.1 | github.com/notofonts/noto-cjk |
| fonts-nanum 20200506-1, fonts-nanum-extra 20200506-1 | NanumGothic, NanumMyeongjo, NanumBarunGothic, NanumSquare(+_ac, Round), Nanum Pen/Brush | OFL-1.1 | hangeul.naver.com |
| fonts-unfonts-core 1:1.0.2-080608-18 | UnBatang, UnDotum, UnGraphic, UnDinaru, UnGungseo, UnPilgi | GPL-2 | kldp.net/unfonts |
| fonts-baekmuk 2.2-13 | Baekmuk Batang/Dotum/Gulim/Headline (`batang.ttf`, `dotum.ttf`, `gulim.ttf`, `hline.ttf`) | Baekmuk licence (MIT-like) | kldp.net/baekmuk |
| fonts-crosextra-carlito 20230309-2 | Carlito (Calibri metrics) | OFL-1.1 | github.com/googlefonts/carlito |
| fonts-crosextra-caladea 20200211-2 | Caladea (Cambria metrics) | OFL-1.1 or Apache-2.0 (Debian copyright lists both) | github.com/huertatipografica/Caladea |
| fonts-liberation 1:2.1.5-3, fonts-liberation2 1:2.1.5-3 | Liberation Sans/Serif/Mono (Arial/Times New Roman/Courier New metrics) | OFL-1.1 | github.com/liberationfonts |

Not available here: **HCR Batang / HCR Dotum (함초롬바탕/돋움)** — hancom.com is blocked by this environment's
egress proxy (CONNECT 403) and they are not on a package mirror; also Malgun Gothic, Batang, Gulim, Dotum,
Aptos, Century Gothic, Georgia, Tahoma (proprietary). Every Hancom and Microsoft name therefore resolves to a
substitute (aliases in `fonts.conf` and `../engines/aliases.txt`):

- Calibri, Calibri Light, Aptos, Aptos Display → Carlito; Cambria → Caladea
- Arial, Helvetica, Tahoma, Segoe UI, Century Gothic → Liberation Sans; Arial Narrow → Liberation Sans Narrow (not in the set, so fontconfig falls back)
- Times New Roman, Georgia, Palatino Linotype → Liberation Serif; Courier New → Liberation Mono
- 맑은 고딕/Malgun Gothic, 돋움(체)/Dotum, 굴림(체)/Gulim, 함초롬돋움/HCR Dotum, 한컴 고딕, HY중고딕, H2hdrM, 한양중고딕, MS Gothic, Arial Unicode MS → Noto Sans CJK KR
- 바탕(체)/Batang, 함초롬바탕/HCR Batang, 한컴바탕/Haansoft Batang, HY견명조/HYmjrE, 휴먼명조/HMKMM, MS Mincho, 궁서/Gungsuh → Noto Serif CJK KR

`fonts-nanum.conf` is the same table with the Korean targets swapped to NanumGothic/NanumMyeongjo; it is used
only by the font-substitution calibration (`calibrate.py`).
