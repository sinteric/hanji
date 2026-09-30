//! `Contents/header.xml`: the file's paragraph styles, the character
//! shapes (`hh:charPr`) that carry bold, italic, underline and strikeout,
//! the paragraph shapes (`hh:paraPr`) that carry outline and list headings,
//! the numbering and bullet definitions, and the border fills a new table
//! is drawn with; with the formatting the text shows (§5.2), what each
//! shape and style sets ([`crate::format`]). Export adds a shape, a
//! numbering, a border fill, a font or a style only when the text asks for
//! one the file does not have, and points a style at new shapes when the
//! style section changed it.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use hanji_core::{ListDefs, StyleDef, StyleSet};
use hanji_format::styled::{self, StyleTable};
use hanji_format::{Key, Marks, Props, Value};
use hanji_package::xml::{self, canon, insert_ordered, remove_child, Element, Node, Scope};

use crate::format;
use crate::owpml::CHARPR_ORDER;

/// A paragraph style (`hh:style type="PARA"`).
#[derive(Clone, Debug)]
pub struct Style {
    pub id: u32,
    pub name: String,
    pub para_pr: u32,
    pub char_pr: u32,
}

/// A paragraph heading (`hh:heading`): outline level, numbered or bulleted list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Heading {
    Outline(u32),
    Number { id: u32, level: u32 },
    Bullet { id: u32, level: u32 },
}

/// List numbers for [`ListDefs`] name a numbering (odd) or a bullet (even).
fn numbering_num(id: u32) -> u32 {
    id * 2 + 1
}

fn bullet_num(id: u32) -> u32 {
    id * 2
}

/// The list number and level of a list heading.
pub fn list_num(h: Heading) -> Option<(u32, u32)> {
    match h {
        Heading::Number { id, level } => Some((numbering_num(id), level)),
        Heading::Bullet { id, level } => Some((bullet_num(id), level)),
        Heading::Outline(_) => None,
    }
}

/// The heading a list number and level stand for.
pub fn list_heading(num: u32, level: u32) -> Heading {
    if num % 2 == 1 {
        Heading::Number { id: num / 2, level }
    } else {
        Heading::Bullet { id: num / 2, level }
    }
}

/// Which list of `header.xml` an added element goes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Container {
    BorderFills,
    CharProperties,
    ParaProperties,
    Numberings,
    Styles,
}

impl Container {
    fn name(self) -> &'static str {
        match self {
            Container::BorderFills => "hh:borderFills",
            Container::CharProperties => "hh:charProperties",
            Container::ParaProperties => "hh:paraProperties",
            Container::Numberings => "hh:numberings",
            Container::Styles => "hh:styles",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Header {
    doc: Option<xml::Doc>,
    scope: Scope,
    pub styles: Vec<Style>,
    char_prs: BTreeMap<u32, Element>,
    para_prs: BTreeMap<u32, Element>,
    numberings: BTreeMap<u32, Element>,
    border_fills: BTreeMap<u32, Element>,
    bullets: BTreeSet<u32>,
    /// Canonical form (without id) → id, for reuse.
    char_index: HashMap<String, u32>,
    para_index: HashMap<String, u32>,
    border_index: HashMap<String, u32>,
    /// Font faces by language (`HANGUL`, `LATIN`, …): `(id, face)`.
    fonts: Vec<(String, Vec<(u32, String)>)>,
    added: Vec<(Container, Element)>,
    /// Fonts added, by language.
    added_fonts: Vec<(String, Element)>,
    /// Styles the style section changed: `(style, paraPrIDRef, charPrIDRef)`.
    style_edits: Vec<(u32, u32, u32)>,
    /// Track-change id → "insertion by NAME" (for summaries and refusals).
    pub tracked: HashMap<String, String>,
}

fn id_of(e: &Element) -> Option<u32> {
    e.get("id").and_then(|v| v.parse().ok())
}

/// Elements of `root > hh:refList > container > item`.
fn items<'a>(root: &'a Element, container: &str, item: &str) -> Vec<&'a Element> {
    let mut out = vec![];
    root.walk(&mut |e| {
        if e.is(container) {
            out.extend(e.elements().filter(|x| x.is(item)));
        }
    });
    out
}

impl Header {
    pub fn read(data: Option<&[u8]>) -> Result<Header, String> {
        let mut h = Header::default();
        let Some(data) = data else { return Ok(h) };
        let doc = xml::parse(data).map_err(|e| format!("Contents/header.xml: {e}"))?;
        h.scope = xml::scope_of(&doc.root);
        for e in items(&doc.root, "hh:charProperties", "hh:charPr") {
            if let Some(id) = id_of(e) {
                h.char_index.entry(h.key(e)).or_insert(id);
                h.char_prs.insert(id, e.clone());
            }
        }
        for e in items(&doc.root, "hh:paraProperties", "hh:paraPr") {
            if let Some(id) = id_of(e) {
                h.para_index.entry(h.key(e)).or_insert(id);
                h.para_prs.insert(id, e.clone());
            }
        }
        for e in items(&doc.root, "hh:numberings", "hh:numbering") {
            if let Some(id) = id_of(e) {
                h.numberings.insert(id, e.clone());
            }
        }
        for e in items(&doc.root, "hh:borderFills", "hh:borderFill") {
            if let Some(id) = id_of(e) {
                h.border_index.entry(h.key(e)).or_insert(id);
                h.border_fills.insert(id, e.clone());
            }
        }
        for ff in items(&doc.root, "hh:fontfaces", "hh:fontface") {
            let fonts = ff
                .elements()
                .filter(|f| f.is("hh:font"))
                .filter_map(|f| Some((id_of(f)?, f.get("face").unwrap_or_default())))
                .collect();
            h.fonts.push((ff.get("lang").unwrap_or_default(), fonts));
        }
        h.bullets = items(&doc.root, "hh:bullets", "hh:bullet").into_iter().filter_map(id_of).collect();
        for s in items(&doc.root, "hh:styles", "hh:style") {
            let n = |k: &str| s.get(k).and_then(|v| v.parse::<u32>().ok());
            if s.get("type").as_deref().unwrap_or("PARA") != "PARA" {
                continue;
            }
            let (Some(id), Some(para_pr), Some(char_pr)) = (n("id"), n("paraPrIDRef"), n("charPrIDRef")) else {
                continue;
            };
            let name = s.get("name").filter(|x| !x.is_empty()).or_else(|| s.get("engName")).unwrap_or(id.to_string());
            h.styles.push(Style { id, name, para_pr, char_pr });
        }
        let authors: HashMap<String, String> = items(&doc.root, "hh:trackChangeAuthors", "hh:trackChangeAuthor")
            .into_iter()
            .map(|a| (a.get("id").unwrap_or_default(), a.get("name").unwrap_or_default()))
            .collect();
        for t in items(&doc.root, "hh:trackChanges", "hh:trackChange") {
            let what = match t.get("type").as_deref() {
                Some("Insert") => "insertion",
                Some("Delete") => "deletion",
                Some("CharShape") | Some("ParaShape") => "formatting change",
                _ => "change",
            };
            let who = t.get("authorID").and_then(|a| authors.get(&a).cloned()).unwrap_or_else(|| "?".into());
            h.tracked.insert(t.get("id").unwrap_or_default(), format!("tracked {what} by {who}"));
        }
        h.doc = Some(doc);
        Ok(h)
    }

    /// Canonical form of a shape without its id.
    fn key(&self, e: &Element) -> String {
        let mut x = e.clone();
        x.remove_attr("id");
        canon(&x, &self.scope)
    }

    pub fn style(&self, id: u32) -> Option<&Style> {
        self.styles.iter().find(|s| s.id == id)
    }

    /// Style `id`, or, when header.xml has no such style, a stand-in named
    /// by its id with the first shapes, so the reference is kept as it is.
    pub fn style_or_missing(&self, id: u32) -> Style {
        self.style(id).cloned().unwrap_or_else(|| Style { id, name: id.to_string(), para_pr: 0, char_pr: 0 })
    }

    pub fn style_named(&self, name: &str) -> Option<&Style> {
        self.styles.iter().find(|s| s.name == name)
    }

    /// The style set the text names: paragraph styles by name, style 0 (바탕글)
    /// as the default, and the styles whose paragraph shape is outline level
    /// 1–6 as Heading 1–6. HWPX has no table styles.
    pub fn style_set(&self) -> StyleSet {
        let mut s = StyleSet::default();
        for st in &self.styles {
            s.paragraph.push(StyleDef::new(st.id.to_string(), st.name.clone()));
            if let Some(Heading::Outline(l)) = self.heading(st.para_pr) {
                if let Some(slot) = s.headings.get_mut(l as usize) {
                    slot.get_or_insert_with(|| st.name.clone());
                }
            }
        }
        s.default_paragraph = match self.styles.iter().find(|x| x.id == 0).or(self.styles.first()) {
            Some(st) => st.name.clone(),
            None => {
                s.paragraph.push(StyleDef::new("0", "바탕글"));
                "바탕글".into()
            }
        };
        self.lines_into(&mut s);
        s
    }

    // ------------------------------------------------------------ formatting (§5.2)

    /// The Hangul font faces by id: the face a text's `font` names.
    fn hangul_fonts(&self) -> HashMap<u32, String> {
        self.fonts.iter().filter(|f| f.0 == "HANGUL").flat_map(|f| f.1.iter().cloned()).collect()
    }

    /// What paragraph shape `id` sets (complete; nothing for a shape the file lacks).
    pub fn para_values(&self, id: u32) -> Props {
        let bf = |b: u32| self.border_fills.get(&b).cloned();
        self.para_prs.get(&id).map(|e| format::para_pr_props(e, &bf)).unwrap_or_default()
    }

    /// What character shape `id` sets: font, size, colour and the flags.
    pub fn char_values(&self, id: u32) -> Props {
        self.char_prs.get(&id).map(|e| format::char_pr_props(e, &self.hangul_fonts())).unwrap_or_default()
    }

    /// What style `st` sets: its paragraph and character shapes' values.
    pub fn style_values(&self, st: &Style) -> Props {
        self.para_values(st.para_pr).overlay(&self.char_values(st.char_pr)).only(&styled::STYLE_KEYS)
    }

    /// A cell's box: its border fill (`borderFillIDRef`) and vertical alignment.
    pub fn cell_box(&self, tc: &Element, sub: &Element) -> Props {
        let bf = tc.get("borderFillIDRef").and_then(|v| v.parse().ok()).and_then(|b: u32| self.border_fills.get(&b));
        let mut p = format::border_fill_props(bf);
        p.set(Key::Valign, Value::Choice(format::valign(sub).into()));
        p
    }

    /// The border fill of a cell's `tc`, drawn as `want` (a full box).
    pub fn cell_border_fill(&mut self, tc: &Element, delta: &Props) -> Result<u32, String> {
        let bf = tc.get("borderFillIDRef").and_then(|v| v.parse().ok());
        self.border_fill_set(bf, delta)
    }

    /// Each paragraph style's line (§5.2) in `set`: the default's complete,
    /// the others' what differs from it.
    fn lines_into(&self, set: &mut StyleSet) {
        let implicit = styled::implicit();
        let default_id: Option<u32> = set.default_paragraph_id().parse().ok();
        let dflt = default_id.and_then(|id| self.style(id)).map(|st| self.style_values(st)).unwrap_or_default();
        let dflt = implicit.overlay(&dflt);
        for s in &mut set.paragraph {
            let Some(st) = s.id.parse().ok().and_then(|id| self.style(id)) else { continue };
            let v = dflt.overlay(&self.style_values(st));
            s.props = if Some(st.id) == default_id { v.diff(&implicit) } else { v.diff(&dflt) };
        }
        set.formatting = true;
    }

    /// The face of font `id` in language `attr` (`hangul`, `latin`, …).
    fn face_of(&self, attr: &str, id: Option<u32>) -> Option<&str> {
        let fonts = &self.fonts.iter().find(|f| f.0.eq_ignore_ascii_case(attr))?.1;
        fonts.iter().find(|f| Some(f.0) == id).map(|f| f.1.as_str())
    }

    /// The ids, by `hh:fontRef` attribute, of font `face` in each language
    /// of `langs`, added to a language that does not have it.
    fn font_refs(&mut self, face: &str, langs: &[String]) -> Vec<(String, u32)> {
        let mut out = vec![];
        for (lang, fonts) in &mut self.fonts {
            if !langs.contains(&lang.to_lowercase()) {
                continue;
            }
            let id = match fonts.iter().find(|f| f.1 == face) {
                Some(f) => f.0,
                None => {
                    let id = fonts.iter().map(|f| f.0 + 1).max().unwrap_or(0);
                    fonts.push((id, face.to_string()));
                    let el = Element::new("hh:font")
                        .with_attr("id", &id.to_string())
                        .with_attr("face", face)
                        .with_attr("type", "TTF")
                        .with_attr("isEmbedded", "0");
                    self.added_fonts.push((lang.clone(), el));
                    id
                }
            };
            out.push((lang.to_lowercase(), id));
        }
        out
    }

    /// A character shape like `base` with `delta` (text keys and flags) set:
    /// `base` itself when nothing changes, an existing equal shape, or a new one.
    pub fn char_pr_set(&mut self, base: u32, delta: &Props) -> Result<u32, String> {
        if delta.is_empty() {
            return Ok(base);
        }
        let mut x =
            self.char_prs.get(&base).cloned().ok_or_else(|| format!("character shape {base} is not in header.xml"))?;
        for (k, v) in delta.iter() {
            format::writable(k, v)?;
            match (k, v) {
                (Key::Font, Value::Text(face)) => {
                    if self.fonts.is_empty() {
                        return Err("header.xml has no font faces (hh:fontfaces), so a font cannot be written".into());
                    }
                    if x.child("hh:fontRef").is_none() {
                        insert_ordered(&mut x, Element::new("hh:fontRef"), CHARPR_ORDER);
                    }
                    // The Hangul face, and each language that had the same face.
                    let fr = x.child("hh:fontRef").unwrap();
                    let id = |a: &str| fr.get(a).and_then(|v| v.parse::<u32>().ok());
                    let was = self.face_of("hangul", id("hangul")).map(str::to_string);
                    let langs: Vec<String> = self
                        .fonts
                        .iter()
                        .map(|f| f.0.to_lowercase())
                        .filter(|l| l == "hangul" || was.is_none() || self.face_of(l, id(l)).map(str::to_string) == was)
                        .collect();
                    let refs = self.font_refs(face, &langs);
                    let fr = x.child_mut("hh:fontRef").unwrap();
                    for (lang, id) in refs {
                        fr.set(&lang, &id.to_string());
                    }
                }
                (Key::Size, Value::Len(n)) => x.set("height", &n.to_string()),
                (Key::Color, Value::Color(c)) => x.set("textColor", &format::rgb(c)?),
                (Key::Bold, Value::Flag(on)) => set_present(&mut x, "hh:bold", *on),
                (Key::Italic, Value::Flag(on)) => set_present(&mut x, "hh:italic", *on),
                (Key::Underline, Value::Flag(on)) => set_line(&mut x, "hh:underline", "type", "BOTTOM", *on),
                (Key::Strike, Value::Flag(on)) => set_line(&mut x, "hh:strikeout", "shape", "SOLID", *on),
                _ => return Err(format!("{k} is not a text property")),
            }
        }
        Ok(self.add(Container::CharProperties, x))
    }

    /// A paragraph shape like `base` with `delta` (layout, fill, borders) set.
    pub fn para_pr_set(&mut self, base: u32, delta: &Props) -> Result<u32, String> {
        if delta.is_empty() {
            return Ok(base);
        }
        let mut x =
            self.para_prs.get(&base).cloned().ok_or_else(|| format!("paragraph shape {base} is not in header.xml"))?;
        let boxed = delta.only(&[Key::Fill, Key::BorderTop, Key::BorderRight, Key::BorderBottom, Key::BorderLeft]);
        for (k, v) in delta.iter().filter(|(k, _)| !boxed.has(*k)) {
            format::set_para(&mut x, k, v)?;
        }
        if !boxed.is_empty() {
            let cur = x.child("hh:border").and_then(|b| b.get("borderFillIDRef")).and_then(|v| v.parse().ok());
            let bf = self.border_fill_set(cur, &boxed)?;
            match x.child_mut("hh:border") {
                Some(b) => b.set("borderFillIDRef", &bf.to_string()),
                None => x.children.push(Node::El(
                    Element::new("hh:border")
                        .with_attr("borderFillIDRef", &bf.to_string())
                        .with_attr("offsetLeft", "0")
                        .with_attr("offsetRight", "0")
                        .with_attr("offsetTop", "0")
                        .with_attr("offsetBottom", "0")
                        .with_attr("connect", "0")
                        .with_attr("ignoreMargin", "0"),
                )),
            }
        }
        Ok(self.add(Container::ParaProperties, x))
    }

    /// A border fill like `base` (none: one that draws nothing) with
    /// `delta` (fill, sides) set.
    pub fn border_fill_set(&mut self, base: Option<u32>, delta: &Props) -> Result<u32, String> {
        let from = base.and_then(|b| self.border_fills.get(&b)).cloned();
        let blank = || {
            xml::fragment(
                r##"<hh:borderFill id="0" threeD="0" shadow="0" centerLine="NONE" breakCellSeparateLine="0"><hh:slash type="NONE" Crooked="0" isCounter="0"/><hh:backSlash type="NONE" Crooked="0" isCounter="0"/><hh:leftBorder type="NONE" width="0.1 mm" color="#000000"/><hh:rightBorder type="NONE" width="0.1 mm" color="#000000"/><hh:topBorder type="NONE" width="0.1 mm" color="#000000"/><hh:bottomBorder type="NONE" width="0.1 mm" color="#000000"/><hh:diagonal type="SOLID" width="0.1 mm" color="#000000"/></hh:borderFill>"##,
            )
        };
        if delta.is_empty() {
            if let Some(b) = base.filter(|_| from.is_some()) {
                return Ok(b);
            }
        }
        let has_list = self.doc.as_ref().is_some_and(|d| !items(&d.root, "hh:refList", "hh:borderFills").is_empty());
        if !has_list {
            return Err("header.xml has no border fills (hh:borderFills), so a fill or border cannot be written".into());
        }
        let mut x = from.unwrap_or_else(blank);
        for (k, v) in delta.iter() {
            match v {
                Value::Border(b) if k.is_side() => format::set_side(&mut x, k, b)?,
                Value::Fill(f) if k == Key::Fill => format::set_fill(&mut x, f)?,
                _ => return Err(format!("{k} is not a fill or border")),
            }
        }
        Ok(self.add(Container::BorderFills, x))
    }

    /// The style section of `set` in `header.xml` (§5.2): a style whose
    /// values differ from its line (and the default style's) points at
    /// shapes that have them, copies of its own; a style the text created
    /// (no id yet) is a new `hh:style` with shapes copied from the default
    /// style's, and gets its id in `set`.
    pub fn write_styles(&mut self, set: &mut StyleSet) -> Result<(), String> {
        let table = StyleTable { default: Some(set.default_paragraph.clone()), lines: set.lines() };
        let implicit = styled::implicit();
        let default_id: Option<u32> = set.default_paragraph_id().parse().ok();
        let dflt_style = default_id.and_then(|id| self.style(id)).cloned();
        let dflt = implicit.overlay(&dflt_style.as_ref().map(|st| self.style_values(st)).unwrap_or_default());
        let para_keys = styled::PARA_OWN;
        let text_keys = [Key::Font, Key::Size, Key::Color, Key::Bold, Key::Italic, Key::Underline, Key::Strike];
        for d in &mut set.paragraph {
            let want = table.values(Some(&d.name)).only(&styled::STYLE_KEYS);
            match d.id.parse::<u32>().ok().and_then(|id| self.style(id)).cloned() {
                Some(st) => {
                    let have =
                        if Some(st.id) == default_id { dflt.clone() } else { dflt.overlay(&self.style_values(&st)) };
                    let delta = want.diff(&have);
                    if delta.is_empty() {
                        continue;
                    }
                    let pp = self.para_pr_set(st.para_pr, &delta.only(&para_keys))?;
                    let cp = self.char_pr_set(st.char_pr, &delta.only(&text_keys))?;
                    self.style_edits.retain(|e| e.0 != st.id);
                    self.style_edits.push((st.id, pp, cp));
                }
                None if d.id.is_empty() => {
                    let Some(base) = dflt_style.clone() else {
                        return Err("header.xml has no default style to base a new style on".into());
                    };
                    let delta = want.diff(&dflt);
                    let pp = self.para_pr_set(base.para_pr, &delta.only(&para_keys))?;
                    let cp = self.char_pr_set(base.char_pr, &delta.only(&text_keys))?;
                    let id = self.styles.iter().map(|s| s.id + 1).max().unwrap_or(0);
                    let lang = self
                        .doc
                        .as_ref()
                        .and_then(|doc| items(&doc.root, "hh:styles", "hh:style").first().and_then(|s| s.get("langID")))
                        .unwrap_or_else(|| "1042".into());
                    let el = Element::new("hh:style")
                        .with_attr("id", &id.to_string())
                        .with_attr("type", "PARA")
                        .with_attr("name", &d.name)
                        .with_attr("engName", "")
                        .with_attr("paraPrIDRef", &pp.to_string())
                        .with_attr("charPrIDRef", &cp.to_string())
                        .with_attr("nextStyleIDRef", &id.to_string())
                        .with_attr("langID", &lang)
                        .with_attr("lockForm", "0");
                    self.added.push((Container::Styles, el));
                    self.styles.push(Style { id, name: d.name.clone(), para_pr: pp, char_pr: cp });
                    d.id = id.to_string();
                }
                // A style missing from header.xml keeps its reference as it is.
                None => {}
            }
        }
        Ok(())
    }

    // ------------------------------------------------------------ character shapes

    /// Bold, italic, underline and strikeout of character shape `id`.
    pub fn flags(&self, id: u32) -> Marks {
        let Some(e) = self.char_prs.get(&id) else { return Marks::NONE };
        let underline = e.child("hh:underline").is_some_and(|u| u.get("type").is_some_and(|t| t != "NONE"));
        let strike = e.child("hh:strikeout").is_some_and(|u| u.get("shape").is_some_and(|t| format::strikes(&t)));
        Marks::NONE
            .with(Marks::BOLD, e.child("hh:bold").is_some())
            .with(Marks::ITALIC, e.child("hh:italic").is_some())
            .with(Marks::UNDERLINE, underline)
            .with(Marks::STRIKE, strike)
    }

    /// Canonical form of character shape `id` without what the text states
    /// (for fingerprints: what the text does not show): the four flags,
    /// its size, colour and Hangul font.
    pub fn char_rest(&self, id: u32) -> String {
        let Some(e) = self.char_prs.get(&id) else { return format!("charPr {id}?") };
        let mut x = e.clone();
        for a in ["id", "height", "textColor"] {
            x.remove_attr(a);
        }
        // The font the text shows is the Hangul face; setting it sets the
        // other languages that had the same face.
        while remove_child(&mut x, "hh:fontRef").is_some() {}
        for n in ["hh:bold", "hh:italic", "hh:underline", "hh:strikeout"] {
            while remove_child(&mut x, n).is_some() {}
        }
        canon(&x, &self.scope)
    }

    /// A character shape like `base` whose flags are `want`: `base` itself,
    /// an existing identical shape, or a new one.
    pub fn char_pr_with(&mut self, base: u32, want: Marks) -> Result<u32, String> {
        if self.flags(base) == want {
            return Ok(base);
        }
        let mut x =
            self.char_prs.get(&base).cloned().ok_or_else(|| format!("character shape {base} is not in header.xml"))?;
        set_present(&mut x, "hh:bold", want.has(Marks::BOLD));
        set_present(&mut x, "hh:italic", want.has(Marks::ITALIC));
        set_line(&mut x, "hh:underline", "type", "BOTTOM", want.has(Marks::UNDERLINE));
        set_line(&mut x, "hh:strikeout", "shape", "SOLID", want.has(Marks::STRIKE));
        Ok(self.add(Container::CharProperties, x))
    }

    // ------------------------------------------------------------ paragraph shapes

    /// The heading of paragraph shape `id`, if it has one.
    pub fn heading(&self, id: u32) -> Option<Heading> {
        let e = self.para_prs.get(&id)?;
        let h = e.child("hh:heading")?;
        let n = |k: &str| h.get(k).and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
        match h.get("type").as_deref() {
            Some("OUTLINE") => Some(Heading::Outline(n("level"))),
            Some("NUMBER") => Some(Heading::Number { id: n("idRef"), level: n("level") }),
            Some("BULLET") => Some(Heading::Bullet { id: n("idRef"), level: n("level") }),
            _ => None,
        }
    }

    /// Canonical form of paragraph shape `id` without what the text states
    /// (for fingerprints): its alignment, margins, line spacing and border fill.
    pub fn para_rest_hidden(&self, id: u32) -> String {
        let Some(e) = self.para_prs.get(&id) else { return format!("paraPr {id}?") };
        let mut x = e.clone();
        x.remove_attr("id");
        if let Some(a) = x.child_mut("hh:align") {
            a.remove_attr("horizontal");
        }
        if let Some(b) = x.child_mut("hh:border") {
            b.remove_attr("borderFillIDRef");
        }
        fn strip(e: &mut Element) {
            for c in e.elements_mut() {
                match c.local() {
                    "margin" => c.children.clear(),
                    "lineSpacing" => {
                        c.remove_attr("type");
                        c.remove_attr("value");
                    }
                    "switch" | "case" | "default" => strip(c),
                    _ => {}
                }
            }
        }
        strip(&mut x);
        canon(&x, &self.scope)
    }

    /// A paragraph shape like `base` with heading `h` (`None`: no heading).
    pub fn para_pr_with(&mut self, base: u32, h: Option<Heading>) -> Result<u32, String> {
        if self.heading(base) == h {
            return Ok(base);
        }
        let mut x =
            self.para_prs.get(&base).cloned().ok_or_else(|| format!("paragraph shape {base} is not in header.xml"))?;
        let (ty, id, level) = match h {
            None => ("NONE", 0, 0),
            Some(Heading::Outline(l)) => ("OUTLINE", 0, l),
            Some(Heading::Number { id, level }) => ("NUMBER", id, level),
            Some(Heading::Bullet { id, level }) => ("BULLET", id, level),
        };
        let el = Element::new("hh:heading")
            .with_attr("type", ty)
            .with_attr("idRef", &id.to_string())
            .with_attr("level", &level.to_string());
        if let Some(cur) = x.child_mut("hh:heading") {
            *cur = el;
        } else {
            insert_ordered(&mut x, el, &["align", "heading", "breakSetting", "autoSpacing"]);
        }
        Ok(self.add(Container::ParaProperties, x))
    }

    fn add(&mut self, c: Container, mut x: Element) -> u32 {
        let key = self.key(&x);
        let (map, index) = match c {
            Container::CharProperties => (&mut self.char_prs, Some(&mut self.char_index)),
            Container::ParaProperties => (&mut self.para_prs, Some(&mut self.para_index)),
            Container::Numberings => (&mut self.numberings, None),
            Container::BorderFills => (&mut self.border_fills, Some(&mut self.border_index)),
            Container::Styles => unreachable!("styles are added by write_styles"),
        };
        if let Some(&id) = index.as_ref().and_then(|i| i.get(&key)) {
            return id;
        }
        // Border fill ids count from 1.
        let id = map.keys().next_back().map_or((c == Container::BorderFills) as u32, |m| m + 1);
        x.set("id", &id.to_string());
        map.insert(id, x.clone());
        if let Some(i) = index {
            i.insert(key, id);
        }
        self.added.push((c, x));
        id
    }

    // ------------------------------------------------------------ border fills

    /// The border fill of a new table and its cells, the look of a table
    /// Hancom inserts: solid 0.12 mm black lines on all four sides, no
    /// diagonal, no fill. The file's first such border fill, or a new one.
    pub fn table_border_fill(&mut self) -> Result<u32, String> {
        if let Some((&id, _)) = self.border_fills.iter().find(|(_, e)| is_table_look(e)) {
            return Ok(id);
        }
        let has_list = self.doc.as_ref().is_some_and(|d| !items(&d.root, "hh:refList", "hh:borderFills").is_empty());
        if !has_list {
            return Err("header.xml has no border fills (hh:borderFills) to draw a new table with".into());
        }
        let line = |side: &str| format!(r##"<hh:{side} type="SOLID" width="0.12 mm" color="#000000"/>"##);
        let x = xml::fragment(&format!(
            r##"<hh:borderFill id="0" threeD="0" shadow="0" centerLine="NONE" breakCellSeparateLine="0"><hh:slash type="NONE" Crooked="0" isCounter="0"/><hh:backSlash type="NONE" Crooked="0" isCounter="0"/>{}{}{}{}<hh:diagonal type="SOLID" width="0.1 mm" color="#000000"/></hh:borderFill>"##,
            line("leftBorder"),
            line("rightBorder"),
            line("topBorder"),
            line("bottomBorder"),
        ));
        Ok(self.add(Container::BorderFills, x))
    }

    /// `header.xml` with the added shapes and numberings, if any were added.
    pub fn part(&self) -> Option<Vec<u8>> {
        if self.added.is_empty() && self.added_fonts.is_empty() && self.style_edits.is_empty() {
            return None;
        }
        let mut d = self.doc.clone()?;
        fn rec(e: &mut Element, h: &Header) {
            for c in e.elements_mut() {
                if c.is("hh:fontface") {
                    let lang = c.get("lang").unwrap_or_default();
                    let mine: Vec<&Element> = h.added_fonts.iter().filter(|a| a.0 == lang).map(|a| &a.1).collect();
                    if !mine.is_empty() {
                        c.children.extend(mine.into_iter().map(|x| Node::El(x.clone())));
                        let n = c.elements().filter(|f| f.is("hh:font")).count();
                        if c.attr("fontCnt").is_some() {
                            c.set("fontCnt", &n.to_string());
                        }
                    }
                    continue;
                }
                if c.is("hh:style") {
                    let id = id_of(c);
                    if let Some((_, pp, cp)) = h.style_edits.iter().find(|e| Some(e.0) == id) {
                        c.set("paraPrIDRef", &pp.to_string());
                        c.set("charPrIDRef", &cp.to_string());
                    }
                    continue;
                }
                let mine: Vec<&Element> = h.added.iter().filter(|a| c.is(a.0.name())).map(|a| &a.1).collect();
                if mine.is_empty() {
                    rec(c, h);
                    continue;
                }
                c.children.extend(mine.into_iter().map(|x| Node::El(x.clone())));
                let n = c.elements().count();
                if c.attr("itemCnt").is_some() {
                    c.set("itemCnt", &n.to_string());
                }
                if c.is("hh:styles") {
                    rec(c, h);
                }
            }
        }
        rec(&mut d.root, self);
        Some(xml::write_doc(&d))
    }
}

/// A border fill that draws a table as Hancom does by default.
fn is_table_look(e: &Element) -> bool {
    let is = |x: &Element, k: &str, v: &str| x.get(k).is_some_and(|a| a.eq_ignore_ascii_case(v));
    let solid = |n: &str| {
        e.child(n).is_some_and(|b| is(b, "type", "SOLID") && is(b, "width", "0.12 mm") && is(b, "color", "#000000"))
    };
    ["hh:leftBorder", "hh:rightBorder", "hh:topBorder", "hh:bottomBorder"].into_iter().all(solid)
        && ["hh:slash", "hh:backSlash"].into_iter().all(|n| e.child(n).is_none_or(|x| is(x, "type", "NONE")))
        && e.child("hc:fillBrush").is_none()
        && !is(e, "threeD", "1")
        && !is(e, "shadow", "1")
}

fn set_present(x: &mut Element, name: &str, on: bool) {
    let has = x.child(name).is_some();
    if has && !on {
        remove_child(x, name);
    } else if !has && on {
        insert_ordered(x, Element::new(name), CHARPR_ORDER);
    }
}

/// An underline (`type`) or strikeout (`shape`): on is `value`, off is `NONE`.
fn set_line(x: &mut Element, name: &str, attr: &str, value: &str, on: bool) {
    let cur = x.child(name).and_then(|e| e.get(attr)).is_some_and(|v| {
        if attr == "shape" {
            format::strikes(&v)
        } else {
            v != "NONE"
        }
    });
    if cur == on {
        return;
    }
    let v = if on { value } else { "NONE" };
    match x.child_mut(name) {
        Some(e) => e.set(attr, v),
        None => {
            let mut e = Element::new(name).with_attr(attr, v);
            if attr != "shape" {
                e = e.with_attr("shape", "SOLID");
            }
            insert_ordered(x, e.with_attr("color", "#000000"), CHARPR_ORDER);
        }
    }
}

impl ListDefs for Header {
    fn ordered(&self, num: u32, ilvl: u32) -> Option<bool> {
        match list_heading(num, ilvl) {
            Heading::Number { id, level } => {
                let n = self.numberings.get(&id)?;
                // `hh:paraHead level` counts from 1.
                let lvl = n.elements().find(|p| p.is("hh:paraHead") && p.get("level") == Some((level + 1).to_string()));
                lvl.is_some().then_some(true)
            }
            Heading::Bullet { id, level } => (self.bullets.contains(&id) && level < 10).then_some(false),
            Heading::Outline(_) => None,
        }
    }

    /// The first numbering whose top level is decimal digits, or the first
    /// bullet.
    fn default_list(&self, ordered: bool) -> Option<(u32, u32)> {
        if ordered {
            let digit = |n: &Element| {
                n.elements().any(|p| {
                    p.is("hh:paraHead")
                        && p.get("level").as_deref() == Some("1")
                        && p.get("numFormat").as_deref() == Some("DIGIT")
                })
            };
            let (id, _) = self.numberings.iter().find(|(_, n)| digit(n))?;
            Some((numbering_num(*id), *id))
        } else {
            let id = self.bullets.iter().next()?;
            Some((bullet_num(*id), *id))
        }
    }

    /// A copy of numbering `base` under a new id, so its count starts again.
    fn new_list(&mut self, base: u32) -> u32 {
        let x = self.numberings.get(&base).cloned().unwrap_or_else(|| Element::new("hh:numbering"));
        numbering_num(self.add(Container::Numberings, x))
    }
}
