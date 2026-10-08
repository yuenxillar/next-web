//! Implementation of the `#[pre_authorize]` attribute macro.
//!
//! The macro turns a declarative access-control rule into a runtime guard that
//! runs before the annotated handler and returns `403 Forbidden` when the
//! current authentication does not satisfy the rule.
//!
//! Two notations are supported.
//!
//! The Spring Security style expression string:
//!
//! ```rust,ignore
//! #[pre_authorize("has_role('ADMIN') and has_authority('order:read')")]
//! #[pre_authorize("has_any_role('ADMIN', 'AUDITOR') or has_authority('order:read')")]
//! #[pre_authorize("!has_role('BANNED')")]
//! #[pre_authorize("has_role('ADMIN') and !has_authority('order:delete')")]
//! ```
//!
//! And the structured key/value form:
//!
//! ```rust,ignore
//! #[pre_authorize(has_role = "ADMIN")]
//! #[pre_authorize(has_any_role = ["ADMIN", "AUDITOR"])]
//! #[pre_authorize(has_authority = "order:read")]
//! #[pre_authorize(has_any_authority = ["order:read", "order:write"])]
//! #[pre_authorize(
//!     has_role = "ADMIN",
//!     has_authority = "order:read",
//!     mode = "And"
//! )]
//! #[pre_authorize(has_role = "ADMIN", negate = true)]
//! ```
//!
//! The structured form combines every configured check with `mode` (`And` by
//! default, `Or` selects "any of them"). `negate = true` inverts the whole rule,
//! which is how "the user must *not* have this role/permission" is expressed.
//!
//! # Placement
//!
//! `#[pre_authorize]` must be the outermost handler attribute, above the
//! `#[get_mapping]`/`#[post_mapping]`/... that registers the handler, exactly
//! like `#[idempotency]`. The routing macros rebuild the function and only
//! re-emit the attributes they receive, so an inner `#[pre_authorize]` would be
//! dropped.
//!
//! # Behaviour
//!
//! The annotation rewrites the handler so that it returns
//! `impl IntoResponse`: the guard runs first and answers `403 Forbidden` when
//! the rule does not hold, otherwise the original body runs and its result is
//! converted into the response.

use from_attr::FromAttr;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{spanned::Spanned, ItemFn, LitStr};

use crate::util::{field_type::FieldType, logic::Logic};

use super::attrs::pre_authorize_attr::PreAuthorizeAttr;

pub fn impl_macro_pre_authorize(macro_attrs: TokenStream, item_fn: ItemFn) -> TokenStream {
    Logic::generate(|| {
        let vis = &item_fn.vis;
        let attrs = &item_fn.attrs;
        let block = &item_fn.block;

        if item_fn.sig.asyncness.is_none() {
            return Err(syn::Error::new(
                item_fn.sig.span(),
                "Function must be declared as async",
            ));
        }

        if matches!(item_fn.sig.output, syn::ReturnType::Default) {
            return Err(syn::Error::new(
                item_fn.sig.output.span(),
                "Function has no return type. Cannot be used as handler",
            ));
        }

        let expression = match parse_attribute(macro_attrs.clone())? {
            ParsedAttribute::Ignore => {
                let sig = &item_fn.sig;
                return Ok(quote! { #(#attrs)* #vis #sig #block });
            }
            ParsedAttribute::Rule(expression) => expression,
        };

        // Rewriting the return type to `impl IntoResponse` lets the guard - and
        // the existing routing macro, which performs the same rewrite - return a
        // rejection response regardless of the concrete type the handler
        // produces. The original `Result<_, _>` type is re-applied to the body
        // so that the error type is still inferable.
        let result_type = match &item_fn.sig.output {
            syn::ReturnType::Type(_, typ) if FieldType::is_result(typ) => quote!(: #typ),
            _ => TokenStream2::new(),
        };

        let mut sig = item_fn.sig.clone();
        sig.output = syn::parse_quote!(-> impl ::next_web::response::IntoResponse);

        let expression = expression.into_tokens();

        Ok(quote! {
            #(#attrs)*
            #vis #sig {
                const __NEXT_WEB_PRE_AUTHORIZE_EXPRESSION:
                    ::next_web::security::access::prepost::pre_authorize::PreAuthorizeExpression =
                    #expression;

                if !::next_web::security::access::prepost::pre_authorize::evaluate(
                    &__NEXT_WEB_PRE_AUTHORIZE_EXPRESSION,
                ) {
                    return ::next_web::response::IntoResponse::into_response(
                        ::next_web::http::StatusCode::FORBIDDEN,
                    );
                }

                let __next_web_pre_authorize_result #result_type = (async move #block).await;

                ::next_web::response::IntoResponse::into_response(
                    __next_web_pre_authorize_result,
                )
            }
        })
    })
}

/// The outcome of parsing the attribute arguments.
enum ParsedAttribute {
    /// `#[pre_authorize(ignore = true)]` - the handler is emitted untouched.
    Ignore,
    /// A compiled access-control rule.
    Rule(AuthExpr),
}

fn parse_attribute(macro_attrs: TokenStream) -> Result<ParsedAttribute, syn::Error> {
    let tokens: TokenStream2 = macro_attrs.into();

    // `#[pre_authorize("has_role('ADMIN')")]`
    if let Ok(expression) = syn::parse2::<LitStr>(tokens.clone()) {
        let parsed = expr::parse(&expression.value())
            .map_err(|error| syn::Error::new(expression.span(), error))?;
        return Ok(ParsedAttribute::Rule(parsed));
    }

    let attribute = PreAuthorizeAttr::from_tokens(tokens)?;

    if attribute.ignore.unwrap_or(false) {
        return Ok(ParsedAttribute::Ignore);
    }

    parse_structured_attribute(&attribute).map(ParsedAttribute::Rule)
}

fn parse_structured_attribute(attribute: &PreAuthorizeAttr) -> Result<AuthExpr, syn::Error> {
    if let Some(basic) = &attribute.basic {
        return Err(syn::Error::new(
            basic.span(),
            "`basic` is not supported by #[pre_authorize]; express the rule with \
             has_role/has_authority instead",
        ));
    }

    let mode = match attribute
        .mode
        .as_ref()
        .map(|mode| mode.value().trim().to_ascii_lowercase())
    {
        None => Combination::And,
        Some(mode) if mode == "and" => Combination::And,
        Some(mode) if mode == "or" => Combination::Or,
        Some(_) => {
            return Err(syn::Error::new(
                attribute.mode.as_ref().unwrap().span(),
                "`mode` must be either \"And\" or \"Or\"",
            ))
        }
    };

    if let Some(expression) = &attribute.expr {
        if has_structured_checks(attribute) {
            return Err(syn::Error::new(
                expression.span(),
                "`expr` cannot be combined with has_role/has_authority/has_any_role/\
                 has_any_authority/role/permission",
            ));
        }

        let parsed = expr::parse(&expression.value())
            .map_err(|error| syn::Error::new(expression.span(), error))?;
        return Ok(apply_negation(parsed, attribute.negate.unwrap_or(false)));
    }

    let mut checks = Vec::new();

    if let Some(role) = &attribute.has_role {
        checks.push(AuthExpr::HasRole(role.value()));
    }

    if let Some(roles) = &attribute.has_any_role {
        checks.push(AuthExpr::HasAnyRole(values(roles)?));
    }

    if let Some(authority) = &attribute.has_authority {
        checks.push(AuthExpr::HasAuthority(authority.value()));
    }

    if let Some(authorities) = &attribute.has_any_authority {
        checks.push(AuthExpr::HasAnyAuthority(values(authorities)?));
    }

    if let Some(roles) = &attribute.role {
        let roles = values(roles)?;
        checks.push(combine(
            roles.into_iter().map(AuthExpr::HasRole).collect(),
            mode,
        ));
    }

    if let Some(permissions) = &attribute.permission {
        let permissions = values(permissions)?;
        checks.push(combine(
            permissions
                .into_iter()
                .map(AuthExpr::HasAuthority)
                .collect(),
            mode,
        ));
    }

    if checks.is_empty() {
        return Err(syn::Error::new(
            Span::call_site(),
            "#[pre_authorize] requires at least one check, for example \
             #[pre_authorize(has_role = \"ADMIN\")] or \
             #[pre_authorize(\"has_role('ADMIN')\")]",
        ));
    }

    let combined = combine(checks, mode);
    Ok(apply_negation(combined, attribute.negate.unwrap_or(false)))
}

fn has_structured_checks(attribute: &PreAuthorizeAttr) -> bool {
    attribute.has_role.is_some()
        || attribute.has_any_role.is_some()
        || attribute.has_authority.is_some()
        || attribute.has_any_authority.is_some()
        || attribute.role.is_some()
        || attribute.permission.is_some()
}

fn apply_negation(expression: AuthExpr, negate: bool) -> AuthExpr {
    if negate {
        AuthExpr::Not(Box::new(expression))
    } else {
        expression
    }
}

fn values(values: &[LitStr]) -> Result<Vec<String>, syn::Error> {
    if values.is_empty() {
        return Err(syn::Error::new(
            Span::call_site(),
            "#[pre_authorize] requires a non-empty list of roles or authorities",
        ));
    }

    Ok(values
        .iter()
        .map(|value| value.value().trim().to_string())
        .collect())
}

#[derive(Clone, Copy)]
enum Combination {
    And,
    Or,
}

fn combine(mut checks: Vec<AuthExpr>, mode: Combination) -> AuthExpr {
    if checks.len() == 1 {
        return checks.pop().expect("checked length");
    }

    match mode {
        Combination::And => AuthExpr::AllOf(checks),
        Combination::Or => AuthExpr::AnyOf(checks),
    }
}

/// The abstract syntax tree shared by the expression parser and the structured
/// attribute form.
#[derive(Clone, Debug, PartialEq, Eq)]
enum AuthExpr {
    PermitAll,
    DenyAll,
    HasRole(String),
    HasAnyRole(Vec<String>),
    HasAuthority(String),
    HasAnyAuthority(Vec<String>),
    Not(Box<AuthExpr>),
    AllOf(Vec<AuthExpr>),
    AnyOf(Vec<AuthExpr>),
}

impl AuthExpr {
    /// Lowers the tree to a constant
    /// `next_web::security::...::PreAuthorizeExpression`.
    fn into_tokens(self) -> TokenStream2 {
        let base =
            quote!(::next_web::security::access::prepost::pre_authorize::PreAuthorizeExpression);

        match self {
            AuthExpr::PermitAll => quote!(#base::PermitAll),
            AuthExpr::DenyAll => quote!(#base::DenyAll),
            AuthExpr::HasRole(role) => {
                let role = LitStr::new(&role, Span::call_site());
                quote!(#base::HasRole(#role))
            }
            AuthExpr::HasAnyRole(roles) => {
                let roles = roles
                    .iter()
                    .map(|role| LitStr::new(role, Span::call_site()));
                quote!(#base::HasAnyRole(&[#(#roles),*]))
            }
            AuthExpr::HasAuthority(authority) => {
                let authority = LitStr::new(&authority, Span::call_site());
                quote!(#base::HasAuthority(#authority))
            }
            AuthExpr::HasAnyAuthority(authorities) => {
                let authorities = authorities
                    .iter()
                    .map(|authority| LitStr::new(authority, Span::call_site()));
                quote!(#base::HasAnyAuthority(&[#(#authorities),*]))
            }
            AuthExpr::Not(expression) => {
                let expression = expression.into_tokens();
                quote!(#base::Not(&#expression))
            }
            AuthExpr::AllOf(expressions) => {
                let expressions = expressions.into_iter().map(AuthExpr::into_tokens);
                quote!(#base::AllOf(&[#(#expressions),*]))
            }
            AuthExpr::AnyOf(expressions) => {
                let expressions = expressions.into_iter().map(AuthExpr::into_tokens);
                quote!(#base::AnyOf(&[#(#expressions),*]))
            }
        }
    }
}

/// A small recursive-descent parser for the Spring Security style expression
/// grammar.
///
/// ```text
/// expression := or
/// or         := and (("||" | "or") and)*
/// and        := unary (("&&" | "and") unary)*
/// unary      := ("!" | "not") unary | primary
/// primary    := "(" expression ")" | call
/// call       := function "(" (string ("," string)*)? ")"
/// ```
mod expr {
    use super::AuthExpr;

    pub(super) fn parse(input: &str) -> Result<AuthExpr, String> {
        let mut parser = Parser::new(input);
        let expression = parser.parse_or()?;
        parser.skip_whitespace();

        if parser.peek().is_some() {
            return Err(format!(
                "unexpected trailing input `{}`",
                parser.remaining()
            ));
        }

        Ok(expression)
    }

    struct Parser {
        chars: Vec<char>,
        position: usize,
    }

    impl Parser {
        fn new(input: &str) -> Self {
            Self {
                chars: input.chars().collect(),
                position: 0,
            }
        }

        fn peek(&self) -> Option<char> {
            self.chars.get(self.position).copied()
        }

        fn remaining(&self) -> String {
            self.chars[self.position..].iter().collect()
        }

        fn skip_whitespace(&mut self) {
            while matches!(self.peek(), Some(character) if character.is_whitespace()) {
                self.position += 1;
            }
        }

        fn eat_symbol(&mut self, symbol: &str) -> bool {
            self.skip_whitespace();

            let end = self.position + symbol.chars().count();
            if end > self.chars.len() {
                return false;
            }

            if self.chars[self.position..end].iter().collect::<String>() != symbol {
                return false;
            }

            self.position = end;
            true
        }

        fn eat_keyword(&mut self, keyword: &str) -> bool {
            self.skip_whitespace();

            let end = self.position + keyword.chars().count();
            if end > self.chars.len() {
                return false;
            }

            if self.chars[self.position..end].iter().collect::<String>() != keyword {
                return false;
            }

            // Make sure `or` does not match a prefix of an identifier such as
            // `order`.
            if matches!(self.chars.get(end), Some(character) if is_identifier_character(*character))
            {
                return false;
            }

            self.position = end;
            true
        }

        fn parse_or(&mut self) -> Result<AuthExpr, String> {
            let mut expressions = vec![self.parse_and()?];

            loop {
                if self.eat_symbol("||") || self.eat_keyword("or") {
                    expressions.push(self.parse_and()?);
                } else {
                    break;
                }
            }

            Ok(combine_or_single(expressions, AuthExpr::AnyOf))
        }

        fn parse_and(&mut self) -> Result<AuthExpr, String> {
            let mut expressions = vec![self.parse_unary()?];

            loop {
                if self.eat_symbol("&&") || self.eat_keyword("and") {
                    expressions.push(self.parse_unary()?);
                } else {
                    break;
                }
            }

            Ok(combine_or_single(expressions, AuthExpr::AllOf))
        }

        fn parse_unary(&mut self) -> Result<AuthExpr, String> {
            if self.eat_symbol("!") || self.eat_keyword("not") {
                let expression = self.parse_unary()?;
                return Ok(AuthExpr::Not(Box::new(expression)));
            }

            self.parse_primary()
        }

        fn parse_primary(&mut self) -> Result<AuthExpr, String> {
            self.skip_whitespace();

            if self.eat_symbol("(") {
                let expression = self.parse_or()?;

                if !self.eat_symbol(")") {
                    return Err("expected `)`".to_string());
                }

                return Ok(expression);
            }

            let function = self.parse_identifier()?;
            let arguments = self.parse_call()?;

            build_call(&function, arguments)
        }

        fn parse_identifier(&mut self) -> Result<String, String> {
            self.skip_whitespace();

            let start = self.position;
            while matches!(self.peek(), Some(character) if is_identifier_character(character)) {
                self.position += 1;
            }

            if start == self.position {
                return Err(format!(
                    "expected an authorization function, found `{}`",
                    self.remaining()
                ));
            }

            Ok(self.chars[start..self.position].iter().collect())
        }

        fn parse_call(&mut self) -> Result<Vec<String>, String> {
            if !self.eat_symbol("(") {
                return Err("expected `(` after the function name".to_string());
            }

            let mut arguments = Vec::new();
            self.skip_whitespace();

            if self.eat_symbol(")") {
                return Ok(arguments);
            }

            loop {
                arguments.push(self.parse_string()?);

                if self.eat_symbol(",") {
                    continue;
                }

                break;
            }

            if !self.eat_symbol(")") {
                return Err("expected `,` or `)` in the argument list".to_string());
            }

            Ok(arguments)
        }

        fn parse_string(&mut self) -> Result<String, String> {
            self.skip_whitespace();

            let quote = match self.peek() {
                Some(character @ ('\'' | '"')) => character,
                _ => {
                    return Err(format!(
                        "expected a quoted argument, found `{}`",
                        self.remaining()
                    ))
                }
            };
            self.position += 1;

            let mut value = String::new();

            loop {
                match self.peek() {
                    None => return Err("unterminated string literal".to_string()),
                    Some('\\') => {
                        self.position += 1;
                        match self.peek() {
                            None => return Err("unterminated string literal".to_string()),
                            Some(escaped) => {
                                value.push(escaped);
                                self.position += 1;
                            }
                        }
                    }
                    Some(character) if character == quote => {
                        self.position += 1;
                        break;
                    }
                    Some(character) => {
                        value.push(character);
                        self.position += 1;
                    }
                }
            }

            Ok(value)
        }
    }

    fn combine_or_single(
        mut expressions: Vec<AuthExpr>,
        combine: fn(Vec<AuthExpr>) -> AuthExpr,
    ) -> AuthExpr {
        if expressions.len() == 1 {
            expressions.pop().expect("checked length")
        } else {
            combine(expressions)
        }
    }

    fn is_identifier_character(character: char) -> bool {
        character.is_alphanumeric() || character == '_'
    }

    fn build_call(function: &str, arguments: Vec<String>) -> Result<AuthExpr, String> {
        // Accept both `has_role` and the Spring Security `hasRole` spelling.
        let normalized: String = function
            .chars()
            .filter(|character| *character != '_')
            .flat_map(char::to_lowercase)
            .collect();

        match normalized.as_str() {
            "hasrole" => require(arguments, 1, function)
                .map(|mut arguments| AuthExpr::HasRole(arguments.remove(0))),
            "hasanyrole" => require_at_least(arguments, 1, function).map(AuthExpr::HasAnyRole),
            "hasauthority" | "haspermission" => require(arguments, 1, function)
                .map(|mut arguments| AuthExpr::HasAuthority(arguments.remove(0))),
            "hasanyauthority" | "hasanypermission" => {
                require_at_least(arguments, 1, function).map(AuthExpr::HasAnyAuthority)
            }
            "permitall" => require(arguments, 0, function).map(|_| AuthExpr::PermitAll),
            "denyall" => require(arguments, 0, function).map(|_| AuthExpr::DenyAll),
            _ => Err(format!(
                "unknown authorization function `{function}`; expected one of \
                 has_role, has_any_role, has_authority, has_any_authority, permit_all, deny_all"
            )),
        }
    }

    fn require(
        arguments: Vec<String>,
        expected: usize,
        function: &str,
    ) -> Result<Vec<String>, String> {
        if arguments.len() != expected {
            return Err(format!(
                "`{function}` expects {expected} argument(s), got {}",
                arguments.len()
            ));
        }

        Ok(arguments)
    }

    fn require_at_least(
        arguments: Vec<String>,
        expected: usize,
        function: &str,
    ) -> Result<Vec<String>, String> {
        if arguments.len() < expected {
            return Err(format!(
                "`{function}` expects at least {expected} argument(s), got {}",
                arguments.len()
            ));
        }

        Ok(arguments)
    }
}

#[cfg(test)]
mod tests {
    use super::{expr, AuthExpr};

    fn role(role: &str) -> AuthExpr {
        AuthExpr::HasRole(role.to_string())
    }

    #[test]
    fn parses_a_single_role() {
        assert_eq!(expr::parse("has_role('ADMIN')").unwrap(), role("ADMIN"));
    }

    #[test]
    fn parses_the_camel_case_spelling() {
        assert_eq!(expr::parse("hasRole('ADMIN')").unwrap(), role("ADMIN"));
    }

    #[test]
    fn parses_any_role_and_any_authority() {
        assert_eq!(
            expr::parse("has_any_role(\"ADMIN\", 'AUDITOR')").unwrap(),
            AuthExpr::HasAnyRole(vec!["ADMIN".to_string(), "AUDITOR".to_string()])
        );
        assert_eq!(
            expr::parse("has_any_authority('order:read', 'order:write')").unwrap(),
            AuthExpr::HasAnyAuthority(vec!["order:read".to_string(), "order:write".to_string()])
        );
    }

    #[test]
    fn parses_conjunction() {
        assert_eq!(
            expr::parse("has_role('ADMIN') and has_authority('order:read')").unwrap(),
            AuthExpr::AllOf(vec![
                role("ADMIN"),
                AuthExpr::HasAuthority("order:read".into())
            ])
        );
        assert_eq!(
            expr::parse("has_role('ADMIN') && has_authority('order:read')").unwrap(),
            AuthExpr::AllOf(vec![
                role("ADMIN"),
                AuthExpr::HasAuthority("order:read".into())
            ])
        );
    }

    #[test]
    fn parses_disjunction() {
        assert_eq!(
            expr::parse("has_role('ADMIN') or has_role('AUDITOR')").unwrap(),
            AuthExpr::AnyOf(vec![role("ADMIN"), role("AUDITOR")])
        );
        assert_eq!(
            expr::parse("has_role('ADMIN') || has_role('AUDITOR')").unwrap(),
            AuthExpr::AnyOf(vec![role("ADMIN"), role("AUDITOR")])
        );
    }

    #[test]
    fn and_binds_tighter_than_or() {
        assert_eq!(
            expr::parse("has_role('ADMIN') and has_role('AUDITOR') or has_role('USER')").unwrap(),
            AuthExpr::AnyOf(vec![
                AuthExpr::AllOf(vec![role("ADMIN"), role("AUDITOR")]),
                role("USER"),
            ])
        );
    }

    #[test]
    fn parses_negation() {
        assert_eq!(
            expr::parse("!has_role('BANNED')").unwrap(),
            AuthExpr::Not(Box::new(role("BANNED")))
        );
        assert_eq!(
            expr::parse("not has_role('BANNED')").unwrap(),
            AuthExpr::Not(Box::new(role("BANNED")))
        );
    }

    #[test]
    fn mixes_negation_with_conjunction() {
        assert_eq!(
            expr::parse("has_role('ADMIN') and !has_authority('order:delete')").unwrap(),
            AuthExpr::AllOf(vec![
                role("ADMIN"),
                AuthExpr::Not(Box::new(AuthExpr::HasAuthority("order:delete".into()))),
            ])
        );
    }

    #[test]
    fn parses_parenthesised_expressions() {
        assert_eq!(
            expr::parse("(has_role('ADMIN') or has_role('USER')) and has_authority('order:read')")
                .unwrap(),
            AuthExpr::AllOf(vec![
                AuthExpr::AnyOf(vec![role("ADMIN"), role("USER")]),
                AuthExpr::HasAuthority("order:read".into()),
            ])
        );
    }

    #[test]
    fn parses_permit_all_and_deny_all() {
        assert_eq!(expr::parse("permit_all()").unwrap(), AuthExpr::PermitAll);
        assert_eq!(expr::parse("denyAll()").unwrap(), AuthExpr::DenyAll);
    }

    #[test]
    fn reports_unknown_functions() {
        let error = expr::parse("has_everything('ADMIN')").unwrap_err();
        assert!(error.contains("unknown authorization function"), "{error}");
    }

    #[test]
    fn reports_arity_errors() {
        assert!(expr::parse("has_role('A', 'B')").is_err());
        assert!(expr::parse("has_role()").is_err());
    }

    #[test]
    fn reports_trailing_input() {
        assert!(expr::parse("has_role('A') garbage").is_err());
    }

    #[test]
    fn reports_unterminated_strings() {
        assert!(expr::parse("has_role('A)").is_err());
    }
}
