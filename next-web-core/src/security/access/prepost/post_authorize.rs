//! The rule a handler is guarded with after it ran.
//!
//! The `#[post_authorize]` attribute compiles its argument into Rust code that is
//! evaluated **after** the body of the handler produced its result, so a handler
//! that is annotated answers only a request whose authentication satisfies a rule
//! about the value the handler computed:
//!
//! ```rust,ignore
//! #[post_authorize("result == authentication.name")]
//! async fn me() -> String {
//!     // the name of the user the request was made for
//! }
//!
//! #[post_authorize("result.owner == authentication.name")]
//! async fn account() -> Json<Account> {
//!     // the account the user asked for
//! }
//! ```
//!
//! # The rule is a Rust expression
//!
//! The rule of a handler is not a string that is interpreted while the request is
//! handled: the attribute turns it into a Rust expression over two bindings,
//! `result` and `authentication`, and the compiler type-checks it together with
//! the handler. A rule can therefore read a field of the result
//! (`result.owner`), call a method of it (`result.owner()`), index a collection
//! (`result.items[0].owner`), and compare it with anything Rust compares with — a
//! `String` result is compared with `authentication.name` directly.
//!
//! The cost of that choice is that a rule that names something the result does
//! not have does not compile, which is the trade this module makes on purpose: a
//! rule that is wrong is found while the application is built rather than while a
//! request is refused.
//!
//! # What the rule may use
//!
//! - `result` - the value the handler returned, borrowed. The type is the return
//!   type of the handler, so its fields and its methods are the ones the handler
//!   declares.
//! - `authentication` - the authentication of the request: the field `name` holds
//!   the name the request was authenticated as, the field `authorities` holds the
//!   authorities of the request, and [`Authentication::has_role`],
//!   [`Authentication::has_any_role`], [`Authentication::has_authority`] and
//!   [`Authentication::has_any_authority`] answer the rules of an authority.
//!
//! The attribute also understands the Spring Security spelling of a rule, so
//! `has_role('ADMIN')`, `has_authority('order:read')`, `and`, `or` and `not` mean
//! what they mean there. Everything else is Rust.
//!
//! # What it does not do
//!
//! The body of the handler already ran when the rule is evaluated, so a handler
//! that writes must not rely on a refused request having changed nothing. Guard
//! the handlers that change the state of the application with a
//! `#[pre_authorize]` rule, and answer the value a part of the users may not see
//! with a rule of this kind.

use std::convert::Infallible;

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};

use crate::security::access::prepost::pre_authorize::{PreAuthorizeContext, DEFAULT_ROLE_PREFIX};

/// The guard of a handler whose rule is evaluated after the handler ran.
///
/// The `#[post_authorize]` attribute takes the guard of the request as the first
/// argument of the handler, exactly like `#[pre_authorize]` does, and evaluates
/// the rule of the handler against the value the handler returned. A request the
/// rule refuses is answered with the refusal of the application, and with
/// `403 Forbidden` when the application configured none.
#[derive(Clone)]
pub struct PostAuthorizeContext {
    guard: PreAuthorizeContext,
}

impl PostAuthorizeContext {
    /// Creates the guard of a request whose authentication is known.
    ///
    /// This is the constructor a test uses.
    ///
    /// # Arguments
    ///
    /// * `authorities` - The authorities of the request.
    /// * `principal_name` - The name the request was authenticated as.
    pub fn for_authentication<I, S>(authorities: I, principal_name: impl Into<String>) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            guard: PreAuthorizeContext::for_authorities(authorities)
                .with_principal_name(principal_name),
        }
    }

    /// Returns the authentication of the request.
    ///
    /// The rule of a handler reads the name and the authorities of the
    /// authentication through it, which is what `authentication.name` and
    /// `authentication.has_role('ADMIN')` are compiled into.
    pub fn authentication(&self) -> Authentication<'_> {
        Authentication {
            name: self.guard.principal_name().unwrap_or_default(),
            authorities: self.guard.authorities(),
        }
    }

    /// Returns the response of a request the rule refused.
    ///
    /// The refusal of the application is used when it configured one, and the
    /// status `403 Forbidden` is used otherwise. See
    /// [`PreAuthorizeContext::denied`] for what the refusal is rendered with.
    pub fn denied(&self) -> Response {
        self.guard.denied()
    }

    /// Returns the response of a request the rule refused, with a status of its
    /// own.
    ///
    /// A handler whose refusal has to hide that the value exists (a resource
    /// that belongs to another user is answered with `404 Not Found` rather than
    /// with `403 Forbidden`) answers with it instead of with [`Self::denied`].
    /// The body of the refusal of the application is kept, so a status that hides
    /// the value does not lose the rendering the application chose.
    ///
    /// # Arguments
    ///
    /// * `status` - The status of the refusal.
    pub fn denied_with(&self, status: StatusCode) -> Response {
        let mut response = self.guard.denied();
        if response.status() == StatusCode::FORBIDDEN {
            *response.status_mut() = status;
        }

        response
    }
}

impl<S> FromRequestParts<S> for PostAuthorizeContext
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let guard = PreAuthorizeContext::from_request_parts(parts, state).await?;

        Ok(Self { guard })
    }
}

/// The authentication of the request, as a rule of a handler reads it.
///
/// The rules of a handler are written against the fields and the methods of this
/// type, so `authentication.name` is the name the request was authenticated as and
/// `authentication.has_role('ADMIN')` asks for a role.
pub struct Authentication<'a> {
    /// The name the request was authenticated as, which is empty for a request
    /// without an authentication.
    pub name: &'a str,
    /// The authorities of the request.
    pub authorities: &'a [String],
}

impl Authentication<'_> {
    /// Returns whether the request carries the role.
    ///
    /// The role is named with or without the role prefix, so `ADMIN` and
    /// `ROLE_ADMIN` name the same role.
    ///
    /// # Arguments
    ///
    /// * `role` - The role of the rule.
    pub fn has_role(&self, role: &str) -> bool {
        let role = if role.starts_with(DEFAULT_ROLE_PREFIX) {
            role.to_string()
        } else {
            format!("{DEFAULT_ROLE_PREFIX}{role}")
        };

        self.has_authority(&role)
    }

    /// Returns whether the request carries one of the roles.
    ///
    /// # Arguments
    ///
    /// * `roles` - The roles of the rule.
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|role| self.has_role(role))
    }

    /// Returns whether the request carries the authority.
    ///
    /// # Arguments
    ///
    /// * `authority` - The authority of the rule.
    pub fn has_authority(&self, authority: &str) -> bool {
        self.authorities.iter().any(|held| held == authority)
    }

    /// Returns whether the request carries one of the authorities.
    ///
    /// # Arguments
    ///
    /// * `authorities` - The authorities of the rule.
    pub fn has_any_authority(&self, authorities: &[&str]) -> bool {
        authorities
            .iter()
            .any(|authority| self.has_authority(authority))
    }

    /// Returns whether the request was authenticated as `name`.
    ///
    /// This is the method form of `result == authentication.name`, which a rule
    /// that compares the name of a field of the result with the name of the
    /// authentication uses.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the rule.
    pub fn is_name(&self, name: &str) -> bool {
        self.name == name
    }
}

/// Answers a request a rule refused.
///
/// This is what the attribute answers with, and it is public so that a handler
/// that evaluates its rule itself answers the same way.
///
/// # Arguments
///
/// * `guard` - The guard of the request.
pub fn refuse(guard: &PostAuthorizeContext) -> impl IntoResponse {
    guard.denied()
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::{Authentication, PostAuthorizeContext};

    /// Returns the authorities of a test.
    ///
    /// # Arguments
    ///
    /// * `authorities` - The authority strings of the test.
    fn authorities(authorities: &[&str]) -> Vec<String> {
        authorities.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn the_name_of_the_authentication_is_the_one_of_the_request() {
        let guard = PostAuthorizeContext::for_authentication(authorities(&[]), "max");
        let authentication = guard.authentication();

        assert_eq!(authentication.name, "max");
        assert!(authentication.is_name("max"));
        assert!(!authentication.is_name("alex"));
    }

    #[test]
    fn a_request_without_an_authentication_has_an_empty_name() {
        let guard = PostAuthorizeContext::for_authentication(authorities(&[]), "");
        let authentication = guard.authentication();

        assert!(authentication.name.is_empty());
        assert!(!authentication.is_name("max"));
    }

    #[test]
    fn a_role_is_looked_up_with_the_role_prefix() {
        let guard = PostAuthorizeContext::for_authentication(authorities(&["ROLE_ADMIN"]), "max");
        let authentication = guard.authentication();

        assert!(authentication.has_role("ADMIN"));
        assert!(authentication.has_role("ROLE_ADMIN"));
        assert!(!authentication.has_role("AUDITOR"));
        assert!(authentication.has_any_role(&["AUDITOR", "ADMIN"]));
        assert!(!authentication.has_any_role(&["AUDITOR"]));
    }

    #[test]
    fn an_authority_is_looked_up_as_it_is() {
        let guard = PostAuthorizeContext::for_authentication(authorities(&["order:read"]), "max");
        let authentication = guard.authentication();

        assert!(authentication.has_authority("order:read"));
        assert!(!authentication.has_authority("order:write"));
        assert!(authentication.has_any_authority(&["order:write", "order:read"]));
        assert!(!authentication.has_any_authority(&["order:write"]));
    }

    #[test]
    fn a_refused_request_is_answered_with_forbidden() {
        let guard = PostAuthorizeContext::for_authentication(authorities(&[]), "max");

        assert_eq!(guard.denied().status(), StatusCode::FORBIDDEN);
        assert_eq!(
            guard.denied_with(StatusCode::NOT_FOUND).status(),
            StatusCode::NOT_FOUND
        );
    }

    #[test]
    fn the_result_is_compared_with_the_name_of_the_authentication() {
        // The rule of the attribute is Rust, so this is what
        // `#[post_authorize("result == authentication.name")]` compiles into.
        let guard = PostAuthorizeContext::for_authentication(authorities(&[]), "max");
        let authentication: Authentication<'_> = guard.authentication();
        let result = String::from("max");

        assert!(result == authentication.name);
    }
}
