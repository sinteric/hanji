# Third-party notices

hanji is licensed under MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`).
The binaries and sources also contain the third-party material below, under
the licences given.

The test files under `crates/*/corpus/` and `prototype/remainder/corpus/` are
not part of the binaries. Each one is redistributed under the licence of the
project it comes from, which its directory's `SOURCES.md` names. The Rust
crates the binaries link are listed with their licences in `Cargo.lock` and on
crates.io.

## rhwp: the blank hwpx package

`crates/hanji-store/blank/hwpx/` is compiled into the `hanji` binary. It
is derived from rhwp's sample `basic-table-01.hwpx`, at commit
`680111ec7bea2fe11110de18c3676ba5a1cf7847`
(<https://github.com/edwardkim/rhwp>). The header, settings and page setup
are kept, and a bullet is added.

```text
MIT License

Copyright (c) 2025-2026 Edward Kim

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Python: the difflib port

`crates/hanji-core/src/diff.rs` is a Rust port of `SequenceMatcher` from
CPython's `Lib/difflib.py`, and `crates/hanji-store/src/merge.rs` follows
`SequenceMatcher.get_grouped_opcodes`.

Changes from the original: the code is translated to Rust and is generic over
hashable elements. The junk heuristics are removed: it always behaves as
`SequenceMatcher(None, a, b, autojunk=False)`. Only `get_matching_blocks`,
`get_opcodes` and `ratio` are ported, with helpers for edit spans. Tests check
its results against CPython 3.11.

```text
PYTHON SOFTWARE FOUNDATION LICENSE VERSION 2
--------------------------------------------

1. This LICENSE AGREEMENT is between the Python Software Foundation
("PSF"), and the Individual or Organization ("Licensee") accessing and
otherwise using this software ("Python") in source or binary form and
its associated documentation.

2. Subject to the terms and conditions of this License Agreement, PSF hereby
grants Licensee a nonexclusive, royalty-free, world-wide license to reproduce,
analyze, test, perform and/or display publicly, prepare derivative works,
distribute, and otherwise use Python alone or in any derivative version,
provided, however, that PSF's License Agreement and PSF's notice of copyright,
i.e., "Copyright (c) 2001, 2002, 2003, 2004, 2005, 2006, 2007, 2008, 2009, 2010,
2011, 2012, 2013, 2014, 2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023 Python Software Foundation;
All Rights Reserved" are retained in Python alone or in any derivative version
prepared by Licensee.

3. In the event Licensee prepares a derivative work that is based on
or incorporates Python or any part thereof, and wants to make
the derivative work available to others as provided herein, then
Licensee hereby agrees to include in any such work a brief summary of
the changes made to Python.

4. PSF is making Python available to Licensee on an "AS IS"
basis.  PSF MAKES NO REPRESENTATIONS OR WARRANTIES, EXPRESS OR
IMPLIED.  BY WAY OF EXAMPLE, BUT NOT LIMITATION, PSF MAKES NO AND
DISCLAIMS ANY REPRESENTATION OR WARRANTY OF MERCHANTABILITY OR FITNESS
FOR ANY PARTICULAR PURPOSE OR THAT THE USE OF PYTHON WILL NOT
INFRINGE ANY THIRD PARTY RIGHTS.

5. PSF SHALL NOT BE LIABLE TO LICENSEE OR ANY OTHER USERS OF PYTHON
FOR ANY INCIDENTAL, SPECIAL, OR CONSEQUENTIAL DAMAGES OR LOSS AS
A RESULT OF MODIFYING, DISTRIBUTING, OR OTHERWISE USING PYTHON,
OR ANY DERIVATIVE THEREOF, EVEN IF ADVISED OF THE POSSIBILITY THEREOF.

6. This License Agreement will automatically terminate upon a material
breach of its terms and conditions.

7. Nothing in this License Agreement shall be deemed to create any
relationship of agency, partnership, or joint venture between PSF and
Licensee.  This License Agreement does not grant permission to use PSF
trademarks or trade name in a trademark sense to endorse or promote
products or services of Licensee, or any third party.

8. By copying, installing or otherwise using Python, Licensee
agrees to be bound by the terms and conditions of this License
Agreement.

```

## rdocx: the SVG lowering

`crates/hanji-preview/src/svg.rs` is copied from rdocx 0.14.0 (`src/svg.rs`,
<https://github.com/tensorbee/rdocx>, by Atul Sharma and the rdocx
contributors), licensed MIT OR Apache-2.0 like hanji (`LICENSE-MIT`,
`LICENSE-APACHE`). Changes: hooks for the fonts and the marks on text (hanji
subsets the fonts, upstream embeds them whole), per-page definition ids, and
`hanji-font-N` font families.

## Preview renderer dependencies

DOCX and PPTX use rdocx / rpptx / oxml-layout from
<https://github.com/sinteric/rdocx>, derived from
<https://github.com/tensorbee/rdocx>, licensed MIT OR Apache-2.0. HWPX uses
rhwp from <https://github.com/sinteric/rhwp>, derived from
<https://github.com/edwardkim/rhwp>, licensed MIT. The immutable source
revisions are recorded in `Cargo.toml` and `Cargo.lock`. The SVG lowering
copy above retains its original provenance independently of these pins.

## Fonts compiled into the binaries

The `hanji` binary contains the fonts oxml-layout 0.12.1
bundles (`fonts/` in the crate), which the preview draws with when nothing
better is installed, and of which it embeds subsets in its SVG and HTML
output:

- Carlito: Copyright (c) 2010-2013 by tyPoland Lukasz Dziedzic with Reserved
  Font Name "Carlito". SIL Open Font License 1.1.
- Liberation Sans, Serif and Mono: Digitized data copyright (c) 2010 Google
  Corporation with Reserved Font Arimo, Tinos and Cousine; Copyright (c) 2012
  Red Hat, Inc. with Reserved Font Name Liberation. SIL Open Font License 1.1.
- Noto Sans Arabic, Devanagari and Thai, and a subset of Noto Sans SC:
  Copyright 2022 The Noto Project Authors. SIL Open Font License 1.1.
- Caladea: Copyright (c) 2012 Huerta Tipografia; Caladea is a trademark of
  Huerta Tipografia; original type designers Carolina Giovagnoli and Andres
  Torresi. Apache License 2.0 (`LICENSE-APACHE`).

```text
SIL OPEN FONT LICENSE Version 1.1 - 26 February 2007
-----------------------------------------------------------

PREAMBLE
The goals of the Open Font License (OFL) are to stimulate worldwide
development of collaborative font projects, to support the font creation
efforts of academic and linguistic communities, and to provide a free and
open framework in which fonts may be shared and improved in partnership
with others.

The OFL allows the licensed fonts to be used, studied, modified and
redistributed freely as long as they are not sold by themselves. The
fonts, including any derivative works, can be bundled, embedded,
redistributed and/or sold with any software provided that any reserved
names are not used by derivative works. The fonts and derivatives,
however, cannot be released under any other type of license. The
requirement for fonts to remain under this license does not apply
to any document created using the fonts or their derivatives.

DEFINITIONS
"Font Software" refers to the set of files released by the Copyright
Holder(s) under this license and clearly marked as such. This may
include source files, build scripts and documentation.

"Reserved Font Name" refers to any names specified as such after the
copyright statement(s).

"Original Version" refers to the collection of Font Software components as
distributed by the Copyright Holder(s).

"Modified Version" refers to any derivative made by adding to, deleting,
or substituting -- in part or in whole -- any of the components of the
Original Version, by changing formats or by porting the Font Software to a
new environment.

"Author" refers to any designer, engineer, programmer, technical
writer or other person who contributed to the Font Software.

PERMISSION & CONDITIONS
Permission is hereby granted, free of charge, to any person obtaining
a copy of the Font Software, to use, study, copy, merge, embed, modify,
redistribute, and sell modified and unmodified copies of the Font
Software, subject to the following conditions:

1) Neither the Font Software nor any of its individual components,
in Original or Modified Versions, may be sold by itself.

2) Original or Modified Versions of the Font Software may be bundled,
redistributed and/or sold with any software, provided that each copy
contains the above copyright notice and this license. These can be
included either as stand-alone text files, human-readable headers or
in the appropriate machine-readable metadata fields within text or
binary files as long as those fields can be easily viewed by the user.

3) No Modified Version of the Font Software may use the Reserved Font
Name(s) unless explicit written permission is granted by the corresponding
Copyright Holder. This restriction only applies to the primary font name as
presented to the users.

4) The name(s) of the Copyright Holder(s) or the Author(s) of the Font
Software shall not be used to promote, endorse or advertise any
Modified Version, except to acknowledge the contribution(s) of the
Copyright Holder(s) and the Author(s) or with their explicit written
permission.

5) The Font Software, modified or unmodified, in part or in whole,
must be distributed entirely under this license, and must not be
distributed under any other license. The requirement for fonts to
remain under this license does not apply to any document created
using the Font Software.

TERMINATION
This license becomes null and void if any of the above conditions are
not met.

DISCLAIMER
THE FONT SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO ANY WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT
OF COPYRIGHT, PATENT, TRADEMARK, OR OTHER RIGHT. IN NO EVENT SHALL THE
COPYRIGHT HOLDER BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
INCLUDING ANY GENERAL, SPECIAL, INDIRECT, INCIDENTAL, OR CONSEQUENTIAL
DAMAGES, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
FROM, OUT OF THE USE OR INABILITY TO USE THE FONT SOFTWARE OR FROM
OTHER DEALINGS IN THE FONT SOFTWARE.
```
