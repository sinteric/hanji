//! The shared string table (`sharedStrings.xml`): read by streaming, so a
//! large one costs its strings and no tree; new strings are appended to the
//! original bytes, which are otherwise copied as they are.

use std::collections::HashMap;

use quick_xml::events::Event;
use quick_xml::Reader;

use hanji_package::xml;

#[derive(Clone, Debug, Default)]
pub struct Sst {
    pub part: String,
    pub strings: Vec<String>,
    /// How many strings the part held at load; the rest were added.
    loaded: usize,
    lookup: Option<HashMap<String, u32>>,
    /// Change in the number of cells referring to the table (the `count` attribute).
    pub refs_delta: i64,
}

/// `_xHHHH_` escapes (ST_Xstring) decoded.
pub fn decode_xstring(s: &str) -> String {
    if !s.contains("_x") {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(k) = rest.find("_x") {
        out.push_str(&rest[..k]);
        let tail = &rest[k..];
        let hex = tail.get(2..6).filter(|h| h.bytes().all(|b| b.is_ascii_hexdigit()));
        match (hex, tail.get(6..7)) {
            (Some(h), Some("_")) => {
                if let Some(c) = u32::from_str_radix(h, 16).ok().and_then(char::from_u32) {
                    out.push(c);
                }
                rest = &tail[7..];
            }
            _ => {
                out.push_str("_x");
                rest = &tail[2..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// A string as a `<t>` holds it: control characters and a literal `_xHHHH_` escaped, then XML-escaped.
pub fn encode_xstring(s: &str) -> String {
    xml::escape_text(&encode_xstring_raw(s))
}

/// [`encode_xstring`] without the XML escaping (for an attribute value set through the tree).
pub fn encode_xstring_raw(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let b: Vec<char> = s.chars().collect();
    for (k, &c) in b.iter().enumerate() {
        let literal_escape = c == '_'
            && b.get(k + 1) == Some(&'x')
            && b.len() >= k + 7
            && b[k + 2..k + 6].iter().all(|h| h.is_ascii_hexdigit())
            && b[k + 6] == '_';
        if literal_escape {
            out.push_str("_x005F_");
        } else if (c as u32) < 0x20 && !matches!(c, '\t' | '\n' | '\r') {
            out.push_str(&format!("_x{:04X}_", c as u32));
        } else {
            out.push(c);
        }
    }
    out
}

/// A `<t>` element for `s`, with `xml:space="preserve"` when its ends or runs of spaces need it.
pub fn t_element(prefix: &str, s: &str) -> String {
    let keep =
        s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) || s.contains("  ") || s.contains('\n');
    let sp = if keep { " xml:space=\"preserve\"" } else { "" };
    format!("<{prefix}t{sp}>{}</{prefix}t>", encode_xstring(s))
}

/// The text of a `<si>` or `<is>`: its `t` runs, phonetic runs (`rPh`) left out.
pub fn rich_text(e: &xml::Element) -> String {
    let mut s = String::new();
    fn walk(e: &xml::Element, s: &mut String) {
        for c in e.elements() {
            match c.local() {
                "rPh" | "phoneticPr" => {}
                "t" => {
                    for n in &c.children {
                        match n {
                            xml::Node::Text(t) => s.push_str(&xml::unescape(t)),
                            xml::Node::CData(t) => s.push_str(t),
                            _ => {}
                        }
                    }
                }
                _ => walk(c, s),
            }
        }
    }
    walk(e, &mut s);
    decode_xstring(&s)
}

impl Sst {
    pub fn parse(part: &str, data: &[u8]) -> Result<Sst, String> {
        let text = std::str::from_utf8(data).map_err(|_| format!("{part}: not UTF-8"))?;
        let mut r = Reader::from_str(text);
        let mut strings = vec![];
        let (mut in_si, mut in_t, mut rph) = (false, false, 0);
        let mut cur = String::new();
        loop {
            match r.read_event().map_err(|e| format!("{part}: {e}"))? {
                Event::Eof => break,
                Event::Start(s) => match xml::local_name(s.name().as_ref()) {
                    "si" => {
                        in_si = true;
                        cur.clear();
                    }
                    "rPh" => rph += 1,
                    "t" if in_si && rph == 0 => in_t = true,
                    _ => {}
                },
                Event::Empty(s) if xml::local_name(s.name().as_ref()) == "si" => strings.push(String::new()),
                Event::End(e) => match xml::local_name(e.name().as_ref()) {
                    "si" => {
                        in_si = false;
                        strings.push(decode_xstring(&std::mem::take(&mut cur)));
                    }
                    "rPh" => rph -= 1,
                    "t" => in_t = false,
                    _ => {}
                },
                Event::Text(t) if in_t => cur.push_str(&xml::unescape(&t)),
                Event::GeneralRef(g) if in_t => cur.push_str(&xml::unescape(&format!("&{};", &*g))),
                Event::CData(c) if in_t => cur.push_str(&c),
                _ => {}
            }
        }
        let loaded = strings.len();
        Ok(Sst { part: part.to_string(), strings, loaded, lookup: None, refs_delta: 0 })
    }

    /// An empty table to be written as a new part.
    pub fn new(part: &str) -> Sst {
        Sst { part: part.into(), ..Default::default() }
    }

    pub fn get(&self, i: usize) -> Option<&str> {
        self.strings.get(i).map(String::as_str)
    }

    /// The index of `s`, appended when the table does not hold it.
    pub fn intern(&mut self, s: &str) -> u32 {
        let strings = &self.strings;
        let lookup = self.lookup.get_or_insert_with(|| {
            let mut m = HashMap::with_capacity(strings.len());
            for (k, x) in strings.iter().enumerate() {
                m.entry(x.clone()).or_insert(k as u32);
            }
            m
        });
        if let Some(&k) = lookup.get(s) {
            return k;
        }
        let k = self.strings.len() as u32;
        self.strings.push(s.to_string());
        lookup.insert(s.to_string(), k);
        k
    }

    pub fn changed(&self) -> bool {
        self.strings.len() != self.loaded || self.refs_delta != 0
    }

    /// The part's bytes: `data` (the part as loaded; `None` for a new one)
    /// with the added strings appended and its counts updated.
    pub fn write(&self, data: Option<&[u8]>) -> Vec<u8> {
        let added: String =
            self.strings[self.loaded..].iter().map(|s| format!("<si>{}</si>", t_element("", s))).collect();
        let Some(data) = data else {
            let n = self.strings.len();
            return format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<sst xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" count=\"{}\" uniqueCount=\"{n}\">{added}</sst>",
                self.refs_delta.max(n as i64)
            )
            .into_bytes();
        };
        let text = String::from_utf8_lossy(data).into_owned();
        let Ok(d) = xml::parse(data) else { return data.to_vec() };
        let prefix = d.root.name.strip_suffix("sst").unwrap_or("").to_string();
        let added = if prefix.is_empty() {
            added
        } else {
            added
                .replace("<si>", &format!("<{prefix}si>"))
                .replace("</si>", &format!("</{prefix}si>"))
                .replace("<t", &format!("<{prefix}t"))
                .replace("</t>", &format!("</{prefix}t>"))
        };
        let mut root = d.root.shell();
        if root.attr("uniqueCount").is_some() {
            root.set("uniqueCount", &self.strings.len().to_string());
        }
        if let Some(c) = root.get("count").and_then(|c| c.parse::<i64>().ok()) {
            root.set("count", &(c + self.refs_delta).max(0).to_string());
        }
        // Splice: the new start tag, the old content, the new strings, the end tag.
        let open_at = text.find(&format!("<{}", d.root.name)).unwrap_or(0);
        let open_end = text[open_at..].find('>').map_or(text.len(), |j| open_at + j + 1);
        let self_closed = text[..open_end].ends_with("/>");
        let close = format!("</{}>", d.root.name);
        let (body, after) = if self_closed {
            ("", &text[open_end..])
        } else {
            let k = text.rfind(&close).unwrap_or(text.len());
            (&text[open_end..k], &text[(k + close.len()).min(text.len())..])
        };
        let mut out = text[..open_at].to_string();
        out.push_str(&root.open_tag());
        out.push_str(body);
        out.push_str(&added);
        out.push_str(&close);
        out.push_str(after);
        out.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_intern_write() {
        let src = "<?xml version=\"1.0\"?>\n<sst xmlns=\"x\" count=\"3\" uniqueCount=\"2\"><si><t>a &amp; b</t></si><si><r><t>매</t></r><r><t xml:space=\"preserve\">출 </t></r><rPh><t>x</t></rPh></si></sst>";
        let mut s = Sst::parse("sst.xml", src.as_bytes()).unwrap();
        assert_eq!(s.strings, ["a & b", "매출 "]);
        assert_eq!(s.intern("매출 "), 1);
        assert_eq!(s.intern(" new"), 2);
        s.refs_delta = 1;
        let out = String::from_utf8(s.write(Some(src.as_bytes()))).unwrap();
        assert!(out.contains("count=\"4\" uniqueCount=\"3\""), "{out}");
        assert!(out.ends_with("<si><t xml:space=\"preserve\"> new</t></si></sst>"), "{out}");
        let again = Sst::parse("sst.xml", out.as_bytes()).unwrap();
        assert_eq!(again.strings, ["a & b", "매출 ", " new"]);
        assert_eq!(decode_xstring("a_x000D_b_x005F_x0041_"), "a\rb_x0041_");
        assert_eq!(decode_xstring(&xml::unescape(&encode_xstring("a\u{1}b_x0041_c"))), "a\u{1}b_x0041_c");
    }
}
