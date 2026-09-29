//! Accept All / Reject All of one author's tracked changes, on
//! `word/document.xml` (the test oracle for §10.2; rdocx and LibreOffice
//! cross-check it outside the workspace). Other authors' revisions stay as
//! they are, so rejecting gives back the imported file's own revisions.
//!
//! The rules are ECMA-376's: `w:ins` content stays on accept and goes on
//! reject, `w:del` the other way round (`w:delText` becomes `w:t`); a
//! removed paragraph mark joins its paragraph's content to the next
//! paragraph, whose properties stay; `w:pPrChange` / `w:rPrChange` put the
//! stored properties back on reject; a row marked in `w:trPr` goes with it.

use hanji_docx::package;
use hanji_docx::xml::{self, Element, Node};

fn ours(e: &Element, author: &str) -> bool {
    e.get("w:author").as_deref() == Some(author)
}

fn has_mark(p: &Element, name: &str, author: &str) -> bool {
    p.child("w:pPr").and_then(|x| x.child("w:rPr")).and_then(|r| r.child(name)).is_some_and(|m| ours(m, author))
}

fn drop_mark(p: &mut Element, name: &str, author: &str) {
    if let Some(r) = p.child_mut("w:pPr").and_then(|x| x.child_mut("w:rPr")) {
        r.children.retain(|n| !matches!(n, Node::El(e) if e.is(name) && ours(e, author)));
    }
    if let Some(ppr) = p.child_mut("w:pPr") {
        ppr.children
            .retain(|n| !matches!(n, Node::El(e) if e.is("w:rPr") && e.children.is_empty() && e.attrs.is_empty()));
    }
}

/// Property changes: accepted, dropped; rejected, the stored properties back.
fn props(e: &mut Element, author: &str, accept: bool) {
    for (holder, change) in [("w:pPr", "w:pPrChange"), ("w:rPr", "w:rPrChange")] {
        if !e.is(holder) {
            continue;
        }
        let at = e.children.iter().position(|n| matches!(n, Node::El(c) if c.is(change) && ours(c, author)));
        let Some(at) = at else { continue };
        let Node::El(c) = e.children.remove(at) else { unreachable!() };
        if accept {
            continue;
        }
        let old = c.child(holder).cloned().unwrap_or_else(|| Element::new(holder));
        // A paragraph's stored properties leave out its mark's run properties and section.
        let keep: Vec<Node> = e
            .children
            .drain(..)
            .filter(|n| holder == "w:pPr" && matches!(n, Node::El(x) if x.is("w:rPr") || x.is("w:sectPr")))
            .collect();
        e.children = old.children;
        for n in keep {
            let Node::El(x) = n else { continue };
            let order = hanji_docx::ooxml::PPR_ORDER;
            xml::insert_ordered(e, x, order);
        }
    }
}

/// Run-level revisions in a paragraph (and in its wrappers).
fn runs(e: &mut Element, author: &str, accept: bool) {
    let mut out = vec![];
    for n in std::mem::take(&mut e.children) {
        let Node::El(mut c) = n else {
            out.push(n);
            continue;
        };
        let (ins, del) = (c.is("w:ins") && ours(&c, author), c.is("w:del") && ours(&c, author));
        if (ins && !accept) || (del && accept) {
            continue;
        }
        c.walk_mut(&mut |x| props(x, author, accept));
        if ins || del {
            if del {
                c.walk_mut(&mut |x| match x.name.as_str() {
                    "w:delText" => x.name = "w:t".into(),
                    "w:delInstrText" => x.name = "w:instrText".into(),
                    _ => {}
                });
            }
            out.extend(c.children);
            continue;
        }
        if !c.is("w:pPr") && !c.is("w:r") {
            runs(&mut c, author, accept);
        }
        out.push(Node::El(c));
    }
    e.children = out;
}

/// A container's children (the body, a cell): rows, tables and paragraph marks.
fn container(e: &mut Element, author: &str, accept: bool) -> Result<(), String> {
    for c in e.elements_mut() {
        match c.name.as_str() {
            "w:p" => runs(c, author, accept),
            "w:tbl" => table(c, author, accept)?,
            "w:sdt" => {
                if let Some(x) = c.child_mut("w:sdtContent") {
                    container(x, author, accept)?;
                }
            }
            _ => {}
        }
    }
    // Tables whose rows all went.
    e.children.retain(|n| !matches!(n, Node::El(t) if t.is("w:tbl") && t.child("w:tr").is_none()));
    // Paragraph marks: a removed one joins its paragraph to the next.
    let (gone, stays) = if accept { ("w:del", "w:ins") } else { ("w:ins", "w:del") };
    let mut k = 0;
    while k < e.children.len() {
        let Node::El(p) = &e.children[k] else {
            k += 1;
            continue;
        };
        if !p.is("w:p") {
            k += 1;
            continue;
        }
        if !has_mark(p, gone, author) {
            let Node::El(p) = &mut e.children[k] else { unreachable!() };
            drop_mark(p, stays, author);
            k += 1;
            continue;
        }
        // The next block (markers between blocks stay where they are).
        let block = |n: &Node| matches!(n, Node::El(x) if matches!(x.local(), "p" | "tbl" | "sdt" | "sectPr" | "customXml" | "altChunk"));
        let next = (k + 1..e.children.len()).find(|&j| block(&e.children[j]));
        let Some(j) = next.filter(|&j| matches!(&e.children[j], Node::El(n) if n.is("w:p"))) else {
            return Err(format!("a removed paragraph mark has no paragraph after it (in <{}>)", e.name));
        };
        let Node::El(p) = e.children.remove(k) else { unreachable!() };
        let content: Vec<Node> =
            p.children.into_iter().filter(|n| !matches!(n, Node::El(x) if x.is("w:pPr"))).collect();
        let Node::El(q) = &mut e.children[j - 1] else { unreachable!() };
        let at = q.children.iter().position(|n| matches!(n, Node::El(x) if x.is("w:pPr"))).map_or(0, |a| a + 1);
        q.children.splice(at..at, content);
    }
    Ok(())
}

fn table(t: &mut Element, author: &str, accept: bool) -> Result<(), String> {
    let (gone, stays) = if accept { ("w:del", "w:ins") } else { ("w:ins", "w:del") };
    let marked =
        |tr: &Element, name: &str| tr.child("w:trPr").and_then(|x| x.child(name)).is_some_and(|m| ours(m, author));
    t.children.retain(|n| !matches!(n, Node::El(tr) if tr.is("w:tr") && marked(tr, gone)));
    for tr in t.elements_mut().filter(|x| x.is("w:tr")) {
        if let Some(pr) = tr.child_mut("w:trPr") {
            pr.children.retain(|n| !matches!(n, Node::El(m) if m.is(stays) && ours(m, author)));
        }
        tr.children
            .retain(|n| !matches!(n, Node::El(x) if x.is("w:trPr") && x.children.is_empty() && x.attrs.is_empty()));
        for tc in tr.elements_mut().filter(|x| x.is("w:tc")) {
            container(tc, author, accept)?;
        }
    }
    Ok(())
}

/// The package with every change by `author` accepted (or rejected).
pub fn resolve(pkg: &[u8], author: &str, accept: bool) -> Result<Vec<u8>, String> {
    let mut parts = package::read(pkg)?;
    let doc = parts.iter_mut().find(|p| p.name == "word/document.xml").ok_or("no document part")?;
    let mut d = xml::parse(&doc.data).map_err(|e| e.to_string())?;
    let body = d.root.child_mut("w:body").ok_or("no body")?;
    container(body, author, accept)?;
    doc.data = xml::write_doc(&d);
    package::write(&parts)
}

/// Revision elements by `author` left in a package's document part.
pub fn count(pkg: &[u8], author: &str) -> usize {
    let parts = package::read(pkg).unwrap();
    let d = xml::parse(package::get(&parts, "word/document.xml").unwrap()).unwrap();
    let mut n = 0;
    d.root.walk(&mut |e| n += (e.get("w:author").as_deref() == Some(author)) as usize);
    n
}
