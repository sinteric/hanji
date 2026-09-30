//! Characters a reader cannot see or tell from a plain space: whitespace
//! variants (U+2007 FIGURE SPACE after `□` in Korean government
//! documents), invisible and format characters, private-use symbols from
//! Hancom and Office symbol fonts, and controls. The text keeps them as
//! they are (GetPut); errors name them by code point, as
//! `⟨U+2007 FIGURE SPACE⟩`, so a model that copied a space or nothing
//! sees what the revision has.

use std::borrow::Cow;

/// The name of `c` if it is one a reader cannot see or tell apart, else
/// `None`. Line feeds are the text's own line breaks and are not named.
pub fn unusual(c: char) -> Option<&'static str> {
    Some(match c {
        '\t' => "CHARACTER TABULATION",
        '\r' => "CARRIAGE RETURN",
        '\u{0}'..='\u{8}' | '\u{b}'..='\u{c}' | '\u{e}'..='\u{1f}' | '\u{7f}'..='\u{84}' | '\u{86}'..='\u{9f}' => {
            "control"
        }
        '\u{85}' => "NEXT LINE",
        '\u{a0}' => "NO-BREAK SPACE",
        '\u{ad}' => "SOFT HYPHEN",
        '\u{34f}' => "COMBINING GRAPHEME JOINER",
        '\u{61c}' => "ARABIC LETTER MARK",
        '\u{115f}' => "HANGUL CHOSEONG FILLER",
        '\u{1160}' => "HANGUL JUNGSEONG FILLER",
        '\u{1680}' => "OGHAM SPACE MARK",
        '\u{180e}' => "MONGOLIAN VOWEL SEPARATOR",
        '\u{2000}' => "EN QUAD",
        '\u{2001}' => "EM QUAD",
        '\u{2002}' => "EN SPACE",
        '\u{2003}' => "EM SPACE",
        '\u{2004}' => "THREE-PER-EM SPACE",
        '\u{2005}' => "FOUR-PER-EM SPACE",
        '\u{2006}' => "SIX-PER-EM SPACE",
        '\u{2007}' => "FIGURE SPACE",
        '\u{2008}' => "PUNCTUATION SPACE",
        '\u{2009}' => "THIN SPACE",
        '\u{200a}' => "HAIR SPACE",
        '\u{200b}' => "ZERO WIDTH SPACE",
        '\u{200c}' => "ZERO WIDTH NON-JOINER",
        '\u{200d}' => "ZERO WIDTH JOINER",
        '\u{200e}' => "LEFT-TO-RIGHT MARK",
        '\u{200f}' => "RIGHT-TO-LEFT MARK",
        '\u{2028}' => "LINE SEPARATOR",
        '\u{2029}' => "PARAGRAPH SEPARATOR",
        '\u{202a}' => "LEFT-TO-RIGHT EMBEDDING",
        '\u{202b}' => "RIGHT-TO-LEFT EMBEDDING",
        '\u{202c}' => "POP DIRECTIONAL FORMATTING",
        '\u{202d}' => "LEFT-TO-RIGHT OVERRIDE",
        '\u{202e}' => "RIGHT-TO-LEFT OVERRIDE",
        '\u{202f}' => "NARROW NO-BREAK SPACE",
        '\u{205f}' => "MEDIUM MATHEMATICAL SPACE",
        '\u{2060}' => "WORD JOINER",
        '\u{2061}' => "FUNCTION APPLICATION",
        '\u{2062}' => "INVISIBLE TIMES",
        '\u{2063}' => "INVISIBLE SEPARATOR",
        '\u{2064}' => "INVISIBLE PLUS",
        '\u{2066}' => "LEFT-TO-RIGHT ISOLATE",
        '\u{2067}' => "RIGHT-TO-LEFT ISOLATE",
        '\u{2068}' => "FIRST STRONG ISOLATE",
        '\u{2069}' => "POP DIRECTIONAL ISOLATE",
        '\u{206a}'..='\u{206f}' => "format",
        '\u{3000}' => "IDEOGRAPHIC SPACE",
        '\u{3164}' => "HANGUL FILLER",
        '\u{fe00}'..='\u{fe0f}' | '\u{e0100}'..='\u{e01ef}' => "variation selector",
        '\u{feff}' => "ZERO WIDTH NO-BREAK SPACE",
        '\u{ffa0}' => "HALFWIDTH HANGUL FILLER",
        '\u{fff9}'..='\u{fffb}' => "interlinear annotation",
        '\u{fffc}' => "OBJECT REPLACEMENT CHARACTER",
        '\u{fffd}' => "REPLACEMENT CHARACTER",
        '\u{e0000}'..='\u{e007f}' => "tag",
        '\u{e000}'..='\u{f8ff}' | '\u{f0000}'..='\u{ffffd}' | '\u{100000}'..='\u{10fffd}' => "private use",
        '\u{fdd0}'..='\u{fdef}' => "noncharacter",
        c if (c as u32) & 0xfffe == 0xfffe => "noncharacter",
        _ => return None,
    })
}

/// `⟨U+2007 FIGURE SPACE⟩` for an unusual `c`.
pub fn label(c: char) -> Option<String> {
    unusual(c).map(|n| format!("⟨U+{:04X} {n}⟩", c as u32))
}

/// `s` with every unusual character replaced by its [`label`].
pub fn show(s: &str) -> Cow<'_, str> {
    if !s.chars().any(|c| unusual(c).is_some()) {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len() + 32);
    for c in s.chars() {
        match label(c) {
            Some(l) => out.push_str(&l),
            None => out.push(c),
        }
    }
    Cow::Owned(out)
}

/// The words every named message ends with, once.
const NOTE: &str = "stands for one character";

/// An error or diagnostic message with every unusual character named: the
/// raw character, or a Rust debug escape of one (`\u{2007}`, from a
/// `{:?}`-quoted string). When anything was named, a note says how to
/// write the character. Applying it twice changes nothing.
pub fn name_in(msg: &str) -> Cow<'_, str> {
    let mut out = String::with_capacity(msg.len() + 64);
    let mut first = None;
    let mut rest = msg;
    while let Some(c) = rest.chars().next() {
        if c == '\\' {
            if let Some(u) = debug_escape(rest) {
                if let Some(l) = label(u.0) {
                    out.push_str(&l);
                    first.get_or_insert(u.0);
                    rest = &rest[u.1..];
                    continue;
                }
            }
            // An escaped backslash stays as it is, and so does what follows it.
            let n = if rest[1..].starts_with('\\') { 2 } else { 1 };
            out.push_str(&rest[..n]);
            rest = &rest[n..];
            continue;
        }
        match label(c) {
            Some(l) => {
                out.push_str(&l);
                first.get_or_insert(c);
            }
            None => out.push(c),
        }
        rest = &rest[c.len_utf8()..];
    }
    let Some(c) = first else { return Cow::Borrowed(msg) };
    note(&mut out, c);
    Cow::Owned(out)
}

/// Append, once, how to write a named character such as `c`.
fn note(out: &mut String, c: char) {
    if out.contains(NOTE) {
        return;
    }
    let l = label(c).unwrap_or_default();
    let json: String = c.encode_utf16(&mut [0; 2]).iter().map(|u| format!("\\u{u:04x}")).collect();
    let s = if out.ends_with('.') || out.ends_with(')') { " " } else { ". " };
    out.push_str(&format!(
        "{s}({l} {NOTE}, named by its code point: the revision holds the character itself, so write that \
         character, as {json} in a JSON string.)"
    ));
}

/// `\u{…}` at the start of `s`: the character and the escape's length.
fn debug_escape(s: &str) -> Option<(char, usize)> {
    let body = s.strip_prefix("\\u{")?;
    let end = body.find('}')?;
    let hex = &body[..end];
    if hex.is_empty() || hex.len() > 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let c = char::from_u32(u32::from_str_radix(hex, 16).ok()?)?;
    Some((c, 3 + end + 1))
}

/// Why an exact `old` did not occur, when unusual characters are the
/// cause: where it would match, what the revision has there, named.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NearMiss {
    /// 1-based line of `text` where the match starts.
    pub line: usize,
    /// A sentence for the error, characters already named.
    pub message: String,
}

/// `old` does not occur in `text`: if it does once spaces are compared
/// loosely and invisible characters left out, and unusual characters are
/// in that stretch of `text` or in `old`, say where and name them. Tries
/// all of `old`, then each of its lines.
pub fn near_miss(text: &str, old: &str) -> Option<NearMiss> {
    let whole = std::iter::once(old);
    let lines = old.lines().filter(|l| l.trim().len() < old.trim().len() && !text.contains(l.trim()));
    for (k, part) in whole.chain(lines).enumerate() {
        for drop_spaces in [false, true] {
            let Some(m) = loose_match(text, part, drop_spaces) else { continue };
            let (s, e) = widen(text, m.0, m.1);
            let there: Vec<char> = text[s..e].chars().filter(|&c| unusual(c).is_some()).collect();
            let theirs: Vec<char> = part.chars().filter(|&c| unusual(c).is_some() && !there.contains(&c)).collect();
            if there.is_empty() && theirs.is_empty() {
                continue;
            }
            let line = text[..m.0].matches('\n').count() + 1;
            let what = if k == 0 { "It".to_string() } else { format!("Its line \"{}\"", show(part.trim())) };
            let ignored = if drop_spaces { "spaces" } else { "the kind of space" };
            let mut msg = format!(
                "{what} matches line {line} if {ignored} and invisible characters are ignored; the revision has \
                 \"{}\" there",
                show(&window(text, m.0, m.1))
            );
            if !there.is_empty() {
                msg.push_str(&format!(", with {}", listed(&there)));
            }
            msg.push('.');
            if !theirs.is_empty() {
                msg.push_str(&format!(
                    " The old text has {}, which the revision does not have there.",
                    listed(&theirs)
                ));
            }
            note(&mut msg, there.first().or(theirs.first()).copied().unwrap_or(' '));
            return Some(NearMiss { line, message: msg });
        }
    }
    None
}

/// `s..e` widened over the spaces and unusual characters beside it on its
/// lines: what `old` left out at its ends.
fn widen(text: &str, mut s: usize, mut e: usize) -> (usize, usize) {
    let skip = |c: char| c != '\n' && (c.is_whitespace() || unusual(c).is_some());
    while let Some(c) = text[..s].chars().next_back().filter(|&c| skip(c)) {
        s -= c.len_utf8();
    }
    while let Some(c) = text[e..].chars().next().filter(|&c| skip(c)) {
        e += c.len_utf8();
    }
    (s, e)
}

/// Distinct characters, labelled, with a count when more than one.
fn listed(cs: &[char]) -> String {
    let mut seen: Vec<(char, usize)> = vec![];
    for &c in cs {
        match seen.iter_mut().find(|(x, _)| *x == c) {
            Some((_, n)) => *n += 1,
            None => seen.push((c, 1)),
        }
    }
    let each: Vec<String> = seen
        .iter()
        .map(|&(c, n)| {
            let l = label(c).unwrap_or_default();
            if n > 1 {
                format!("{l} ×{n}")
            } else {
                l
            }
        })
        .collect();
    each.join(", ")
}

/// `text[s..e]` widened to its line(s), at most 40 characters either side.
fn window(text: &str, s: usize, e: usize) -> String {
    let ls = text[..s].rfind('\n').map_or(0, |k| k + 1);
    let le = text[e..].find('\n').map_or(text.len(), |k| e + k);
    let pre: Vec<char> = text[ls..s].chars().collect();
    let post: Vec<char> = text[e..le].chars().collect();
    let pre: String = pre[pre.len().saturating_sub(40)..].iter().collect();
    let post: String = post[..post.len().min(40)].iter().collect();
    let body = &text[s..e];
    let body = if body.chars().count() > 200 {
        let head: String = body.chars().take(200).collect();
        format!("{head}…")
    } else {
        body.to_string()
    };
    format!("{pre}{body}{post}")
}

/// The byte span of `text` where `part` occurs once whitespace runs
/// compare equal (or, with `drop_spaces`, are left out) and invisible
/// characters are left out. `None` when it does not, or `part` is blank.
fn loose_match(text: &str, part: &str, drop_spaces: bool) -> Option<(usize, usize)> {
    let (t, map) = loose(text, drop_spaces);
    let (p, _) = loose(part, drop_spaces);
    let p = p.trim_matches(' ');
    if p.is_empty() {
        return None;
    }
    let k = t.find(p)?;
    Some((map[k].0, map[k + p.len() - 1].1))
}

/// `s` with each whitespace run one space (or none) and invisible
/// characters left out, and for each byte of it the span of `s` it stands for.
fn loose(s: &str, drop_spaces: bool) -> (String, Vec<(usize, usize)>) {
    let mut out = String::with_capacity(s.len());
    let mut map: Vec<(usize, usize)> = Vec::with_capacity(s.len());
    for (i, c) in s.char_indices() {
        let end = i + c.len_utf8();
        if c.is_whitespace() {
            if drop_spaces {
                continue;
            }
            if out.ends_with(' ') {
                if let Some(last) = map.last_mut() {
                    last.1 = end;
                }
                continue;
            }
            out.push(' ');
            map.push((i, end));
        } else if unusual(c).is_some() {
            continue;
        } else {
            out.push(c);
            map.extend(std::iter::repeat_n((i, end), c.len_utf8()));
        }
    }
    (out, map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_by_code_point() {
        assert_eq!(label('\u{2007}').as_deref(), Some("⟨U+2007 FIGURE SPACE⟩"));
        assert_eq!(label('\u{f076}').as_deref(), Some("⟨U+F076 private use⟩"));
        assert_eq!(label('\u{1}').as_deref(), Some("⟨U+0001 control⟩"));
        assert_eq!(label('\u{f0000}').as_deref(), Some("⟨U+F0000 private use⟩"));
        for c in [' ', '\n', 'a', '가', '□', 'ㅇ', '⟨', '—'] {
            assert_eq!(unusual(c), None, "{c:?}");
        }
        assert_eq!(show("□\u{2007}추진"), "□⟨U+2007 FIGURE SPACE⟩추진");
        assert!(matches!(show("plain 텍스트"), Cow::Borrowed(_)));
    }

    #[test]
    fn names_raw_characters_and_debug_escapes_once() {
        let m = name_in("its first line \"□\\u{2007}추진\" is at line 3");
        assert!(m.starts_with("its first line \"□⟨U+2007 FIGURE SPACE⟩추진\" is at line 3. (⟨U+2007"), "{m}");
        assert!(m.contains("\\u2007 in a JSON string"), "{m}");
        assert_eq!(name_in(&m), m, "idempotent");
        let m = name_in("banner \u{f076} here.");
        assert!(m.starts_with("banner ⟨U+F076 private use⟩ here. ("), "{m}");
        assert!(m.contains("\\uf076"), "{m}");
        // A supplementary character is two UTF-16 escapes in JSON.
        assert!(name_in("x\u{f0001}").contains("\\udb80\\udc01"));
        // Plain messages and ordinary escapes are left alone.
        assert!(matches!(name_in("nor any of [ ] : * ? / \\)"), Cow::Borrowed(_)));
        assert!(matches!(name_in("\"a\\u{41}b\\\\u{2007}\""), Cow::Borrowed(_)));
    }

    #[test]
    fn near_miss_names_a_figure_space_the_model_wrote_as_a_space() {
        let text = "# 계획\n\n□\u{2007}추진 배경\n\nㅇ\u{2007}세부 내용\n";
        let m = near_miss(text, "□ 추진 배경").expect("a near miss");
        assert_eq!(m.line, 3);
        assert!(m.message.contains("\"□⟨U+2007 FIGURE SPACE⟩추진 배경\""), "{}", m.message);
        assert!(m.message.contains("with ⟨U+2007 FIGURE SPACE⟩."), "{}", m.message);
        // Across lines, and the count of each.
        let m = near_miss(text, "□ 추진 배경\n\nㅇ 세부 내용").expect("a near miss");
        assert!(m.message.contains("⟨U+2007 FIGURE SPACE⟩ ×2"), "{}", m.message);
    }

    #[test]
    fn near_miss_names_a_private_use_symbol_written_as_nothing_or_a_space() {
        let text = "앞 문단\n\u{f076} 2025년 사업 계획 \u{f076}\n";
        for old in ["2025년 사업 계획 \n", " 2025년 사업 계획", "앞 문단\n 2025년 사업 계획  "] {
            let m = near_miss(text, old).unwrap_or_else(|| panic!("{old:?}"));
            assert!(m.message.contains("⟨U+F076 private use⟩"), "{old:?}: {}", m.message);
            assert_eq!(m.line, if old.starts_with('앞') { 1 } else { 2 }, "{old:?}");
        }
        let text = "배너\u{f076}제목\n";
        let m = near_miss(text, "배너 제목").expect("a near miss, spaces left out");
        assert!(m.message.contains("\"배너⟨U+F076 private use⟩제목\""), "{}", m.message);
    }

    #[test]
    fn near_miss_names_what_the_old_text_has_and_the_revision_does_not() {
        let m = near_miss("금액 100 원\n", "금액\u{a0}100 원").expect("a near miss");
        assert!(m.message.contains("The old text has ⟨U+00A0 NO-BREAK SPACE⟩"), "{}", m.message);
    }

    #[test]
    fn no_near_miss_without_unusual_characters() {
        assert_eq!(near_miss("a  b\n", "a b"), None);
        assert_eq!(near_miss("abc\n", "xyz"), None);
        assert_eq!(near_miss("abc\n", "   "), None);
    }

    #[test]
    fn a_line_of_old_that_differs_by_an_unusual_character() {
        let text = "첫 줄\n□\u{2007}둘째 줄\n셋째 줄\n";
        let m = near_miss(text, "첫 줄\n□ 둘째 줄\n넷째 줄").expect("a near miss on a line");
        assert_eq!(m.line, 2);
        assert!(m.message.starts_with("Its line \"□ 둘째 줄\" matches line 2"), "{}", m.message);
    }
}
