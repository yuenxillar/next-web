use std::{collections::HashMap, ops::Deref};

use crate::util::MimeType;

pub struct MimeTypeUtils;

impl MimeTypeUtils {
    pub fn tokenize(s: &str) -> Vec<&str> {
        if s.is_empty() {
            return Vec::new();
        }

        let mut tokens = Vec::new();
        let mut in_quotes = false;
        let mut start_index = 0;
        let mut i = 0;

        while i < s.len() {
            let ch = s[i..].chars().next().unwrap();
            match ch {
                '"' => in_quotes = !in_quotes,
                ',' if !in_quotes => {
                    tokens.push(&s[start_index..i]);
                    start_index = i + 1;
                }
                '\\' => {
                    i += ch.len_utf8();
                    if i < s.len() {
                        let escaped = s[i..].chars().next().unwrap();
                        i += escaped.len_utf8();
                        continue;
                    }
                }
                _ => {}
            }
            i += ch.len_utf8();
        }

        tokens.push(&s[start_index..]);
        tokens
    }

    pub fn to_string(mime_types: Vec<&MimeType>) -> String {
        let mut builder = String::new();
        for (index, mime_type) in mime_types.iter().enumerate() {
            if index > 0 {
                builder.push_str(", ");
            }
            builder.push_str(&mime_type.to_string());
        }
        builder
    }

    pub fn parse_mime_type(mime_type: &str) -> MimeType {
        let trimmed = mime_type.trim();
        if trimmed.is_empty() {
            panic!("'mimeType' must not be empty");
        }

        let first_semicolon = mime_type.find(';');
        let full_type = match first_semicolon {
            Some(index) => mime_type[..index].trim(),
            None => trimmed,
        };

        if full_type.is_empty() {
            panic!("'mimeType' must not be empty");
        }

        let full_type = if full_type == "*" { "*/*" } else { full_type };

        let sub_index = match full_type.find('/') {
            Some(index) => index,
            None => panic!("Invalid MimeType '{}': does not contain '/'", mime_type),
        };
        if sub_index == full_type.len() - 1 {
            panic!(
                "Invalid MimeType '{}': does not contain subtype after '/'",
                mime_type
            );
        }

        let type_ = &full_type[..sub_index];
        let subtype = &full_type[sub_index + 1..];
        if type_ == "*" && subtype != "*" {
            panic!(
                "Invalid MimeType '{}': wildcard type is legal only in '*/*' (all mime types)",
                mime_type
            );
        }

        let mut parameters: HashMap<String, String> = HashMap::new();
        let len = mime_type.len();
        let mut index: isize = first_semicolon.map(|i| i as isize).unwrap_or(-1);

        loop {
            let start = (index + 1) as usize;
            let next_index = scan_next_semicolon(mime_type, start);
            let parameter = mime_type[start..next_index].trim();
            if !parameter.is_empty() {
                if let Some(eq_index) = parameter.find('=') {
                    let attribute = parameter[..eq_index].trim();
                    let value = parameter[eq_index + 1..].trim();
                    parameters.insert(attribute.to_string(), value.to_string());
                }
            }

            index = next_index as isize;
            if index as usize >= len {
                break;
            }
        }

        MimeType::with_parameters(type_, subtype, parameters)
    }

    pub fn sort_by_specificity<T>(mime_types: &mut Vec<T>) -> Result<(), &'static str>
    where
        T: Deref<Target = MimeType>,
    {
        if mime_types.len() > 50 {
            return Err("Too many elements");
        }

        let len = mime_types.len();
        let mut i = 0;
        while i < len {
            let mut j = 1;
            while j < len - i {
                let swap = {
                    let prev = &mime_types[j - 1];
                    let cur = &mime_types[j];
                    is_less_specific(&*prev, &*cur)
                };
                if swap {
                    mime_types.swap(j - 1, j);
                }
                j += 1;
            }
            i += 1;
        }

        Ok(())
    }
}

fn is_less_specific(a: &MimeType, b: &MimeType) -> bool {
    if a.is_wildcard_type() && !b.is_wildcard_type() {
        return true;
    }
    if b.is_wildcard_type() && !a.is_wildcard_type() {
        return false;
    }
    if a.is_wildcard_subtype() && !b.is_wildcard_subtype() {
        return true;
    }
    if b.is_wildcard_subtype() && !a.is_wildcard_subtype() {
        return false;
    }
    a.get_parameters().len() < b.get_parameters().len()
}

fn scan_next_semicolon(mime_type: &str, start: usize) -> usize {
    let mut index = start;
    let mut quoted = false;

    while index < mime_type.len() {
        let ch = mime_type[index..].chars().next().unwrap();
        if ch == ';' {
            if !quoted {
                return index;
            }
        } else if ch == '"' {
            let escaped = index > 0 && mime_type[..index].chars().next_back() == Some('\\');
            if !escaped {
                quoted = !quoted;
            }
        }
        index += ch.len_utf8();
    }

    index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_mime_type() {
        let mime_type = MimeTypeUtils::parse_mime_type("text/plain");
        assert_eq!(mime_type.get_type(), "text");
        assert_eq!(mime_type.get_subtype(), "plain");
    }

    #[test]
    fn parses_mime_type_with_parameters() {
        let mime_type = MimeTypeUtils::parse_mime_type("text/html; charset=UTF-8");
        assert_eq!(mime_type.get_type(), "text");
        assert_eq!(mime_type.get_subtype(), "html");
        assert_eq!(
            mime_type.get_parameter("charset"),
            Some(&"UTF-8".to_string())
        );
    }

    #[test]
    fn parses_wildcard_all() {
        let mime_type = MimeTypeUtils::parse_mime_type("*");
        assert!(mime_type.is_wildcard_type());
        assert!(mime_type.is_wildcard_subtype());
    }

    #[test]
    fn parse_rejects_missing_slash() {
        assert!(std::panic::catch_unwind(|| MimeTypeUtils::parse_mime_type("text")).is_err());
    }

    #[test]
    fn tokenizes_comma_separated_with_quotes() {
        let tokens = MimeTypeUtils::tokenize("a/b, \"c/d\",e/f");
        assert_eq!(tokens, vec!["a/b", " \"c/d\"", "e/f"]);
    }

    #[test]
    fn tokenize_keeps_commas_inside_quotes() {
        let tokens = MimeTypeUtils::tokenize("\"a,b/c\"");
        assert_eq!(tokens, vec!["\"a,b/c\""]);
    }

    #[test]
    fn sorts_by_specificity() {
        let json = MimeType::with_subtype("application", "json");
        let json_utf8 = MimeType::with_parameters(
            "application",
            "json",
            HashMap::from([("charset".to_string(), "UTF-8".to_string())]),
        );
        let wildcard = MimeType::with_subtype("*", "*");

        let mut types = vec![&wildcard, &json, &json_utf8];
        MimeTypeUtils::sort_by_specificity(&mut types).unwrap();

        assert_eq!(
            types,
            vec![&json_utf8, &json, &wildcard],
            "most specific should come first"
        );
    }

    #[test]
    fn rejects_too_many_elements() {
        let mime_type = MimeType::with_subtype("application", "json");
        let mut types = vec![&mime_type; 51];
        assert_eq!(
            MimeTypeUtils::sort_by_specificity(&mut types),
            Err("Too many elements")
        );
    }
}
