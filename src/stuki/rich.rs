//! Text with per-character colors: what the web app keeps as contentEditable HTML
//! (`x:=<span style="color: rgb(255, 0, 0);">1</span>`).

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Text color and highlight of one character, as `0xRRGGBB`.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct Style {
    pub color: Option<u32>,
    pub bg: Option<u32>,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Rich {
    pub text: String,
    /// One per character of `text`.
    pub styles: Vec<Style>,
}

impl Serialize for Rich {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_html())
    }
}

impl<'de> Deserialize<'de> for Rich {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Rich::from_html(&String::deserialize(d)?))
    }
}

/// `#rgb`, `#rrggbb` or `rgb(r, g, b)` → `0xRRGGBB`.
pub fn parse_color(s: &str) -> Option<u32> {
    let s = s.trim();
    if let Some(h) = s.strip_prefix('#') {
        let h = if h.len() == 3 {
            h.chars().flat_map(|c| [c, c]).collect()
        } else {
            h.get(..6)?.to_string()
        };
        return u32::from_str_radix(&h, 16).ok();
    }
    let inner = s
        .strip_prefix("rgba(")
        .or_else(|| s.strip_prefix("rgb("))?
        .trim_end_matches(')');
    let v: Vec<u32> = inner
        .split(',')
        .take(3)
        .map(|x| x.trim().parse::<f32>().map(|f| f.clamp(0.0, 255.0) as u32))
        .collect::<Result<_, _>>()
        .ok()?;
    (v.len() == 3).then(|| v[0] << 16 | v[1] << 8 | v[2])
}

pub fn hex(c: u32) -> String {
    format!("#{c:06x}")
}

fn css_rgb(c: u32) -> String {
    format!("rgb({}, {}, {})", c >> 16 & 255, c >> 8 & 255, c & 255)
}

fn escape(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\n' => out.push_str("<br>"),
            '\u{a0}' => out.push_str("&nbsp;"),
            c => out.push(c),
        }
    }
}

impl Rich {
    pub fn plain(s: &str) -> Rich {
        Rich {
            text: s.to_string(),
            styles: vec![Style::default(); s.chars().count()],
        }
    }

    /// `styles` padded / cut to the text length (after an edit, before [`Rich::edited`] runs).
    pub fn styles_for(&self, text: &str) -> Vec<Style> {
        let n = text.chars().count();
        let mut s = self.styles.clone();
        s.resize(n, Style::default());
        s
    }

    /// The text was changed from `old` (by typing): keep the colors of the untouched parts, and give
    /// typed characters the color of the character before them, as a browser does.
    pub fn edited(&mut self, old: &str) {
        let a: Vec<char> = old.chars().collect();
        let b: Vec<char> = self.text.chars().collect();
        let mut styles = self.styles.clone();
        styles.resize(a.len(), Style::default());
        let pre = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
        let max_suf = a.len().min(b.len()) - pre;
        let suf = a
            .iter()
            .rev()
            .zip(b.iter().rev())
            .take(max_suf)
            .take_while(|(x, y)| x == y)
            .count();
        let inherit = if pre > 0 {
            styles[pre - 1]
        } else {
            styles.get(pre).copied().unwrap_or_default()
        };
        let mut out = styles[..pre].to_vec();
        out.extend(std::iter::repeat_n(inherit, b.len() - pre - suf));
        out.extend_from_slice(&styles[a.len() - suf..]);
        self.styles = out;
    }

    /// Color characters `from..to` (character indices); an empty range colors everything.
    pub fn paint(&mut self, from: usize, to: usize, f: impl Fn(&mut Style)) {
        let n = self.text.chars().count();
        self.styles.resize(n, Style::default());
        let (mut a, mut b) = (from.min(to).min(n), to.max(from).min(n));
        if a == b {
            (a, b) = (0, n);
        }
        self.styles[a..b].iter_mut().for_each(f);
    }

    /// Runs of equally styled text.
    pub fn runs(&self) -> Vec<(String, Style)> {
        let mut out: Vec<(String, Style)> = Vec::new();
        for (i, c) in self.text.chars().enumerate() {
            let st = self.styles.get(i).copied().unwrap_or_default();
            match out.last_mut() {
                Some((s, last)) if *last == st => s.push(c),
                _ => out.push((c.to_string(), st)),
            }
        }
        out
    }

    pub fn to_html(&self) -> String {
        let mut out = String::new();
        for (s, st) in self.runs() {
            let mut css = Vec::new();
            if let Some(c) = st.color {
                css.push(format!("color: {};", css_rgb(c)));
            }
            if let Some(c) = st.bg {
                css.push(format!("background-color: {};", css_rgb(c)));
            }
            if css.is_empty() {
                escape(&s, &mut out);
            } else {
                out.push_str(&format!("<span style=\"{}\">", css.join(" ")));
                escape(&s, &mut out);
                out.push_str("</span>");
            }
        }
        out
    }

    /// Read contentEditable HTML: `<span style>` / `<font color>` colors, `<br>` and `<div>` lines,
    /// entities. Other tags are dropped, their text kept.
    pub fn from_html(html: &str) -> Rich {
        let mut text = String::new();
        let mut styles = Vec::new();
        // style in effect, and for each open tag the style to restore when it closes
        let mut cur = Style::default();
        let mut stack: Vec<(String, Style)> = Vec::new();
        let push = |text: &mut String, styles: &mut Vec<Style>, c: char, st: Style| {
            text.push(c);
            styles.push(st);
        };
        let newline = |text: &mut String, styles: &mut Vec<Style>| {
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
                styles.push(Style::default());
            }
        };
        let mut rest = html;
        while let Some(c) = rest.chars().next() {
            if c == '<'
                && let Some(end) = rest.find('>')
            {
                let tag = &rest[1..end];
                rest = &rest[end + 1..];
                let closing = tag.starts_with('/');
                let tag = tag.trim_start_matches('/').trim_end_matches('/');
                let name: String = tag
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric())
                    .collect::<String>()
                    .to_ascii_lowercase();
                if name == "br" {
                    text.push('\n');
                    styles.push(Style::default());
                    continue;
                }
                if closing {
                    if let Some(i) = stack.iter().rposition(|(n, _)| *n == name) {
                        cur = stack[i].1;
                        stack.truncate(i);
                    }
                    continue;
                }
                if matches!(name.as_str(), "div" | "p") {
                    newline(&mut text, &mut styles);
                }
                stack.push((name.clone(), cur));
                if let Some(style) = attr(tag, "style") {
                    for decl in style.split(';') {
                        if let Some((k, v)) = decl.split_once(':') {
                            match k.trim().to_ascii_lowercase().as_str() {
                                "color" => cur.color = parse_color(v).or(cur.color),
                                "background-color" | "background" => {
                                    cur.bg = parse_color(v).or(cur.bg)
                                }
                                _ => {}
                            }
                        }
                    }
                }
                if name == "font"
                    && let Some(c) = attr(tag, "color").and_then(|c| parse_color(&c))
                {
                    cur.color = Some(c);
                }
            } else if c == '&' {
                let end = rest.as_bytes()[..rest.len().min(12)]
                    .iter()
                    .position(|&b| b == b';');
                let (ch, len) = match end.map(|e| &rest[1..e]) {
                    Some("amp") => ('&', 5),
                    Some("lt") => ('<', 4),
                    Some("gt") => ('>', 4),
                    Some("quot") => ('"', 6),
                    Some("apos") => ('\'', 6),
                    Some("nbsp") => (' ', 6),
                    Some(e) if e.starts_with('#') => {
                        let n = if let Some(h) =
                            e.strip_prefix("#x").or_else(|| e.strip_prefix("#X"))
                        {
                            u32::from_str_radix(h, 16).ok()
                        } else {
                            e[1..].parse().ok()
                        };
                        match n.and_then(char::from_u32) {
                            Some(ch) => (if ch == '\u{a0}' { ' ' } else { ch }, e.len() + 2),
                            None => ('&', 1),
                        }
                    }
                    _ => ('&', 1),
                };
                push(&mut text, &mut styles, ch, cur);
                rest = &rest[len..];
            } else {
                let ch = if c == '\u{a0}' { ' ' } else { c };
                if ch != '\r' {
                    push(
                        &mut text,
                        &mut styles,
                        if ch == '\n' { ' ' } else { ch },
                        cur,
                    );
                }
                rest = &rest[c.len_utf8()..];
            }
        }
        // browsers leave a trailing <br> in edited fields
        while text.ends_with('\n') {
            text.pop();
            styles.pop();
        }
        Rich { text, styles }
    }
}

/// Value of attribute `name` in the inside of a tag (`span style="…"`).
fn attr(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = lower[from..].find(name) {
        let at = from + i;
        from = at + name.len();
        let before_ok = at == 0 || lower.as_bytes()[at - 1].is_ascii_whitespace();
        let after = lower[from..].trim_start();
        if !before_ok || !after.starts_with('=') {
            continue;
        }
        let v = tag[tag.len() - after.len() + 1..].trim_start();
        return Some(match v.chars().next() {
            Some(q @ ('"' | '\'')) => v[1..].split(q).next().unwrap_or("").to_string(),
            _ => v.split_whitespace().next().unwrap_or("").to_string(),
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_round_trip() {
        let mut r = Rich::plain("a<b & c\nd");
        r.paint(2, 3, |s| s.color = Some(0xff0000));
        r.paint(4, 5, |s| s.bg = Some(0x00ff00));
        let h = r.to_html();
        assert_eq!(Rich::from_html(&h), r);
    }

    #[test]
    fn reads_browser_html() {
        let r = Rich::from_html("x<div>y&nbsp;<font color=\"#00f\">z</font></div><div><br></div>");
        assert_eq!(r.text, "x\ny z");
        assert_eq!(r.styles[4].color, Some(0x0000ff));
        let r = Rich::from_html(
            "<span style=\"color: rgb(255, 0, 0); background-color: #ffb2b2\">a</span>b&#8712;&#x2200;",
        );
        assert_eq!(r.text, "ab∈∀");
        assert_eq!(
            r.styles[0],
            Style {
                color: Some(0xff0000),
                bg: Some(0xffb2b2)
            }
        );
        assert_eq!(r.styles[1], Style::default());
        assert_eq!(Rich::from_html("a &amp b <").text, "a &amp b <");
    }

    #[test]
    fn typing_keeps_colors() {
        let mut r = Rich::plain("abcd");
        r.paint(1, 3, |s| s.color = Some(1)); // b c
        let old = r.text.clone();
        r.text = "abXcd".into();
        r.edited(&old);
        let c: Vec<_> = r.styles.iter().map(|s| s.color).collect();
        assert_eq!(c, vec![None, Some(1), Some(1), Some(1), None]);
        let old = r.text.clone();
        r.text = "ad".into();
        r.edited(&old);
        assert_eq!(r.styles.len(), 2);
        assert_eq!(r.styles[1].color, None);
    }
}
