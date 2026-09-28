---
type: document
format: docx
template: sample-docx.docx
schema: 1
---

<div style="Title">Docx sample document</div>

This is a document exhibiting basic docx features.  

# This is style Heading 1

<div style="Normal"></div>

Some text.

# Tables

<div style="Normal"></div>

| Cell text |  | Shaded grey |
|---|---|---|
| Vertical merge |  | Shaded grey |
| ^^ |  |  |
|  | Horizontal merge ||

<div style="Normal"></div>

(There is another document which tests tables more thoroughly)

# Paragraph properties

<div style="Normal"></div>

Left indent

Centred 

Align Right

Justified text

<div style="Normal"></div>

Indented indented indented indented indented indented indented indented indented indented indented indented indented indented indented indented indented indented indented indented 

<div style="Normal"></div>

First line indent, Left indent, Hanging indent aaa bbb aaa bbb aaa bbb aaa bbb aaa bbb aaa bbb aaa bbb aaa bbb aaa bbb aaa bbb aaa bbb aaa bbb aaa bbb aaa bbb

<div style="Normal"></div>

<div style="Normal"></div>

Normal

A short paragraph with 10 points spacing before, 20 points after.

# Run properties

<div style="Normal"></div>

Font styles Aerial Black

Font styles 18 point

Font styles **bold**

Font styles *italic*

Font styles underline

# Bullets & numbering

<div style="Normal"></div>

Bullets

<div style="List Paragraph">Level 1</div>

<div style="List Paragraph">Level 2</div>

<div style="Normal"></div>

Numbering

<div style="List Paragraph">Level 1</div>

<div style="List Paragraph">Level 2</div>

<div style="List Paragraph">Level 3</div>

<div style="Normal"></div>

# Images

<div style="Normal"></div>

Jpeg:

<keep id="k1" kind="drawing" summary="C:\Documents and Settings\Jason Harrop\My Documents\tmp-test"/>

<div style="Normal"></div>

<div style="Normal"></div>

Gif (scaled):

<keep id="k2" kind="drawing" summary="Escher: Liberation"/>

<div style="Normal"></div>

<div style="Normal"></div>

Png (from http://davidpritchard.org/images/pacsoc-s1b.png )

<keep id="k3" kind="drawing" summary="http://davidpritchard.org/images/pacsoc-s1b.png"/>

<div style="Normal"></div>

(TODO: we really should have both 2003 & 2007 pictures)

<div style="Normal"></div>

<keep id="k4" kind="break" summary="br"/>

That was a page break

<div style="Normal"></div>

Here is some change tracking. <keep id="k5" kind="tracked-insert" summary="ins by Jason Harrop: An insertion"/> Followed by<keep id="k6" kind="tracked-delete" summary="del by Jason Harrop: A deletion"/>.

<div style="Normal"></div>

This line contains a soft return<br/>and here it continues

<div style="Normal"></div>
