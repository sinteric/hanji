//! A worksheet part as the engine holds it (§2 rule 11): the part without
//! its rows as a small XML tree (the skeleton), and the rows as byte ranges
//! of the part as loaded, parsed only when a window reads them or an
//! operation changes them. An untouched row is written back as the bytes it
//! was read from, so a 100k-row sheet costs its bytes plus an index.

use hanji_core::cells::{col_index, col_letters};
use hanji_package::xml::{self, Element, Node};
use quick_xml::events::Event;
use quick_xml::Reader;

/// Where the rows go in the skeleton.
const ROWS_MARK: &str = "hanji-rows";

#[derive(Clone, Debug)]
pub struct Store {
    /// The part as loaded: the bytes raw rows point into.
    raw: std::sync::Arc<Vec<u8>>,
    /// The worksheet without its rows; `sheetData` holds one marker.
    pub skel: xml::Doc,
    /// `""` or the prefix SpreadsheetML elements use (`x:`).
    pub prefix: String,
    /// Rows by row number, ascending.
    pub rows: Vec<RowSlot>,
    /// The part wrote `<sheetData/>`.
    self_closed: bool,
}

/// Rows taken out of a sheet ([`Store::take_tail`]), parsed as they are read.
pub struct Tail {
    slots: std::vec::IntoIter<RowSlot>,
    raw: std::sync::Arc<Vec<u8>>,
}

impl Iterator for Tail {
    type Item = Result<Row, String>;
    fn next(&mut self) -> Option<Self::Item> {
        Some(match self.slots.next()? {
            RowSlot::Parsed(row) => Ok(row),
            RowSlot::Raw(raw) => Row::from_bytes(&self.raw[raw.start as usize..raw.end as usize], raw.r),
        })
    }
}

#[derive(Clone, Debug)]
pub enum RowSlot {
    Raw(RawRow),
    Parsed(Row),
}

#[derive(Clone, Copy, Debug)]
pub struct RawRow {
    pub r: u32,
    pub start: u32,
    pub end: u32,
    /// The first and last column holding a cell.
    pub cols: Option<(u32, u32)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row {
    pub r: u32,
    /// The row's attributes as written, `r` included.
    pub attrs: Vec<(String, String)>,
    pub cells: Vec<Cell>,
    /// Children that are not cells (`extLst`), after the cells.
    pub extra: Vec<Node>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cell {
    pub col: u32,
    /// The cell's attributes as written (`r`, `s`, `t`, `cm`, …).
    pub attrs: Vec<(String, String)>,
    pub f: Option<Element>,
    /// `<v>` content, raw (escaped).
    pub v: Option<String>,
    /// `<is>` (an inline string).
    pub is: Option<Element>,
    pub extra: Vec<Node>,
}

fn attr<'a>(attrs: &'a [(String, String)], k: &str) -> Option<&'a str> {
    attrs.iter().find(|a| a.0 == k).map(|a| a.1.as_str())
}

fn set_attr(attrs: &mut Vec<(String, String)>, k: &str, v: &str, first: bool) {
    let v = xml::escape_attr(v);
    match attrs.iter_mut().find(|a| a.0 == k) {
        Some(a) => a.1 = v,
        None if first => attrs.insert(0, (k.into(), v)),
        None => attrs.push((k.into(), v)),
    }
}

impl Cell {
    pub fn new(col: u32, row: u32) -> Cell {
        Cell { col, attrs: vec![("r".into(), format!("{}{row}", col_letters(col)))], ..Default::default() }
    }
    pub fn get(&self, k: &str) -> Option<&str> {
        attr(&self.attrs, k)
    }
    pub fn set(&mut self, k: &str, v: &str) {
        set_attr(&mut self.attrs, k, v, false);
    }
    pub fn remove(&mut self, k: &str) {
        self.attrs.retain(|a| a.0 != k);
    }
    /// The cell style index (`s`), 0 when unset.
    pub fn style(&self) -> u32 {
        self.get("s").and_then(|v| v.parse().ok()).unwrap_or(0)
    }
    /// The value type (`t`), `n` when unset.
    pub fn ty(&self) -> &str {
        self.get("t").unwrap_or("n")
    }
    /// The formula text (decoded), if the cell has one of its own.
    pub fn formula(&self) -> Option<String> {
        self.f.as_ref().map(|f| f.text_of(&[f.name.as_str()]))
    }
    pub fn set_row(&mut self, row: u32) {
        set_attr(&mut self.attrs, "r", &format!("{}{row}", col_letters(self.col)), true);
    }
    /// Whether the cell holds nothing but its address and style.
    pub fn is_blank(&self) -> bool {
        self.f.is_none() && self.v.is_none() && self.is.is_none()
    }
    pub fn clear_value(&mut self) {
        self.f = None;
        self.v = None;
        self.is = None;
        self.remove("t");
        self.remove("cm");
        self.remove("vm");
    }

    pub fn write(&self, p: &str, out: &mut String) {
        out.push('<');
        out.push_str(p);
        out.push('c');
        for (k, v) in &self.attrs {
            out.push(' ');
            out.push_str(k);
            out.push_str("=\"");
            out.push_str(v);
            out.push('"');
        }
        if self.f.is_none() && self.v.is_none() && self.is.is_none() && self.extra.is_empty() {
            out.push_str("/>");
            return;
        }
        out.push('>');
        if let Some(f) = &self.f {
            xml::write_element(f, out);
        }
        if let Some(v) = &self.v {
            out.push_str(&format!("<{p}v>{v}</{p}v>"));
        }
        if let Some(is) = &self.is {
            xml::write_element(is, out);
        }
        out.push_str(&xml::write_nodes(&self.extra));
        out.push_str(&format!("</{p}c>"));
    }
}

impl Row {
    pub fn new(r: u32) -> Row {
        Row { r, attrs: vec![("r".into(), r.to_string())], ..Default::default() }
    }
    pub fn get(&self, k: &str) -> Option<&str> {
        attr(&self.attrs, k)
    }
    pub fn set(&mut self, k: &str, v: &str) {
        set_attr(&mut self.attrs, k, v, false);
    }
    pub fn cell(&self, col: u32) -> Option<&Cell> {
        self.cells.binary_search_by_key(&col, |c| c.col).ok().map(|k| &self.cells[k])
    }
    /// The cell at `col`, made when missing (with the row's own style, if it has one).
    pub fn cell_mut(&mut self, col: u32) -> &mut Cell {
        match self.cells.binary_search_by_key(&col, |c| c.col) {
            Ok(k) => &mut self.cells[k],
            Err(k) => {
                let mut c = Cell::new(col, self.r);
                if self.get("customFormat") == Some("1") {
                    if let Some(s) = self.get("s") {
                        c.set("s", s);
                    }
                }
                self.cells.insert(k, c);
                &mut self.cells[k]
            }
        }
    }
    pub fn set_row(&mut self, r: u32) {
        self.r = r;
        set_attr(&mut self.attrs, "r", &r.to_string(), true);
        for c in &mut self.cells {
            c.set_row(r);
        }
    }
    pub fn cols(&self) -> Option<(u32, u32)> {
        Some((self.cells.first()?.col, self.cells.last()?.col))
    }
    /// Refresh `spans` (a hint readers may use) to the cells the row holds.
    pub fn fix_spans(&mut self) {
        if self.get("spans").is_some() {
            match self.cols() {
                Some((a, b)) => {
                    let old = self.get("spans").unwrap().to_string();
                    let within = old
                        .split_once(':')
                        .and_then(|(x, y)| Some((x.parse::<u32>().ok()?, y.parse::<u32>().ok()?)))
                        .is_some_and(|(x, y)| x <= a + 1 && b < y + 1);
                    if !within {
                        self.set("spans", &format!("{}:{}", a + 1, b + 1));
                    }
                }
                None => self.attrs.retain(|x| x.0 != "spans"),
            }
        }
    }

    pub fn write(&self, p: &str, out: &mut String) {
        out.push('<');
        out.push_str(p);
        out.push_str("row");
        for (k, v) in &self.attrs {
            out.push(' ');
            out.push_str(k);
            out.push_str("=\"");
            out.push_str(v);
            out.push('"');
        }
        if self.cells.is_empty() && self.extra.is_empty() {
            out.push_str("/>");
            return;
        }
        out.push('>');
        for c in &self.cells {
            c.write(p, out);
        }
        out.push_str(&xml::write_nodes(&self.extra));
        out.push_str(&format!("</{p}row>"));
    }

    /// A row read from its bytes, as [`Row::from_element`] reads its parse,
    /// without building the element tree (the common cell, `<c …><v>…</v></c>`,
    /// costs its attributes and its value; `<f>`, `<is>` and anything else
    /// are parsed as elements).
    pub fn from_bytes(bytes: &[u8], implied: u32) -> Result<Row, String> {
        let text = std::str::from_utf8(bytes).map_err(|_| "a row is not UTF-8".to_string())?;
        let mut rd = Reader::from_str(text);
        rd.config_mut().check_end_names = true;
        let bad = |e: &dyn std::fmt::Display| format!("row {implied}: {e}");
        let attrs_of = |s: &quick_xml::events::BytesStart<'_>| -> Result<Vec<(String, String)>, String> {
            let mut out = vec![];
            for a in s.attributes() {
                let a = a.map_err(|e| bad(&e))?;
                let mut v = a.value.to_string();
                if v.contains('"') {
                    v = v.replace('"', "&quot;");
                }
                out.push((a.key.as_ref().to_string(), v));
            }
            Ok(out)
        };
        // The element starting at `from` whose start tag was just read (`empty`: `<x/>`), parsed whole.
        let element = |rd: &mut Reader<&[u8]>, from: usize, empty: bool| -> Result<Element, String> {
            if !empty {
                let mut depth = 1;
                while depth > 0 {
                    match rd.read_event().map_err(|e| bad(&e))? {
                        Event::Start(_) => depth += 1,
                        Event::End(_) => depth -= 1,
                        Event::Eof => return Err(bad(&"unclosed element")),
                        _ => {}
                    }
                }
            }
            let to = rd.buffer_position() as usize;
            xml::parse(&bytes[from..to]).map(|d| d.root).map_err(|e| bad(&e))
        };
        let mut row: Option<Row> = None;
        let mut next_col = 0;
        loop {
            let before = rd.buffer_position() as usize;
            let ev = rd.read_event().map_err(|e| bad(&e))?;
            let (s, empty) = match ev {
                Event::Eof => break,
                Event::End(_) => continue,
                Event::Start(s) => (s, false),
                Event::Empty(s) => (s, true),
                _ => continue,
            };
            let Some(row) = row.as_mut() else {
                let mut attrs = attrs_of(&s)?;
                let r = attr(&attrs, "r").and_then(|v| v.parse().ok()).unwrap_or(implied);
                if attr(&attrs, "r").is_none() {
                    attrs.insert(0, ("r".into(), r.to_string()));
                }
                row = Some(Row { r, attrs, cells: vec![], extra: vec![] });
                if empty {
                    break;
                }
                continue;
            };
            let r = row.r;
            let name = s.name();
            if local(name.as_ref()) != "c" {
                row.extra.push(Node::El(element(&mut rd, before, empty)?));
                continue;
            }
            let attrs = attrs_of(&s)?;
            let col = match attr(&attrs, "r") {
                Some(a) => {
                    let k = a.find(|ch: char| !ch.is_ascii_alphabetic()).unwrap_or(a.len());
                    col_index(&a[..k]).ok_or_else(|| format!("row {r}: bad cell address {a:?}"))?
                }
                None => next_col,
            };
            next_col = col + 1;
            let mut cell = Cell { col, attrs, ..Default::default() };
            if cell.get("r").is_none() {
                cell.set_row(r);
            }
            if !empty {
                loop {
                    let at = rd.buffer_position() as usize;
                    let (x, x_empty) = match rd.read_event().map_err(|e| bad(&e))? {
                        Event::End(_) => break,
                        Event::Eof => return Err(bad(&"unclosed cell")),
                        Event::Start(x) => (x, false),
                        Event::Empty(x) => (x, true),
                        _ => continue,
                    };
                    let xn = x.name();
                    let xl = local(xn.as_ref()).to_string();
                    if xl == "v" && x.attributes().next().is_none() {
                        let from = rd.buffer_position() as usize;
                        let mut to = from;
                        if !x_empty {
                            loop {
                                to = rd.buffer_position() as usize;
                                match rd.read_event().map_err(|e| bad(&e))? {
                                    Event::End(_) => break,
                                    Event::Text(_) | Event::GeneralRef(_) => {}
                                    Event::Eof => return Err(bad(&"unclosed value")),
                                    // Markup inside a value: read it as an element.
                                    _ => {
                                        let mut e = element(&mut rd, at, false)?;
                                        cell.v = Some(xml::write_nodes(&std::mem::take(&mut e.children)));
                                        to = usize::MAX;
                                        break;
                                    }
                                }
                            }
                        }
                        if to != usize::MAX {
                            cell.v = Some(text[from..to].to_string());
                        }
                        continue;
                    }
                    let e = element(&mut rd, at, x_empty)?;
                    match xl.as_str() {
                        "f" => cell.f = Some(e),
                        "v" => cell.v = Some(xml::write_nodes(&e.children)),
                        "is" => cell.is = Some(e),
                        _ => cell.extra.push(Node::El(e)),
                    }
                }
            }
            if row.cells.last().is_some_and(|l| l.col >= col) {
                return Err(format!("row {r}: cells are not in column order"));
            }
            row.cells.push(cell);
        }
        row.ok_or_else(|| bad(&"no row element"))
    }

    /// A row read from its element.
    pub fn from_element(e: &Element, implied: u32) -> Result<Row, String> {
        let r = e.get("r").and_then(|v| v.parse().ok()).unwrap_or(implied);
        let mut attrs = e.attrs.clone();
        if attr(&attrs, "r").is_none() {
            attrs.insert(0, ("r".into(), r.to_string()));
        }
        let mut row = Row { r, attrs, cells: vec![], extra: vec![] };
        let mut next_col = 0;
        for n in &e.children {
            match n {
                Node::El(c) if c.local() == "c" => {
                    let col = match c.get("r") {
                        Some(a) => {
                            let k = a.find(|ch: char| !ch.is_ascii_alphabetic()).unwrap_or(a.len());
                            col_index(&a[..k]).ok_or_else(|| format!("row {r}: bad cell address {a:?}"))?
                        }
                        None => next_col,
                    };
                    next_col = col + 1;
                    let mut cell = Cell { col, attrs: c.attrs.clone(), ..Default::default() };
                    if cell.get("r").is_none() {
                        cell.set_row(r);
                    }
                    for x in &c.children {
                        match x {
                            Node::El(y) if y.local() == "f" => cell.f = Some(y.clone()),
                            Node::El(y) if y.local() == "v" => cell.v = Some(xml::write_nodes(&y.children)),
                            Node::El(y) if y.local() == "is" => cell.is = Some(y.clone()),
                            Node::El(_) => cell.extra.push(x.clone()),
                            _ => {}
                        }
                    }
                    if row.cells.last().is_some_and(|l| l.col >= col) {
                        return Err(format!("row {r}: cells are not in column order"));
                    }
                    row.cells.push(cell);
                }
                Node::El(_) => row.extra.push(n.clone()),
                _ => {}
            }
        }
        Ok(row)
    }
}

fn local(name: &str) -> &str {
    match name.rfind(':') {
        Some(k) => &name[k + 1..],
        None => name,
    }
}

impl Store {
    /// Index a worksheet part: find its rows and build the skeleton.
    pub fn load(data: Vec<u8>) -> Result<Store, String> {
        let text = std::str::from_utf8(&data).map_err(|_| "the part is not UTF-8".to_string())?;
        let mut r = Reader::from_str(text);
        let mut depth = 0usize;
        let mut data_depth = None;
        let (mut sd_start, mut sd_inner, mut sd_end, mut self_closed) = (None, 0, 0, false);
        let mut rows: Vec<RawRow> = vec![];
        let mut row_start = 0;
        let mut row_r = 0u32;
        let mut cols: Option<(u32, u32)> = None;
        let mut implicit = false;
        let mut next_col = 0u32;
        loop {
            let before = r.buffer_position() as usize;
            let ev = r.read_event().map_err(|e| format!("malformed XML at byte {before}: {e}"))?;
            match ev {
                Event::Eof => break,
                Event::Start(s) => {
                    let name = s.name();
                    let l = local(name.as_ref());
                    if data_depth.is_none() && l == "sheetData" && depth == 1 {
                        sd_start = Some(before);
                        sd_inner = r.buffer_position() as usize;
                        data_depth = Some(depth);
                    } else if data_depth.is_some_and(|d| depth == d + 1) && l == "row" {
                        row_start = before;
                        let prev = rows.last().map_or(0, |x| x.r);
                        row_r = match s.try_get_attribute("r").ok().flatten() {
                            Some(a) => a.value.parse().ok().ok_or("a row has a bad r")?,
                            None => {
                                implicit = true;
                                prev + 1
                            }
                        };
                        cols = None;
                        next_col = 0;
                    } else if data_depth.is_some_and(|d| depth == d + 2) && l == "c" {
                        let col = cell_col(&s, &mut implicit, next_col)?;
                        next_col = col + 1;
                        cols = Some(cols.map_or((col, col), |(a, b)| (a.min(col), b.max(col))));
                    }
                    depth += 1;
                }
                Event::Empty(s) => {
                    let name = s.name();
                    let l = local(name.as_ref());
                    if data_depth.is_none() && l == "sheetData" && depth == 1 {
                        sd_start = Some(before);
                        sd_inner = r.buffer_position() as usize;
                        sd_end = sd_inner;
                        self_closed = true;
                    } else if data_depth.is_some_and(|d| depth == d + 1) && l == "row" {
                        let prev = rows.last().map_or(0, |x| x.r);
                        let rr = match s.try_get_attribute("r").ok().flatten() {
                            Some(a) => a.value.parse().ok().ok_or("a row has a bad r")?,
                            None => {
                                implicit = true;
                                prev + 1
                            }
                        };
                        rows.push(RawRow { r: rr, start: before as u32, end: r.buffer_position() as u32, cols: None });
                    } else if data_depth.is_some_and(|d| depth == d + 2) && l == "c" {
                        let col = cell_col(&s, &mut implicit, next_col)?;
                        next_col = col + 1;
                        cols = Some(cols.map_or((col, col), |(a, b)| (a.min(col), b.max(col))));
                    }
                }
                Event::End(_) => {
                    depth -= 1;
                    if data_depth == Some(depth) {
                        sd_end = before;
                        data_depth = None;
                    } else if data_depth.is_some_and(|d| depth == d + 1) {
                        rows.push(RawRow { r: row_r, start: row_start as u32, end: r.buffer_position() as u32, cols });
                    }
                }
                _ => {}
            }
        }
        let sd_start = sd_start.ok_or("the worksheet has no sheetData")?;
        if rows.windows(2).any(|w| w[0].r >= w[1].r) {
            implicit = true;
        }
        // The skeleton: the part with its rows replaced by one marker.
        let mut sk = String::with_capacity(sd_inner + (data.len() - sd_end) + 32);
        if self_closed {
            let open = &text[sd_start..sd_inner];
            sk.push_str(&text[..sd_start]);
            sk.push_str(&open[..open.len() - 2]);
            sk.push('>');
            sk.push_str(&format!("<?{ROWS_MARK}?>"));
            let name = open[1..].split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("sheetData");
            sk.push_str(&format!("</{name}>"));
            sk.push_str(&text[sd_inner..]);
        } else {
            sk.push_str(&text[..sd_inner]);
            sk.push_str(&format!("<?{ROWS_MARK}?>"));
            sk.push_str(&text[sd_end..]);
        }
        let skel = xml::parse(sk.as_bytes()).map_err(|e| e.to_string())?;
        let prefix = skel.root.name.strip_suffix("worksheet").unwrap_or("").to_string();
        let raw = std::sync::Arc::new(data);
        let mut st = Store { raw, skel, prefix, rows: rows.into_iter().map(RowSlot::Raw).collect(), self_closed };
        if implicit {
            // Rows or cells without addresses, or out of order: give every row its address.
            let mut parsed = vec![];
            let mut prev = 0;
            for k in 0..st.rows.len() {
                let mut row = st.parse_slot(k, prev + 1)?;
                if row.r <= prev {
                    return Err(format!("row {} comes after row {prev}", row.r));
                }
                row.set_row(row.r);
                prev = row.r;
                parsed.push(RowSlot::Parsed(row));
            }
            st.rows = parsed;
        }
        Ok(st)
    }

    fn parse_slot(&self, k: usize, implied: u32) -> Result<Row, String> {
        match &self.rows[k] {
            RowSlot::Parsed(r) => Ok(r.clone()),
            RowSlot::Raw(raw) => Row::from_bytes(&self.raw[raw.start as usize..raw.end as usize], implied),
        }
    }

    pub fn row_numbers(&self) -> impl Iterator<Item = u32> + '_ {
        self.rows.iter().map(|s| match s {
            RowSlot::Raw(r) => r.r,
            RowSlot::Parsed(r) => r.r,
        })
    }

    pub fn slot_row(s: &RowSlot) -> u32 {
        match s {
            RowSlot::Raw(r) => r.r,
            RowSlot::Parsed(r) => r.r,
        }
    }

    pub fn find(&self, r: u32) -> Result<usize, usize> {
        self.rows.binary_search_by_key(&r, Store::slot_row)
    }

    /// Row `r`, parsed (a copy for a raw row); `None` when the sheet has no such row.
    pub fn row(&self, r: u32) -> Option<Row> {
        let k = self.find(r).ok()?;
        self.parse_slot(k, r).ok()
    }

    /// Row `r`, parsed in place, made when missing.
    pub fn row_mut(&mut self, r: u32) -> &mut Row {
        let k = match self.find(r) {
            Ok(k) => {
                if matches!(self.rows[k], RowSlot::Raw(_)) {
                    let row = self.parse_slot(k, r).expect("an indexed row parses");
                    self.rows[k] = RowSlot::Parsed(row);
                }
                k
            }
            Err(k) => {
                self.rows.insert(k, RowSlot::Parsed(Row::new(r)));
                k
            }
        };
        match &mut self.rows[k] {
            RowSlot::Parsed(row) => row,
            RowSlot::Raw(_) => unreachable!(),
        }
    }

    /// Edit many rows in one pass: each of `rows` (ascending; a missing one is
    /// created) goes through `f` and is written back to bytes at once, so the
    /// sheet never holds them all parsed. Untouched rows keep their bytes.
    pub fn edit_rows(&mut self, rows: &[u32], f: &mut dyn FnMut(&mut Row)) {
        if rows.len() < 64 {
            for &r in rows {
                f(self.row_mut(r));
            }
            return;
        }
        for &r in rows {
            if self.find(r).is_err() {
                self.row_mut(r);
            }
        }
        let mut buf: Vec<u8> = Vec::with_capacity(self.raw.len() + rows.len() * 16);
        let mut text = String::new();
        let mut want = rows.iter().copied().peekable();
        let old = std::mem::take(&mut self.rows);
        let mut slots = Vec::with_capacity(old.len());
        for slot in old {
            let r = Store::slot_row(&slot);
            while want.next_if(|&w| w < r).is_some() {}
            let edit = want.next_if_eq(&r).is_some();
            match slot {
                RowSlot::Parsed(mut row) => {
                    if edit {
                        f(&mut row);
                    }
                    slots.push(RowSlot::Parsed(row));
                }
                RowSlot::Raw(raw) => {
                    let start = buf.len() as u32;
                    let bytes = &self.raw[raw.start as usize..raw.end as usize];
                    let mut cols = raw.cols;
                    if edit {
                        let mut row = Row::from_bytes(bytes, r).expect("an indexed row parses");
                        f(&mut row);
                        text.clear();
                        row.write(&self.prefix, &mut text);
                        buf.extend_from_slice(text.as_bytes());
                        cols = row.cols();
                    } else {
                        buf.extend_from_slice(bytes);
                    }
                    slots.push(RowSlot::Raw(RawRow { r, start, end: buf.len() as u32, cols }));
                }
            }
        }
        self.rows = slots;
        self.raw = std::sync::Arc::new(buf);
    }

    /// Rows `a..=b` that exist, parsed (copies).
    /// Rows `a..=b`, parsed one at a time (a raw row is not kept parsed).
    pub fn for_each_row_in(&self, a: u32, b: u32, f: &mut dyn FnMut(&Row)) {
        let from = self.find(a).unwrap_or_else(|k| k);
        for k in from..self.rows.len() {
            match &self.rows[k] {
                RowSlot::Parsed(row) if row.r > b => break,
                RowSlot::Parsed(row) => f(row),
                RowSlot::Raw(raw) if raw.r > b => break,
                RowSlot::Raw(raw) => {
                    if let Ok(row) = self.parse_slot(k, raw.r) {
                        f(&row);
                    }
                }
            }
        }
    }

    pub fn rows_in(&self, a: u32, b: u32) -> Vec<Row> {
        let from = self.find(a).unwrap_or_else(|k| k);
        let mut out = vec![];
        for k in from..self.rows.len() {
            let r = Store::slot_row(&self.rows[k]);
            if r > b {
                break;
            }
            if let Ok(row) = self.parse_slot(k, r) {
                out.push(row);
            }
        }
        out
    }

    /// Take rows `a..=b` out of the sheet, parsed.
    pub fn take_rows(&mut self, a: u32, b: u32) -> Vec<Row> {
        let from = self.find(a).unwrap_or_else(|k| k);
        let to = match self.find(b) {
            Ok(k) => k + 1,
            Err(k) => k,
        };
        let out: Vec<Row> = (from..to)
            .map(|k| self.parse_slot(k, Store::slot_row(&self.rows[k])).expect("an indexed row parses"))
            .collect();
        self.rows.drain(from..to);
        out
    }

    /// Rows `from` on, taken out to be read one at a time and written back
    /// through [`Store::push_row`] (so a shift never holds them all parsed).
    pub fn take_tail(&mut self, from: u32) -> Tail {
        let k = self.find(from).unwrap_or_else(|k| k);
        Tail { slots: self.rows.split_off(k).into_iter(), raw: self.raw.clone() }
    }

    /// Append a row after every row the sheet holds, as bytes.
    pub fn push_row(&mut self, row: &Row) {
        debug_assert!(self.rows.last().is_none_or(|s| Store::slot_row(s) < row.r));
        let mut text = String::new();
        row.write(&self.prefix, &mut text);
        let raw = std::sync::Arc::make_mut(&mut self.raw);
        let start = raw.len() as u32;
        raw.extend_from_slice(text.as_bytes());
        self.rows.push(RowSlot::Raw(RawRow { r: row.r, start, end: raw.len() as u32, cols: row.cols() }));
    }

    /// Put parsed rows back (their numbers set; none may collide with a row the sheet holds).
    pub fn put_rows(&mut self, rows: Vec<Row>) {
        for row in rows {
            match self.find(row.r) {
                Ok(k) => self.rows[k] = RowSlot::Parsed(row),
                Err(k) => self.rows.insert(k, RowSlot::Parsed(row)),
            }
        }
    }

    /// Every row from `from` down, parsed in place (before rows move).
    pub fn parse_from(&mut self, from: u32) {
        let start = self.find(from).unwrap_or_else(|k| k);
        for k in start..self.rows.len() {
            if let RowSlot::Raw(raw) = &self.rows[k] {
                let r = raw.r;
                let row = self.parse_slot(k, r).expect("an indexed row parses");
                self.rows[k] = RowSlot::Parsed(row);
            }
        }
    }

    /// The first and last row and column holding a cell.
    pub fn used(&self) -> Option<(u32, u32, u32, u32)> {
        let mut out: Option<(u32, u32, u32, u32)> = None;
        for s in &self.rows {
            let (r, cols) = match s {
                RowSlot::Raw(x) => (x.r, x.cols),
                RowSlot::Parsed(x) => (x.r, x.cols()),
            };
            if let Some((a, b)) = cols {
                out = Some(match out {
                    None => (a, r, b, r),
                    Some((c0, r0, c1, _)) => (c0.min(a), r0, c1.max(b), r),
                });
            }
        }
        out
    }

    /// Visit every cell (parsing rows as needed, without keeping them parsed).
    pub fn for_each_cell(&self, f: &mut dyn FnMut(u32, &Cell)) {
        self.for_each_cell_if(&|_| true, f)
    }

    /// [`Store::for_each_cell`] over the rows whose bytes pass `pre` (a raw
    /// row is parsed only then; a parsed row always passes).
    pub fn for_each_cell_if(&self, pre: &dyn Fn(&[u8]) -> bool, f: &mut dyn FnMut(u32, &Cell)) {
        for k in 0..self.rows.len() {
            match &self.rows[k] {
                RowSlot::Parsed(row) => row.cells.iter().for_each(|c| f(row.r, c)),
                RowSlot::Raw(raw) => {
                    if !pre(&self.raw[raw.start as usize..raw.end as usize]) {
                        continue;
                    }
                    if let Ok(row) = self.parse_slot(k, raw.r) {
                        row.cells.iter().for_each(|c| f(row.r, c));
                    }
                }
            }
        }
    }

    /// Whether any row is parsed (the sheet may have changed).
    pub fn is_touched(&self) -> bool {
        self.rows.iter().any(|s| matches!(s, RowSlot::Parsed(_)))
    }

    /// The part's bytes.
    pub fn write(&self) -> Vec<u8> {
        let sk = String::from_utf8(xml::write_doc(&self.skel)).unwrap();
        let mark = format!("<?{ROWS_MARK}?>");
        let k = sk.find(&mark).expect("the skeleton keeps its row marker");
        if self.rows.is_empty() && self.self_closed && sk[..k].ends_with('>') {
            let close = sk[k + mark.len()..].find('>').map_or(0, |j| j + 1);
            return format!("{}/>{}", &sk[..k - 1], &sk[k + mark.len() + close..]).into_bytes();
        }
        let mut out = String::with_capacity(self.raw.len() + 1024);
        out.push_str(&sk[..k]);
        let mut buf = String::new();
        for s in &self.rows {
            match s {
                RowSlot::Raw(r) => {
                    out.push_str(std::str::from_utf8(&self.raw[r.start as usize..r.end as usize]).unwrap())
                }
                RowSlot::Parsed(row) => {
                    buf.clear();
                    row.write(&self.prefix, &mut buf);
                    out.push_str(&buf);
                }
            }
        }
        out.push_str(&sk[k + mark.len()..]);
        out.into_bytes()
    }

    /// The skeleton's top-level element named `local`.
    pub fn child(&self, local: &str) -> Option<&Element> {
        self.skel.root.elements().find(|e| e.local() == local)
    }

    pub fn child_mut(&mut self, local: &str) -> Option<&mut Element> {
        self.skel.root.elements_mut().find(|e| e.local() == local)
    }
}

fn cell_col(s: &quick_xml::events::BytesStart<'_>, implicit: &mut bool, next: u32) -> Result<u32, String> {
    match s.try_get_attribute("r").ok().flatten() {
        Some(a) => {
            let v: &str = &a.value;
            let k = v.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(v.len());
            col_index(&v[..k]).ok_or_else(|| format!("bad cell address {v:?}"))
        }
        None => {
            *implicit = true;
            Ok(next)
        }
    }
}

/// Whether `hay` holds `needle`.
pub fn holds(hay: &[u8], needle: &[u8]) -> bool {
    memchr::memmem::find(hay, needle).is_some()
}

/// A row pre-check: the row holds a formula (`<f>` or `<f …`, any prefix).
pub fn has_formula(row: &[u8]) -> bool {
    holds(row, b"f>") || holds(row, b"f ")
}

/// Worksheet children in schema order (CT_Worksheet).
pub const SHEET_ORDER: &[&str] = &[
    "sheetPr",
    "dimension",
    "sheetViews",
    "sheetFormatPr",
    "cols",
    "sheetData",
    "sheetCalcPr",
    "sheetProtection",
    "protectedRanges",
    "scenarios",
    "autoFilter",
    "sortState",
    "dataConsolidate",
    "customSheetViews",
    "mergeCells",
    "phoneticPr",
    "conditionalFormatting",
    "dataValidations",
    "hyperlinks",
    "printOptions",
    "pageMargins",
    "pageSetup",
    "headerFooter",
    "rowBreaks",
    "colBreaks",
    "customProperties",
    "cellWatches",
    "ignoredErrors",
    "smartTags",
    "drawing",
    "legacyDrawing",
    "legacyDrawingHF",
    "drawingHF",
    "picture",
    "oleObjects",
    "controls",
    "webPublishItems",
    "tableParts",
    "extLst",
];

#[cfg(test)]
mod tests {
    use super::*;

    const SHEET: &str = "<?xml version=\"1.0\"?>\n<worksheet xmlns=\"m\"><dimension ref=\"A1:B3\"/><sheetData><row r=\"1\" spans=\"1:2\"><c r=\"A1\" t=\"s\"><v>0</v></c><c r=\"B1\" s=\"2\"><v>5</v></c></row><row r=\"3\"><c r=\"B3\"><f>B1*2</f><v>10</v></c></row></sheetData><pageMargins left=\"0.7\"/></worksheet>";

    #[test]
    fn untouched_rows_come_back_as_they_were() {
        let st = Store::load(SHEET.as_bytes().to_vec()).unwrap();
        assert_eq!(st.rows.len(), 2);
        assert_eq!(st.used(), Some((0, 1, 1, 3)));
        assert_eq!(String::from_utf8(st.write()).unwrap(), SHEET);
        let r3 = st.row(3).unwrap();
        assert_eq!(r3.cells[0].formula().as_deref(), Some("B1*2"));
    }

    #[test]
    fn changed_rows_are_written_from_their_parse() {
        let mut st = Store::load(SHEET.as_bytes().to_vec()).unwrap();
        let row = st.row_mut(2);
        let c = row.cell_mut(0);
        c.v = Some("7".into());
        st.row_mut(1).cell_mut(1).v = Some("6".into());
        let out = String::from_utf8(st.write()).unwrap();
        assert!(out.contains("<row r=\"1\" spans=\"1:2\"><c r=\"A1\" t=\"s\"><v>0</v></c><c r=\"B1\" s=\"2\"><v>6</v></c></row><row r=\"2\"><c r=\"A2\"><v>7</v></c></row><row r=\"3\">"), "{out}");
        let again = Store::load(out.into_bytes()).unwrap();
        assert_eq!(again.rows.len(), 3);
    }

    #[test]
    fn rows_read_from_bytes_as_from_their_parse() {
        let tricky = [
            "<row r=\"4\" spans=\"1:3\" x14ac:dyDescent=\"0.25\"><c r=\"A4\" t=\"s\"><v>0</v></c><c r=\"B4\"><f t=\"shared\" si=\"0\"/><v>1.5</v></c><c r=\"C4\" t=\"inlineStr\"><is><t xml:space=\"preserve\"> a &amp; b </t></is></c></row>",
            "<x:row><x:c><x:v>&lt;1&gt;</x:v></x:c><x:c t=\"str\"><x:f>\"a\"&amp;\"b\"</x:f><x:v></x:v></x:c><x:c/><x:extLst><x:ext uri=\"u\"/></x:extLst></x:row>",
            "<row r=\"2\"/>",
            "<row r=\"9\"><c r=\"A9\"><v><![CDATA[7]]></v></c><c r=\"B9\"><!-- note --><v>8</v></c></row>",
        ];
        for t in tricky {
            let a = Row::from_bytes(t.as_bytes(), 3).unwrap();
            let b = Row::from_element(&xml::parse(t.as_bytes()).unwrap().root, 3).unwrap();
            assert_eq!(a, b, "{t}");
        }
        // Every row of every corpus worksheet.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
        let mut n = 0;
        for f in std::fs::read_dir(dir).unwrap() {
            let p = f.unwrap().path();
            if !p.extension().is_some_and(|e| e == "xlsx" || e == "xlsm") {
                continue;
            }
            let parts = hanji_package::package::read(&std::fs::read(&p).unwrap()).unwrap();
            for part in parts.iter().filter(|x| x.name.starts_with("xl/worksheets/sheet")) {
                let Ok(st) = Store::load(part.data.clone()) else { continue };
                for slot in &st.rows {
                    if let RowSlot::Raw(raw) = slot {
                        let bytes = &st.raw[raw.start as usize..raw.end as usize];
                        let a = Row::from_bytes(bytes, raw.r).unwrap();
                        let b = Row::from_element(&xml::parse(bytes).unwrap().root, raw.r).unwrap();
                        assert_eq!(a, b, "{}: {}", p.display(), String::from_utf8_lossy(bytes));
                        n += 1;
                    }
                }
            }
        }
        assert!(n > 1000, "{n} rows");
    }

    #[test]
    fn rows_without_addresses_get_them() {
        let src = "<worksheet xmlns=\"m\"><sheetData><row><c><v>1</v></c><c><v>2</v></c></row><row><c r=\"C2\"><v>3</v></c></row></sheetData></worksheet>";
        let st = Store::load(src.as_bytes().to_vec()).unwrap();
        let out = String::from_utf8(st.write()).unwrap();
        assert!(
            out.contains(
                "<row r=\"1\"><c r=\"A1\"><v>1</v></c><c r=\"B1\"><v>2</v></c></row><row r=\"2\"><c r=\"C2\">"
            ),
            "{out}"
        );
        let e = "<worksheet><sheetData/></worksheet>";
        let st = Store::load(e.as_bytes().to_vec()).unwrap();
        assert!(st.rows.is_empty());
        assert_eq!(String::from_utf8(st.write()).unwrap(), e);
    }
}
