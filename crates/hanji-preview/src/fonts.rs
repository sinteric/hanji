//! Which face draws a requested font (DESIGN.md §7.1): the font directories
//! (`--font-dir`, `HANJI_FONT_DIR`), the document's embedded fonts, system
//! fonts, then the faces compiled into hanji (rpptx's Carlito, Caladea and
//! Liberation), looked up by the requested face's names, then its metric
//! twin, its substitutes and its class's fallbacks from the alias table
//! (`fonts/aliases.toml`, and an `aliases.toml` in a font directory over it).

use std::collections::BTreeMap;
#[cfg(all(feature = "host-fonts", not(target_family = "wasm")))]
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// Where a face came from (the report's `source`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    Supplied,
    FontDir,
    Embedded,
    System,
    Bundled,
}

/// The kind of text a face is asked to draw: rpptx picks a run's East Asian
/// typeface for East Asian text and its Latin one otherwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Script {
    Latin,
    Hangul,
    Cjk,
}

impl Script {
    /// The script of a run's text, as rpptx chooses its typeface.
    pub fn of(text: &str) -> Script {
        if !text.chars().any(crate::prep::is_east_asian) {
            Script::Latin
        } else if text.chars().any(is_hangul) {
            Script::Hangul
        } else {
            Script::Cjk
        }
    }

    /// A character a face must have to draw this script.
    fn probe(self) -> Option<char> {
        match self {
            Script::Latin => None,
            Script::Hangul => Some('가'),
            Script::Cjk => Some('一'),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Script::Latin => "latin",
            Script::Hangul => "hangul",
            Script::Cjk => "cjk",
        }
    }
}

pub fn is_hangul(c: char) -> bool {
    matches!(c as u32, 0xAC00..=0xD7A3 | 0x1100..=0x11FF | 0x3130..=0x318F | 0xA960..=0xA97F | 0xD7B0..=0xD7FF)
}

/// How the drawn face's metrics relate to the requested face's (the report's `metrics`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Metrics {
    /// The requested face itself.
    Original,
    /// A face with the requested face's advances.
    Compatible(String),
    /// A substitute laid out with the requested face's measured East Asian advance.
    Table,
    /// The substitute's own metrics.
    Substitute,
}

impl Metrics {
    pub fn name(&self) -> String {
        match self {
            Metrics::Original => "original".into(),
            Metrics::Compatible(f) => format!("compatible:{f}"),
            Metrics::Table => "table".into(),
            Metrics::Substitute => "substitute".into(),
        }
    }
}

impl Serialize for Metrics {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.name())
    }
}

/// One family of the alias table.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Family {
    pub names: Vec<String>,
    #[serde(default)]
    pub metric: Vec<String>,
    #[serde(default)]
    pub substitute: Vec<String>,
    #[serde(default)]
    pub class: Option<String>,
    #[serde(default)]
    pub ea_advance: Option<f64>,
}

/// A class's last candidates, per script.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Class {
    #[serde(default)]
    pub latin: Vec<String>,
    #[serde(default)]
    pub hangul: Vec<String>,
}

/// The alias table.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Aliases {
    #[serde(default)]
    pub family: Vec<Family>,
    #[serde(default)]
    pub class: BTreeMap<String, Class>,
}

const BUILTIN: &str = include_str!("../fonts/aliases.toml");

impl Aliases {
    /// The table compiled in.
    pub fn builtin() -> Aliases {
        toml::from_str(BUILTIN).expect("fonts/aliases.toml parses")
    }

    pub fn parse(s: &str) -> Result<Aliases, String> {
        toml::from_str(s).map_err(|e| e.to_string())
    }

    /// `over`'s entries replace those that share a name with them; its classes replace these.
    pub fn merge(&mut self, over: Aliases) {
        for f in over.family {
            self.family.retain(|g| !g.names.iter().any(|n| f.names.iter().any(|m| same_name(m, n))));
            self.family.push(f);
        }
        self.class.extend(over.class);
    }

    pub fn family(&self, name: &str) -> Option<&Family> {
        self.family.iter().find(|f| f.names.iter().any(|n| same_name(n, name)))
    }
}

/// Font names match ignoring case, spaces and hyphens (`Malgun Gothic` = `MalgunGothic`).
pub fn same_name(a: &str, b: &str) -> bool {
    let key =
        |s: &str| s.chars().filter(|c| !matches!(c, ' ' | '-' | '_')).flat_map(char::to_lowercase).collect::<String>();
    key(a) == key(b)
}

/// A face found for a request.
#[derive(Clone, Debug)]
pub struct Drawn {
    /// The family name the face gives itself.
    pub family: String,
    pub source: Source,
    pub metrics: Metrics,
    /// The East Asian advance to lay the face out with (`metrics: table`).
    pub ea_advance: Option<f64>,
    tier: usize,
}

/// A caller-owned font file or selected face of a font collection.
#[derive(Clone, Debug)]
pub struct FontData {
    pub data: Arc<[u8]>,
    /// Zero-based face index; use zero for a single-face font.
    pub face_index: u32,
}

impl FontData {
    pub fn new(data: impl Into<Arc<[u8]>>) -> FontData {
        FontData { data: data.into(), face_index: 0 }
    }
}

/// The face selected by a resolver for one request and style.
#[derive(Clone, Debug)]
pub struct ResolvedFont {
    pub family: String,
    pub source: Source,
    pub metrics: Metrics,
    pub ea_advance: Option<f64>,
    pub font: FontData,
}

/// Font selection inside a document job. Implementations return bytes, never paths.
/// Select the nearest available style when an exact bold/italic face is absent.
pub trait FontResolver {
    fn resolve_font(&self, requested: &str, script: Script, bold: bool, italic: bool) -> Option<ResolvedFont>;

    /// Select a drawing face for the actual repertoire. Existing resolvers
    /// retain their behavior; missing glyphs are still reported by the job.
    fn resolve_font_for_text(
        &self,
        requested: &str,
        script: Script,
        bold: bool,
        italic: bool,
        _text: &str,
    ) -> Option<ResolvedFont> {
        self.resolve_font(requested, script, bold, italic)
    }

    fn warnings(&self) -> &[String] {
        &[]
    }
}

/// Whether face `id` of `db` has a glyph for `c`.
fn draws(db: &fontdb::Database, id: fontdb::ID, c: char) -> bool {
    db.with_face_data(id, |d, i| covers_text(d, i, &c.to_string())) == Some(true)
}

// OpenType head.flags bit 14 means generic LastResort symbols rather than
// character-specific outlines. A nonzero cmap entry is not character coverage.
pub(crate) fn last_resort(face: &ttf_parser::Face<'_>) -> bool {
    face.raw_face()
        .table(ttf_parser::Tag::from_bytes(b"head"))
        .and_then(|head| head.get(16..18))
        .is_some_and(|flags| u16::from_be_bytes([flags[0], flags[1]]) & (1 << 14) != 0)
}

fn private_use(c: char) -> bool {
    matches!(c as u32, 0xE000..=0xF8FF | 0xF0000..=0xFFFFD | 0x100000..=0x10FFFD)
}

pub(crate) fn covers_text(data: &[u8], index: u32, text: &str) -> bool {
    ttf_parser::Face::parse(data, index).is_ok_and(|face| {
        !last_resort(&face)
            && text
                .chars()
                .filter(|c| !c.is_whitespace() && !c.is_control())
                .all(|c| face.glyph_index(c).is_some_and(|g| g.0 != 0))
    })
}

/// The faces a preview can draw with.
pub struct Fonts {
    tiers: Vec<(Source, fontdb::Database)>,
    /// [`Fonts::any_face`] per script: it reads every face.
    any_face: std::cell::RefCell<std::collections::HashMap<Script, Option<Drawn>>>,
    pub aliases: Aliases,
    /// Problems reading the font directories or their `aliases.toml`.
    pub warnings: Vec<String>,
}

/// Font files and directories in `HANJI_FONT_DIR` (a path list).
#[cfg(all(feature = "host-fonts", not(target_family = "wasm")))]
pub fn env_font_dirs() -> Vec<PathBuf> {
    std::env::var_os("HANJI_FONT_DIR")
        .map(|v| std::env::split_paths(&v).filter(|p| !p.as_os_str().is_empty()).collect())
        .unwrap_or_default()
}

impl Fonts {
    /// Caller fonts, document-embedded fonts, then bundled faces. No host discovery.
    /// An alias table overrides matching entries in the built-in font policy.
    pub fn from_bytes(
        supplied: &[FontData],
        embedded: &[oxml_layout::FontFile],
        aliases: Option<Aliases>,
    ) -> Result<Fonts, String> {
        let mut supplied_db = fontdb::Database::new();
        for (i, font) in supplied.iter().enumerate() {
            ttf_parser::Face::parse(&font.data, font.face_index)
                .map_err(|e| format!("supplied font {i}, face {} is invalid: {e}", font.face_index))?;
            let ids = supplied_db.load_font_source(fontdb::Source::Binary(Arc::new(font.data.clone())));
            let mut loaded = false;
            for id in ids {
                if supplied_db.face(id).is_some_and(|face| face.index == font.face_index) {
                    loaded = true;
                } else {
                    supplied_db.remove_face(id);
                }
            }
            if !loaded {
                return Err(format!("supplied font {i}, face {} has no usable family name", font.face_index));
            }
        }
        let mut fonts = Self::from_tiers(vec![(Source::Supplied, supplied_db)], embedded, vec![]);
        if let Some(aliases) = aliases {
            fonts.aliases.merge(aliases);
        }
        Ok(fonts)
    }

    /// The tiers: `font_dirs`, `embedded` (the document's own fonts),
    /// system fonts when `system`, and the bundled faces.
    #[cfg(all(feature = "host-fonts", not(target_family = "wasm")))]
    pub fn load(font_dirs: &[PathBuf], embedded: &[oxml_layout::FontFile], system: bool) -> Fonts {
        let mut warnings = vec![];
        let mut aliases = Aliases::builtin();
        let mut dir_db = fontdb::Database::new();
        for d in font_dirs {
            if d.is_file() {
                if let Err(e) = dir_db.load_font_file(d) {
                    warnings.push(format!("cannot read the font {}: {e}", d.display()));
                }
                continue;
            }
            if !d.is_dir() {
                warnings.push(format!("the font directory {} does not exist", d.display()));
                continue;
            }
            dir_db.load_fonts_dir(d);
            let table = d.join("aliases.toml");
            if table.is_file() {
                match std::fs::read_to_string(&table).map_err(|e| e.to_string()).and_then(|s| Aliases::parse(&s)) {
                    Ok(t) => aliases.merge(t),
                    Err(e) => warnings.push(format!("{} is not used: {e}", table.display())),
                }
            }
        }
        let mut system_db = fontdb::Database::new();
        if system {
            system_db.load_system_fonts();
        }
        let mut fonts = Self::from_tiers(vec![(Source::FontDir, dir_db)], embedded, vec![(Source::System, system_db)]);
        fonts.aliases = aliases;
        fonts.warnings = warnings;
        fonts
    }

    fn from_tiers(
        mut before: Vec<(Source, fontdb::Database)>,
        embedded: &[oxml_layout::FontFile],
        after: Vec<(Source, fontdb::Database)>,
    ) -> Fonts {
        let mut embedded_db = fontdb::Database::new();
        for f in embedded {
            embedded_db.load_font_data(f.data.clone());
        }
        let mut bundled = fontdb::Database::new();
        for (_, data) in oxml_layout::bundled_fonts::bundled_font_data() {
            bundled.load_font_source(fontdb::Source::Binary(std::sync::Arc::new(data)));
        }
        before.push((Source::Embedded, embedded_db));
        before.extend(after);
        before.push((Source::Bundled, bundled));
        Fonts { tiers: before, any_face: Default::default(), aliases: Aliases::builtin(), warnings: vec![] }
    }

    /// The face of tier `tier` named `name` (any of its localized family
    /// names) that has `probe`, nearest to `bold`/`italic`.
    fn query(
        &self,
        tier: usize,
        name: &str,
        bold: bool,
        italic: bool,
        probe: Option<char>,
    ) -> Option<(fontdb::ID, String)> {
        let db = &self.tiers[tier].1;
        let exact = db.faces().flat_map(|f| f.families.iter()).find(|(n, _)| same_name(n, name))?.0.clone();
        let q = fontdb::Query {
            families: &[fontdb::Family::Name(&exact)],
            weight: if bold { fontdb::Weight::BOLD } else { fontdb::Weight::NORMAL },
            style: if italic { fontdb::Style::Italic } else { fontdb::Style::Normal },
            stretch: fontdb::Stretch::Normal,
        };
        let id = db.query(&q)?;
        if db.with_face_data(id, |d, i| ttf_parser::Face::parse(d, i).is_ok_and(|f| last_resort(&f))) == Some(true) {
            return None;
        }
        if probe.is_some_and(|c| !draws(db, id, c)) {
            return None;
        }
        let face = db.face(id)?;
        let family = face.families.first().map_or(exact, |(n, _)| n.clone());
        Some((id, family))
    }

    /// The first tier with a face named `name` that draws `script`.
    fn find(&self, name: &str, script: Script, metrics: Metrics, ea_advance: Option<f64>) -> Option<Drawn> {
        (0..self.tiers.len()).find_map(|tier| {
            let (_, family) = self.query(tier, name, false, false, script.probe())?;
            Some(Drawn { family, source: self.tiers[tier].0, metrics: metrics.clone(), ea_advance, tier })
        })
    }

    /// The face that draws `requested` for `script` text, or `None` when no
    /// face here can (no Korean font at all, for Hangul).
    pub fn resolve(&self, requested: &str, script: Script) -> Option<Drawn> {
        let entry = self.aliases.family(requested);
        let ea = script != Script::Latin;
        let mut names: Vec<&str> = vec![requested];
        if let Some(e) = entry {
            names.extend(e.names.iter().map(String::as_str));
        }
        let advance = entry.and_then(|e| e.ea_advance).filter(|_| ea);
        let as_substitute = if advance.is_some() { Metrics::Table } else { Metrics::Substitute };
        let class = entry.and_then(|e| e.class.as_deref()).unwrap_or("sans");
        let fallbacks = self.aliases.class.get(class).or_else(|| self.aliases.class.get("sans"));
        let by_class = fallbacks.map(|c| if ea { &c.hangul } else { &c.latin }).into_iter().flatten();
        names
            .iter()
            .find_map(|n| self.find(n, script, Metrics::Original, None))
            .or_else(|| {
                let mut metric = entry.into_iter().flat_map(|e| &e.metric);
                metric.find_map(|m| self.find(m, script, Metrics::Compatible(m.clone()), None))
            })
            .or_else(|| {
                let mut substitutes = entry.into_iter().flat_map(|e| &e.substitute).chain(by_class);
                substitutes.find_map(|s| self.find(s, script, as_substitute.clone(), advance))
            })
            .or_else(|| self.any_face(script))
    }

    /// The first face here that draws `script` at all.
    fn any_face(&self, script: Script) -> Option<Drawn> {
        let probe = script.probe()?;
        let mut memo = self.any_face.borrow_mut();
        memo.entry(script)
            .or_insert_with(|| {
                self.tiers.iter().enumerate().find_map(|(tier, (source, db))| {
                    let face = db.faces().find(|f| draws(db, f.id, probe))?;
                    let family = face.families.first()?.0.clone();
                    Some(Drawn { family, source: *source, metrics: Metrics::Substitute, ea_advance: None, tier })
                })
            })
            .clone()
    }

    /// The bytes and face index of `drawn`'s face nearest to `bold`/`italic`.
    pub fn face_data(&self, drawn: &Drawn, bold: bool, italic: bool) -> Option<(Vec<u8>, u32)> {
        let (id, _) = self
            .query(drawn.tier, &drawn.family, bold, italic, None)
            .or_else(|| self.query(drawn.tier, &drawn.family, false, false, None))?;
        self.tiers[drawn.tier].1.with_face_data(id, |d, i| (d.to_vec(), i))
    }
}

impl FontResolver for Fonts {
    fn resolve_font(&self, requested: &str, script: Script, bold: bool, italic: bool) -> Option<ResolvedFont> {
        let drawn = self.resolve(requested, script)?;
        let (data, face_index) = self.face_data(&drawn, bold, italic)?;
        Some(ResolvedFont {
            family: drawn.family,
            source: drawn.source,
            metrics: drawn.metrics,
            ea_advance: drawn.ea_advance,
            font: FontData { data: data.into(), face_index },
        })
    }

    fn resolve_font_for_text(
        &self,
        requested: &str,
        script: Script,
        bold: bool,
        italic: bool,
        text: &str,
    ) -> Option<ResolvedFont> {
        let primary = self.resolve_font(requested, script, bold, italic);
        if primary.as_ref().is_some_and(|f| covers_text(&f.font.data, f.font.face_index, text)) {
            return primary;
        }
        // Private-use symbols have meaning only in their authored font. An
        // unrelated font with the same code point is not a semantic fallback.
        if text.chars().any(private_use) {
            return primary;
        }
        // Preserve the existing Latin coverage fallback before considering
        // other available families. Never rescue one character by dropping
        // another: every non-control, non-whitespace character must be covered.
        if script == Script::Latin {
            if let Some(mut font) = self.resolve_font("Arial", script, bold, italic) {
                if covers_text(&font.font.data, font.font.face_index, text) {
                    font.metrics = Metrics::Substitute;
                    font.ea_advance = None;
                    return Some(font);
                }
            }
        }
        for (tier, (source, db)) in self.tiers.iter().enumerate() {
            let families: std::collections::BTreeSet<_> =
                db.faces().filter_map(|f| f.families.first().map(|(name, _)| name.as_str())).collect();
            for family in families {
                let Some((id, family)) = self.query(tier, family, bold, italic, None) else { continue };
                if db.with_face_data(id, |data, index| covers_text(data, index, text)) != Some(true) {
                    continue;
                }
                let (data, face_index) = db.with_face_data(id, |data, index| (data.to_vec(), index))?;
                return Some(ResolvedFont {
                    family,
                    source: *source,
                    metrics: Metrics::Substitute,
                    ea_advance: None,
                    font: FontData { data: data.into(), face_index },
                });
            }
        }
        // No complete drawing face is available. Keep the selected face and
        // exact text so the job's missing-character diagnostics remain honest.
        primary
    }

    fn warnings(&self) -> &[String] {
        &self.warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundled_only() -> Fonts {
        Fonts::from_bytes(&[], &[], None).unwrap()
    }

    #[test]
    fn the_builtin_table_parses_and_names_the_common_faces() {
        let a = Aliases::builtin();
        for n in [
            "맑은 고딕",
            "Malgun Gothic",
            "Calibri",
            "Arial",
            "Cambria",
            "Times New Roman",
            "굴림",
            "돋움",
            "바탕",
            "함초롬바탕",
            "함초롬돋움",
            "HCR Batang",
        ] {
            assert!(a.family(n).is_some(), "{n}");
        }
        assert_eq!(a.family("malgungothic").unwrap().names[0], "맑은 고딕");
        assert_eq!(a.family("Calibri").unwrap().metric, vec!["Carlito"]);
        assert!(a.class.contains_key("sans") && a.class.contains_key("serif") && a.class.contains_key("mono"));
    }

    #[test]
    fn an_org_table_replaces_entries_by_name() {
        let mut a = Aliases::builtin();
        let n = a.family.len();
        a.merge(Aliases::parse("[[family]]\nnames = [\"Malgun Gothic\"]\nsubstitute = [\"Org Sans\"]\n").unwrap());
        assert_eq!(a.family.len(), n);
        assert_eq!(a.family("맑은 고딕").map(|f| f.substitute.clone()), None, "the old entry went");
        assert_eq!(a.family("Malgun Gothic").unwrap().substitute, vec!["Org Sans"]);
        assert!(Aliases::parse("[[family]]\nname = \"x\"\n").is_err(), "unknown keys are refused");
    }

    #[test]
    fn latin_faces_resolve_to_bundled_metric_twins() {
        let f = bundled_only();
        let d = f.resolve("Calibri", Script::Latin).unwrap();
        assert_eq!(
            (d.family.as_str(), d.source, d.metrics.name()),
            ("Carlito", Source::Bundled, "compatible:Carlito".into())
        );
        let d = f.resolve("Times New Roman", Script::Latin).unwrap();
        assert_eq!(d.metrics.name(), "compatible:Liberation Serif");
        let d = f.resolve("Carlito", Script::Latin).unwrap();
        assert_eq!(d.metrics, Metrics::Original);
        let d = f.resolve("Aptos", Script::Latin).unwrap();
        assert_eq!((d.family.as_str(), d.metrics.clone()), ("Carlito", Metrics::Substitute));
        // A face nobody knows falls back to its class, sans: Carlito, as
        // PowerPoint draws a missing face in Calibri.
        let d = f.resolve("Nonexistent Grotesk", Script::Latin).unwrap();
        assert_eq!((d.family.as_str(), d.metrics), ("Carlito", Metrics::Substitute));
        let (data, _) = f.face_data(&f.resolve("Arial", Script::Latin).unwrap(), true, false).unwrap();
        assert!(ttf_parser::Face::parse(&data, 0).unwrap().is_bold());
    }

    #[test]
    fn hangul_needs_a_face_that_has_it() {
        // The bundled faces have no Hangul: with no font directory and no
        // system fonts, Korean text has no face.
        let f = bundled_only();
        assert!(f.resolve("맑은 고딕", Script::Hangul).is_none());
        assert!(f.resolve("Calibri", Script::Hangul).is_none());
    }

    #[test]
    #[cfg(all(feature = "host-fonts", not(target_family = "wasm")))]
    fn a_missing_font_directory_is_a_warning() {
        let f = Fonts::load(&[PathBuf::from("/nonexistent/hanji-fonts")], &[], false);
        assert_eq!(f.warnings.len(), 1, "{:?}", f.warnings);
        assert!(f.warnings[0].contains("does not exist"));
    }

    #[test]
    fn names_match_across_spelling() {
        assert!(same_name("Malgun Gothic", "MalgunGothic") && same_name("noto sans cjk kr", "Noto Sans CJK KR"));
        assert!(!same_name("Gulim", "GulimChe"));
        assert_eq!(Script::of("abc 1"), Script::Latin);
        assert_eq!(Script::of("매출"), Script::Hangul);
        assert_eq!(Script::of("世界"), Script::Cjk);
    }
}
