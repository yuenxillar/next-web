//! Port of `java.text.MessageFormat`, limited to the features a message source
//! needs.
//!
//! A pattern is a sequence of literal text and of argument references:
//!
//! ```text
//! Your name is {1}, {0}.
//! ```
//!
//! Single quotes start and end a literal region in which `{` and `}` lose their
//! meaning, and `''` produces a single quote, exactly like the JDK does.
//! Argument references may carry a type (`{0,number}`); since Rust arguments
//! already know how to format themselves, the type only documents the intent
//! and the argument is rendered through its [`Display`] implementation.
//!
//! # Examples
//!
//! ```rust
//! use next_web_context::util::MessageFormat;
//!
//! let format = MessageFormat::new("Hello {0}, you are {1}.");
//! assert_eq!(format.format(&[&"Rust", &2024]), "Hello Rust, you are 2024.");
//! ```

use std::fmt::{self, Display, Write};

/// A compiled message pattern.
#[derive(Debug, Clone)]
pub struct MessageFormat {
    pattern: String,
    segments: Vec<Segment>,
}

/// One piece of a compiled pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    /// Text rendered as it is.
    Literal(String),

    /// A reference to the argument with the given index.
    Argument {
        index: usize,
        kind: ArgumentKind,
        placeholder: String,
    },
}

/// The type of an argument reference, as declared by the pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgumentKind {
    /// `{0}`, without a type.
    Unformatted,

    /// `{0,number}`.
    Number,

    /// `{0,date}`.
    Date,

    /// `{0,time}`.
    Time,

    /// `{0,choice}`, or `{0,choice,...}` with a choice style.
    Choice,
}

impl MessageFormat {
    /// Compiles the given pattern.
    ///
    /// # Arguments
    ///
    /// * `pattern` - The pattern to compile.
    pub fn new(pattern: impl Into<String>) -> Self {
        let pattern = pattern.into();
        let segments = parse(&pattern);

        Self { pattern, segments }
    }

    /// Returns the pattern this instance was compiled from.
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Returns the indexes of the arguments the pattern references, in the
    /// order they appear.
    pub fn argument_indexes(&self) -> Vec<usize> {
        self.segments
            .iter()
            .filter_map(|segment| match segment {
                Segment::Literal(_) => None,
                Segment::Argument { index, .. } => Some(*index),
            })
            .collect()
    }

    /// Renders the pattern with the given arguments.
    ///
    /// An argument reference without a matching argument is kept verbatim, so a
    /// message that declares more arguments than the caller supplies is still
    /// rendered instead of failing.
    ///
    /// # Arguments
    ///
    /// * `args` - The arguments the references are resolved against.
    pub fn format(&self, args: &[&dyn Display]) -> String {
        let mut rendered = String::with_capacity(self.pattern.len());

        for segment in &self.segments {
            match segment {
                Segment::Literal(literal) => rendered.push_str(literal),
                Segment::Argument {
                    index,
                    kind,
                    placeholder,
                } => {
                    let Some(argument) = args.get(*index) else {
                        rendered.push_str(placeholder);
                        continue;
                    };

                    match kind {
                        // The JDK hands the argument to the `Format` delegate
                        // registered for the declared type. A Rust argument
                        // carries its own formatting, which is its `Display`
                        // implementation here.
                        ArgumentKind::Unformatted
                        | ArgumentKind::Number
                        | ArgumentKind::Date
                        | ArgumentKind::Time
                        | ArgumentKind::Choice => {
                            let _ = write!(rendered, "{argument}");
                        }
                    }
                }
            }
        }

        rendered
    }
}

impl Display for MessageFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.pattern)
    }
}

impl From<&str> for MessageFormat {
    fn from(pattern: &str) -> Self {
        Self::new(pattern)
    }
}

impl From<String> for MessageFormat {
    fn from(pattern: String) -> Self {
        Self::new(pattern)
    }
}

/// Splits a pattern into its literal and argument segments.
fn parse(pattern: &str) -> Vec<Segment> {
    let characters = pattern.char_indices().collect::<Vec<_>>();
    let mut segments = Vec::new();
    let mut literal = String::new();
    let mut position = 0;

    while position < characters.len() {
        let (index, character) = characters[position];

        match character {
            '\'' => {
                // `''` is a single quote, a single `'` starts a quoted region
                // whose end is the next single quote.
                if characters.get(position + 1).map(|(_, next)| *next) == Some('\'') {
                    literal.push('\'');
                    position += 2;
                    continue;
                }

                position += 1;
                while position < characters.len() {
                    let (_, quoted) = characters[position];
                    if quoted == '\'' {
                        if characters.get(position + 1).map(|(_, next)| *next) == Some('\'') {
                            literal.push('\'');
                            position += 2;
                            continue;
                        }

                        position += 1;
                        break;
                    }

                    literal.push(quoted);
                    position += 1;
                }
            }
            '{' => {
                let Some(end) = find_argument_end(&characters, position) else {
                    literal.push(character);
                    position += 1;
                    continue;
                };

                let (end_index, _) = characters[end];
                let content = &pattern[index + character.len_utf8()..end_index];
                let (argument_index, kind) = parse_argument(content);

                if !literal.is_empty() {
                    segments.push(Segment::Literal(std::mem::take(&mut literal)));
                }

                segments.push(Segment::Argument {
                    index: argument_index,
                    kind,
                    placeholder: pattern[index..end_index + 1].to_string(),
                });

                position = end + 1;
            }
            _ => {
                literal.push(character);
                position += 1;
            }
        }
    }

    if !literal.is_empty() {
        segments.push(Segment::Literal(literal));
    }

    segments
}

/// Returns the position of the `}` closing the argument that starts at
/// `start`, or `None` when the argument is not closed.
fn find_argument_end(characters: &[(usize, char)], start: usize) -> Option<usize> {
    let mut position = start + 1;
    let mut quoted = false;

    while position < characters.len() {
        match characters[position].1 {
            '\'' => quoted = !quoted,
            '}' if !quoted => return Some(position),
            _ => {}
        }

        position += 1;
    }

    None
}

/// Reads the index and the type of an argument reference.
fn parse_argument(content: &str) -> (usize, ArgumentKind) {
    let mut parts = content.split(',');
    let index = parts
        .next()
        .map(str::trim)
        .and_then(|index| index.parse::<usize>().ok())
        .unwrap_or_default();

    let kind = match parts.next().map(str::trim) {
        Some(kind) if kind.eq_ignore_ascii_case("number") => ArgumentKind::Number,
        Some(kind) if kind.eq_ignore_ascii_case("date") => ArgumentKind::Date,
        Some(kind) if kind.eq_ignore_ascii_case("time") => ArgumentKind::Time,
        Some(kind) if kind.eq_ignore_ascii_case("choice") => ArgumentKind::Choice,
        _ => ArgumentKind::Unformatted,
    };

    (index, kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_arguments_in_any_order() {
        let format = MessageFormat::new("Your name is {2}, {1}, {0}.");

        assert_eq!(
            format.format(&[&"first", &"second", &"third"]),
            "Your name is third, second, first."
        );
        assert_eq!(format.argument_indexes(), [2, 1, 0]);
    }

    #[test]
    fn renders_arguments_without_arguments() {
        let format = MessageFormat::new("Hello, world!");
        assert_eq!(format.format(&[]), "Hello, world!");
    }

    #[test]
    fn keeps_missing_arguments_verbatim() {
        let format = MessageFormat::new("Hello {0}, bye {1}.");
        assert_eq!(format.format(&[&"Rust"]), "Hello Rust, bye {1}.");
    }

    #[test]
    fn applies_the_jdk_quoting_rules() {
        assert_eq!(MessageFormat::new("'{0}'").format(&[&"x"]), "{0}");
        assert_eq!(MessageFormat::new("''{0}''").format(&[&"x"]), "'x'");
        assert_eq!(
            MessageFormat::new("a 'quoted' text").format(&[]),
            "a quoted text"
        );
    }

    #[test]
    fn records_the_declared_argument_kind() {
        assert_eq!(
            parse_argument(" 0 , number , integer "),
            (0, ArgumentKind::Number)
        );
        assert_eq!(parse_argument("1"), (1, ArgumentKind::Unformatted));

        let format = MessageFormat::new("{0,number} files");
        assert_eq!(format.format(&[&42]), "42 files");
    }

    #[test]
    fn keeps_unclosed_arguments_verbatim() {
        assert_eq!(MessageFormat::new("Hello {0").format(&[]), "Hello {0");
    }
}
