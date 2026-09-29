//! A small lossless XML tree over quick-xml: names, attribute values and
//! text are kept as written (escaped), so untouched fragments serialize back
//! to the same bytes. Canonical form (for GetPut and fingerprints) follows
//! the prototype: namespaces resolved, attributes sorted, inter-element
//! whitespace and comments dropped.

use std::collections::HashMap;

use quick_xml::events::Event;
use quick_xml::Reader;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    El(Element),
    /// Raw (escaped) character data.
    Text(String),
    Comment(String),
    Pi(String),
    CData(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Element {
    pub name: String,
    /// `(qualified name, raw escaped value)` in source order.
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
}

#[derive(Debug)]
pub struct XmlError(pub String);

impl std::fmt::Display for XmlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A parsed part: text before the root (XML declaration), the root, text after.
#[derive(Clone, Debug)]
pub struct Doc {
    pub prolog: String,
    pub root: Element,
    pub epilog: String,
}

impl Element {
    pub fn new(name: &str) -> Element {
        Element { name: name.into(), ..Default::default() }
    }
    pub fn with_attr(mut self, k: &str, v: &str) -> Element {
        self.attrs.push((k.into(), escape_attr(v)));
        self
    }
    pub fn local(&self) -> &str {
        self.name.rsplit(':').next().unwrap()
    }
    pub fn is(&self, qname: &str) -> bool {
        self.name == qname
    }
    /// Raw attribute value.
    pub fn attr(&self, k: &str) -> Option<&str> {
        self.attrs.iter().find(|a| a.0 == k).map(|a| a.1.as_str())
    }
    /// Decoded attribute value.
    pub fn get(&self, k: &str) -> Option<String> {
        self.attr(k).map(unescape)
    }
    pub fn set(&mut self, k: &str, v: &str) {
        let v = escape_attr(v);
        match self.attrs.iter_mut().find(|a| a.0 == k) {
            Some(a) => a.1 = v,
            None => self.attrs.push((k.into(), v)),
        }
    }
    pub fn remove_attr(&mut self, k: &str) -> bool {
        let n = self.attrs.len();
        self.attrs.retain(|a| a.0 != k);
        n != self.attrs.len()
    }
    pub fn elements(&self) -> impl Iterator<Item = &Element> {
        self.children.iter().filter_map(|n| match n {
            Node::El(e) => Some(e),
            _ => None,
        })
    }
    pub fn elements_mut(&mut self) -> impl Iterator<Item = &mut Element> {
        self.children.iter_mut().filter_map(|n| match n {
            Node::El(e) => Some(e),
            _ => None,
        })
    }
    pub fn child(&self, qname: &str) -> Option<&Element> {
        self.elements().find(|e| e.name == qname)
    }
    pub fn child_mut(&mut self, qname: &str) -> Option<&mut Element> {
        self.elements_mut().find(|e| e.name == qname)
    }
    pub fn has_elements(&self) -> bool {
        self.elements().next().is_some()
    }
    /// A copy with no children (the element's "shell").
    pub fn shell(&self) -> Element {
        Element { name: self.name.clone(), attrs: self.attrs.clone(), children: vec![] }
    }
    /// Pre-order walk over all descendant elements (self included).
    pub fn walk<'a>(&'a self, f: &mut dyn FnMut(&'a Element)) {
        f(self);
        for c in self.elements() {
            c.walk(f);
        }
    }
    /// [`Element::walk`] with mutable access.
    pub fn walk_mut(&mut self, f: &mut dyn FnMut(&mut Element)) {
        f(self);
        for c in self.elements_mut() {
            c.walk_mut(f);
        }
    }
    pub fn descendants(&self, qname: &str) -> Vec<&Element> {
        let mut out = vec![];
        self.walk(&mut |e| {
            if e.name == qname {
                out.push(e)
            }
        });
        out
    }
    /// Concatenated decoded text of `w:t`-like descendants.
    pub fn text_of(&self, names: &[&str]) -> String {
        let mut s = String::new();
        self.walk(&mut |e| {
            if names.contains(&e.name.as_str()) {
                for n in &e.children {
                    if let Node::Text(t) = n {
                        s.push_str(&unescape(t));
                    }
                }
            }
        });
        s
    }
    pub fn to_xml(&self) -> String {
        let mut s = String::new();
        write_element(self, &mut s);
        s
    }
}

pub fn parse(data: &[u8]) -> Result<Doc, XmlError> {
    let text = std::str::from_utf8(data).map_err(|_| XmlError("the part is not UTF-8".into()))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut r = Reader::from_str(text);
    r.config_mut().check_end_names = true;
    let mut stack: Vec<Element> = vec![];
    let mut root = None;
    let mut root_start = None;
    let mut root_end = 0;
    loop {
        let before = r.buffer_position() as usize;
        let ev = r.read_event().map_err(|e| XmlError(format!("malformed XML at byte {before}: {e}")))?;
        let node = match ev {
            Event::Eof => break,
            Event::DocType(_) => return Err(XmlError("a DOCTYPE is not allowed in a package part".into())),
            Event::Decl(_) => continue,
            Event::Start(_) | Event::Empty(_) if stack.is_empty() && root.is_some() => {
                return Err(XmlError("more than one root element".into()));
            }
            Event::Start(s) => {
                root_start.get_or_insert(before);
                stack.push(start(&s)?);
                continue;
            }
            Event::Empty(s) => {
                root_start.get_or_insert(before);
                Node::El(start(&s)?)
            }
            Event::End(_) => {
                let el = stack.pop().ok_or_else(|| XmlError("unmatched end tag".into()))?;
                Node::El(el)
            }
            Event::Text(t) => Node::Text(t.to_string()),
            Event::GeneralRef(g) => Node::Text(format!("&{};", &*g)),
            Event::CData(c) => Node::CData(c.to_string()),
            Event::Comment(c) => Node::Comment(c.to_string()),
            Event::PI(p) => Node::Pi(p.to_string()),
        };
        match stack.last_mut() {
            Some(parent) => push_merged(&mut parent.children, node),
            None => {
                if let Node::El(e) = node {
                    root = Some(e);
                    root_end = r.buffer_position() as usize;
                }
            }
        }
    }
    if !stack.is_empty() {
        return Err(XmlError("unclosed element".into()));
    }
    let root = root.ok_or_else(|| XmlError("no root element".into()))?;
    let start = root_start.unwrap_or(0);
    Ok(Doc { prolog: text[..start].to_string(), root, epilog: text[root_end..].to_string() })
}

/// Parse one element written by [`Element::to_xml`].
pub fn fragment(s: &str) -> Element {
    parse(s.as_bytes()).expect("stored fragment is well-formed").root
}

fn push_merged(children: &mut Vec<Node>, node: Node) {
    if let (Node::Text(t), Some(Node::Text(prev))) = (&node, children.last_mut()) {
        prev.push_str(t);
        return;
    }
    children.push(node);
}

fn start(s: &quick_xml::events::BytesStart<'_>) -> Result<Element, XmlError> {
    let name = s.name().as_ref().to_string();
    let mut attrs = vec![];
    for a in s.attributes() {
        let a = a.map_err(|e| XmlError(format!("bad attribute in <{name}>: {e}")))?;
        let mut v = a.value.to_string();
        if v.contains('"') {
            v = v.replace('"', "&quot;");
        }
        attrs.push((a.key.as_ref().to_string(), v));
    }
    Ok(Element { name, attrs, children: vec![] })
}

pub fn write_element(e: &Element, out: &mut String) {
    out.push('<');
    out.push_str(&e.name);
    for (k, v) in &e.attrs {
        out.push(' ');
        out.push_str(k);
        out.push_str("=\"");
        out.push_str(v);
        out.push('"');
    }
    if e.children.is_empty() {
        out.push_str("/>");
        return;
    }
    out.push('>');
    for c in &e.children {
        write_node(c, out);
    }
    out.push_str("</");
    out.push_str(&e.name);
    out.push('>');
}

fn write_node(n: &Node, out: &mut String) {
    let (open, t, close) = match n {
        Node::El(x) => return write_element(x, out),
        Node::Text(t) => return out.push_str(t),
        Node::Comment(t) => ("<!--", t, "-->"),
        Node::Pi(t) => ("<?", t, "?>"),
        Node::CData(t) => ("<![CDATA[", t, "]]>"),
    };
    out.push_str(open);
    out.push_str(t);
    out.push_str(close);
}

pub fn write_doc(d: &Doc) -> Vec<u8> {
    let mut s = d.prolog.clone();
    write_element(&d.root, &mut s);
    s.push_str(&d.epilog);
    s.into_bytes()
}

pub fn unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(k) = rest.find('&') {
        out.push_str(&rest[..k]);
        rest = &rest[k..];
        let Some(end) = rest.find(';') else {
            out.push_str(rest);
            return out;
        };
        let ent = &rest[1..end];
        let ch = match ent {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ if ent.starts_with("#x") || ent.starts_with("#X") => {
                u32::from_str_radix(&ent[2..], 16).ok().and_then(char::from_u32)
            }
            _ if ent.starts_with('#') => ent[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        match ch {
            Some(c) => out.push(c),
            None => out.push_str(&rest[..=end]),
        }
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

pub fn escape_text(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

pub fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('"', "&quot;")
}

// ---------------------------------------------------------------- canonical form

/// In-scope namespace bindings.
pub type Scope = HashMap<String, String>;

pub fn scope_of(root: &Element) -> Scope {
    let mut s = Scope::new();
    push_scope(&mut s, root);
    s
}

/// The prefix an attribute declares (`""` for `xmlns`), if it is a namespace declaration.
pub fn ns_prefix(attr: &str) -> Option<&str> {
    if attr == "xmlns" {
        Some("")
    } else {
        attr.strip_prefix("xmlns:")
    }
}

fn push_scope(scope: &mut Scope, e: &Element) {
    for (k, v) in &e.attrs {
        if let Some(p) = ns_prefix(k) {
            scope.insert(p.to_string(), unescape(v));
        }
    }
}

fn expand(scope: &Scope, qname: &str, is_attr: bool) -> String {
    match qname.split_once(':') {
        Some(("xml", l)) => format!("{{http://www.w3.org/XML/1998/namespace}}{l}"),
        Some((p, l)) => format!("{{{}}}{l}", scope.get(p).map_or("?", String::as_str)),
        None if is_attr => qname.to_string(),
        None => match scope.get("") {
            Some(u) => format!("{{{u}}}{qname}"),
            None => qname.to_string(),
        },
    }
}

const KEEP_TEXT: [&str; 4] = ["t", "instrText", "delText", "delInstrText"];

/// Canonical text of an element in `scope`.
pub fn canon(e: &Element, scope: &Scope) -> String {
    let mut out = String::new();
    canon_into(e, scope, &mut out);
    out
}

fn canon_into(e: &Element, scope: &Scope, out: &mut String) {
    let own = e.attrs.iter().any(|a| ns_prefix(&a.0).is_some());
    let mut local_scope;
    let scope = if own {
        local_scope = scope.clone();
        push_scope(&mut local_scope, e);
        &local_scope
    } else {
        scope
    };
    out.push('<');
    out.push_str(&expand(scope, &e.name, false));
    let mut attrs: Vec<(String, String)> = e
        .attrs
        .iter()
        .filter(|a| ns_prefix(&a.0).is_none())
        .map(|(k, v)| (expand(scope, k, true), unescape(v).replace(['\t', '\n', '\r'], " ")))
        .collect();
    attrs.sort();
    for (k, v) in attrs {
        out.push(' ');
        out.push_str(&k);
        out.push_str("=\"");
        out.push_str(&escape_attr(&v));
        out.push('"');
    }
    out.push('>');
    let drop_ws = e.has_elements() && !KEEP_TEXT.contains(&e.local());
    let mut text = String::new();
    let flush = |text: &mut String, out: &mut String| {
        if !(text.is_empty() || drop_ws && text.trim().is_empty()) {
            out.push_str(&escape_text(text).replace('\r', "&#xD;"));
        }
        text.clear();
    };
    for c in &e.children {
        match c {
            Node::Text(t) => text.push_str(&unescape(t)),
            Node::CData(t) => text.push_str(t),
            Node::Comment(_) => {}
            Node::Pi(p) => {
                flush(&mut text, out);
                out.push_str(&format!("<?{p}?>"));
            }
            Node::El(x) => {
                flush(&mut text, out);
                canon_into(x, scope, out);
            }
        }
    }
    flush(&mut text, out);
    out.push_str("</>");
}

/// Canonical bytes of a whole part.
pub fn canon_part(data: &[u8]) -> Result<String, XmlError> {
    let d = parse(data)?;
    Ok(canon(&d.root, &Scope::new()))
}

/// Serialize a sequence of nodes.
pub fn write_nodes(nodes: &[Node]) -> String {
    let mut s = String::new();
    for n in nodes {
        write_node(n, &mut s);
    }
    s
}

// ---------------------------------------------------------------- tree edits and fingerprints

/// Insert `el` among `parent`'s children following `order` (by local name).
pub fn insert_ordered(parent: &mut Element, el: Element, order: &[&str]) {
    let rank = |e: &Element| order.iter().position(|x| *x == e.local());
    let mine = rank(&el).unwrap_or(order.len());
    let at = parent.children.iter().position(|n| matches!(n, Node::El(c) if rank(c).is_some_and(|r| r > mine)));
    match at {
        Some(k) => parent.children.insert(k, Node::El(el)),
        None => parent.children.push(Node::El(el)),
    }
}

/// Remove the first child element named `qname`.
pub fn remove_child(e: &mut Element, qname: &str) -> Option<Element> {
    let k = e.children.iter().position(|n| matches!(n, Node::El(c) if c.name == qname))?;
    match e.children.remove(k) {
        Node::El(x) => Some(x),
        _ => None,
    }
}

/// Fingerprint of fragments (canonical, joined; `-` for none).
pub fn fp(scope: &Scope, els: &[Option<&Element>]) -> String {
    els.iter().map(|e| e.map_or_else(|| "-".to_string(), |e| canon(e, scope))).collect::<Vec<_>>().join("|")
}

/// Whitespace-only character data (between elements).
pub fn is_blank(n: &Node) -> bool {
    matches!(n, Node::Text(t) if unescape(t).trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lossless_roundtrip() {
        let src = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\r\n<w:document xmlns:w=\"urn:w\" a='x\"y'><w:t xml:space=\"preserve\"> a &amp; b &#x41;</w:t><!-- c --><w:e/></w:document>";
        let d = parse(src.as_bytes()).unwrap();
        let out = String::from_utf8(write_doc(&d)).unwrap();
        assert_eq!(out, src.replace("a='x\"y'", "a=\"x&quot;y\""));
        assert_eq!(d.root.child("w:t").unwrap().text_of(&["w:t"]), " a & b A");
    }

    #[test]
    fn canonical_form_ignores_prefix_and_attribute_order() {
        let a = parse(b"<a:r xmlns:a=\"u\" a:x=\"1\" a:y=\"2\">\n  <a:t>x</a:t>\n</a:r>").unwrap();
        let b = parse(b"<b:r xmlns:b=\"u\" b:y=\"2\" b:x=\"1\"><b:t>x</b:t></b:r>").unwrap();
        assert_eq!(canon(&a.root, &Scope::new()), canon(&b.root, &Scope::new()));
        assert!(parse(b"<!DOCTYPE x><x/>").is_err());
    }
}
