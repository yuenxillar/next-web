use std::sync::Arc;

use next_web_core::async_trait;
use next_web_core::traits::http::http_request::HttpRequest;
use tracing::enabled;
use tracing::Level;

use crate::access::intercept::RequestAuthorizationContext;
use crate::authorization::AuthenticatedAuthorizationManager;
use crate::authorization::AuthorityAuthorizationManager;
use crate::authorization::AuthorizationDecision;
use crate::authorization::AuthorizationManager;
use crate::authorization::AuthorizationResult;
use crate::authorization::SingleResultAuthorizationManager;
use crate::core::Authentication;
use crate::web::util::matcher::AnyRequestMatcher;
use crate::web::util::matcher::RequestMatcher;
use crate::web::util::matcher::RequestMatcherEntry;
use crate::web::util::UrlUtils;

/// An AuthorizationManager which delegates to a specific AuthorizationManager based on a RequestMatcher evaluation.
#[derive(Clone)]
pub struct RequestMatcherDelegatingAuthorizationManager {
    mappings: Vec<RequestMatcherEntry<Arc<dyn AuthorizationManager<RequestAuthorizationContext>>>>,
}

impl RequestMatcherDelegatingAuthorizationManager {
    pub fn new(
        mappings: Vec<
            RequestMatcherEntry<Arc<dyn AuthorizationManager<RequestAuthorizationContext>>>,
        >,
    ) -> Self {
        assert!(!mappings.is_empty(), "mappings cannot be empty");
        Self { mappings }
    }

    fn request_line(ctx: &RequestAuthorizationContext) -> String {
        format!(
            "{} {}",
            ctx.request().method(),
            UrlUtils::build_request_url(ctx.request())
        )
    }

    /// Creates a builder for `RequestMatcherDelegatingAuthorizationManager`.
    pub fn builder() -> RequestMatcherDelegatingAuthorizationManagerBuilder {
        RequestMatcherDelegatingAuthorizationManagerBuilder::default()
    }
}

#[async_trait]
impl AuthorizationManager<RequestAuthorizationContext>
    for RequestMatcherDelegatingAuthorizationManager
{
    async fn authorize(
        &self,
        authentication: &dyn Authentication,
        context: &RequestAuthorizationContext,
    ) -> Option<Box<dyn AuthorizationResult>> {
        if enabled!(Level::TRACE) {
            tracing::trace!("Authorizing {}", Self::request_line(context));
        }
        for mapping in &self.mappings {
            let matcher = mapping.request_matcher();
            let match_result = matcher.matcher(context.request());
            if match_result.is_match() {
                if enabled!(Level::TRACE) {
                    tracing::trace!(
                        "Checking authorization on {} using {:?}",
                        Self::request_line(context),
                        matcher
                    );
                }
                context.set_variables(match_result.get_own_variables());
                return mapping.entry().authorize(authentication, context).await;
            }
        }

        if enabled!(Level::TRACE) {
            tracing::trace!("Denying request since did not find matching RequestMatcher");
        }

        Some(Box::new(AuthorizationDecision::new(false)))
    }
}

/// A builder for `RequestMatcherDelegatingAuthorizationManager`.
#[derive(Clone)]
pub struct RequestMatcherDelegatingAuthorizationManagerBuilder {
    any_request_configured: bool,
    pub(crate) mappings:
        Vec<RequestMatcherEntry<Arc<dyn AuthorizationManager<RequestAuthorizationContext>>>>,
}

impl RequestMatcherDelegatingAuthorizationManagerBuilder {
    /// Maps a `RequestMatcher` to an `AuthorizationManager`.
    pub fn add(
        &mut self,
        matcher: Arc<dyn RequestMatcher>,
        manager: Arc<dyn AuthorizationManager<RequestAuthorizationContext>>,
    ) -> &mut Self {
        assert!(
            !self.any_request_configured,
            "Can't add mappings after anyRequest"
        );
        self.mappings
            .push(RequestMatcherEntry::new(matcher, manager));
        self
    }

    /// Allows to configure the `RequestMatcher` to `AuthorizationManager` mappings.
    pub fn mappings(
        &mut self,
        mappings: impl FnOnce(
            &mut Vec<
                RequestMatcherEntry<Arc<dyn AuthorizationManager<RequestAuthorizationContext>>>,
            >,
        ),
    ) -> &mut Self {
        assert!(
            !self.any_request_configured,
            "Can't configure mappings after anyRequest"
        );
        mappings(&mut self.mappings);
        self
    }

    /// Maps any request.
    pub fn any_request(&mut self) -> AuthorizedUrl<'_> {
        assert!(
            !self.any_request_configured,
            "Can't configure anyRequest after itself"
        );
        self.any_request_configured = true;
        AuthorizedUrl::new(vec![AnyRequestMatcher::instance()], self)
    }

    /// Maps `RequestMatcher`s to `AuthorizationManager`.
    pub fn request_matchers(
        &mut self,
        matchers: Vec<Arc<dyn RequestMatcher>>,
    ) -> AuthorizedUrl<'_> {
        assert!(
            !self.any_request_configured,
            "Can't configure requestMatchers after anyRequest"
        );
        AuthorizedUrl::new(matchers, self)
    }

    /// Creates a `RequestMatcherDelegatingAuthorizationManager` instance.
    pub fn build(&mut self) -> Arc<dyn AuthorizationManager<RequestAuthorizationContext>> {
        Arc::new(RequestMatcherDelegatingAuthorizationManager::new(
            std::mem::take(&mut self.mappings),
        ))
    }
}

impl Default for RequestMatcherDelegatingAuthorizationManagerBuilder {
    fn default() -> Self {
        Self {
            any_request_configured: false,
            mappings: Vec::new(),
        }
    }
}

/// An object that allows configuring the `AuthorizationManager` for `RequestMatcher`s.
pub struct AuthorizedUrl<'a> {
    matchers: Vec<Arc<dyn RequestMatcher>>,
    builder: &'a mut RequestMatcherDelegatingAuthorizationManagerBuilder,
}

impl<'a> AuthorizedUrl<'a> {
    fn new(
        matchers: Vec<Arc<dyn RequestMatcher>>,
        builder: &'a mut RequestMatcherDelegatingAuthorizationManagerBuilder,
    ) -> Self {
        Self { matchers, builder }
    }

    /// Specify that URLs are allowed by anyone.
    pub fn permit_all(&mut self) -> &mut RequestMatcherDelegatingAuthorizationManagerBuilder {
        self.access(Arc::new(SingleResultAuthorizationManager::<
            RequestAuthorizationContext,
        >::permit_all()))
    }

    /// Specify that URLs are not allowed by anyone.
    pub fn deny_all(&mut self) -> &mut RequestMatcherDelegatingAuthorizationManagerBuilder {
        self.access(Arc::new(SingleResultAuthorizationManager::<
            RequestAuthorizationContext,
        >::deny_all()))
    }

    /// Specify that URLs are allowed by any authenticated user.
    pub fn authenticated(&mut self) -> &mut RequestMatcherDelegatingAuthorizationManagerBuilder {
        self.access(Arc::new(AuthenticatedAuthorizationManager::authenticated()))
    }

    /// Specify that URLs are allowed by users who have authenticated and were not "remembered".
    pub fn fully_authenticated(
        &mut self,
    ) -> &mut RequestMatcherDelegatingAuthorizationManagerBuilder {
        self.access(Arc::new(
            AuthenticatedAuthorizationManager::fully_authenticated(),
        ))
    }

    /// Specify that URLs are allowed by users that have been remembered.
    pub fn remember_me(&mut self) -> &mut RequestMatcherDelegatingAuthorizationManagerBuilder {
        self.access(Arc::new(AuthenticatedAuthorizationManager::remember_me()))
    }

    /// Specify that URLs are allowed by anonymous users.
    pub fn anonymous(&mut self) -> &mut RequestMatcherDelegatingAuthorizationManagerBuilder {
        self.access(Arc::new(AuthenticatedAuthorizationManager::anonymous()))
    }

    /// Specifies that a user requires a role. The role is prepended with `ROLE_` automatically.
    pub fn has_role(
        &mut self,
        role: &str,
    ) -> &mut RequestMatcherDelegatingAuthorizationManagerBuilder {
        self.access(Arc::new(AuthorityAuthorizationManager::<
            RequestAuthorizationContext,
        >::has_any_role(
            "ROLE_", [role.to_string()]
        )))
    }

    /// Specifies that a user requires one of many roles. Each role is prepended with `ROLE_` automatically.
    pub fn has_any_role(
        &mut self,
        roles: &[&str],
    ) -> &mut RequestMatcherDelegatingAuthorizationManagerBuilder {
        self.access(Arc::new(AuthorityAuthorizationManager::<
            RequestAuthorizationContext,
        >::has_any_role(
            "ROLE_", roles.iter().map(ToString::to_string)
        )))
    }

    /// Specifies that a user requires an authority.
    pub fn has_authority(
        &mut self,
        authority: &str,
    ) -> &mut RequestMatcherDelegatingAuthorizationManagerBuilder {
        self.access(Arc::new(AuthorityAuthorizationManager::<
            RequestAuthorizationContext,
        >::has_authority(authority)))
    }

    /// Specifies that a user requires one of many authorities.
    pub fn has_any_authority(
        &mut self,
        authorities: &[&str],
    ) -> &mut RequestMatcherDelegatingAuthorizationManagerBuilder {
        self.access(Arc::new(AuthorityAuthorizationManager::<
            RequestAuthorizationContext,
        >::has_any_authority(
            authorities.iter().map(ToString::to_string).collect(),
        )))
    }

    fn access(
        &mut self,
        manager: Arc<dyn AuthorizationManager<RequestAuthorizationContext>>,
    ) -> &mut RequestMatcherDelegatingAuthorizationManagerBuilder {
        for matcher in &self.matchers {
            self.builder
                .mappings
                .push(RequestMatcherEntry::new(matcher.clone(), manager.clone()));
        }
        &mut *self.builder
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, sync::Arc};

    use axum::{body::Body, extract::Request};
    use next_web_core::http::HttpRequestShare;

    use crate::{
        core::{authority_utils::AuthorityUtils, Authentication, SimpleAuthentication},
        web::util::matcher::MatchResult,
    };

    use super::*;

    #[derive(Debug)]
    struct PathMatcher(&'static str);

    impl RequestMatcher for PathMatcher {
        fn matches(&self, request: &dyn HttpRequest) -> bool {
            request.path() == self.0
        }
    }

    #[derive(Debug)]
    struct VariablesMatcher;

    impl RequestMatcher for VariablesMatcher {
        fn matches(&self, _request: &dyn HttpRequest) -> bool {
            true
        }

        fn matcher(&self, _request: &dyn HttpRequest) -> MatchResult {
            MatchResult::match_with_variables(BTreeMap::from([(
                String::from("id"),
                String::from("42"),
            )]))
        }
    }

    #[derive(Clone, Default)]
    struct RecordingManager(Arc<std::sync::Mutex<Option<BTreeMap<String, String>>>>);

    #[async_trait]
    impl AuthorizationManager<RequestAuthorizationContext> for RecordingManager {
        async fn authorize(
            &self,
            _authentication: &dyn Authentication,
            context: &RequestAuthorizationContext,
        ) -> Option<Box<dyn AuthorizationResult>> {
            *self.0.lock().unwrap() = context.variables().cloned();
            Some(Box::new(AuthorizationDecision::new(true)))
        }
    }

    fn authentication(granted: &[&str]) -> Arc<dyn Authentication> {
        let mut builder = SimpleAuthentication::default().to_builder();
        builder.principal(Some(Arc::new(String::from("alice"))));
        builder.authorities(Box::new(|authorities| {
            authorities.extend(AuthorityUtils::create_authority_list(
                granted.iter().copied(),
            ));
        }));
        builder.authenticated(true);
        builder.build()
    }

    fn new_context(path: &str) -> RequestAuthorizationContext {
        let request = Request::builder().uri(path).body(Body::empty()).unwrap();
        RequestAuthorizationContext::new(HttpRequestShare::from(&request as &dyn HttpRequest), None)
    }

    #[tokio::test]
    async fn unmatched_requests_are_denied() {
        let manager =
            RequestMatcherDelegatingAuthorizationManager::new(vec![RequestMatcherEntry::new(
                Arc::new(PathMatcher("/api/**")),
                Arc::new(SingleResultAuthorizationManager::<
                    RequestAuthorizationContext,
                >::permit_all()),
            )]);
        let mut context = new_context("/other");

        let result = manager
            .authorize(authentication(&[]).as_ref(), &mut context)
            .await;

        assert!(result.is_some());
        assert!(!result.unwrap().is_granted());
    }

    #[tokio::test]
    async fn delegates_to_matching_managers_in_order() {
        let manager = RequestMatcherDelegatingAuthorizationManager::new(vec![
            RequestMatcherEntry::new(
                Arc::new(PathMatcher("/permit")),
                Arc::new(SingleResultAuthorizationManager::<
                    RequestAuthorizationContext,
                >::deny_all()),
            ),
            RequestMatcherEntry::new(
                Arc::new(PathMatcher("/deny")),
                Arc::new(SingleResultAuthorizationManager::<
                    RequestAuthorizationContext,
                >::permit_all()),
            ),
        ]);

        let mut context = new_context("/deny");
        let result = manager
            .authorize(authentication(&[]).as_ref(), &mut context)
            .await;
        assert!(result.unwrap().is_granted());

        let mut context = new_context("/permit");
        let result = manager
            .authorize(authentication(&[]).as_ref(), &mut context)
            .await;
        assert!(!result.unwrap().is_granted());
    }

    #[tokio::test]
    async fn matched_request_variables_are_passed_to_manager() {
        let recorder = RecordingManager::default();
        let manager =
            RequestMatcherDelegatingAuthorizationManager::new(vec![RequestMatcherEntry::new(
                Arc::new(VariablesMatcher),
                Arc::new(recorder.clone()),
            )]);
        let mut context = new_context("/any");

        let result = manager
            .authorize(authentication(&[]).as_ref(), &mut context)
            .await;

        assert!(result.unwrap().is_granted());
        assert_eq!(
            *recorder.0.lock().unwrap(),
            Some(BTreeMap::from([(String::from("id"), String::from("42"))]))
        );
    }

    #[tokio::test]
    async fn builder_fluent_api_builds_expected_mappings() {
        let mut builder = RequestMatcherDelegatingAuthorizationManagerBuilder::default();
        builder
            .request_matchers(vec![Arc::new(PathMatcher("/admin/**"))])
            .has_role("ADMIN");
        let manager = builder.build();

        let mut granted_context = new_context("/admin/users");
        let result = manager
            .authorize(
                authentication(&["ROLE_ADMIN"]).as_ref(),
                &mut granted_context,
            )
            .await;
        assert!(result.unwrap().is_granted());

        let mut denied_context = new_context("/admin/users");
        let result = manager
            .authorize(authentication(&["ROLE_USER"]).as_ref(), &mut denied_context)
            .await;
        assert!(!result.unwrap().is_granted());
    }

    #[test]
    #[should_panic(expected = "Can't add mappings after anyRequest")]
    fn adding_mappings_after_any_request_panics() {
        let mut builder = RequestMatcherDelegatingAuthorizationManagerBuilder::default();
        builder.any_request();
        builder.add(
            Arc::new(PathMatcher("/api/**")),
            Arc::new(SingleResultAuthorizationManager::<
                RequestAuthorizationContext,
            >::deny_all()),
        );
    }

    #[test]
    #[should_panic(expected = "Can't configure anyRequest after itself")]
    fn configuring_any_request_twice_panics() {
        let mut builder = RequestMatcherDelegatingAuthorizationManagerBuilder::default();
        builder.any_request();
        builder.any_request();
    }

    #[test]
    #[should_panic(expected = "Can't configure requestMatchers after anyRequest")]
    fn configuring_request_matchers_after_any_request_panics() {
        let mut builder = RequestMatcherDelegatingAuthorizationManagerBuilder::default();
        builder.any_request();
        builder.request_matchers(vec![Arc::new(PathMatcher("/api/**"))]);
    }
}
