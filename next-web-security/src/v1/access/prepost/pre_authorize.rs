use std::collections::HashSet;

use crate::core::context::SecurityContextHolder;

/// Rust equivalent of Spring Security's `@PreAuthorize` annotation.
///
/// Specifies a method access-control expression evaluated before method invocation.
/// In Java: `@PreAuthorize("hasRole('ADMIN')")`
///
/// In Rust, use this struct programmatically or with a proc-macro attribute
/// like `#[pre_authorize("has_role('ADMIN')")]`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreAuthorize {
    pub expression: String,
}

impl PreAuthorize {
    pub fn new(expression: impl Into<String>) -> Self {
        Self {
            expression: expression.into(),
        }
    }

    pub fn expression(&self) -> &str {
        &self.expression
    }
    pub fn value(&self) -> &str {
        &self.expression
    }
    pub fn set_expression(&mut self, expression: impl Into<String>) {
        self.expression = expression.into();
    }
}

impl super::PrePostExpressionAttribute for PreAuthorize {
    fn expression(&self) -> &str {
        &self.expression
    }
}

/// The prefix the [`PreAuthorizeExpression::HasRole`] check prepends to a role
/// name before comparing it against the granted authorities.
///
/// It mirrors Spring Security's default `ROLE_` prefix, which means
/// `has_role("ADMIN")` matches a granted authority called `ROLE_ADMIN`.
pub const DEFAULT_ROLE_PREFIX: &str = "ROLE_";

/// A compiled `#[pre_authorize]` expression.
///
/// The proc-macro turns the attribute - either a Spring style expression such
/// as `has_role('ADMIN') and has_authority('order:read')` or the structured
/// key/value form - into a constant value of this type and looks it up against
/// the current [`SecurityContextHolder`] authentication.
///
/// Boolean combinations are represented by [`PreAuthorizeExpression::AllOf`]
/// (all of the checks must match), [`PreAuthorizeExpression::AnyOf`] (at least
/// one of the checks must match) and [`PreAuthorizeExpression::Not`] (the
/// wrapped check must not match).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreAuthorizeExpression {
    /// Always grants access. Produced by `permit_all()`.
    PermitAll,
    /// Always denies access. Produced by `deny_all()`.
    DenyAll,
    /// Grants access when the authenticated user holds the role.
    HasRole(&'static str),
    /// Grants access when the authenticated user holds any of the roles.
    HasAnyRole(&'static [&'static str]),
    /// Grants access when the authenticated user holds the authority.
    HasAuthority(&'static str),
    /// Grants access when the authenticated user holds any of the authorities.
    HasAnyAuthority(&'static [&'static str]),
    /// Inverts the wrapped expression.
    Not(&'static PreAuthorizeExpression),
    /// Every wrapped expression must match.
    AllOf(&'static [PreAuthorizeExpression]),
    /// At least one wrapped expression must match.
    AnyOf(&'static [PreAuthorizeExpression]),
}

impl PreAuthorizeExpression {
    /// Evaluates the expression against the authentication currently stored in
    /// the [`SecurityContextHolder`].
    ///
    /// When no authentication is present every predicate fails, except for
    /// [`PreAuthorizeExpression::DenyAll`] which always fails anyway and
    /// [`PreAuthorizeExpression::Not`] of a failing predicate which succeeds.
    pub fn evaluate(&self) -> bool {
        let authorities = current_authorities();
        self.is_satisfied_by(&authorities)
    }

    /// Evaluates the expression against an explicit set of granted authority
    /// names.
    ///
    /// This is the pure part of [`PreAuthorizeExpression::evaluate`] and is
    /// handy for testing.
    pub fn is_satisfied_by(&self, authorities: &HashSet<String>) -> bool {
        match self {
            Self::PermitAll => true,
            Self::DenyAll => false,
            Self::HasRole(role) => authorities.contains(&role_authority(role)),
            Self::HasAnyRole(roles) => roles
                .iter()
                .any(|role| authorities.contains(role_authority(role).as_str())),
            Self::HasAuthority(authority) => authorities.contains(*authority),
            Self::HasAnyAuthority(authorities_to_check) => authorities_to_check
                .iter()
                .any(|authority| authorities.contains(*authority)),
            Self::Not(expression) => !expression.is_satisfied_by(authorities),
            Self::AllOf(expressions) => expressions
                .iter()
                .all(|expression| expression.is_satisfied_by(authorities)),
            Self::AnyOf(expressions) => expressions
                .iter()
                .any(|expression| expression.is_satisfied_by(authorities)),
        }
    }
}

/// Evaluates a compiled [`PreAuthorizeExpression`] against the authentication
/// currently stored in the [`SecurityContextHolder`].
///
/// This is the entry point the `#[pre_authorize]` proc-macro expands to.
pub fn evaluate(expression: &PreAuthorizeExpression) -> bool {
    expression.evaluate()
}

/// Collects the granted authority names of the current authentication.
pub fn current_authorities() -> HashSet<String> {
    SecurityContextHolder::get_context()
        .get_authentication()
        .map(|authentication| {
            authentication
                .authorities()
                .iter()
                .filter_map(|authority| authority.authority().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn role_authority(role: &str) -> String {
    if role.starts_with(DEFAULT_ROLE_PREFIX) {
        role.to_string()
    } else {
        format!("{DEFAULT_ROLE_PREFIX}{role}")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::PreAuthorizeExpression;

    fn authorities(values: &[&str]) -> HashSet<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn has_role_matches_the_prefixed_authority() {
        let authorities = authorities(&["ROLE_ADMIN", "order:read"]);

        assert!(PreAuthorizeExpression::HasRole("ADMIN").is_satisfied_by(&authorities));
        assert!(PreAuthorizeExpression::HasRole("ROLE_ADMIN").is_satisfied_by(&authorities));
        assert!(!PreAuthorizeExpression::HasRole("USER").is_satisfied_by(&authorities));
    }

    #[test]
    fn has_authority_matches_the_raw_authority() {
        let authorities = authorities(&["ROLE_ADMIN", "order:read"]);

        assert!(PreAuthorizeExpression::HasAuthority("order:read").is_satisfied_by(&authorities));
        assert!(!PreAuthorizeExpression::HasAuthority("ORDER_READ").is_satisfied_by(&authorities));
    }

    #[test]
    fn has_any_matches_when_one_entry_matches() {
        let authorities = authorities(&["ROLE_ADMIN"]);

        assert!(
            PreAuthorizeExpression::HasAnyRole(&["USER", "ADMIN"]).is_satisfied_by(&authorities)
        );
        assert!(
            !PreAuthorizeExpression::HasAnyRole(&["USER", "GUEST"]).is_satisfied_by(&authorities)
        );
        assert!(
            !PreAuthorizeExpression::HasAnyAuthority(&["order:read", "order:write"])
                .is_satisfied_by(&authorities)
        );
    }

    #[test]
    fn all_of_requires_every_expression_to_match() {
        let authorities = authorities(&["ROLE_ADMIN", "order:read"]);
        let expression = PreAuthorizeExpression::AllOf(&[
            PreAuthorizeExpression::HasRole("ADMIN"),
            PreAuthorizeExpression::HasAuthority("order:read"),
        ]);
        let failing = PreAuthorizeExpression::AllOf(&[
            PreAuthorizeExpression::HasRole("ADMIN"),
            PreAuthorizeExpression::HasAuthority("order:write"),
        ]);

        assert!(expression.is_satisfied_by(&authorities));
        assert!(!failing.is_satisfied_by(&authorities));
    }

    #[test]
    fn any_of_requires_one_expression_to_match() {
        let authorities = authorities(&["ROLE_USER"]);
        let expression = PreAuthorizeExpression::AnyOf(&[
            PreAuthorizeExpression::HasRole("ADMIN"),
            PreAuthorizeExpression::HasAuthority("order:read"),
        ]);

        assert!(!expression.is_satisfied_by(&authorities));
        assert!(PreAuthorizeExpression::AnyOf(&[
            PreAuthorizeExpression::HasRole("USER"),
            PreAuthorizeExpression::HasAuthority("order:read"),
        ])
        .is_satisfied_by(&authorities));
    }

    #[test]
    fn not_inverts_the_wrapped_expression() {
        let authorities = authorities(&["ROLE_USER"]);
        let expression = PreAuthorizeExpression::Not(&PreAuthorizeExpression::HasRole("ADMIN"));

        assert!(expression.is_satisfied_by(&authorities));
        assert!(
            !PreAuthorizeExpression::Not(&PreAuthorizeExpression::HasRole("USER"))
                .is_satisfied_by(&authorities)
        );
    }

    #[test]
    fn empty_authentication_denies_role_and_authority_checks() {
        let authorities = HashSet::new();

        assert!(!PreAuthorizeExpression::HasRole("ADMIN").is_satisfied_by(&authorities));
        assert!(!PreAuthorizeExpression::HasAuthority("order:read").is_satisfied_by(&authorities));
        assert!(
            PreAuthorizeExpression::Not(&PreAuthorizeExpression::HasRole("ADMIN"))
                .is_satisfied_by(&authorities)
        );
    }
}
