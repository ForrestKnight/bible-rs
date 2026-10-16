//! A small, forgiving XML tokenizer for Bible markup (OSIS, Zefania, SWORD verse fragments,
//! API.Bible HTML).
//!
//! It never fails: SWORD stores one XML fragment per verse, and elements routinely open in one
//! verse and close in another, so end tags are not matched against start tags. Comments,
//! processing instructions and doctypes are skipped; CDATA becomes text; the five predefined
//! entities and numeric character references are decoded.

use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tok<'a> {
    Start { name: &'a str, attrs: &'a str },
    Empty { name: &'a str, attrs: &'a str },
    End { name: &'a str },
    Text(Cow<'a, str>),
}

pub struct Tokens<'a> {
    s: &'a str,
    pos: usize,
}

pub fn tokens(s: &str) -> Tokens<'_> {
    Tokens { s, pos: 0 }
}

/// Local name without a namespace prefix (`osis:verse` → `verse`).
fn local(name: &str) -> &str {
    name.rsplit_once(':').map_or(name, |(_, l)| l)
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Tok<'a>;

    fn next(&mut self) -> Option<Tok<'a>> {
        loop {
            let rest = &self.s[self.pos..];
            if rest.is_empty() {
                return None;
            }
            if !rest.starts_with('<') {
                let end = memchr::memchr(b'<', rest.as_bytes()).unwrap_or(rest.len());
                self.pos += end;
                return Some(Tok::Text(unescape(&rest[..end])));
            }
            if let Some(body) = rest.strip_prefix("<!--") {
                self.pos += 4 + body.find("-->").map_or(body.len(), |i| i + 3);
                continue;
            }
            if let Some(body) = rest.strip_prefix("<![CDATA[") {
                let end = body.find("]]>").unwrap_or(body.len());
                self.pos += 9 + (end + 3).min(body.len());
                return Some(Tok::Text(Cow::Borrowed(&body[..end])));
            }
            if rest.starts_with("<?") || rest.starts_with("<!") {
                let end = memchr::memchr(b'>', rest.as_bytes()).map_or(rest.len(), |i| i + 1);
                self.pos += end;
                continue;
            }
            // A tag: find the closing '>' outside quoted attribute values.
            let bytes = rest.as_bytes();
            let mut i = 1;
            let mut quote = 0u8;
            while i < bytes.len() {
                let c = bytes[i];
                if quote != 0 {
                    if c == quote {
                        quote = 0;
                    }
                } else if c == b'"' || c == b'\'' {
                    quote = c;
                } else if c == b'>' {
                    break;
                }
                i += 1;
            }
            if i >= bytes.len() {
                // Unterminated tag: treat the rest as text.
                self.pos = self.s.len();
                return Some(Tok::Text(unescape(rest)));
            }
            self.pos += i + 1;
            let inner = &rest[1..i];
            if let Some(name) = inner.strip_prefix('/') {
                return Some(Tok::End { name: local(name.trim()) });
            }
            let (inner, empty) = match inner.strip_suffix('/') {
                Some(x) => (x, true),
                None => (inner, false),
            };
            let name_end = inner.find(|c: char| c.is_whitespace()).unwrap_or(inner.len());
            let name = local(&inner[..name_end]);
            let attrs = &inner[name_end..];
            return Some(if empty { Tok::Empty { name, attrs } } else { Tok::Start { name, attrs } });
        }
    }
}

/// Looks up an attribute by local name in a tag's raw attribute string.
pub fn attr<'a>(attrs: &'a str, key: &str) -> Option<Cow<'a, str>> {
    let mut rest = attrs;
    loop {
        rest = rest.trim_start();
        if rest.is_empty() {
            return None;
        }
        let eq = rest.find('=')?;
        let name = rest[..eq].trim();
        let after = rest[eq + 1..].trim_start();
        let q = after.chars().next()?;
        let (value, next) = if q == '"' || q == '\'' {
            let close = after[1..].find(q)? + 1;
            (&after[1..close], &after[close + 1..])
        } else {
            let end = after.find(char::is_whitespace).unwrap_or(after.len());
            (&after[..end], &after[end..])
        };
        if local(name) == key {
            return Some(unescape(value));
        }
        rest = next;
    }
}

pub fn unescape(s: &str) -> Cow<'_, str> {
    if !s.contains('&') {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        let Some(semi) = tail[..tail.len().min(12)].find(';') else {
            out.push('&');
            rest = &tail[1..];
            continue;
        };
        let ent = &tail[1..semi];
        let decoded = match ent {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some('\u{a0}'),
            // HTML typography (the NLT API's passages).
            "lsquo" => Some('‘'),
            "rsquo" => Some('’'),
            "ldquo" => Some('“'),
            "rdquo" => Some('”'),
            "ndash" => Some('–'),
            "mdash" => Some('—'),
            "hellip" => Some('…'),
            _ if ent.starts_with("#x") || ent.starts_with("#X") => {
                u32::from_str_radix(&ent[2..], 16).ok().and_then(char::from_u32)
            }
            _ if ent.starts_with('#') => ent[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &tail[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_forgivingly() {
        let s = r#"<?xml version="1.0"?><!-- c --><osis:verse osisID="Gen.1.1" sID='a>b'/>In &amp; out&#8217;s <q who="Jesus">x</q></p><![CDATA[<raw>]]><bad"#;
        let toks: Vec<_> = tokens(s).collect();
        assert_eq!(toks[0], Tok::Empty { name: "verse", attrs: r#" osisID="Gen.1.1" sID='a>b'"# });
        assert_eq!(toks[1], Tok::Text(Cow::Owned("In & out’s ".into())));
        assert_eq!(toks[2], Tok::Start { name: "q", attrs: r#" who="Jesus""# });
        assert_eq!(toks[4], Tok::End { name: "q" });
        assert_eq!(toks[5], Tok::End { name: "p" });
        assert_eq!(toks[6], Tok::Text(Cow::Borrowed("<raw>")));
        assert_eq!(toks[7], Tok::Text(Cow::Borrowed("<bad")));
        assert_eq!(attr(r#" osisID="Gen.1.1" sID='a>b'"#, "sID").as_deref(), Some("a>b"));
        assert_eq!(attr(r#" a=1 b="x &lt; y""#, "b").as_deref(), Some("x < y"));
        assert_eq!(attr(r#" xml:lang="en""#, "lang").as_deref(), Some("en"));
        assert_eq!(unescape("a &unknown; b &#x41;"), "a &unknown; b A");
    }
}
