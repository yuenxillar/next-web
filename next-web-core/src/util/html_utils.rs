// Copyright 2002-present the original author or authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::borrow::Cow;

/// HTML 4.01 character entity references.
const NAMED_ENTITIES: &[(&str, char)] = &[
    ("amp", '&'),
    ("lt", '<'),
    ("gt", '>'),
    ("quot", '"'),
    ("apos", '\''),
    ("nbsp", '\u{00A0}'),
    ("copy", '\u{00A9}'),
    ("reg", '\u{00AE}'),
    ("trade", '\u{2122}'),
    ("hellip", '\u{2026}'),
    ("mdash", '\u{2014}'),
    ("ndash", '\u{2013}'),
    ("lsquo", '\u{2018}'),
    ("rsquo", '\u{2019}'),
    ("ldquo", '\u{201C}'),
    ("rdquo", '\u{201D}'),
    ("bull", '\u{2022}'),
    ("middot", '\u{00B7}'),
    ("laquo", '\u{00AB}'),
    ("raquo", '\u{00BB}'),
    ("euro", '\u{20AC}'),
    ("pound", '\u{00A3}'),
    ("yen", '\u{00A5}'),
    ("cent", '\u{00A2}'),
    ("sect", '\u{00A7}'),
    ("para", '\u{00B6}'),
    ("deg", '\u{00B0}'),
    ("plusmn", '\u{00B1}'),
    ("times", '\u{00D7}'),
    ("divide", '\u{00F7}'),
];

/// Utility class for HTML escaping.
///
/// Escapes and unescapes based on the W3C HTML 4.01 recommendation,
/// handling character entity references.
///
/// Equivalent to Spring's `org.springframework.web.util.HtmlUtils`.
pub struct HtmlUtils;

impl HtmlUtils {
    /// Returns the named entity for a character, if one exists.
    fn named_entity_for(ch: char) -> Option<&'static str> {
        match ch {
            '&' => Some("amp"),
            '<' => Some("lt"),
            '>' => Some("gt"),
            '"' => Some("quot"),
            '\'' => Some("apos"),
            '\u{00A0}' => Some("nbsp"),
            '\u{00A9}' => Some("copy"),
            '\u{00AE}' => Some("reg"),
            '\u{2122}' => Some("trade"),
            '\u{2026}' => Some("hellip"),
            '\u{2014}' => Some("mdash"),
            '\u{2013}' => Some("ndash"),
            '\u{2018}' => Some("lsquo"),
            '\u{2019}' => Some("rsquo"),
            '\u{201C}' => Some("ldquo"),
            '\u{201D}' => Some("rdquo"),
            '\u{2022}' => Some("bull"),
            '\u{00B7}' => Some("middot"),
            '\u{00AB}' => Some("laquo"),
            '\u{00BB}' => Some("raquo"),
            '\u{20AC}' => Some("euro"),
            '\u{00A3}' => Some("pound"),
            '\u{00A5}' => Some("yen"),
            '\u{00A2}' => Some("cent"),
            '\u{00A7}' => Some("sect"),
            '\u{00B6}' => Some("para"),
            '\u{00B0}' => Some("deg"),
            '\u{00B1}' => Some("plusmn"),
            '\u{00D7}' => Some("times"),
            '\u{00F7}' => Some("divide"),
            _ => None,
        }
    }

    /// Returns `true` if the input contains any character that needs escaping.
    #[inline]
    fn needs_escape(input: &str) -> bool {
        input.chars().any(|ch| Self::named_entity_for(ch).is_some())
    }

    /// Turn special characters into HTML character references.
    ///
    /// Handles the complete character set defined in the HTML 4.01 recommendation.
    /// Escapes all special characters to their corresponding entity reference
    /// (for example, `&lt;`).
    ///
    /// Uses `Cow` to avoid allocation when no escaping is needed.
    pub fn html_escape(input: &str) -> Cow<'_, str> {
        if !Self::needs_escape(input) {
            return Cow::Borrowed(input);
        }

        let mut escaped = String::with_capacity(input.len() * 2);
        for ch in input.chars() {
            if let Some(name) = Self::named_entity_for(ch) {
                escaped.push('&');
                escaped.push_str(name);
                escaped.push(';');
            } else {
                escaped.push(ch);
            }
        }
        Cow::Owned(escaped)
    }

    /// Turn special characters into HTML character references.
    ///
    /// Escapes all special characters to their corresponding numeric
    /// reference in decimal format (`&#Decimal;`).
    pub fn html_escape_decimal(input: &str) -> Cow<'_, str> {
        if !Self::needs_escape(input) {
            return Cow::Borrowed(input);
        }

        let mut escaped = String::with_capacity(input.len() * 2);
        for ch in input.chars() {
            if Self::named_entity_for(ch).is_some() {
                escaped.push_str("&#");
                escaped.push_str(&(ch as u32).to_string());
                escaped.push(';');
            } else {
                escaped.push(ch);
            }
        }
        Cow::Owned(escaped)
    }

    /// Turn special characters into HTML character references.
    ///
    /// Escapes all special characters to their corresponding numeric
    /// reference in hex format (`&#xHex;`).
    pub fn html_escape_hex(input: &str) -> Cow<'_, str> {
        if !Self::needs_escape(input) {
            return Cow::Borrowed(input);
        }

        let mut escaped = String::with_capacity(input.len() * 2);
        for ch in input.chars() {
            if Self::named_entity_for(ch).is_some() {
                escaped.push_str("&#x");
                // Uppercase hex, matching `Integer.toString(character, 16)`.
                escaped.push_str(&format!("{:X}", ch as u32));
                escaped.push(';');
            } else {
                escaped.push(ch);
            }
        }
        Cow::Owned(escaped)
    }

    /// Turn HTML character references into their plain text UNICODE equivalent.
    ///
    /// Handles the complete character set defined in the HTML 4.01 recommendation
    /// and all reference types (decimal, hex, and entity).
    ///
    /// Correctly converts the following formats:
    /// - `&Entity;` (example: `&amp;`) — case sensitive
    /// - `&#Decimal;` (example: `&#68;`)
    /// - `&#xHex;` (example: `&#xE5;`) — case insensitive
    ///
    /// Gracefully handles malformed character references by copying original
    /// characters as is when encountered.
    pub fn html_unescape(input: &str) -> Cow<'_, str> {
        if !input.contains('&') {
            return Cow::Borrowed(input);
        }

        let mut result = String::with_capacity(input.len());
        let mut chars = input.char_indices().peekable();

        while let Some((_, ch)) = chars.next() {
            if ch != '&' {
                result.push(ch);
                continue;
            }

            // Try to parse a reference starting at '&'.
            let mut body = String::new();
            let mut terminated = false;

            // Collect up to 32 characters for the reference body.
            while let Some(&(idx, c)) = chars.peek() {
                if c == ';' {
                    chars.next();
                    let _ = idx;
                    terminated = true;
                    break;
                }
                if body.len() >= 32 || c.is_whitespace() || c == '&' {
                    break;
                }
                body.push(c);
                chars.next();
            }

            let mut decoded: Option<char> = None;
            if terminated && !body.is_empty() {
                if let Some(stripped) = body.strip_prefix('#') {
                    let (radix, digits) = if let Some(hex) = stripped
                        .strip_prefix('x')
                        .or_else(|| stripped.strip_prefix('X'))
                    {
                        (16, hex)
                    } else {
                        (10, stripped)
                    };
                    if let Ok(code) = u32::from_str_radix(digits, radix) {
                        decoded = char::from_u32(code);
                    }
                } else if let Some((_, c)) = NAMED_ENTITIES.iter().find(|(name, _)| *name == body) {
                    decoded = Some(*c);
                }
            }

            if let Some(c) = decoded {
                result.push(c);
            } else {
                // Malformed: copy the original text as-is.
                result.push('&');
                result.push_str(&body);
                if terminated {
                    result.push(';');
                }
            }
        }

        Cow::Owned(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_basic() {
        assert_eq!(
            HtmlUtils::html_escape("<a href=\"x\">&</a>"),
            "&lt;a href=&quot;x&quot;&gt;&amp;&lt;/a&gt;"
        );
    }

    #[test]
    fn escape_no_change() {
        assert!(matches!(
            HtmlUtils::html_escape("hello world"),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn escape_decimal() {
        assert_eq!(HtmlUtils::html_escape_decimal("<"), "&#60;");
    }

    #[test]
    fn escape_hex() {
        assert_eq!(HtmlUtils::html_escape_hex("<"), "&#x3C;");
    }

    #[test]
    fn unescape_named() {
        assert_eq!(HtmlUtils::html_unescape("&lt;&amp;&gt;"), "<&>");
    }

    #[test]
    fn unescape_decimal() {
        assert_eq!(HtmlUtils::html_unescape("&#68;"), "D");
    }

    #[test]
    fn unescape_hex() {
        assert_eq!(HtmlUtils::html_unescape("&#xE5;"), "å");
    }

    #[test]
    fn unescape_malformed() {
        assert_eq!(HtmlUtils::html_unescape("&unknown;"), "&unknown;");
        assert_eq!(HtmlUtils::html_unescape("&"), "&");
    }

    #[test]
    fn unescape_no_amp() {
        assert!(matches!(
            HtmlUtils::html_unescape("hello"),
            Cow::Borrowed(_)
        ));
    }
}
