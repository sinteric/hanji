//! Values as displayed (§5.4: the row window shows `12,000,000`, `00417`):
//! Excel number format codes — sections and conditions, digit placeholders,
//! grouping and scaling, percent, scientific, fractions, dates and times,
//! literals, colours and locale tags — and the General format. Rounding is
//! Excel's: half away from zero on the 15-digit decimal value.

use crate::value::CellValue;

/// Built-in number formats (ECMA-376 §18.8.30), as Excel shows them in en-US.
pub fn builtin(id: u32) -> Option<&'static str> {
    Some(match id {
        0 => "General",
        1 => "0",
        2 => "0.00",
        3 => "#,##0",
        4 => "#,##0.00",
        5 => "\"$\"#,##0_);(\"$\"#,##0)",
        6 => "\"$\"#,##0_);[Red](\"$\"#,##0)",
        7 => "\"$\"#,##0.00_);(\"$\"#,##0.00)",
        8 => "\"$\"#,##0.00_);[Red](\"$\"#,##0.00)",
        9 => "0%",
        10 => "0.00%",
        11 => "0.00E+00",
        12 => "# ?/?",
        13 => "# ??/??",
        14 => "m/d/yyyy",
        15 => "d-mmm-yy",
        16 => "d-mmm",
        17 => "mmm-yy",
        18 => "h:mm AM/PM",
        19 => "h:mm:ss AM/PM",
        20 => "h:mm",
        21 => "h:mm:ss",
        22 => "m/d/yyyy h:mm",
        37 => "#,##0 ;(#,##0)",
        38 => "#,##0 ;[Red](#,##0)",
        39 => "#,##0.00;(#,##0.00)",
        40 => "#,##0.00;[Red](#,##0.00)",
        41 => "_(* #,##0_);_(* (#,##0);_(* \"-\"_);_(@_)",
        42 => "_(\"$\"* #,##0_);_(\"$\"* (#,##0);_(\"$\"* \"-\"_);_(@_)",
        43 => "_(* #,##0.00_);_(* (#,##0.00);_(* \"-\"??_);_(@_)",
        44 => "_(\"$\"* #,##0.00_);_(\"$\"* (#,##0.00);_(\"$\"* \"-\"??_);_(@_)",
        45 => "mm:ss",
        46 => "[h]:mm:ss",
        47 => "mm:ss.0",
        48 => "##0.0E+0",
        49 => "@",
        _ => return None,
    })
}

/// The built-in id of a format code, if it is one.
pub fn builtin_id(code: &str) -> Option<u32> {
    (0..=49).find(|&i| builtin(i) == Some(code))
}

/// A value as `code` displays it.
pub fn display(v: &CellValue, code: &str, date1904: bool) -> String {
    match v {
        CellValue::Empty => String::new(),
        CellValue::Bool(b) => (if *b { "TRUE" } else { "FALSE" }).into(),
        CellValue::Error(e) => e.clone(),
        CellValue::Text(s) => text_section(code).map_or_else(|| s.clone(), |sec| render_text(&sec, s)),
        CellValue::Number(x) => number(*x, code, date1904),
    }
}

/// Split a code into sections at `;` outside quotes, brackets and escapes.
fn sections(code: &str) -> Vec<String> {
    let mut out = vec![];
    let mut cur = String::new();
    let mut it = code.chars();
    while let Some(c) = it.next() {
        match c {
            '"' => {
                cur.push(c);
                for d in it.by_ref() {
                    cur.push(d);
                    if d == '"' {
                        break;
                    }
                }
            }
            '\\' | '_' | '*' => {
                cur.push(c);
                if let Some(d) = it.next() {
                    cur.push(d);
                }
            }
            '[' => {
                cur.push(c);
                for d in it.by_ref() {
                    cur.push(d);
                    if d == ']' {
                        break;
                    }
                }
            }
            ';' => out.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    out.push(cur);
    out
}

fn text_section(code: &str) -> Option<String> {
    let s = sections(code);
    if s.len() >= 4 {
        return Some(s[3].clone());
    }
    s.iter().find(|x| has_outside(x, '@')).cloned()
}

/// Whether `c` appears outside quotes and escapes.
fn has_outside(s: &str, c: char) -> bool {
    tokens(s).iter().any(|t| matches!(t, Tk::Code(x) if *x == c))
}

#[derive(Clone, Debug, PartialEq)]
enum Tk {
    Lit(String),
    Code(char),
    /// `[…]` contents.
    Bracket(String),
    /// `_x`: a space as wide as x.
    Space,
    /// `*x`: fill; nothing in plain text.
    Fill,
}

fn tokens(s: &str) -> Vec<Tk> {
    let mut out = vec![];
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '"' => {
                let mut lit = String::new();
                for d in it.by_ref() {
                    if d == '"' {
                        break;
                    }
                    lit.push(d);
                }
                out.push(Tk::Lit(lit));
            }
            '\\' => {
                if let Some(d) = it.next() {
                    out.push(Tk::Lit(d.to_string()));
                }
            }
            '_' => {
                it.next();
                out.push(Tk::Space);
            }
            '*' => {
                it.next();
                out.push(Tk::Fill);
            }
            '[' => {
                let mut b = String::new();
                for d in it.by_ref() {
                    if d == ']' {
                        break;
                    }
                    b.push(d);
                }
                out.push(Tk::Bracket(b));
            }
            c => out.push(Tk::Code(c)),
        }
    }
    out
}

/// A bracket's literal text: `[$₩-412]` → `₩`; colours, conditions and locales → nothing.
fn bracket_text(b: &str) -> Option<String> {
    let rest = b.strip_prefix('$')?;
    Some(rest.split('-').next().unwrap_or("").to_string())
}

fn render_text(sec: &str, s: &str) -> String {
    let mut out = String::new();
    for t in tokens(sec) {
        match t {
            Tk::Lit(l) => out.push_str(&l),
            Tk::Code('@') => out.push_str(s),
            Tk::Code(c) => out.push(c),
            Tk::Bracket(b) => out.push_str(&bracket_text(&b).unwrap_or_default()),
            Tk::Space => out.push(' '),
            Tk::Fill => {}
        }
    }
    out
}

fn condition(b: &str) -> Option<(String, f64)> {
    for op in ["<=", ">=", "<>", "<", ">", "="] {
        if let Some(v) = b.strip_prefix(op) {
            return v.trim().parse().ok().map(|v| (op.to_string(), v));
        }
    }
    None
}

fn holds(cond: &(String, f64), x: f64) -> bool {
    let v = cond.1;
    match cond.0.as_str() {
        "<=" => x <= v,
        ">=" => x >= v,
        "<>" => x != v,
        "<" => x < v,
        ">" => x > v,
        _ => x == v,
    }
}

fn number(x: f64, code: &str, date1904: bool) -> String {
    if !x.is_finite() {
        return "#NUM!".into();
    }
    let secs: Vec<String> = sections(code).into_iter().collect();
    let conds: Vec<Option<(String, f64)>> = secs
        .iter()
        .map(|s| tokens(s).into_iter().find_map(|t| if let Tk::Bracket(b) = t { condition(&b) } else { None }))
        .collect();
    // Which section, and whether it shows the sign itself.
    let (sec, signed) = if conds.iter().any(Option::is_some) {
        let k = (0..secs.len().min(3)).find(|&k| conds[k].as_ref().is_some_and(|c| holds(c, x)));
        match k {
            Some(k) => (secs[k].clone(), k == 0 && conds[0].as_ref().is_some_and(|c| c.1 >= 0.0 || x >= 0.0)),
            None => {
                let k = (0..secs.len().min(3)).find(|&k| conds[k].is_none()).unwrap_or(secs.len() - 1);
                (secs[k].clone(), true)
            }
        }
    } else {
        match secs.len() {
            1 => (secs[0].clone(), true),
            2 => {
                if x < 0.0 {
                    (secs[1].clone(), false)
                } else {
                    (secs[0].clone(), true)
                }
            }
            _ => {
                if x < 0.0 {
                    (secs[1].clone(), false)
                } else if x == 0.0 {
                    (secs[2].clone(), true)
                } else {
                    (secs[0].clone(), true)
                }
            }
        }
    };
    let v = if signed { x } else { x.abs() };
    let toks = tokens(&sec);
    if sec.trim().eq_ignore_ascii_case("general") || toks.iter().all(|t| matches!(t, Tk::Bracket(_))) {
        let mut out = String::new();
        for t in &toks {
            if let Tk::Bracket(b) = t {
                out.push_str(&bracket_text(b).unwrap_or_default());
            }
        }
        return out + &general(v);
    }
    if is_date(&toks) {
        return date(v, &toks, date1904);
    }
    if toks.iter().any(|t| matches!(t, Tk::Code(c) if c.eq_ignore_ascii_case(&'g'))) {
        // `General` with literals around it
        let mut out = String::new();
        let mut skip = 0;
        for t in &toks {
            if skip > 0 {
                skip -= 1;
                continue;
            }
            match t {
                Tk::Code(c) if c.eq_ignore_ascii_case(&'g') => {
                    out.push_str(&general(v));
                    skip = 6;
                }
                Tk::Lit(l) => out.push_str(l),
                Tk::Code(c) => out.push(*c),
                Tk::Bracket(b) => out.push_str(&bracket_text(b).unwrap_or_default()),
                Tk::Space => out.push(' '),
                Tk::Fill => {}
            }
        }
        return out;
    }
    numeric(v, &toks)
}

fn is_date(toks: &[Tk]) -> bool {
    toks.iter().any(|t| match t {
        Tk::Code(c) => matches!(c.to_ascii_lowercase(), 'y' | 'm' | 'd' | 'h' | 's'),
        Tk::Bracket(b) => {
            let l = b.to_ascii_lowercase();
            !l.is_empty() && l.chars().all(|c| matches!(c, 'h' | 'm' | 's'))
        }
        _ => false,
    })
}

/// Excel's General: up to 11 characters, else scientific.
pub fn general(x: f64) -> String {
    if x == 0.0 {
        return "0".into();
    }
    let a = x.abs();
    if !(1e-9..1e11).contains(&a) {
        let s = format!("{:.5E}", x);
        let (m, e) = s.split_once('E').unwrap();
        let m = m.trim_end_matches('0').trim_end_matches('.');
        let e: i32 = e.parse().unwrap();
        return format!("{m}E{}{:02}", if e < 0 { '-' } else { '+' }, e.abs());
    }
    let int_digits = if a < 1.0 { 1 } else { a.log10().floor() as i32 + 1 };
    let places = (10 - int_digits).max(0) as usize;
    let (neg, i, f) = round_decimal(x, places);
    let f = f.trim_end_matches('0');
    let mut s = String::new();
    if neg && (i != "0" || !f.is_empty()) {
        s.push('-');
    }
    s.push_str(&i);
    if !f.is_empty() {
        s.push('.');
        s.push_str(f);
    }
    s
}

/// `x` rounded half away from zero to `places` decimals, on its 15-digit
/// decimal value: (negative, integer digits, fraction digits).
pub fn round_decimal(x: f64, places: usize) -> (bool, String, String) {
    let neg = x < 0.0;
    let s = format!("{:.14e}", x.abs());
    let (m, e) = s.split_once('e').unwrap();
    let e: i32 = e.parse().unwrap();
    let digits: Vec<u8> = m.bytes().filter(|b| b.is_ascii_digit()).map(|b| b - b'0').collect();
    // value = 0.d1d2d3… × 10^(e+1)
    let point = e + 1;
    let keep = point + places as i32;
    let mut ds: Vec<u8> = if keep <= 0 { vec![] } else { digits.iter().copied().take(keep as usize).collect() };
    while (ds.len() as i32) < keep {
        ds.push(0);
    }
    let next = if keep < 0 { 0 } else { digits.get(keep as usize).copied().unwrap_or(0) };
    let mut point = point;
    if next >= 5 {
        let mut k = ds.len();
        loop {
            if k == 0 {
                ds.insert(0, 1);
                point += 1;
                break;
            }
            k -= 1;
            if ds[k] == 9 {
                ds[k] = 0;
            } else {
                ds[k] += 1;
                break;
            }
        }
    }
    // Split at the point.
    let total = ds.len() as i32;
    let int_len = point.clamp(0, total) as usize;
    let mut int: String = ds[..int_len].iter().map(|d| (b'0' + d) as char).collect();
    let mut frac: String = ds[int_len..].iter().map(|d| (b'0' + d) as char).collect();
    if point < 0 {
        frac = "0".repeat((-point) as usize) + &frac;
    }
    frac.truncate(places);
    while frac.len() < places {
        frac.push('0');
    }
    let int_trim = int.trim_start_matches('0');
    int = if int_trim.is_empty() { "0".into() } else { int_trim.into() };
    let zero = int == "0" && frac.bytes().all(|b| b == b'0');
    (neg && !zero, int, frac)
}

fn group(int: &str) -> String {
    let b = int.as_bytes();
    let mut out = String::new();
    for (k, c) in b.iter().enumerate() {
        if k > 0 && (b.len() - k) % 3 == 0 {
            out.push(',');
        }
        out.push(*c as char);
    }
    out
}

fn numeric(x: f64, toks: &[Tk]) -> String {
    // Placeholders: integer part, fraction part, exponent.
    let is_ph = |c: char| matches!(c, '0' | '#' | '?');
    let codes: Vec<(usize, char)> =
        toks.iter().enumerate().filter_map(|(k, t)| if let Tk::Code(c) = t { Some((k, *c)) } else { None }).collect();
    let first_ph = codes.iter().find(|(_, c)| is_ph(*c)).map(|x| x.0);
    let last_ph = codes.iter().rev().find(|(_, c)| is_ph(*c)).map(|x| x.0);
    let Some((first_ph, last_ph)) = first_ph.zip(last_ph) else {
        // No digits: literals only (e.g. `"-"`).
        return literals(toks);
    };
    if codes.iter().any(|(_, c)| *c == '/') && codes.iter().any(|(k, c)| *c == '/' && *k > first_ph && *k < last_ph) {
        return fraction(x, toks);
    }
    let percent = codes.iter().filter(|(_, c)| *c == '%').count() as i32;
    let exp_at = codes.iter().find(|(k, c)| c.eq_ignore_ascii_case(&'e') && *k > first_ph).map(|x| x.0);
    let dot = codes
        .iter()
        .find(|(k, c)| *c == '.' && *k > first_ph.saturating_sub(1) && exp_at.is_none_or(|e| *k < e))
        .map(|x| x.0);
    let dot = dot.or_else(|| codes.iter().find(|(_, c)| *c == '.').map(|x| x.0).filter(|k| *k < last_ph));
    let mant_end = exp_at.unwrap_or(toks.len());
    let int_end = dot.unwrap_or(mant_end);
    let int_ph: Vec<char> = (first_ph..int_end)
        .filter_map(|k| if let Tk::Code(c) = &toks[k] { is_ph(*c).then_some(*c) } else { None })
        .collect();
    let frac_ph: Vec<char> = dot.map_or(vec![], |d| {
        (d + 1..mant_end)
            .filter_map(|k| if let Tk::Code(c) = &toks[k] { is_ph(*c).then_some(*c) } else { None })
            .collect()
    });
    let grouping = (first_ph..int_end).any(|k| {
        toks[k] == Tk::Code(',')
            && k > first_ph
            && k < int_end
            && (k + 1..int_end).any(|j| matches!(&toks[j], Tk::Code(c) if is_ph(*c)))
    });
    // Trailing commas after the last integer placeholder scale by 1000.
    let last_int_ph =
        (first_ph..int_end).rev().find(|&k| matches!(&toks[k], Tk::Code(c) if is_ph(*c))).unwrap_or(first_ph);
    let mut scale = 0;
    let mut k = last_int_ph + 1;
    while k < toks.len() && toks[k] == Tk::Code(',') {
        scale += 1;
        k += 1;
    }
    let mut v = x * 10f64.powi(2 * percent) / 1000f64.powi(scale);
    let mut exp_text = String::new();
    if let Some(e) = exp_at {
        let sign_plus = matches!(toks.get(e + 1), Some(Tk::Code('+')));
        let exp_digits: Vec<char> = (e + 1..toks.len())
            .filter_map(|k| if let Tk::Code(c) = &toks[k] { is_ph(*c).then_some(*c) } else { None })
            .collect();
        let mut ex = 0;
        if v != 0.0 {
            // Engineering form (`##0.0E+0`): the exponent a multiple of the integer placeholders.
            let step = if int_ph.len() > 1 && int_ph.contains(&'#') { int_ph.len() as i32 } else { 1 };
            ex = v.abs().log10().floor() as i32;
            if step > 1 {
                ex = ex.div_euclid(step) * step;
            } else {
                ex -= int_ph.len().saturating_sub(1) as i32;
            }
            v /= 10f64.powi(ex);
            let (_, i, _) = round_decimal(v, frac_ph.len());
            if i.len() > int_ph.len().max(1) && step == 1 {
                ex += 1;
                v /= 10.0;
            }
        }
        let digits = format!("{}", ex.abs());
        let width = exp_digits.iter().filter(|c| **c == '0').count();
        exp_text = format!(
            "{}{}{}",
            toks[e].clone().code().unwrap_or('E'),
            if ex < 0 {
                "-"
            } else if sign_plus {
                "+"
            } else {
                ""
            },
            "0".repeat(width.saturating_sub(digits.len())) + &digits
        );
    }
    let (neg, int, frac) = round_decimal(v, frac_ph.len());
    // Integer digits with the placeholders' minimum width.
    let min_int = int_ph.iter().filter(|c| **c == '0').count();
    let q_int = int_ph.iter().filter(|c| **c == '?').count();
    let mut int_s = if int == "0" && min_int == 0 { String::new() } else { int.clone() };
    while int_s.len() < min_int {
        int_s.insert(0, '0');
    }
    if grouping {
        int_s = group(&int_s);
    }
    let pad = (min_int + q_int).saturating_sub(int_s.len());
    if q_int > 0 && pad > 0 {
        int_s = " ".repeat(pad.min(q_int)) + &int_s;
    }
    // Literals between integer placeholders (`###-####`): digits fill the
    // placeholders from the right, the first taking what is left.
    let int_pos: Vec<usize> = (first_ph..int_end).filter(|&k| matches!(&toks[k], Tk::Code(c) if is_ph(*c))).collect();
    let interleaved =
        !grouping && int_pos.windows(2).any(|w| (w[0] + 1..w[1]).any(|k| !matches!(&toks[k], Tk::Code(','))));
    let mut int_at: std::collections::HashMap<usize, String> = std::collections::HashMap::new();
    if interleaved {
        let digits: Vec<char> = if int == "0" && min_int == 0 { vec![] } else { int.chars().collect() };
        let mut di = digits.len();
        for (j, &k) in int_pos.iter().enumerate().rev() {
            let ph = if let Tk::Code(c) = &toks[k] { *c } else { '#' };
            let piece = if j == 0 && di > 0 {
                digits[..di].iter().collect()
            } else if di > 0 {
                di -= 1;
                digits[di].to_string()
            } else {
                match ph {
                    '0' => "0".into(),
                    '?' => " ".into(),
                    _ => String::new(),
                }
            };
            if j == 0 {
                di = 0;
            }
            int_at.insert(k, piece);
        }
    }
    // Fraction digits: `#` drops trailing zeros, `?` makes them spaces.
    let mut frac_s: Vec<char> = frac.chars().collect();
    for k in (0..frac_ph.len()).rev() {
        if frac_s[k] != '0' || frac_ph[k] == '0' {
            break;
        }
        frac_s[k] = if frac_ph[k] == '?' { ' ' } else { '\0' };
    }
    let frac_s: String = frac_s.into_iter().filter(|c| *c != '\0').collect();
    // Assemble: literals in place, digits at the first placeholder of each part.
    let mut out = String::new();
    if neg {
        out.push('-');
    }
    let mut int_done = false;
    let mut frac_done = false;
    let mut k = 0;
    while k < toks.len() {
        let t = &toks[k];
        if Some(k) == exp_at {
            out.push_str(&exp_text);
            k += 1;
            while k < toks.len() && matches!(&toks[k], Tk::Code(c) if is_ph(*c) || *c == '+' || *c == '-') {
                k += 1;
            }
            continue;
        }
        match t {
            Tk::Code(c) if is_ph(*c) && k < int_end && interleaved => out.push_str(&int_at[&k]),
            Tk::Code(c) if is_ph(*c) && k < int_end => {
                if !int_done {
                    out.push_str(&int_s);
                    int_done = true;
                }
            }
            Tk::Code(c) if is_ph(*c) => {
                if !frac_done {
                    out.push_str(&frac_s);
                    frac_done = true;
                }
            }
            Tk::Code('.') if Some(k) == dot => out.push('.'),
            Tk::Code(',') if k > first_ph && k < int_end => {}
            Tk::Code(',') if k > last_int_ph && k <= last_int_ph + scale as usize => {}
            Tk::Code('%') => out.push('%'),
            Tk::Code(c) => out.push(*c),
            Tk::Lit(l) => out.push_str(l),
            Tk::Bracket(b) => out.push_str(&bracket_text(b).unwrap_or_default()),
            Tk::Space => out.push(' '),
            Tk::Fill => {}
        }
        k += 1;
    }
    out
}

impl Tk {
    fn code(self) -> Option<char> {
        if let Tk::Code(c) = self {
            Some(c)
        } else {
            None
        }
    }
}

fn literals(toks: &[Tk]) -> String {
    let mut out = String::new();
    for t in toks {
        match t {
            Tk::Lit(l) => out.push_str(l),
            Tk::Code(c) => out.push(*c),
            Tk::Bracket(b) => out.push_str(&bracket_text(b).unwrap_or_default()),
            Tk::Space => out.push(' '),
            Tk::Fill => {}
        }
    }
    out
}

fn fraction(x: f64, toks: &[Tk]) -> String {
    // `# ?/?`, `# ??/??`, `?/8`: whole part (if a placeholder precedes a space), then numerator/denominator.
    let codes: String = toks.iter().map(|t| if let Tk::Code(c) = t { *c } else { ' ' }).collect();
    let slash = codes.find('/').unwrap();
    let den_part = &codes[slash + 1..];
    let den_digits = den_part.chars().take_while(|c| matches!(c, '0' | '#' | '?' | '1'..='9')).collect::<String>();
    let fixed: Option<u64> =
        den_digits.parse().ok().filter(|d| *d > 0 && den_digits.chars().all(|c| c.is_ascii_digit()));
    let before = codes[..slash].trim_end();
    let has_whole = before.contains(' ') || before.split_whitespace().count() > 1;
    let neg = x < 0.0;
    let a = x.abs();
    let (whole, f) = if has_whole { (a.trunc(), a.fract()) } else { (0.0, a) };
    let max_den = 10u64.pow(den_digits.len().max(1) as u32) - 1;
    let (mut n, d) = match fixed {
        Some(d) => ((f * d as f64).round() as u64, d),
        None => best_fraction(f, max_den),
    };
    let mut whole = whole as u64;
    if n == d && has_whole {
        whole += 1;
        n = 0;
    }
    let mut out = String::new();
    if neg {
        out.push('-');
    }
    if has_whole {
        if whole > 0 || n == 0 {
            out.push_str(&whole.to_string());
        }
        if n > 0 {
            if whole > 0 {
                out.push(' ');
            }
            out.push_str(&format!("{n}/{d}"));
        }
    } else {
        out.push_str(&format!("{n}/{d}"));
    }
    out
}

fn best_fraction(f: f64, max_den: u64) -> (u64, u64) {
    let (mut best_n, mut best_d, mut best_e) = (0, 1, f);
    for d in 1..=max_den {
        let n = (f * d as f64).round() as u64;
        let e = (f - n as f64 / d as f64).abs();
        if e < best_e - 1e-12 {
            best_n = n;
            best_d = d;
            best_e = e;
        }
    }
    (best_n, best_d)
}

/// Civil date of a serial (the 1900 system counts the phantom 1900-02-29).
pub fn civil(serial: f64, date1904: bool) -> Option<(i64, u32, u32)> {
    let days = serial.floor() as i64;
    if !date1904 && days == 60 {
        return Some((1900, 2, 29));
    }
    let base = if date1904 {
        days_from_civil(1904, 1, 1)
    } else if days < 60 {
        days_from_civil(1899, 12, 31)
    } else {
        days_from_civil(1899, 12, 30)
    };
    (days >= 0).then(|| civil_from_days(base + days))
}

/// Days from 1970-01-01 of a civil date.
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// The serial of a civil date.
pub fn serial_of(y: i64, m: u32, d: u32, date1904: bool) -> f64 {
    let days = days_from_civil(y, m, d);
    if date1904 {
        return (days - days_from_civil(1904, 1, 1)) as f64;
    }
    let s = days - days_from_civil(1899, 12, 30);
    // Before 1900-03-01 Excel's serials are one lower (its phantom 29 February).
    (if s < 61 { s - 1 } else { s }) as f64
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const DAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

fn date(x: f64, toks: &[Tk], date1904: bool) -> String {
    if x < 0.0 {
        return "#".repeat(11);
    }
    // Fractional seconds shown: the count of 0s after `s.`.
    let codes: Vec<char> = toks.iter().filter_map(|t| if let Tk::Code(c) = t { Some(*c) } else { None }).collect();
    let low: String = codes.iter().collect::<String>().to_ascii_lowercase();
    let sub_digits = low.find("s.").map_or(0, |k| low[k + 2..].chars().take_while(|c| *c == '0').count());
    let unit = 86_400.0 * 10f64.powi(sub_digits as i32);
    let ticks = (x * unit).round();
    let day = (ticks / unit).floor();
    let rem = ticks - day * unit;
    let total_secs = rem / 10f64.powi(sub_digits as i32);
    let (hh, mm, ss) = (
        (total_secs / 3600.0).floor() as u64,
        ((total_secs % 3600.0) / 60.0).floor() as u64,
        (total_secs % 60.0).floor() as u64,
    );
    let sub = (rem % 10f64.powi(sub_digits as i32)) as u64;
    let Some((y, mo, d)) = civil(day, date1904) else { return "#".repeat(11) };
    let ampm = low.contains("am/pm") || low.contains("a/p");
    let weekday = ((days_from_civil(y, mo, d) % 7 + 11) % 7) as usize; // 1970-01-01 was a Thursday
    let mut out = String::new();
    let mut k = 0;
    let n = toks.len();
    // Whether an `m` run at token k means minutes: after an h (skipping literals) or before an s.
    let is_minute = |k: usize| -> bool {
        let prev = toks[..k].iter().rev().find_map(|t| match t {
            Tk::Code(c) => c.is_ascii_alphabetic().then_some(c.to_ascii_lowercase()),
            Tk::Bracket(b) if b.to_ascii_lowercase().starts_with('h') => Some('h'),
            _ => None,
        });
        let mut j = k;
        while j < n && matches!(&toks[j], Tk::Code(c) if c.eq_ignore_ascii_case(&'m')) {
            j += 1;
        }
        let next = toks[j..].iter().find_map(|t| {
            if let Tk::Code(c) = t {
                c.is_ascii_alphabetic().then_some(c.to_ascii_lowercase())
            } else {
                None
            }
        });
        prev == Some('h') || next == Some('s')
    };
    while k < n {
        match &toks[k] {
            Tk::Code(c) => {
                let lc = c.to_ascii_lowercase();
                let run =
                    toks[k..].iter().take_while(|t| matches!(t, Tk::Code(x) if x.eq_ignore_ascii_case(c))).count();
                match lc {
                    'y' => {
                        out.push_str(&if run <= 2 { format!("{:02}", y % 100) } else { format!("{y:04}") });
                        k += run;
                        continue;
                    }
                    'm' if run <= 2 && is_minute(k) => {
                        out.push_str(&if run == 2 { format!("{mm:02}") } else { mm.to_string() });
                        k += run;
                        continue;
                    }
                    'm' => {
                        let name = MONTHS[mo as usize - 1];
                        out.push_str(&match run {
                            1 => mo.to_string(),
                            2 => format!("{mo:02}"),
                            3 => name[..3].to_string(),
                            5 => name[..1].to_string(),
                            _ => name.to_string(),
                        });
                        k += run;
                        continue;
                    }
                    'd' => {
                        let name = DAYS[weekday];
                        out.push_str(&match run {
                            1 => d.to_string(),
                            2 => format!("{d:02}"),
                            3 => name[..3].to_string(),
                            _ => name.to_string(),
                        });
                        k += run;
                        continue;
                    }
                    'h' => {
                        let h = if ampm { (hh + 11) % 12 + 1 } else { hh };
                        out.push_str(&if run >= 2 { format!("{h:02}") } else { h.to_string() });
                        k += run;
                        continue;
                    }
                    's' => {
                        out.push_str(&if run >= 2 { format!("{ss:02}") } else { ss.to_string() });
                        k += run;
                        continue;
                    }
                    'a' => {
                        let rest: String = toks[k..]
                            .iter()
                            .take(5)
                            .filter_map(|t| if let Tk::Code(c) = t { Some(*c) } else { None })
                            .collect();
                        if rest.to_ascii_lowercase().starts_with("am/pm") {
                            out.push_str(if hh < 12 { "AM" } else { "PM" });
                            k += 5;
                            continue;
                        }
                        if rest.to_ascii_lowercase().starts_with("a/p") {
                            out.push_str(if hh < 12 { "A" } else { "P" });
                            k += 3;
                            continue;
                        }
                        out.push(*c);
                    }
                    '.' if sub_digits > 0 && low.contains("s.") && matches!(toks.get(k + 1), Some(Tk::Code('0'))) => {
                        out.push('.');
                        out.push_str(&format!("{:0w$}", sub, w = sub_digits));
                        k += 1 + sub_digits;
                        continue;
                    }
                    _ => out.push(*c),
                }
            }
            Tk::Lit(l) => out.push_str(l),
            Tk::Bracket(b) => {
                let l = b.to_ascii_lowercase();
                if !l.is_empty() && l.chars().all(|c| matches!(c, 'h' | 'm' | 's')) {
                    let total = (x * 86_400.0).round() as u64;
                    let v = match l.chars().next().unwrap() {
                        'h' => total / 3600,
                        'm' => total / 60,
                        _ => total,
                    };
                    out.push_str(&format!("{:0w$}", v, w = l.len()));
                } else if let Some(t) = bracket_text(b) {
                    out.push_str(&t);
                }
            }
            Tk::Space => out.push(' '),
            Tk::Fill => {}
        }
        k += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(x: f64, code: &str) -> String {
        display(&CellValue::Number(x), code, false)
    }

    #[test]
    fn numbers() {
        assert_eq!(n(12_000_000.0, "#,##0"), "12,000,000");
        assert_eq!(n(417.0, "00000"), "00417");
        assert_eq!(n(0.923, "0.0%"), "92.3%");
        assert_eq!(n(2.675, "0.00"), "2.68");
        assert_eq!(n(-1234.5, "#,##0.00"), "-1,234.50");
        assert_eq!(n(-1234.5, "#,##0;(#,##0)"), "(1,235)");
        assert_eq!(n(0.0, "#,##0;(#,##0);\"-\""), "-");
        assert_eq!(n(1234.0, "[$₩-412]#,##0"), "₩1,234");
        assert_eq!(n(1_234_567.0, "#,##0,\"K\""), "1,235K");
        assert_eq!(n(12345.678, "0.00E+00"), "1.23E+04");
        assert_eq!(n(0.000123, "0.00E+00"), "1.23E-04");
        assert_eq!(n(1.5, "# ?/?"), "1 1/2");
        assert_eq!(n(0.0, "0.0"), "0.0");
        assert_eq!(n(-0.001, "0.0"), "0.0");
        assert_eq!(n(5.0, "#.##"), "5.");
        assert_eq!(n(1234.0, "_-* #,##0_-;-* #,##0_-;_-* \"-\"_-;_-@_-"), " 1,234 ");
        assert_eq!(n(7.0, "[Red][<=9999999]###-####;(###) ###-####"), "-7");
    }

    #[test]
    fn general_format() {
        assert_eq!(general(1.0 / 3.0), "0.333333333");
        assert_eq!(general(1234.56789012), "1234.56789");
        assert_eq!(general(0.1 + 0.2), "0.3");
        assert_eq!(general(123456789012.0), "1.23457E+11");
        assert_eq!(general(-5.0), "-5");
        assert_eq!(general(1e-10), "1E-10");
    }

    #[test]
    fn dates() {
        // 2026-03-01 is serial 46082.
        assert_eq!(serial_of(2026, 3, 1, false), 46082.0);
        assert_eq!(n(46082.0, "yyyy-mm"), "2026-03");
        assert_eq!(n(46082.0, "yyyy-mm-dd"), "2026-03-01");
        assert_eq!(n(46082.0, "m/d/yyyy"), "3/1/2026");
        assert_eq!(n(46082.0, "d-mmm-yy"), "1-Mar-26");
        assert_eq!(n(46082.0, "dddd, mmmm d"), "Sunday, March 1");
        assert_eq!(n(46082.75, "h:mm AM/PM"), "6:00 PM");
        assert_eq!(n(0.5 + 1.0 / 86_400.0 * 5.0, "hh:mm:ss"), "12:00:05");
        assert_eq!(n(1.25, "[h]:mm"), "30:00");
        assert_eq!(n(46082.0, "yyyy\"년\" m\"월\" d\"일\""), "2026년 3월 1일");
        assert_eq!(n(60.0, "yyyy-mm-dd"), "1900-02-29");
        assert_eq!(n(61.0, "yyyy-mm-dd"), "1900-03-01");
        assert_eq!(n(1.0, "yyyy-mm-dd"), "1900-01-01");
        assert_eq!(serial_of(1900, 1, 1, false), 1.0);
    }

    #[test]
    fn text() {
        let t = |s: &str, code: &str| display(&CellValue::Text(s.into()), code, false);
        assert_eq!(t("00417", "@"), "00417");
        assert_eq!(t("abc", "0.00;-0.00;0;\"[\"@\"]\""), "[abc]");
        assert_eq!(t("abc", "#,##0"), "abc");
    }
}
