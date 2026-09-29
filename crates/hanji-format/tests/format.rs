use hanji_format::*;

const FM: &str = "---\ntype: document\nformat: docx\ntemplate: org/report\nschema: 1\n---\n";

fn doc(body: &str) -> String {
    format!("{FM}{body}")
}

fn roundtrip(text: &str) -> String {
    let d = parse(text).unwrap_or_else(|e| panic!("{}\n---\n{text}", diag::render(&e)));
    let s = serialize(&d);
    let d2 = parse(&s).unwrap_or_else(|e| panic!("reparse: {}\n---\n{s}", diag::render(&e)));
    assert_eq!(d, d2, "parse → serialize → parse changed the AST\n{s}");
    assert_eq!(serialize(&d2), s, "not idempotent");
    s
}

fn errors(body: &str) -> Vec<String> {
    errors_with(body, &Names::default())
}

fn errors_with(body: &str, names: &Names) -> Vec<String> {
    match parse_with(&doc(body), names) {
        Ok(_) => vec![],
        Err(e) => e.iter().map(|d| d.to_string()).collect(),
    }
}

#[test]
fn design_example_is_canonical() {
    let text = doc("# 3분기 영업 보고\n\n매출은 전년 대비 **12%** 증가했다.[^1]\n\n<div style=\"Note\">신규 고객 34곳 중 21곳이 수도권.</div>\n\n| 지역 | 지점 | 매출 |\n|---|---|---|\n| 서울 | 강남 | 120 |\n| ^^ | 종로 | 95 |\n| 합계 || 215 |\n\n{style=\"Grid Table 4\"}\n| 지역 | 매출 | 증감 |\n|---|---|---|\n| 수도권 | 1,204 | +15% |\n\n<field name=\"작성자\">홍길동</field>\n\n<keep id=\"k3\" kind=\"drawing\" summary=\"조직도, 상자 5개\"/>\n\n<pagebreak/>\n\n[^1]: 내부 집계 기준.\n");
    assert_eq!(roundtrip(&text), text);
    let d = parse(&text).unwrap();
    assert!(matches!(&d.blocks[4], Block::Table(t) if t.style.as_deref() == Some("Grid Table 4")));
    assert!(matches!(&d.blocks[6], Block::Keep(k) if k.id == "k3"));
    assert!(matches!(&d.blocks[7], Block::PageBreak));
    match &d.blocks[3] {
        Block::Table(t) => {
            assert_eq!(t.rows[2][0], Cell::Up);
            assert_eq!(t.rows[3][2], Cell::text(Inline::plain("215")));
            assert_eq!(t.rows[3][1], Cell::Left);
        }
        b => panic!("{b:?}"),
    }
}

#[test]
fn non_canonical_input_is_canonicalised() {
    let cases = [
        ("heading\n# Title\nnext line\n", "heading\n\n# Title\n\nnext line\n"),
        ("|a|b|\n| --- | --- |\n|c|d|\n", "| a | b |\n|---|---|\n| c | d |\n"),
        // 2×2 merge written ^^ ^^ becomes ^^ ||
        ("| 합산 || 10 |\n|---|---|---|\n| ^^ | ^^ | 20 |\n", "| 합산 || 10 |\n|---|---|---|\n| ^^ || 20 |\n"),
        ("a*b*c **x**y\n", "a*b*c **x**y\n"),
        ("<keep summary=\"s\" kind=\"k\" id=\"k1\"/>\n", "<keep id=\"k1\" kind=\"k\" summary=\"s\"/>\n"),
        ("x<br>y\n", "x<br/>y\n"),
        ("{style='Grid'}\n| a |\n|---|\n", "{style=\"Grid\"}\n| a |\n|---|\n"),
    ];
    for (input, want) in cases {
        let got = serialize(&parse(&doc(input)).unwrap());
        assert_eq!(got, doc(want), "input: {input}");
    }
}

#[test]
fn three_column_span_and_spaced_pipes() {
    let d = parse(&doc("| a | b | c | d |\n|---|---|---|---|\n| 합계 ||| 215 |\n")).unwrap();
    let Block::Table(t) = &d.blocks[0] else { panic!() };
    assert_eq!(t.rows[1][1], Cell::Left);
    assert_eq!(t.rows[1][2], Cell::Left);
    // `|| ||` is two separate merges with an empty cell between: one cell too many here
    let e = errors("| a | b | c | d |\n|---|---|---|---|\n| 합계 || || 215 |\n");
    assert!(e[0].contains("this row has 5 cells but the header row has 4"), "{e:?}");
    let d = parse(&doc("| a | b | c | d | e |\n|---|---|---|---|---|\n| 합계 || || 215 |\n")).unwrap();
    let Block::Table(t) = &d.blocks[0] else { panic!() };
    assert_eq!(t.rows[1][1..4], [Cell::Left, Cell::text(Inline::default()), Cell::Left]);
}

#[test]
fn boundary_spaces_go_outside_emphasis() {
    let mut i = Inline::plain("The fox jumps");
    for u in &mut i.units[4..8] {
        u.marks = Marks::BOLD;
    }
    i.normalize();
    assert_eq!(serialize_inline(&i), "The **fox** jumps");
    let mut i = Inline::plain("a b");
    for u in &mut i.units {
        u.marks = Marks::BOLD;
    }
    assert_eq!(serialize_inline(&i), "**a b**");
    // crossing marks toggle rather than reopen (no **** runs)
    let mut i = Inline::plain("abcdefgh");
    for (k, u) in i.units.iter_mut().enumerate() {
        u.marks = Marks(u8::from(k < 5) | (u8::from(k >= 3) << 1));
    }
    let s = serialize_inline(&i);
    let d = parse(&doc(&format!("{s}\n"))).unwrap();
    let Block::Para(p) = &d.blocks[0] else { panic!() };
    assert_eq!(p.content, i, "{s}");
}

#[test]
fn escapes_only_where_needed() {
    let cases = [
        ("a*b", "a\\*b"),
        ("10~20%", "10~20%"),
        ("a~~b", "a~\\~b"),
        ("a < b", "a < b"),
        ("<div>", "\\<div>"),
        ("[참고] 자료", "[참고] 자료"),
        ("[x](y)", "[x\\](y)"),
        ("[^1]", "\\[^1]"),
        ("$5 and $10", "$5 and $10"),
        ("$a$", "\\$a$"),
        ("back\\slash", "back\\\\slash"),
        ("snake_case", "snake_case"),
    ];
    for (text, want) in cases {
        assert_eq!(serialize_inline(&Inline::plain(text)), want, "{text}");
        let line = serialize(&Document {
            front: FrontMatter::document("docx", None),
            blocks: vec![Block::Para(Para { style: ParaStyle::Plain, content: Inline::plain(text) })],
        });
        let d = parse(&line).unwrap();
        assert_eq!(
            d.blocks,
            vec![Block::Para(Para { style: ParaStyle::Plain, content: Inline::plain(text) })],
            "{line}"
        );
    }
    for start in
        ["# not a heading", "- not a list", "1. not a list", "> quote", "| not a table", "{style}", "---", "  |x"]
    {
        let d = Document {
            front: FrontMatter::document("docx", None),
            blocks: vec![Block::Para(Para { style: ParaStyle::Plain, content: Inline::plain(start) })],
        };
        let s = serialize(&d);
        assert_eq!(parse(&s).unwrap(), d, "{s}");
    }
}

#[test]
fn table_cell_escapes() {
    let cell = |s: &str| Cell::text(Inline::plain(s));
    let d = Document {
        front: FrontMatter::document("docx", None),
        blocks: vec![Block::Table(Table {
            style: None,
            rows: vec![
                vec![cell("a|b"), cell("^^"), cell(""), cell(" pad ")],
                vec![cell("x\\"), cell("\\|"), Cell::Left, cell("$|x|$")],
            ],
        })],
    };
    let s = serialize(&d);
    assert!(s.contains("| a\\|b | \\^^ |  |  pad  |"), "{s}");
    assert_eq!(parse(&s).unwrap(), d, "{s}");
}

#[test]
fn spans_and_atoms_roundtrip() {
    let text = doc("see [the **report**](https://x.org/a_(b)) and <field name=\"작성자\">*홍*</field>, $x^2 \\$ y$<br/><u>under</u> ~~gone~~ <keep id=\"k1\" kind=\"field\" summary=\"a &quot;b&quot;\"/>.[^n1]\n\n[^n1]: note\n");
    roundtrip(&text);
    let d = parse(&text).unwrap();
    let Block::Para(p) = &d.blocks[0] else { panic!() };
    assert_eq!(p.content.spans.len(), 2);
    assert!(matches!(&p.content.spans[0].kind, SpanKind::Link(u) if u == "https://x.org/a_(b"));
}

// ---------------------------------------------------------------- random round trip

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

const ALPHABET: &[char] = &[
    'a', 'b', 'Z', '1', '9', ' ', ' ', '가', '한', '*', '~', '\\', '<', '>', '[', ']', '(', ')', '$', '|', '^', '#',
    '-', '_', '&', '"', '\t', '{', '}', '.', '!', '/',
];

fn random_inline(r: &mut Rng, notes: &mut Vec<String>, allow_spans: bool) -> Inline {
    let n = r.below(14);
    let mut units = vec![];
    let mut marks = Marks::NONE;
    for _ in 0..n {
        if r.below(4) == 0 {
            marks = Marks(r.below(16) as u8);
        }
        let atom = match r.below(20) {
            0 => Atom::Break,
            1 => Atom::Keep(Keep {
                id: format!("k{}", r.below(99)),
                kind: "drawing".into(),
                summary: "a \"b\" <c> & d".into(),
            }),
            // Bodies `$…$` cannot hold are written <math>…</math>.
            2 => {
                Atom::Math(["x^2", "\\frac{a}{b}", "a|b", "y_1", " a", "b ", "p$q", "c\\", "<p/>d"][r.below(9)].into())
            }
            3 => {
                let l = format!("n{}", r.below(3));
                notes.push(l.clone());
                Atom::NoteRef(l)
            }
            _ => Atom::Char(*r.pick(ALPHABET)),
        };
        units.push(Unit { atom, marks });
    }
    let mut spans = vec![];
    if allow_spans && units.len() > 2 && r.below(3) == 0 {
        let a = r.below(units.len() - 1);
        let b = a + 1 + r.below(units.len() - a - 1);
        let kind = if r.below(2) == 0 {
            SpanKind::Link("https://e.x/p?q=1&r=(2)".into())
        } else {
            SpanKind::Field("이름".into())
        };
        spans.push(Span { start: a, end: b, kind });
    }
    let mut i = Inline { units, spans };
    i.normalize();
    i
}

fn random_doc(seed: u64) -> Document {
    let mut r = Rng(seed * 2654435761 + 1);
    let mut notes = vec![];
    let mut blocks = vec![];
    for _ in 0..1 + r.below(6) {
        let b = match r.below(8) {
            0 | 1 => {
                let style = match r.below(3) {
                    0 => ParaStyle::Plain,
                    1 => ParaStyle::Heading(1 + r.below(6) as u8),
                    _ => ParaStyle::Named("Body Text".into()),
                };
                let mut content = random_inline(&mut r, &mut notes, true);
                if style == ParaStyle::Plain
                    && !content.units.is_empty()
                    && content.units.iter().all(|u| u.atom.is_space())
                {
                    content = Inline::plain("x");
                }
                Block::Para(Para { style, content })
            }
            2 => {
                let w = 1 + r.below(4);
                let h = 1 + r.below(3);
                let rows = (0..h)
                    .map(|_| {
                        (0..w)
                            .map(|_| {
                                let n = 1 + r.below(3) * r.below(2);
                                let ps = (0..n)
                                    .map(|_| CellPara {
                                        style: (r.below(3) == 0).then(|| "표 참고".to_string()),
                                        content: random_inline(&mut r, &mut notes, true),
                                    })
                                    .collect();
                                Cell::Text(ps)
                            })
                            .collect()
                    })
                    .collect();
                Block::Table(Table { style: (r.below(2) == 0).then(|| "Grid Table 4".into()), rows })
            }
            3 => Block::Keep(Keep { id: "kb".into(), kind: "table".into(), summary: "x".into() }),
            4 => Block::PageBreak,
            6 => Block::List(
                (0..1 + r.below(4))
                    .map(|_| Item {
                        ordered: r.below(2) == 0,
                        level: r.below(3),
                        content: random_inline(&mut r, &mut notes, true),
                    })
                    .collect(),
            ),
            5 => Block::Para(Para {
                style: if r.below(2) == 0 { ParaStyle::Plain } else { ParaStyle::Named("좁은 간격".into()) },
                content: Inline::default(),
            }),
            _ => Block::Para(Para { style: ParaStyle::Plain, content: Inline::plain("plain") }),
        };
        blocks.push(b);
    }
    notes.sort();
    notes.dedup();
    for l in notes {
        blocks.push(Block::FootnoteDef(FootnoteDef { label: l, content: Inline::plain("def") }));
    }
    let mut d = Document { front: FrontMatter::document("docx", Some("t")), blocks };
    d.normalize();
    d
}

#[test]
fn random_documents_roundtrip() {
    for seed in 0..3000 {
        let d = random_doc(seed);
        let s = serialize(&d);
        let back = parse(&s).unwrap_or_else(|e| panic!("seed {seed}: {}\n{s}", diag::render(&e)));
        if back != d {
            for (a, b) in back.blocks.iter().zip(&d.blocks) {
                if a != b {
                    let inl = |b: &Block| -> Vec<Inline> {
                        match b {
                            Block::Para(p) => vec![p.content.clone()],
                            Block::FootnoteDef(f) => vec![f.content.clone()],
                            Block::List(items) => items.iter().map(|i| i.content.clone()).collect(),
                            Block::Table(t) => t
                                .rows
                                .iter()
                                .flatten()
                                .filter_map(|c| {
                                    if let Cell::Text(ps) = c {
                                        Some(ps.iter().map(|p| p.content.clone()))
                                    } else {
                                        None
                                    }
                                })
                                .flatten()
                                .collect(),
                            _ => vec![],
                        }
                    };
                    for (x, y) in inl(a).iter().zip(inl(b).iter()) {
                        if x != y {
                            let k = x.units.iter().zip(&y.units).position(|(p, q)| p != q).unwrap_or(0);
                            panic!(
                                "seed {seed}: {}\nfrom unit {k}\ngot  {:?} {:?}\nwant {:?} {:?}",
                                serialize_inline(y),
                                &x.units[k.min(x.units.len())..],
                                x.spans,
                                &y.units[k..],
                                y.spans
                            );
                        }
                    }
                    panic!("seed {seed}\n{s}\ngot  {a:?}\nwant {b:?}");
                }
            }
        }
        assert_eq!(serialize(&back), s, "seed {seed}: not idempotent");
    }
}

#[test]
fn source_map_points_at_units() {
    let text = doc("ab **c**\n\n| x | ^^y |\n|---|---|\n");
    let p = parse_with(&text, &Names::default()).unwrap();
    let BlockMapKind::Para(pm) = &p.map.blocks[0].kind else { panic!() };
    let at: Vec<char> = pm.units.iter().map(|&o| text[o..].chars().next().unwrap()).collect();
    assert_eq!(at, vec!['a', 'b', ' ', 'c']);
    assert_eq!(&text[pm.mark..pm.mark + 1], "\n");
    let BlockMapKind::Table(cells) = &p.map.blocks[1].kind else { panic!() };
    assert_eq!(&text[cells[0][1].as_ref().unwrap()[0].units[0]..][..1], "^");
}

// ---------------------------------------------------------------- errors

#[test]
fn errors_have_line_column_and_form() {
    let e = errors("para\n\n| a | b |\n|---|---|\n| ^^ | x | y |\n");
    assert_eq!(e, vec!["line 11, column 1: this row has 3 cells but the header row has 2. Every row has one cell per column: a cell merged into the one above is written ^^, and a column covered by the cell to its left is written || (nothing between the pipes; three columns are |||)."]);
    let e = errors("| ^^ | a |\n|---|---|\n");
    assert_eq!(e, vec!["line 7, column 2: ^^ in the header row: there is no cell above to merge into."]);
    let e = errors("| a | b |\n|---|---|\n|| x |\n");
    assert!(e[0].starts_with("line 9, column 2: || at the start of a row"), "{e:?}");
    let e = errors("| a || x |\n|---|---|---|\n| ^^ | y | z |\n");
    assert!(e[0].contains("is not a rectangle"), "{e:?}");
    let e = errors("the **fox ** jumps\n");
    assert!(e[0].starts_with("line 7, column 11: ** closes bold only directly after text"), "{e:?}");
    let e = errors("the ** fox** jumps\n");
    assert!(e[0].contains("closes bold that was never opened"), "{e:?}");
    let e = errors("**open\n");
    assert!(e[0].starts_with("line 7, column 1: bold opened here is never closed"), "{e:?}");
    let e = errors("x <span>y</span>\n");
    assert!(e[0].starts_with("line 7, column 3: <span> is not a tag of this format. Inline tags are <u>"), "{e:?}");
    let e = errors("* item\n");
    assert!(e[0].contains("a bullet item is written - text"), "{e:?}");
    let e = errors("1) item\n");
    assert!(e[0].contains("a numbered item is written 1. text"), "{e:?}");
    let e = errors("{style=\"Grid\"}\n\n| a |\n|---|\n");
    assert!(e[0].contains("must be directly followed by the header row"), "{e:?}");
    let e = errors("{style=\"Grid\" color=\"red\"}\n| a |\n|---|\n");
    assert!(e[0].contains("has no attribute \"color\""), "{e:?}");
    let e = errors("<table style=\"Grid\">\n| a |\n|---|\n</table>\n");
    assert!(e[0].contains("{style=\"Name\"}"), "{e:?}");
    let e = errors("| a |\n|:---|\n");
    assert!(e[0].contains("alignment is direct formatting"), "{e:?}");
    let e = errors("x[^9]\n");
    assert!(e[0].contains("footnote [^9] has no definition"), "{e:?}");
    let e = errors("<div style=\"Note\">x\n");
    assert!(e[0].contains("<div> is not closed by </div>"), "{e:?}");
    let e = parse("# no front matter\n").unwrap_err();
    assert!(e[0].to_string().starts_with("line 1, column 1: the file must begin with its front matter"));
}

#[test]
fn name_errors_list_allowed_names() {
    let names = Names {
        paragraph_styles: Some(vec!["Normal".into(), "Note".into(), "Body Text".into()]),
        table_styles: Some(vec!["Grid Table 4".into()]),
        fields: Some(vec!["작성자".into()]),
        keeps: Some(vec![Keep { id: "k1".into(), kind: "drawing".into(), summary: "logo".into() }]),
        formats: Some(vec!["docx".into()]),
        ..Default::default()
    };
    let e = errors_with("<div style=\"note\">x</div>\n", &names);
    assert_eq!(e, vec!["line 7, column 6: style=\"note\" is not a paragraph style of this file; names are case-sensitive: \"Note\". The value is exactly one style name, spaces included. Allowed paragraph styles: \"Normal\", \"Note\", \"Body Text\"."]);
    let e = errors_with("<div style=\"BodyText\">x</div>\n", &names);
    assert!(e[0].contains("did you mean \"Body Text\"?"), "{e:?}");
    let e = errors_with("<div style=\"color: red\">x</div>\n", &names);
    assert!(e[0].contains("is direct formatting"), "{e:?}");
    let e = errors_with("<div style=\"Grid Table 4\">x</div>\n", &names);
    assert!(e[0].contains("is a table style; a paragraph takes a paragraph style"), "{e:?}");
    let e = errors_with("{style=\"Grid\"}\n| a |\n|---|\n", &names);
    assert!(e[0].contains("Allowed table styles: \"Grid Table 4\""), "{e:?}");
    let e = errors_with("<field name=\"날짜\">x</field>\n", &names);
    assert!(e[0].contains("Fields: \"작성자\""), "{e:?}");
    let e = errors_with("<keep id=\"k2\" kind=\"drawing\" summary=\"logo\"/>\n", &names);
    assert!(e[0].contains("never create one"), "{e:?}");
    let e = errors_with("<keep id=\"k1\" kind=\"drawing\" summary=\"changed\"/>\n", &names);
    assert!(e[0].contains("was altered"), "{e:?}");
    let e = errors_with(
        "<keep id=\"k1\" kind=\"drawing\" summary=\"logo\"/> <keep id=\"k1\" kind=\"drawing\" summary=\"logo\"/>\n",
        &names,
    );
    assert!(e[0].contains("appears twice"), "{e:?}");
    assert!(errors_with(
        "<keep id=\"k1\" kind=\"drawing\" summary=\"logo\"/>\n\n<div style=\"Body Text\">x</div>\n",
        &names
    )
    .is_empty());
}

// ---------------------------------------------------------------- §5.2 bullets (DESIGN.md at 6a3530b)

#[test]
fn spans_three_columns_and_two_by_two_merges() {
    // `|||` spans three columns; `|| ||` is a || and then another cell.
    let d = parse(&doc("| a | b | c | d |\n|---|---|---|---|\n| 합계 ||| 215 |\n")).unwrap();
    let Block::Table(t) = &d.blocks[0] else { panic!() };
    assert_eq!(t.rows[1][..3], [Cell::text(Inline::plain("합계")), Cell::Left, Cell::Left]);
    // 2×2: text + || in the first row, ^^ || below.
    let two = doc("| 합산 || 10 |\n|---|---|---|\n| ^^ || 20 |\n");
    let d = parse(&two).unwrap();
    let Block::Table(t) = &d.blocks[0] else { panic!() };
    assert_eq!(t.rows[1][..2], [Cell::Up, Cell::Left]);
    assert_eq!(serialize(&d), two);
}

#[test]
fn empty_paragraph_lines() {
    // consecutive <p/> lines stay consecutive; a blank line separates the group
    let text = doc("앞 문단\n\n<p/>\n<p/>\n<p style=\"좁은 간격\"/>\n\n뒤 문단\n");
    assert_eq!(roundtrip(&text), text);
    let d = parse(&text).unwrap();
    assert_eq!(d.blocks.len(), 5);
    assert_eq!(d.blocks[1], Block::Para(Para { style: ParaStyle::Plain, content: Inline::default() }));
    assert_eq!(
        d.blocks[3],
        Block::Para(Para { style: ParaStyle::Named("좁은 간격".into()), content: Inline::default() })
    );
    // blank lines between them are dropped in canonical form; a <br/> line is text, not an empty paragraph
    let loose = parse(&doc("<p/>\n\n<p/>\n\n<br/>\n")).unwrap();
    assert_eq!(serialize(&loose), doc("<p/>\n<p/>\n\n<br/>\n"));
    assert!(
        matches!(&loose.blocks[2], Block::Para(p) if p.content.units == vec![Unit { atom: Atom::Break, marks: Marks::NONE }])
    );
}

#[test]
fn p_tag_errors() {
    for (body, want) in [
        ("<p></p>\n", "single tags that start a paragraph: no </p>"),
        ("</p>\n", "no </p>"),
        ("<p class=\"x\"/>\n", "has no attribute \"class\""),
        ("<div style=\"Note\"></div>\n", "an empty paragraph is <p style=\"Note\"/>, not an empty <div>"),
        ("text<p/>more\n", "<p/> inside a line starts a paragraph only in a table cell"),
        ("| a |\n|---|\n| <div style=\"Note\">x</div> |\n", "cannot contain another <div>"),
        ("| a |\n|---|\n| **a<p/>b** |\n", "close bold with ** before <p/>"),
        ("| a |\n|---|\n| x<p></p> |\n", "no </p>"),
    ] {
        let e = errors(body);
        assert!(e.iter().any(|m| m.contains(want)), "{body:?}: {e:?}");
    }
    let names = Names { paragraph_styles: Some(vec!["Normal".into(), "표 참고".into()]), ..Default::default() };
    let e = errors_with("<p style=\"표참고\"/>\n", &names);
    assert!(e[0].contains("did you mean \"표 참고\"?"), "{e:?}");
    let e = errors_with("| a |\n|---|\n| x<p style=\"Nope\"/>y |\n", &names);
    assert!(e[0].contains("Allowed paragraph styles"), "{e:?}");
}

#[test]
fn multi_paragraph_cells() {
    let cp = |style: Option<&str>, t: &str| CellPara { style: style.map(str::to_string), content: Inline::plain(t) };
    let cell = |src: &str| -> Vec<CellPara> {
        let d = parse(&doc(&format!("| h |\n|---|\n| {src} |\n"))).unwrap();
        let Block::Table(t) = &d.blocks[0] else { panic!() };
        let Cell::Text(ps) = &t.rows[1][0] else { panic!() };
        ps.clone()
    };
    assert_eq!(cell("해운대 1곳<p/>서면 1곳"), vec![cp(None, "해운대 1곳"), cp(None, "서면 1곳")]);
    assert_eq!(
        cell("812<p/>부산 신규 2곳<p style=\"표 참고\"/>잠정치"),
        vec![cp(None, "812"), cp(None, "부산 신규 2곳"), cp(Some("표 참고"), "잠정치")]
    );
    // a tag written first starts the first paragraph itself
    assert_eq!(cell("<p style=\"표 참고\"/>a"), vec![cp(Some("표 참고"), "a")]);
    assert_eq!(cell("<p/>a"), vec![cp(None, "a")]);
    // <p/><p/> gives an empty paragraph; a trailing <p/> ends with one
    assert_eq!(cell("a<p/><p/>b"), vec![cp(None, "a"), cp(None, ""), cp(None, "b")]);
    assert_eq!(cell("a<p/>"), vec![cp(None, "a"), cp(None, "")]);
    assert_eq!(cell("<p/><p/>b"), vec![cp(None, ""), cp(None, "b")]);
    // <br/> is a line break inside one paragraph
    assert_eq!(cell("a<br/>b").len(), 1);
    // canonical form drops a leading plain <p/>, keeps the others
    let text = doc("| h |\n|---|\n| <p/>a<p/>b |\n");
    assert_eq!(serialize(&parse(&text).unwrap()), doc("| h |\n|---|\n| a<p/>b |\n"));
    for canon in [
        "| a<p/><p/>b |",
        "| <p/><p/>b |",
        "| a<p/> |",
        "| <p style=\"표 참고\"/> |",
        "| <p/><p/> |",
        "| 지방 | 812 | +4%<p/>부산 신규 2곳<p style=\"표 참고\"/>잠정치 |",
    ] {
        let width = canon.matches(" | ").count() + 1;
        let t = doc(&format!("{}\n|{}\n{canon}\n", "| h ".repeat(width) + "|", "---|".repeat(width)));
        assert_eq!(roundtrip(&t), t, "{canon}");
    }
    // the source map gives each cell paragraph its mark: the next <p/> or the closing pipe
    let t = doc("| h |\n|---|\n| a<p/>b |\n");
    let p = parse_with(&t, &Names::default()).unwrap();
    let BlockMapKind::Table(cells) = &p.map.blocks[0].kind else { panic!() };
    let ms = cells[1][0].as_ref().unwrap();
    assert_eq!(&t[ms[0].mark..ms[0].mark + 4], "<p/>");
    assert_eq!(&t[ms[1].mark..ms[1].mark + 1], "|");
    assert_eq!(&t[ms[1].units[0]..ms[1].units[0] + 1], "b");
}

#[test]
fn table_style_line_sits_on_the_header_row() {
    let ok = doc("{style=\"Grid Table 4\"}\n| a |\n|---|\n");
    assert_eq!(roundtrip(&ok), ok);
    let e = errors("{style=\"Grid Table 4\"}\n\n| a |\n|---|\n");
    assert!(e[0].contains("directly followed by the header row"), "{e:?}");
    // Trailing whitespace on the style line is not part of it.
    assert_eq!(roundtrip(&doc("{style=\"Grid Table 4\"}  \n| a |\n|---|\n")), ok);
}

#[test]
fn spaces_only_paragraphs_name_their_style() {
    // A plain line cannot hold spaces alone, so the style is named and the
    // spaces stay text; the serializer never invents a style name for it.
    let named = doc("<div style=\"Note\">   </div>\n");
    assert_eq!(roundtrip(&named), named);
    assert_eq!(roundtrip(&doc("|    |\n|---|\n")), doc("|    |\n|---|\n"));
    let mut d = parse(&doc("x\n")).unwrap();
    let Block::Para(p) = &mut d.blocks[0] else { panic!() };
    p.content = Inline::plain("   ");
    let out = serialize(&d);
    assert!(!out.contains("Normal"), "{out}");
    assert_eq!(out, doc("<p/>\n"));
    d.normalize();
    assert_eq!(parse(&out).unwrap(), d, "canonical form and the serializer agree");
}

#[test]
fn lists_nest_by_content_column_and_write_1_everywhere() {
    let text = doc("- a\n  - b\n    1. c\n       - d\n  - e\n- f\n\n7. g\n10. h\n    1. i\n");
    let d = parse(&text).unwrap();
    let Block::List(items) = &d.blocks[0] else { panic!("{:?}", d.blocks) };
    let got: Vec<(bool, usize, String)> = items.iter().map(|i| (i.ordered, i.level, i.content.text())).collect();
    let want = [(false, 0, "a"), (false, 1, "b"), (true, 2, "c"), (false, 3, "d"), (false, 1, "e"), (false, 0, "f")];
    assert_eq!(got, want.map(|(o, l, t)| (o, l, t.to_string())));
    // A blank line ends a list; `10. ` has content column 4, canonical `1. ` has 3.
    let Block::List(items) = &d.blocks[1] else { panic!() };
    assert_eq!(items.iter().map(|i| i.level).collect::<Vec<_>>(), vec![0, 0, 1]);
    assert_eq!(roundtrip(&text), doc("- a\n  - b\n    1. c\n       - d\n  - e\n- f\n\n1. g\n1. h\n   1. i\n"));
    // Empty items, inline markup and <br/> inside items.
    let t = doc("-\n- **bold** <br/>next\n1.\n");
    assert_eq!(roundtrip(&t), t);
}

#[test]
fn list_errors_say_where_to_indent() {
    let e = errors("  - a\n");
    assert!(e[0].starts_with("line 7, column 1: a list starts at the left margin"), "{e:?}");
    let e = errors("- a\n   - b\n");
    assert!(e[0].contains("indented 3 spaces") && e[0].contains("indented 0, 2 spaces"), "{e:?}");
    let e = errors("1. a\n  - b\n");
    assert!(e[0].contains("indented 0, 3 spaces"), "{e:?}");
    let e = errors("- a\n\t- b\n");
    assert!(e[0].contains("with spaces, not tabs"), "{e:?}");
}

#[test]
fn text_that_looks_like_a_list_is_escaped() {
    for line in ["- a", "-", "1. a", "1.", "2023. It was", "  - a", "  3. b", "+ a", "1) a"] {
        let mut d = parse(&doc("x\n")).unwrap();
        let Block::Para(p) = &mut d.blocks[0] else { panic!() };
        p.content = Inline::plain(line);
        let s = serialize(&d);
        let back = parse(&s).unwrap_or_else(|e| panic!("{line:?}: {}\n{s}", diag::render(&e)));
        assert_eq!(back, d, "{line:?} → {s}");
    }
}

#[test]
fn math_before_a_digit_uses_the_math_tag() {
    // `$5 and $10` is text: a closing `$` needs a non-space before it.
    let d = parse(&doc("costs $5 and $10\n")).unwrap();
    let Block::Para(p) = &d.blocks[0] else { panic!() };
    assert!(p.content.units.iter().all(|u| matches!(u.atom, Atom::Char(_))), "{:?}", p.content);
    // Math followed by a digit cannot be `$x$5` (that is text): it is <math>.
    let t = doc("<math>x</math>5 and $y$ and <math> padded </math> and <math>p$q</math>\n");
    let d = parse(&t).unwrap();
    let Block::Para(p) = &d.blocks[0] else { panic!() };
    let math: Vec<&str> = p
        .content
        .units
        .iter()
        .filter_map(|u| if let Atom::Math(m) = &u.atom { Some(m.as_str()) } else { None })
        .collect();
    assert_eq!(math, ["x", "y", " padded ", "p$q"]);
    assert_eq!(roundtrip(&t), t, "the tag is written only where $…$ would not read back");
    assert_eq!(roundtrip(&doc("<math>z</math> w\n")), doc("$z$ w\n"));
    let e = errors("<math>x\n");
    assert!(e[0].contains("<math> is not closed by </math>"), "{e:?}");
    let e = errors("<math></math>\n");
    assert!(e[0].contains("is empty"), "{e:?}");
}

#[test]
fn br_is_a_line_break_in_paragraphs_headings_cells_and_items() {
    let t = doc("# a<br/>b\n\nc<br/>d\n\n| e<br/>f |\n|---|\n\n- g<br/>h\n");
    let d = parse(&t).unwrap();
    let breaks = |i: &Inline| i.units.iter().filter(|u| u.atom == Atom::Break).count();
    let mut n = 0;
    for b in &d.blocks {
        n += match b {
            Block::Para(p) => breaks(&p.content),
            Block::Table(t) => {
                t.rows[0].iter().map(|c| if let Cell::Text(ps) = c { breaks(&ps[0].content) } else { 0 }).sum()
            }
            Block::List(items) => breaks(&items[0].content),
            _ => 0,
        };
    }
    assert_eq!((d.blocks.len(), n), (4, 4));
    assert_eq!(roundtrip(&t), t);
    // A `<br/>` line is a paragraph holding a line break, never an empty paragraph.
    let d = parse(&doc("<br/>\n")).unwrap();
    assert!(
        matches!(&d.blocks[0], Block::Para(p) if p.content.units.len() == 1 && p.content.units[0].atom == Atom::Break)
    );
}

#[test]
fn the_design_md_example_parses() {
    // DESIGN.md §5.2's example, read from the spec itself so the two cannot drift.
    let spec = include_str!("../../../DESIGN.md");
    let start = spec.find("### 5.2 Document").unwrap();
    let body = &spec[start..];
    let a = body.find("```\n").unwrap() + 4;
    let b = a + body[a..].find("```\n").unwrap();
    let d = parse(&body[a..b]).unwrap_or_else(|e| panic!("{}", diag::render(&e)));
    let lists: Vec<&Vec<Item>> =
        d.blocks.iter().filter_map(|b| if let Block::List(l) = b { Some(l) } else { None }).collect();
    assert_eq!(lists.len(), 1);
    let got: Vec<(bool, usize)> = lists[0].iter().map(|i| (i.ordered, i.level)).collect();
    assert_eq!(got, [(false, 0), (false, 1), (true, 0)]);
    let again = serialize(&d);
    assert_eq!(parse(&again).unwrap(), d);
}
