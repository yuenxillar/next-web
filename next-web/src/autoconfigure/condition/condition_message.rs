use std::fmt;

/// A message associated with a condition outcome.
///
/// Provides a fluent builder-style API so that condition messages stay
/// consistent across all conditions. A message may be empty, in which case it
/// is treated as "no message".
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConditionMessage {
    /// The rendered message. `None` means the message is empty.
    message: Option<String>,
}

impl ConditionMessage {
    /// Creates an empty message.
    pub fn empty() -> Self {
        Self { message: None }
    }

    /// Creates a message from a raw string.
    ///
    /// If `args` is non-empty, `message` is treated as a format string and the
    /// arguments are substituted positionally via `{}` placeholders.
    pub fn of(message: impl Into<String>, args: &[&str]) -> Self {
        let message = message.into();
        if args.is_empty() {
            Self {
                message: Some(message),
            }
        } else {
            let mut formatted = message;
            for arg in args {
                if let Some(pos) = formatted.find("{}") {
                    formatted.replace_range(pos..pos + 2, arg);
                }
            }
            Self {
                message: Some(formatted),
            }
        }
    }

    /// Creates a message by joining the given messages with `"; "`.
    ///
    /// Empty messages are skipped, matching the append semantics of the
    /// original implementation.
    pub fn of_messages(messages: &[ConditionMessage]) -> Self {
        let mut result = Self::empty();
        for message in messages {
            result = result.append_str(&message.to_string());
        }
        result
    }

    /// Returns `true` if the message is empty.
    pub fn is_empty(&self) -> bool {
        self.message.as_deref().map_or(true, |m| m.is_empty())
    }

    /// Appends a message, returning a new [`ConditionMessage`].
    ///
    /// Empty input is ignored. When both the existing message and the new one
    /// are non-empty, they are separated by a single space.
    pub fn append_str(&self, message: &str) -> Self {
        if message.is_empty() {
            return self.clone();
        }
        match &self.message {
            None => Self {
                message: Some(message.to_string()),
            },
            Some(existing) if existing.is_empty() => Self {
                message: Some(message.to_string()),
            },
            Some(existing) => Self {
                message: Some(format!("{existing} {message}")),
            },
        }
    }

    /// Returns a new builder rooted at this message for an additional condition.
    ///
    /// `condition` is the condition name (typically an annotation's short
    /// name), and `details` are optional trailing details.
    pub fn and_condition(&self, condition: &str, details: &[&str]) -> Builder {
        let detail = details.join(" ");
        let condition_text = if detail.is_empty() {
            condition.to_string()
        } else {
            format!("{condition} {detail}")
        };
        Builder {
            base: self.clone(),
            condition: condition_text,
        }
    }

    /// Factory for a builder rooted at an empty message.
    pub fn for_condition(condition: &str, details: &[&str]) -> Builder {
        ConditionMessage::empty().and_condition(condition, details)
    }
}

/// Renders the message, or an empty string when there is no message.
impl fmt::Display for ConditionMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.message {
            Some(message) => f.write_str(message),
            None => Ok(()),
        }
    }
}

/// Builder used to construct a [`ConditionMessage`] for a condition.
#[derive(Debug, Clone)]
pub struct Builder {
    /// The message accumulated so far.
    base: ConditionMessage,
    /// The condition text (e.g. `"@ConditionalOnClass Foo"`).
    condition: String,
}

impl Builder {
    /// Indicates an exact result was found.
    ///
    /// For example `found_exactly("foo")` yields `"found foo"`.
    pub fn found_exactly(&self, result: &str) -> ConditionMessage {
        self.found("", "").items(&[result])
    }

    /// Indicates one or more results were found.
    ///
    /// For example `found("bean", "beans").items(&["x"])` yields
    /// `"found bean x"`.
    pub fn found(&self, singular: &str, plural: &str) -> ItemsBuilder {
        ItemsBuilder {
            condition: self.clone(),
            reason: "found".to_string(),
            singular: singular.to_string(),
            plural: plural.to_string(),
        }
    }

    /// Indicates one or more results were not found.
    ///
    /// For example `did_not_find("bean", "beans").items(&["x", "y"])` yields
    /// `"did not find beans x, y"`.
    pub fn did_not_find(&self, singular: &str, plural: &str) -> ItemsBuilder {
        ItemsBuilder {
            condition: self.clone(),
            reason: "did not find".to_string(),
            singular: singular.to_string(),
            plural: plural.to_string(),
        }
    }

    /// Indicates a single result.
    ///
    /// For example `resulted_in("yes")` yields `"resulted in yes"`.
    pub fn resulted_in(&self, result: &str) -> ConditionMessage {
        self.because(&format!("resulted in {result}"))
    }

    /// Indicates something is available.
    ///
    /// For example `available("money")` yields `"money is available"`.
    pub fn available(&self, item: &str) -> ConditionMessage {
        self.because(&format!("{item} is available"))
    }

    /// Indicates something is not available.
    ///
    /// For example `not_available("time")` yields `"time is not available"`.
    pub fn not_available(&self, item: &str) -> ConditionMessage {
        self.because(&format!("{item} is not available"))
    }

    /// Indicates the reason for the message.
    ///
    /// For example `because("running Linux")` yields `"running Linux"`.
    pub fn because(&self, reason: &str) -> ConditionMessage {
        let text = if !reason.is_empty() {
            if self.condition.is_empty() {
                reason.to_string()
            } else {
                format!("{} {reason}", self.condition)
            }
        } else {
            self.condition.clone()
        };
        self.base.append_str(&text)
    }
}

/// Builder used to construct items for a condition message.
#[derive(Debug, Clone)]
pub struct ItemsBuilder {
    /// The parent builder.
    condition: Builder,
    /// The reason phrase, e.g. `"found"` or `"did not find"`.
    reason: String,
    /// The singular form of the item.
    singular: String,
    /// The plural form of the item.
    plural: String,
}

impl ItemsBuilder {
    /// Indicates no items are available.
    ///
    /// For example `did_not_find("any beans", "any beans").at_all()` yields
    /// `"did not find any beans"`.
    pub fn at_all(&self) -> ConditionMessage {
        self.items_with_style(Style::Normal, &[])
    }

    /// Indicates the items using the normal style.
    pub fn items(&self, items: &[&str]) -> ConditionMessage {
        self.items_with_style(Style::Normal, items)
    }

    /// Indicates the items using the given render style.
    pub fn items_with_style(&self, style: Style, items: &[&str]) -> ConditionMessage {
        let styled: Vec<String> = items.iter().map(|item| style.apply_to_item(item)).collect();
        let mut message = self.reason.clone();

        let use_singular = styled.len() <= 1 && !self.singular.is_empty();
        if use_singular {
            message.push(' ');
            message.push_str(&self.singular);
        } else if !self.plural.is_empty() {
            message.push(' ');
            message.push_str(&self.plural);
        }

        if !styled.is_empty() {
            message.push(' ');
            message.push_str(&styled.join(", "));
        }

        self.condition.because(&message)
    }
}

/// Render styles for message items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// Render with normal styling.
    Normal,
    /// Render with the item surrounded by single quotes.
    Quote,
}

impl Style {
    /// Applies this style to a single item.
    fn apply_to_item(&self, item: &str) -> String {
        match self {
            Style::Normal => item.to_string(),
            Style::Quote => format!("'{item}'"),
        }
    }
}
