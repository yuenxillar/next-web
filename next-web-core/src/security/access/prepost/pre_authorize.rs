//! The rule a handler is guarded with before it runs.
//!
//! The `#[pre_authorize]` attribute compiles its argument into a
//! [`PreAuthorizeExpression`] and evaluates it before the body of the handler
//! runs, so a handler that is annotated is only reached by a request whose
//! authentication satisfies the rule.
//!
//! The rule is a constant, so it is compiled into the handler and costs nothing
//! to build while a request is handled.
//!
//! ```rust,ignore
//! #[pre_authorize("has_role('ADMIN') and has_authority('order:read')")]
//! async fn report() -> &'static str {
//!     "the report"
//! }
//! ```

use std::{collections::HashMap, convert::Infallible, sync::Arc};

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, Method, StatusCode, Uri, Version, request::Parts},
    response::Response,
};

use crate::{
    any::AnyValue,
    error::BoxError,
    http::{HttpRequest, HttpResponse, ResourceDispatcher},
    security::current_authorities,
    web::ViewModel,
};

/// The prefix a role is looked up with.
///
/// A role that already carries the prefix is looked up as it is, so
/// `has_role('ADMIN')` and `has_role('ROLE_ADMIN')` name the same role.
pub const DEFAULT_ROLE_PREFIX: &str = "ROLE_";

/// The name of the request attribute that carries the authorities of the
/// request.
///
/// The crate that authenticates the requests writes the authorities of the
/// authentication it established into the request, so the rule of a handler
/// reads them from the request it is handled with instead of from the thread
/// that happens to run the handler.
pub const AUTHORITIES_ATTRIBUTE: &str = "next.security.pre_authorize.authorities";

/// The name of the request attribute that carries the handler of a refusal.
///
/// The handler renders the refusal of a rule the way the refusals of the filter
/// chain of the application are rendered.
pub const DENIAL_HANDLER_ATTRIBUTE: &str = "next.security.pre_authorize.denial_handler";

/// Renders the response of a request a rule refused.
pub trait PreAuthorizeDenialHandler: Send + Sync {
    /// Renders the refusal into the response.
    ///
    /// # Arguments
    ///
    /// * `request` - The request the refusal is rendered for.
    /// * `response` - The response the refusal is written to.
    fn render(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), BoxError>;
}

/// The guard of a handler.
///
/// The `#[pre_authorize]` attribute reads the guard of a request with an
/// extractor, and the body of the handler evaluates its rule against the
/// authorities the guard read. A request the rule refuses is answered with the
/// refusal of the application, and with `403 Forbidden` when the application
/// configured none.
#[derive(Clone)]
pub struct PreAuthorizeContext {
    authorities: Vec<String>,
    denial_handler: Option<Arc<dyn PreAuthorizeDenialHandler>>,
    // The parts of the request the refusal is rendered with. The request of the
    // handler was consumed by its extractors, and a refusal does not carry a
    // body, so the parts the guard kept are used.
    method: Method,
    uri: Uri,
    version: Version,
    headers: HeaderMap,
    /// The attributes of the request, which the refusal is rendered with.
    attributes: HashMap<String, AnyValue>,
    /// The model of the views of the request, which the refusal is rendered
    /// with.
    view_model: Option<ViewModel>,
    /// The dispatcher the application installed, which an error page of a
    /// refusal is rendered with.
    resource_dispatcher: Option<Arc<dyn ResourceDispatcher>>,
}

impl PreAuthorizeContext {
    /// Creates the guard of a request whose authorities are known.
    ///
    /// This is the constructor a test uses, and the one an application uses
    /// when it builds a request of its own.
    ///
    /// # Arguments
    ///
    /// * `authorities` - The authorities of the request.
    pub fn for_authorities<I, S>(authorities: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            authorities: authorities.into_iter().map(Into::into).collect(),
            denial_handler: None,
            method: Method::GET,
            uri: Uri::default(),
            version: Version::HTTP_11,
            headers: HeaderMap::new(),
            attributes: HashMap::new(),
            view_model: None,
            resource_dispatcher: None,
        }
    }

    /// Sets the handler of a refusal.
    ///
    /// # Arguments
    ///
    /// * `denial_handler` - The handler that renders a refusal.
    pub fn with_denial_handler(
        mut self,
        denial_handler: Arc<dyn PreAuthorizeDenialHandler>,
    ) -> Self {
        self.denial_handler = Some(denial_handler);
        self
    }

    /// Returns the authorities of the request.
    pub fn authorities(&self) -> &[String] {
        &self.authorities
    }

    /// Evaluates a rule against the authorities of the request.
    ///
    /// # Arguments
    ///
    /// * `expression` - The rule to evaluate.
    pub fn evaluate(&self, expression: &PreAuthorizeExpression) -> bool {
        evaluate_with_authorities(&self.authorities, expression)
    }

    /// Returns the response of a request the rule refused.
    ///
    /// The refusal of the application is used when it configured one, and the
    /// status `403 Forbidden` is used otherwise.
    ///
    /// The request the refusal is rendered with carries the parts the guard
    /// read: the method, the URI, the headers, the attributes of the request,
    /// the model its views are rendered with, and the dispatcher of the
    /// application. The body of the request is empty, and the state a transport
    /// holds (such as the session of the client) is not part of it, because the
    /// request of the handler was consumed by its extractors.
    pub fn denied(&self) -> Response {
        let mut response = Response::new(Body::empty());

        if let Some(denial_handler) = self.denial_handler.as_ref() {
            let mut request = Request::new(Body::empty());
            *request.method_mut() = self.method.clone();
            *request.uri_mut() = self.uri.clone();
            *request.version_mut() = self.version;
            *request.headers_mut() = self.headers.clone();
            // The refusal is rendered with the state of the request it belongs
            // to, so a handler that reads an attribute or that writes one, and an
            // error page that is dispatched to, work the way they do for a
            // refusal of the filter chain.
            let extensions = request.extensions_mut();
            extensions.insert(self.attributes.clone());
            if let Some(view_model) = self.view_model.clone() {
                extensions.insert(view_model);
            }
            if let Some(resource_dispatcher) = self.resource_dispatcher.clone() {
                extensions.insert(resource_dispatcher);
            }

            // A handler that rendered the refusal produced a response, which is
            // what the guard answers with. A handler that failed does not
            // change the response, and the status is used instead.
            if denial_handler.render(&mut request, &mut response).is_ok() && response.is_committed()
            {
                return response;
            }
        }

        *response.status_mut() = StatusCode::FORBIDDEN;

        response
    }
}

impl<S> axum::extract::FromRequestParts<S> for PreAuthorizeContext
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self {
            authorities: authorities_of(parts).unwrap_or_else(current_authorities),
            denial_handler: denial_handler_of(parts),
            method: parts.method.clone(),
            uri: parts.uri.clone(),
            version: parts.version,
            headers: parts.headers.clone(),
            attributes: attributes(parts),
            view_model: parts.extensions.get::<ViewModel>().cloned(),
            resource_dispatcher: parts
                .extensions
                .get::<Arc<dyn ResourceDispatcher>>()
                .cloned(),
        })
    }
}

/// Returns the attributes of the request.
///
/// # Arguments
///
/// * `parts` - The parts of the request.
fn attributes(parts: &Parts) -> HashMap<String, AnyValue> {
    parts
        .extensions
        .get::<HashMap<String, AnyValue>>()
        .cloned()
        .unwrap_or_default()
}

/// Returns the authorities the request carries, if the crate that authenticates
/// the requests wrote them.
///
/// # Arguments
///
/// * `parts` - The parts of the request.
fn authorities_of(parts: &Parts) -> Option<Vec<String>> {
    attribute(parts, AUTHORITIES_ATTRIBUTE)
        .and_then(AnyValue::as_vec)
        .map(|authorities| {
            authorities
                .iter()
                .filter_map(AnyValue::as_str)
                .map(ToString::to_string)
                .collect()
        })
}

/// Returns the handler the request carries for a refusal, if the crate that
/// authenticates the requests wrote one.
///
/// # Arguments
///
/// * `parts` - The parts of the request.
fn denial_handler_of(parts: &Parts) -> Option<Arc<dyn PreAuthorizeDenialHandler>> {
    match attribute(parts, DENIAL_HANDLER_ATTRIBUTE)? {
        AnyValue::BoxedValue(handler) => handler
            .as_any()
            .downcast_ref::<Arc<dyn PreAuthorizeDenialHandler>>()
            .cloned(),
        _ => None,
    }
}

/// Returns an attribute of the request.
///
/// The attributes of a request are held in the map the preparation of the
/// request installs, which is where a filter of the application stores them.
///
/// # Arguments
///
/// * `parts` - The parts of the request.
/// * `name` - The name of the attribute.
fn attribute<'a>(parts: &'a Parts, name: &str) -> Option<&'a AnyValue> {
    parts
        .extensions
        .get::<HashMap<String, AnyValue>>()
        .and_then(|attributes| attributes.get(name))
}

/// A compiled access-control rule.
///
/// The variants are the ones the `#[pre_authorize]` attribute emits, and the
/// values are constants: a rule is a value of this type that is stored in the
/// handler it guards.
#[derive(Clone, Copy, Debug)]
pub enum PreAuthorizeExpression {
    /// Every request is allowed.
    PermitAll,
    /// Every request is refused.
    DenyAll,
    /// The request has the role.
    HasRole(&'static str),
    /// The request has one of the roles.
    HasAnyRole(&'static [&'static str]),
    /// The request has the authority.
    HasAuthority(&'static str),
    /// The request has one of the authorities.
    HasAnyAuthority(&'static [&'static str]),
    /// The request does not satisfy the rule.
    Not(&'static PreAuthorizeExpression),
    /// The request satisfies every rule.
    AllOf(&'static [PreAuthorizeExpression]),
    /// The request satisfies one of the rules.
    AnyOf(&'static [PreAuthorizeExpression]),
}

/// Evaluates a rule against the authorities of the request being handled.
///
/// # Arguments
///
/// * `expression` - The rule to evaluate.
pub fn evaluate(expression: &PreAuthorizeExpression) -> bool {
    evaluate_with_authorities(&current_authorities(), expression)
}

/// Evaluates a rule against the given authorities.
///
/// This is the evaluation of [`evaluate`] with the authorities named by the
/// caller, which lets a deployment or a test evaluate a rule without a request.
///
/// # Arguments
///
/// * `authorities` - The authorities the rule is evaluated against.
/// * `expression` - The rule to evaluate.
pub fn evaluate_with_authorities(
    authorities: &[String],
    expression: &PreAuthorizeExpression,
) -> bool {
    match expression {
        PreAuthorizeExpression::PermitAll => true,
        PreAuthorizeExpression::DenyAll => false,
        PreAuthorizeExpression::HasRole(role) => has_role(authorities, role),
        PreAuthorizeExpression::HasAnyRole(roles) => {
            roles.iter().any(|role| has_role(authorities, role))
        }
        PreAuthorizeExpression::HasAuthority(authority) => {
            authorities.iter().any(|held| held == authority)
        }
        PreAuthorizeExpression::HasAnyAuthority(authorities_of_rule) => authorities_of_rule
            .iter()
            .any(|authority| authorities.iter().any(|held| held == authority)),
        PreAuthorizeExpression::Not(expression) => {
            !evaluate_with_authorities(authorities, expression)
        }
        PreAuthorizeExpression::AllOf(expressions) => expressions
            .iter()
            .all(|expression| evaluate_with_authorities(authorities, expression)),
        PreAuthorizeExpression::AnyOf(expressions) => expressions
            .iter()
            .any(|expression| evaluate_with_authorities(authorities, expression)),
    }
}

/// Returns whether the authorities carry the role.
///
/// # Arguments
///
/// * `authorities` - The authorities of the request.
/// * `role` - The role of the rule, with or without the role prefix.
fn has_role(authorities: &[String], role: &str) -> bool {
    let role = if role.starts_with(DEFAULT_ROLE_PREFIX) {
        role.to_string()
    } else {
        format!("{DEFAULT_ROLE_PREFIX}{role}")
    };

    authorities.iter().any(|held| held == &role)
}

#[cfg(test)]
mod tests {
    use super::{PreAuthorizeContext, PreAuthorizeExpression, evaluate_with_authorities};

    // The rules of the tests are declared the way the attribute declares them:
    // as constants, so the references a rule holds are `'static`.
    const ADMIN: PreAuthorizeExpression = PreAuthorizeExpression::HasRole("ADMIN");
    const READ: PreAuthorizeExpression = PreAuthorizeExpression::HasAuthority("order:read");
    const ADMIN_AND_READ: PreAuthorizeExpression = PreAuthorizeExpression::AllOf(&[ADMIN, READ]);
    const ADMIN_OR_READ: PreAuthorizeExpression = PreAuthorizeExpression::AnyOf(&[ADMIN, READ]);
    const NOT_ADMIN: PreAuthorizeExpression =
        PreAuthorizeExpression::Not(&PreAuthorizeExpression::HasRole("ADMIN"));
    const NOT_ADMIN_AND_READ: PreAuthorizeExpression = PreAuthorizeExpression::AllOf(&[
        PreAuthorizeExpression::Not(&PreAuthorizeExpression::HasRole("ADMIN")),
        READ,
    ]);
    const ANY_ROLE: PreAuthorizeExpression =
        PreAuthorizeExpression::HasAnyRole(&["ADMIN", "AUDITOR"]);
    const ANY_AUTHORITY: PreAuthorizeExpression =
        PreAuthorizeExpression::HasAnyAuthority(&["order:read", "order:write"]);

    /// Returns authorities of a test.
    ///
    /// # Arguments
    ///
    /// * `authorities` - The authority strings of the test.
    fn authorities(authorities: &[&str]) -> Vec<String> {
        authorities
            .iter()
            .map(|authority| authority.to_string())
            .collect()
    }

    #[test]
    fn a_role_is_looked_up_with_the_role_prefix() {
        assert!(evaluate_with_authorities(
            &authorities(&["ROLE_ADMIN"]),
            &ADMIN
        ));
        assert!(evaluate_with_authorities(
            &authorities(&["ROLE_ADMIN"]),
            &PreAuthorizeExpression::HasRole("ROLE_ADMIN")
        ));
        assert!(!evaluate_with_authorities(
            &authorities(&["ROLE_USER"]),
            &ADMIN
        ));
        assert!(!evaluate_with_authorities(&authorities(&[]), &ADMIN));
    }

    #[test]
    fn an_authority_is_looked_up_as_it_is_written() {
        assert!(evaluate_with_authorities(
            &authorities(&["order:read"]),
            &READ
        ));
        assert!(!evaluate_with_authorities(
            &authorities(&["ROLE_order:read"]),
            &READ
        ));
    }

    #[test]
    fn any_of_several_roles_or_authorities_satisfies_the_rule() {
        assert!(evaluate_with_authorities(
            &authorities(&["ROLE_AUDITOR"]),
            &ANY_ROLE
        ));
        assert!(!evaluate_with_authorities(
            &authorities(&["ROLE_USER"]),
            &ANY_ROLE
        ));
        assert!(evaluate_with_authorities(
            &authorities(&["order:write"]),
            &ANY_AUTHORITY
        ));
        assert!(!evaluate_with_authorities(
            &authorities(&["order:delete"]),
            &ANY_AUTHORITY
        ));
    }

    #[test]
    fn negation_and_combinations_are_evaluated() {
        assert!(evaluate_with_authorities(
            &authorities(&["ROLE_ADMIN", "order:read"]),
            &ADMIN_AND_READ
        ));
        assert!(!evaluate_with_authorities(
            &authorities(&["ROLE_ADMIN"]),
            &ADMIN_AND_READ
        ));
        assert!(evaluate_with_authorities(
            &authorities(&["order:read"]),
            &ADMIN_OR_READ
        ));
        assert!(evaluate_with_authorities(
            &authorities(&["ROLE_USER"]),
            &NOT_ADMIN
        ));
        assert!(!evaluate_with_authorities(
            &authorities(&["ROLE_ADMIN"]),
            &NOT_ADMIN
        ));
        assert!(evaluate_with_authorities(
            &authorities(&["order:read"]),
            &NOT_ADMIN_AND_READ
        ));
        assert!(!evaluate_with_authorities(
            &authorities(&["ROLE_ADMIN", "order:read"]),
            &NOT_ADMIN_AND_READ
        ));
    }

    #[test]
    fn permit_all_and_deny_all_hold_for_every_authentication() {
        assert!(evaluate_with_authorities(
            &[],
            &PreAuthorizeExpression::PermitAll
        ));
        assert!(!evaluate_with_authorities(
            &authorities(&["ROLE_ADMIN"]),
            &PreAuthorizeExpression::DenyAll
        ));
    }

    #[tokio::test]
    async fn the_guard_reads_the_authorities_of_the_request() {
        use axum::extract::FromRequestParts;

        use crate::{any::AnyValue, http::HttpRequest, security::AUTHORITIES_ATTRIBUTE};

        let mut request = axum::extract::Request::new(axum::body::Body::empty());
        HttpRequest::ready(&mut request);
        request.set_attribute(
            AUTHORITIES_ATTRIBUTE,
            AnyValue::Vec(vec![AnyValue::from("ROLE_ADMIN")]),
        );
        let (mut parts, _body) = request.into_parts();

        let guard = PreAuthorizeContext::from_request_parts(&mut parts, &())
            .await
            .expect("the guard always reads the request");

        assert_eq!(guard.authorities(), ["ROLE_ADMIN"]);
        assert!(guard.evaluate(&ADMIN));
        assert!(!guard.evaluate(&READ));
    }

    #[tokio::test]
    async fn a_refused_request_is_rendered_by_the_handler_of_the_application() {
        use std::sync::Arc;

        use crate::http::{HttpRequest, HttpResponse};

        struct JsonRefusal;

        impl super::PreAuthorizeDenialHandler for JsonRefusal {
            fn render(
                &self,
                _request: &mut dyn HttpRequest,
                response: &mut dyn HttpResponse,
            ) -> Result<(), crate::error::BoxError> {
                response.insert_header("Content-Type", "application/json;charset=UTF-8");
                response.set_status_code(axum::http::StatusCode::FORBIDDEN);
                response.set_body(br#"{"code":403,"message":"Insufficient permissions"}"#.to_vec());

                Ok(())
            }
        }

        let guard = PreAuthorizeContext::for_authorities(Vec::<String>::new())
            .with_denial_handler(Arc::new(JsonRefusal));

        let response = guard.denied();

        assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);
        assert_eq!(
            response
                .headers()
                .get("Content-Type")
                .and_then(|content_type| content_type.to_str().ok()),
            Some("application/json;charset=UTF-8")
        );
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .expect("the body of the refusal is read");
        assert_eq!(
            body.as_ref(),
            br#"{"code":403,"message":"Insufficient permissions"}"#
        );
    }

    #[test]
    fn a_refused_request_without_a_handler_answers_forbidden() {
        let guard = PreAuthorizeContext::for_authorities(Vec::<String>::new());

        assert_eq!(guard.denied().status(), axum::http::StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn the_refusal_carries_the_attributes_of_the_request() {
        use std::sync::{Arc, Mutex};

        use axum::extract::FromRequestParts;

        use crate::{
            any::AnyValue,
            http::{HttpRequest, HttpResponse},
            security::{AUTHORITIES_ATTRIBUTE, DENIAL_HANDLER_ATTRIBUTE},
        };

        /// A refusal that answers with an attribute of the request.
        struct AttributeRefusal(Arc<Mutex<Option<String>>>);

        impl super::PreAuthorizeDenialHandler for AttributeRefusal {
            fn render(
                &self,
                request: &mut dyn HttpRequest,
                response: &mut dyn HttpResponse,
            ) -> Result<(), crate::error::BoxError> {
                let attribute = request
                    .get_attribute("next.test.denied")
                    .and_then(AnyValue::as_str)
                    .map(ToString::to_string);
                *self.0.lock().expect("the refusal is not poisoned") = attribute;

                response.set_status_code(axum::http::StatusCode::FORBIDDEN);
                response.set_body(b"refused".to_vec());

                Ok(())
            }
        }

        let seen = Arc::new(Mutex::new(None));
        let mut request = axum::extract::Request::new(axum::body::Body::empty());
        HttpRequest::ready(&mut request);
        request.set_attribute(AUTHORITIES_ATTRIBUTE, AnyValue::Vec(Vec::new()));
        request.set_attribute(
            "next.test.denied",
            AnyValue::from("the state of the request"),
        );
        request.set_attribute(
            DENIAL_HANDLER_ATTRIBUTE,
            AnyValue::BoxedValue(Box::new(Arc::new(AttributeRefusal(seen.clone()))
                as Arc<dyn super::PreAuthorizeDenialHandler>)),
        );
        let (mut parts, _body) = request.into_parts();
        let guard = PreAuthorizeContext::from_request_parts(&mut parts, &())
            .await
            .expect("the guard always reads the request");

        let response = guard.denied();

        assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);
        assert_eq!(
            seen.lock().expect("the refusal is not poisoned").as_deref(),
            Some("the state of the request"),
            "the refusal reads the attributes of the request"
        );
    }

    #[test]
    fn a_request_without_an_authentication_only_satisfies_permit_all() {
        let authorities = authorities(&[]);

        assert!(evaluate_with_authorities(
            &authorities,
            &PreAuthorizeExpression::PermitAll
        ));
        assert!(!evaluate_with_authorities(
            &authorities,
            &PreAuthorizeExpression::HasRole("USER")
        ));
        assert!(!evaluate_with_authorities(
            &authorities,
            &PreAuthorizeExpression::HasAuthority("order:read")
        ));
    }
}
