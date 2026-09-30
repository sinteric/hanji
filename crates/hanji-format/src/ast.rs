//! Typed AST of a Document file (§5.1, §5.2).
//!
//! Inline content is flat: a sequence of [`Unit`]s, each one character or one
//! atom with its emphasis [`Marks`], plus non-overlapping [`Span`]s for links
//! and fields. Canonical form is a function of this structure alone.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Document {
    pub front: FrontMatter,
    pub blocks: Vec<Block>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FrontMatter {
    pub doc_type: String,
    pub format: String,
    pub template: Option<String>,
    pub schema: u32,
    /// A Presentation's slide size, width and height in EMU (`size: 720 x 540 pt`).
    pub size: Option<(i64, i64)>,
}

impl FrontMatter {
    pub fn document(format: &str, template: Option<&str>) -> Self {
        Self::of("document", format, template)
    }

    pub fn presentation(format: &str, template: Option<&str>) -> Self {
        Self::of("presentation", format, template)
    }

    pub fn spreadsheet(format: &str, template: Option<&str>) -> Self {
        Self::of("spreadsheet", format, template)
    }

    fn of(doc_type: &str, format: &str, template: Option<&str>) -> Self {
        FrontMatter {
            doc_type: doc_type.into(),
            format: format.into(),
            template: template.map(str::to_string),
            schema: crate::SCHEMA_VERSION,
            size: None,
        }
    }
}

/// A Presentation file (§5.3): slides, each a canvas of objects in z-order.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Presentation {
    pub front: FrontMatter,
    pub slides: Vec<Slide>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Slide {
    /// `layout: Name`, as the file lists it.
    pub layout: String,
    /// Slots, shapes and objects in the order written (the slide's z-order); notes last.
    pub items: Vec<SlideItem>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SlideItem {
    /// `::name box="…"::` and the lines after it: paragraphs, list items,
    /// empty paragraphs and placeholders.
    Slot(Slot),
    /// `<shape id="…" name="…" box="…">text</shape>`: a shape that is not a
    /// layout placeholder; `<p/>` starts another paragraph. Without text it
    /// is `<shape … />`; a new text box has no id or name.
    Shape(ShapeText),
    /// A `<keep … box="…"/>` line outside a slot: a picture, chart, table or
    /// other object of the slide the text does not model (rule 8). It can be
    /// moved, resized or deleted, never created or changed.
    Object(ObjectItem),
    /// `<line id="…" name="…" from="x y" to="x y"/>`: a line or connector.
    Line(LineItem),
    /// `<group id="…" name="…" box="…">`, its objects, `</group>`.
    Group(GroupItem),
    /// `<picture id="…" name="…" box="…" src="…"/>`: a picture, with its
    /// image, crop, mask and alternative text; a new one has no id or name.
    Picture(PictureItem),
}

/// Where an object is (§5.3): its box in EMU as written (the text shows
/// whole points, 1 pt = 12,700 EMU), its rotation in 60,000ths of a degree
/// clockwise, and its flips.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Geom {
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
    pub rot: i64,
    pub flip_h: bool,
    pub flip_v: bool,
}

/// A line's two ends in EMU, as written.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Ends {
    pub from: (i64, i64),
    pub to: (i64, i64),
}

/// EMU per point.
pub const EMU_PER_PT: i64 = 12_700;

/// A length in EMU as the text shows it: whole points.
pub fn shown_pt(emu: i64) -> i64 {
    (emu as f64 / EMU_PER_PT as f64).round() as i64
}

/// A rotation in 60,000ths of a degree as the text shows it: whole degrees.
pub fn shown_deg(rot: i64) -> i64 {
    (rot as f64 / 60_000.0).round() as i64
}

/// A full turn in 60,000ths of a degree.
pub const FULL_TURN: i64 = 21_600_000;

/// Whether two rotations (60,000ths of a degree) turn an object the same
/// way: -90° is 270°, 360° is 0°. Files store either form.
pub fn same_turn(a: i64, b: i64) -> bool {
    (a - b).rem_euclid(FULL_TURN) == 0
}

/// `written` where it differs from how `stored` is shown, else `stored`:
/// a number left as shown keeps its exact value (§5.3).
pub fn keep_or(stored: i64, written: i64) -> i64 {
    if shown_pt(stored) * EMU_PER_PT == written {
        stored
    } else {
        written
    }
}

impl Geom {
    /// The same box as `other` as the text shows them (whole points and degrees, same flips).
    pub fn shows_as(&self, other: &Geom) -> bool {
        let p = |a: i64, b: i64| shown_pt(a) == shown_pt(b);
        p(self.x, other.x)
            && p(self.y, other.y)
            && p(self.w, other.w)
            && p(self.h, other.h)
            && same_turn(shown_deg(self.rot) * 60_000, shown_deg(other.rot) * 60_000)
            && self.flip_h == other.flip_h
            && self.flip_v == other.flip_v
    }

    /// `written`, with each number left as `self` shows it kept exact. A
    /// rotation that turns the way `self` is shown keeps its stored value,
    /// whatever its sign or range (`rot="-5400000"`, shown 270, stays); a
    /// changed one is from 0 up to a full turn, as the parse reads it.
    pub fn merged(&self, written: &Geom) -> Geom {
        let rot = if same_turn(shown_deg(self.rot) * 60_000, written.rot) {
            self.rot
        } else {
            written.rot.rem_euclid(FULL_TURN)
        };
        Geom {
            x: keep_or(self.x, written.x),
            y: keep_or(self.y, written.y),
            w: keep_or(self.w, written.w),
            h: keep_or(self.h, written.h),
            rot,
            flip_h: written.flip_h,
            flip_v: written.flip_v,
        }
    }

    /// As the text shows it: every number rounded.
    pub fn shown(&self) -> Geom {
        let r = |v: i64| shown_pt(v) * EMU_PER_PT;
        Geom { x: r(self.x), y: r(self.y), w: r(self.w), h: r(self.h), rot: shown_deg(self.rot) * 60_000, ..*self }
    }
}

impl Ends {
    pub fn shows_as(&self, other: &Ends) -> bool {
        let p = |a: (i64, i64), b: (i64, i64)| shown_pt(a.0) == shown_pt(b.0) && shown_pt(a.1) == shown_pt(b.1);
        p(self.from, other.from) && p(self.to, other.to)
    }

    pub fn merged(&self, written: &Ends) -> Ends {
        let k = |a: (i64, i64), b: (i64, i64)| (keep_or(a.0, b.0), keep_or(a.1, b.1));
        Ends { from: k(self.from, written.from), to: k(self.to, written.to) }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Slot {
    pub name: String,
    /// Where the slot is; `None` (a bare marker) is where its layout puts it.
    pub geom: Option<Geom>,
    pub blocks: Vec<Block>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ShapeText {
    /// Empty for a new text box (the write gives it an id and a name).
    pub id: String,
    pub name: String,
    pub geom: Option<Geom>,
    pub paras: Vec<Inline>,
}

/// A slide object's `<keep/>` line and its box.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ObjectItem {
    pub keep: Keep,
    pub geom: Option<Geom>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LineItem {
    /// Empty for a new line.
    pub id: String,
    pub name: String,
    pub ends: Ends,
}

/// A picture (§5.3): where it is, the image it shows (`src`, a part of the
/// package such as `media/image1.png`, or a file the host hands the write),
/// its crop, the preset shape it is cut to (`mask`, none for a rectangle)
/// and its alternative text.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PictureItem {
    /// Empty for a new picture (the write gives it an id and a name).
    pub id: String,
    pub name: String,
    pub geom: Option<Geom>,
    pub src: String,
    pub crop: Option<Crop>,
    pub mask: Option<String>,
    pub alt: Option<String>,
}

/// How much of the image is cut off at each edge, left, top, right and
/// bottom, in thousandths of a percent of its size (`a:srcRect`); the text
/// shows percent. A negative value leaves space beside the image.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Crop {
    pub l: i64,
    pub t: i64,
    pub r: i64,
    pub b: i64,
}

/// Thousandths of a percent per percent (`a:srcRect` units).
pub const PER_PERCENT: i64 = 1_000;

/// A crop value (thousandths of a percent) as the text shows it: percent, at most one decimal.
pub fn shown_pct(v: i64) -> String {
    let tenths = (v as f64 / 100.0).round() as i64;
    let (sign, a) = if tenths < 0 { ("-", -tenths) } else { ("", tenths) };
    if a % 10 == 0 {
        format!("{sign}{}", a / 10)
    } else {
        format!("{sign}{}.{}", a / 10, a % 10)
    }
}

impl Crop {
    pub fn is_zero(&self) -> bool {
        (self.l, self.t, self.r, self.b) == (0, 0, 0, 0)
    }

    fn vals(&self) -> [i64; 4] {
        [self.l, self.t, self.r, self.b]
    }

    /// The same crop as `other` as the text shows them.
    pub fn shows_as(&self, other: &Crop) -> bool {
        self.vals().iter().zip(other.vals()).all(|(a, b)| shown_pct(*a) == shown_pct(b))
    }

    /// `written`, each value left as `self` shows it keeping its exact value.
    pub fn merged(&self, written: &Crop) -> Crop {
        let k = |s: i64, w: i64| if shown_pct(s) == shown_pct(w) { s } else { w };
        Crop { l: k(self.l, written.l), t: k(self.t, written.t), r: k(self.r, written.r), b: k(self.b, written.b) }
    }

    /// As the text shows it: every value rounded to a tenth of a percent.
    pub fn shown(&self) -> Crop {
        let r = |v: i64| (v as f64 / 100.0).round() as i64 * 100;
        Crop { l: r(self.l), t: r(self.t), r: r(self.r), b: r(self.b) }
    }
}

/// A group: its box (the box around its objects) and its objects, in slide
/// coordinates. A member `<keep/>` stands for the member by its shape id.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GroupItem {
    pub id: String,
    pub name: String,
    pub geom: Option<Geom>,
    pub items: Vec<SlideItem>,
}

impl Presentation {
    /// Canonical shapes, as [`Document::normalize`] gives a document's blocks.
    pub fn normalize(&mut self) {
        fn items(its: &mut [SlideItem]) {
            for it in its {
                match it {
                    SlideItem::Slot(slot) => normalize_blocks(&mut slot.blocks),
                    SlideItem::Shape(sh) => sh.paras.iter_mut().for_each(Inline::normalize),
                    SlideItem::Group(g) => items(&mut g.items),
                    SlideItem::Object(_) | SlideItem::Line(_) | SlideItem::Picture(_) => {}
                }
            }
        }
        for s in &mut self.slides {
            items(&mut s.items);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Block {
    Para(Para),
    Table(Table),
    /// A line holding only a placeholder. Whether it stands for a block
    /// entry or an inline object alone in a default-style paragraph is the
    /// engine's call (it knows the placeholder registry).
    Keep(Keep),
    PageBreak,
    FootnoteDef(FootnoteDef),
    /// Consecutive list item lines (`- ` / `1. `); a blank line ends the list.
    List(Vec<Item>),
}

/// A list item: one paragraph at a nesting level (0 = the margin).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Item {
    /// `1.` (ordered) or `-` (bullet).
    pub ordered: bool,
    pub level: usize,
    pub content: Inline,
}

impl Item {
    /// Canonical marker and the content column it gives nested items.
    pub fn marker(&self) -> &'static str {
        if self.ordered {
            "1."
        } else {
            "-"
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Para {
    pub style: ParaStyle,
    pub content: Inline,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ParaStyle {
    /// A plain line: the file's default paragraph style.
    Plain,
    /// `#`–`######`: the file's own Heading 1–6.
    Heading(u8),
    /// `<div style="Name">`.
    Named(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Table {
    /// `{style="Name"}` line before the header row; `None` is the default table style.
    pub style: Option<String>,
    /// Every row has one cell per column; row 0 is the header row.
    pub rows: Vec<Vec<Cell>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Cell {
    /// One or more paragraphs (§5.2: `<p/>` starts another).
    Text(Vec<CellPara>),
    /// `^^`: merged into the cell above.
    Up,
    /// `||`: the cell to the left extends into this column.
    Left,
}

impl Cell {
    /// A cell of one default-style paragraph.
    pub fn text(content: Inline) -> Cell {
        Cell::Text(vec![CellPara { style: None, content }])
    }
}

/// A paragraph in a table cell; `None` is the default paragraph style.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CellPara {
    pub style: Option<String>,
    pub content: Inline,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FootnoteDef {
    pub label: String,
    pub content: Inline,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Inline {
    pub units: Vec<Unit>,
    /// Sorted, non-overlapping, over `units` indices `[start, end)`.
    pub spans: Vec<Span>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Unit {
    pub atom: Atom,
    pub marks: Marks,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Atom {
    Char(char),
    /// `<br/>`.
    Break,
    /// Inline `<keep/>`.
    Keep(Keep),
    /// `[^label]`.
    NoteRef(String),
    /// `$…$`, kept opaque.
    Math(String),
    /// Only as the whole content of a resolved page-break paragraph; the
    /// text form is the block line `<pagebreak/>`.
    PageBreak,
}

impl Atom {
    pub fn is_space(&self) -> bool {
        matches!(self, Atom::Char(c) if c.is_whitespace())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Keep {
    pub id: String,
    pub kind: String,
    pub summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub kind: SpanKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SpanKind {
    Link(String),
    Field(String),
    /// A Presentation's text formatting (§5.3): the effective font, size
    /// and colour of the units it covers.
    Style(crate::inline_style::TextStyle),
}

/// Emphasis flags of one unit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct Marks(pub u8);

impl Marks {
    pub const NONE: Marks = Marks(0);
    pub const BOLD: Marks = Marks(1);
    pub const ITALIC: Marks = Marks(2);
    pub const STRIKE: Marks = Marks(4);
    pub const UNDERLINE: Marks = Marks(8);
    /// Opening order when several marks start together (outermost first).
    pub const ALL: [Marks; 4] = [Marks::BOLD, Marks::ITALIC, Marks::STRIKE, Marks::UNDERLINE];

    pub fn has(self, m: Marks) -> bool {
        self.0 & m.0 == m.0 && m.0 != 0
    }
    pub fn with(self, m: Marks, on: bool) -> Marks {
        if on {
            Marks(self.0 | m.0)
        } else {
            Marks(self.0 & !m.0)
        }
    }
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl fmt::Display for Marks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names = ["bold", "italic", "strike", "underline"];
        let on: Vec<&str> = Marks::ALL.iter().zip(names).filter(|(m, _)| self.has(**m)).map(|(_, n)| n).collect();
        write!(f, "{}", if on.is_empty() { "plain".into() } else { on.join("+") })
    }
}

impl Inline {
    pub fn plain(text: &str) -> Inline {
        Inline {
            units: text
                .chars()
                .map(|c| Unit { atom: if c == '\n' { Atom::Break } else { Atom::Char(c) }, marks: Marks::NONE })
                .collect(),
            spans: vec![],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.units.is_empty() && self.spans.is_empty()
    }

    /// Spaces only (no tab, break or atom): a plain line cannot hold it.
    pub fn spaces_only(&self) -> bool {
        !self.units.is_empty()
            && self.spans.is_empty()
            && self.units.iter().all(|u| matches!(u.atom, Atom::Char(c) if c.is_whitespace() && c != '\t'))
    }

    /// The text with atoms shown as `\u{FFFC}` (for summaries and messages).
    pub fn text(&self) -> String {
        self.units
            .iter()
            .map(|u| match &u.atom {
                Atom::Char(c) => *c,
                Atom::Break => '\n',
                _ => '\u{FFFC}',
            })
            .collect()
    }

    /// Segment boundaries: span edges split a paragraph into independently
    /// marked stretches.
    pub(crate) fn segments(&self) -> Vec<(usize, usize, Option<&Span>)> {
        let mut out = vec![];
        let mut at = 0;
        for s in &self.spans {
            if s.start > at {
                out.push((at, s.start, None));
            }
            out.push((s.start, s.end, Some(s)));
            at = s.end;
        }
        if at < self.units.len() || out.is_empty() {
            out.push((at, self.units.len(), None));
        }
        out
    }

    /// Canonical marks: every stretch of a mark starts and ends on a
    /// non-space unit inside its segment. Text cannot express a mark that
    /// starts or ends on a space (`**fox **` is not emphasis), so boundary
    /// spaces go outside.
    pub fn normalize(&mut self) {
        let segs: Vec<(usize, usize)> = self.segments().iter().map(|s| (s.0, s.1)).collect();
        for (a, b) in segs {
            for m in Marks::ALL {
                let mut i = a;
                while i < b {
                    if !self.units[i].marks.has(m) {
                        i += 1;
                        continue;
                    }
                    let mut e = i;
                    while e < b && self.units[e].marks.has(m) {
                        e += 1;
                    }
                    let (mut s, mut t) = (i, e);
                    while s < t && self.units[s].atom.is_space() {
                        self.units[s].marks = self.units[s].marks.with(m, false);
                        s += 1;
                    }
                    while t > s && self.units[t - 1].atom.is_space() {
                        self.units[t - 1].marks = self.units[t - 1].marks.with(m, false);
                        t -= 1;
                    }
                    i = e;
                }
            }
        }
    }

    /// The marks to write for each unit: the text's own, except where the
    /// text cannot state a mark. A page break carries none, and a space at a
    /// mark's edge cannot carry it (see [`Inline::normalize`]): there
    /// `fallback(i)` (what the unit's run had) is kept, as long as the text
    /// reads back the same. The one case it would not is a stretch of spaces
    /// between two units with the mark that all get it back: the text shows
    /// those spaces without the mark, so they are written without it.
    pub fn written_marks(&self, fallback: &dyn Fn(usize) -> Marks) -> Vec<Marks> {
        let mut out: Vec<Marks> = self.units.iter().map(|u| u.marks).collect();
        for (a, b, _) in self.segments() {
            for m in Marks::ALL {
                let unstated = |i: usize| !self.units[i].marks.has(m);
                for (i, x) in out.iter_mut().enumerate().take(b).skip(a) {
                    if self.units[i].atom == Atom::PageBreak && unstated(i) {
                        *x = x.with(m, fallback(i).has(m));
                    }
                }
                let mut i = a;
                while i < b {
                    if !(self.units[i].atom.is_space() && unstated(i)) {
                        i += 1;
                        continue;
                    }
                    let s = i;
                    while i < b && self.units[i].atom.is_space() && unstated(i) {
                        out[i] = out[i].with(m, fallback(i).has(m));
                        i += 1;
                    }
                    let marked = |k: Option<usize>| {
                        k.is_some_and(|k| !self.units[k].atom.is_space() && self.units[k].marks.has(m))
                    };
                    let (left, right) = (s.checked_sub(1).filter(|&k| k >= a), (i < b).then_some(i));
                    if marked(left) && marked(right) && out[s..i].iter().all(|x| x.has(m)) {
                        for x in &mut out[s..i] {
                            *x = x.with(m, false);
                        }
                    }
                }
            }
        }
        out
    }
}

impl Document {
    /// Canonical block shapes: a plain paragraph holding only an unmarked
    /// placeholder is a keep line, one holding only a page break is
    /// `<pagebreak/>`, a plain one of spaces only is empty (`<p/>`), and every
    /// inline is normalized.
    pub fn normalize(&mut self) {
        normalize_blocks(&mut self.blocks);
    }
}

/// [`Document::normalize`] over a list of blocks (a document body, a presentation slot).
pub(crate) fn normalize_blocks(blocks: &mut [Block]) {
    for b in blocks {
        match b {
            Block::Para(p) => {
                // A plain paragraph of spaces only has no line form (engines
                // write it as `<div style="Name">`, naming the file's style).
                if p.style == ParaStyle::Plain && p.content.spaces_only() {
                    p.content.units.clear();
                }
                p.content.normalize();
                if p.style == ParaStyle::Plain && p.content.spans.is_empty() && p.content.units.len() == 1 {
                    let u = &p.content.units[0];
                    match &u.atom {
                        Atom::Keep(k) if u.marks.is_empty() => *b = Block::Keep(k.clone()),
                        Atom::PageBreak => *b = Block::PageBreak,
                        _ => {}
                    }
                }
            }
            Block::Table(t) => {
                for row in &mut t.rows {
                    for c in row {
                        if let Cell::Text(ps) = c {
                            for p in ps {
                                p.content.normalize();
                            }
                        }
                    }
                }
            }
            Block::FootnoteDef(f) => f.content.normalize(),
            Block::List(items) => {
                // A list starts at the margin and nests one level at a time.
                let mut prev: Option<usize> = None;
                for it in items {
                    it.level = it.level.min(prev.map_or(0, |p| p + 1));
                    prev = Some(it.level);
                    it.content.normalize();
                }
            }
            Block::Keep(_) | Block::PageBreak => {}
        }
    }
}
