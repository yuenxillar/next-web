use crate::env::{ParseError, Profiles};

/// Internal parser used by Profiles.of.
pub struct ProfilesParser;

impl ProfilesParser {
    /// Create a new [`Profiles`] from the given profile expressions.
    ///
    /// The returned instance matches if **any** of the expressions matches.
    ///
    /// A profile expression is either a simple profile name (for example
    /// `"production"`) or a compound expression (for example
    /// `"production & cloud"`).
    ///
    /// The following operators are supported:
    ///
    /// - `!` — logical NOT of a profile name or compound expression
    /// - `&` — logical AND of profile names or compound expressions
    /// - `|` — logical OR of profile names or compound expressions
    ///
    /// The `&` and `|` operators may not be mixed without parentheses.
    ///
    /// # Errors
    ///
    /// Returns a [`ParseError`] if any expression is malformed.
    pub fn parse(profile_expressions: &[&str]) -> Result<Box<dyn Profiles>, ParseError> {
        if profile_expressions.is_empty() {
            return Err(ParseError::with_message(
                "must specify at least one profile expression",
            ));
        }

        let mut parsed = Vec::with_capacity(profile_expressions.len());
        for expr in profile_expressions {
            parsed.push(Self::parse_expression(expr)?);
        }

        Ok(Box::new(ParsedProfiles::new(Expr::Or(parsed))))
    }

    fn parse_expression(expression: &str) -> Result<Expr, ParseError> {
        if expression.trim().is_empty() {
            return Err(ParseError::new(expression.to_owned(), "must contain text"));
        }
        let tokens = Self::tokenize(expression);
        let mut parser = TokenParser {
            expression,
            tokens,
            pos: 0,
        };
        parser.parse_tokens(Context::None)
    }

    /// Split `expression` into tokens, keeping the delimiters `( ) & | !`.
    fn tokenize(expression: &str) -> Vec<&str> {
        let mut tokens = Vec::new();
        let mut start = 0usize;

        for (idx, ch) in expression.char_indices() {
            if matches!(ch, '(' | ')' | '&' | '|' | '!') {
                if start < idx {
                    tokens.push(&expression[start..idx]);
                }
                tokens.push(&expression[idx..idx + ch.len_utf8()]);
                start = idx + ch.len_utf8();
            }
        }
        if start < expression.len() {
            tokens.push(&expression[start..]);
        }
        tokens
    }
}

struct ParsedProfiles {
    /// Parsed expression tree. Multiple top-level expressions are OR-ed.
    expr: Expr,
}

impl ParsedProfiles {
    /// Create a new `ParsedProfiles`.
    ///
    /// The `source` expressions are deduplicated while preserving their
    /// first-seen order, matching `LinkedHashSet` semantics.
    pub fn new(expr: Expr) -> Self {
        Self { expr }
    }
}

impl Profiles for ParsedProfiles {
    fn matches(&self, is_profile_active: &dyn Fn(&str) -> bool) -> bool {
        self.expr.matches(is_profile_active)
    }
}

#[derive(Debug, Clone)]
enum Expr {
    Name(String),
    Not(Box<Expr>),
    And(Vec<Expr>),
    Or(Vec<Expr>),
}

impl Expr {
    fn matches(&self, is_profile_active: &dyn Fn(&str) -> bool) -> bool {
        match self {
            Self::Name(name) => is_profile_active(name),
            Self::Not(inner) => !inner.matches(is_profile_active),
            Self::And(children) => children.iter().all(|e| e.matches(is_profile_active)),
            Self::Or(children) => children.iter().any(|e| e.matches(is_profile_active)),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Context {
    None,
    Negate,
    Parenthesis,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Operator {
    And,
    Or,
}

struct TokenParser<'a> {
    expression: &'a str,
    tokens: Vec<&'a str>,
    pos: usize,
}

impl<'a> TokenParser<'a> {
    fn next_token(&mut self) -> Option<&'a str> {
        while self.pos < self.tokens.len() {
            let token = self.tokens[self.pos].trim();
            self.pos += 1;
            if !token.is_empty() {
                return Some(token);
            }
        }
        None
    }

    fn parse_tokens(&mut self, context: Context) -> Result<Expr, ParseError> {
        let mut elements: Vec<Expr> = Vec::new();
        let mut operator: Option<Operator> = None;

        while let Some(token) = self.next_token() {
            match token {
                "(" => {
                    let contents = self.parse_tokens(Context::Parenthesis)?;
                    if context == Context::Negate {
                        return Ok(contents);
                    }
                    elements.push(contents);
                }
                "&" => {
                    if !(operator.is_none() || operator == Some(Operator::And)) {
                        return Err(self.malformed());
                    }
                    operator = Some(Operator::And);
                }
                "|" => {
                    if !(operator.is_none() || operator == Some(Operator::Or)) {
                        return Err(self.malformed());
                    }
                    operator = Some(Operator::Or);
                }
                "!" => {
                    let inner = self.parse_tokens(Context::Negate)?;
                    elements.push(Expr::Not(Box::new(inner)));
                }
                ")" => {
                    let merged = self.merge(elements, operator)?;
                    if context == Context::Parenthesis {
                        return Ok(merged);
                    }
                    elements = vec![merged];
                    operator = None;
                }
                name => {
                    let value = Expr::Name(name.to_owned());
                    if context == Context::Negate {
                        return Ok(value);
                    }
                    elements.push(value);
                }
            }
        }

        self.merge(elements, operator)
    }

    fn merge(
        &self,
        mut elements: Vec<Expr>,
        operator: Option<Operator>,
    ) -> Result<Expr, ParseError> {
        if elements.is_empty() {
            return Err(self.malformed());
        }
        if elements.len() == 1 {
            return Ok(elements.pop().unwrap());
        }
        match operator {
            Some(Operator::And) => Ok(Expr::And(elements)),
            _ => Ok(Expr::Or(elements)),
        }
    }

    fn malformed(&self) -> ParseError {
        ParseError::new(self.expression.to_owned(), "malformed profile expression")
    }
}
