use std::ops::{Deref, DerefMut};

use next_web_core::{
    async_trait,
    http::{HttpRequest, HttpResponse},
    Named, Required,
};

use crate::{
    core::util::{object::Object, web::WebUtils},
    web::filter::{
        access_control_filter::AccessControlFilterExt, advice_filter::AdviceFilterExt,
        authz::authorization_filter::AuthorizationFilter,
        once_per_request_filter::OncePerRequestFilter,
        path_matching_filter::{PathMatchingFilter, PathMatchingFilterExt},
    },
};

#[derive(Clone)]
pub struct PermissionsAuthorizationFilter {
    authorization_filter: AuthorizationFilter,
}

#[async_trait]
impl AccessControlFilterExt for PermissionsAuthorizationFilter {
    async fn is_access_allowed(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        mapped_value: Option<Object>,
    ) -> bool {
        let subject = WebUtils::get_subject(request, response).await;

        let mut is_permitted = true;

        match mapped_value {
            Some(value) => {
                if let Some(perms) = value.as_list_str() {
                    if !perms.is_empty() {
                        if perms.len() == 1 {
                            if !subject.is_permitted(&perms[0]).await {
                                is_permitted = false;
                            }
                        } else {
                            if !subject.is_permitted_all(&perms).await {
                                is_permitted = false;
                            }
                        }
                    }
                }
            }
            None => {}
        };

        is_permitted
    }

    async fn on_access_denied(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        mapped_value: Option<Object>,
    ) -> bool {
        // The chain requires a permission the subject does not hold: the answer
        // of an authorization filter is the answer of this filter.
        self.authorization_filter
            .on_access_denied(request, response, mapped_value)
            .await
    }
}

#[async_trait]
impl AdviceFilterExt for PermissionsAuthorizationFilter {
    async fn pre_handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        ext: Option<&dyn PathMatchingFilterExt>,
    ) -> bool {
        // The permission check is a path-configuration check: it runs for the
        // path the filter was configured for, and only then is the permission
        // requirement of that path consulted.
        <Self as Required<PathMatchingFilter>>::get_object(self)
            .pre_handle(request, response, ext)
            .await
    }
}

impl Required<OncePerRequestFilter> for PermissionsAuthorizationFilter {
    fn get_object(&self) -> &OncePerRequestFilter {
        &self.access_control_filter.once_per_request_filter
    }

    fn get_mut_object(&mut self) -> &mut OncePerRequestFilter {
        &mut self.access_control_filter.once_per_request_filter
    }
}

impl Required<PathMatchingFilter> for PermissionsAuthorizationFilter {
    fn get_object(&self) -> &PathMatchingFilter {
        &self.access_control_filter.path_matching_filter
    }

    fn get_mut_object(&mut self) -> &mut PathMatchingFilter {
        &mut self.access_control_filter.path_matching_filter
    }
}

impl Named for PermissionsAuthorizationFilter {
    fn name(&self) -> &str {
        "PermissionsAuthorizationFilter"
    }
}

impl Deref for PermissionsAuthorizationFilter {
    type Target = AuthorizationFilter;

    fn deref(&self) -> &Self::Target {
        &self.authorization_filter
    }
}

impl DerefMut for PermissionsAuthorizationFilter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.authorization_filter
    }
}

impl Default for PermissionsAuthorizationFilter {
    fn default() -> Self {
        Self {
            authorization_filter: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{body::Body, http::Request};
    use next_web_core::any::AnyValue;
    use next_web_core::http::HttpRequest;

    use super::PermissionsAuthorizationFilter;
    use crate::{
        core::util::{object::Object, web::WebUtils},
        web::{
            filter::access_control_filter::AccessControlFilterExt,
            mgt::{
                default_web_security_manager::DefaultWebSecurityManager,
                web_security_manager::WebSecurityManager,
            },
            subject::{
                support::web_delegating_subject::WebDelegatingSubject, web_subject::WebSubject,
            },
        },
    };

    fn request_of_an_anonymous_call() -> Request<Body> {
        let manager: Arc<dyn WebSecurityManager> = Arc::new(DefaultWebSecurityManager::default());
        let subject: Box<dyn WebSubject> =
            Box::new(WebDelegatingSubject::new(None, false, None, None, false, manager));

        let mut request = Request::builder()
            .uri("/printer/print")
            .body(Body::empty())
            .expect("the request is built");
        request.set_attribute(
            WebUtils::SHARED_SUBJECT_KEY,
            AnyValue::BoxedValue(Box::new(Arc::<dyn WebSubject>::from(subject))),
        );

        request
    }

    fn permissions(permissions: &[&str]) -> Option<Object> {
        Some(Object::ListStr(
            permissions.iter().map(ToString::to_string).collect(),
        ))
    }

    #[tokio::test]
    async fn a_chain_that_requires_no_permission_lets_the_request_through() {
        let filter = PermissionsAuthorizationFilter::default();
        let mut request = request_of_an_anonymous_call();
        let mut response = axum::response::Response::default();

        assert!(filter.is_access_allowed(&mut request, &mut response, None).await);
    }

    #[tokio::test]
    async fn a_subject_without_the_permission_is_refused() {
        let filter = PermissionsAuthorizationFilter::default();
        let mut request = request_of_an_anonymous_call();
        let mut response = axum::response::Response::default();

        // One permission or all of them: the anonymous subject of the call holds
        // neither.
        assert!(!filter
            .is_access_allowed(&mut request, &mut response, permissions(&["printer:print"]))
            .await);
        assert!(!filter
            .is_access_allowed(
                &mut request,
                &mut response,
                permissions(&["printer:print", "printer:query"])
            )
            .await);
    }
}

/// The allow branch: a logged in subject that holds the permission the chain requires.
#[cfg(test)]
mod allow_branch_tests {
    use std::sync::Arc;

    use axum::{body::Body, http::Request};
    use next_web_core::any::AnyValue;
    use next_web_core::http::HttpRequest;

    use super::PermissionsAuthorizationFilter;
    use crate::{
        core::{
            realm::simple_account_realm::{SimpleAccount, SimpleAccountRealm},
            util::{object::Object, web::WebUtils},
        },
        web::{
            filter::access_control_filter::AccessControlFilterExt,
            mgt::{
                default_web_security_manager::DefaultWebSecurityManager,
                web_security_manager::WebSecurityManager,
            },
            subject::{
                support::web_delegating_subject::WebDelegatingSubject, web_subject::WebSubject,
            },
        },
    };

    fn request_of_root(manager: Arc<dyn WebSecurityManager>) -> Request<Body> {
        let principals: Arc<dyn crate::core::subject::principal_collection::PrincipalCollection> =
            Arc::new(
                crate::core::subject::simple_principal_collection::SimplePrincipalCollection::new(
                    [Object::Str("root".to_string())],
                    "iniRealm",
                ),
            );
        let subject: Box<dyn WebSubject> = Box::new(WebDelegatingSubject::new(
            Some(principals),
            true,
            None,
            None,
            false,
            manager,
        ));

        let mut request = Request::builder()
            .uri("/printer/print")
            .body(Body::empty())
            .expect("the request is built");
        request.set_attribute(
            WebUtils::SHARED_SUBJECT_KEY,
            AnyValue::BoxedValue(Box::new(Arc::<dyn WebSubject>::from(subject))),
        );

        request
    }

    fn manager_with_printer() -> Arc<dyn WebSecurityManager> {
        let mut realm = SimpleAccountRealm::with_name("iniRealm");
        realm.add_account(SimpleAccount::new("root").with_permission("printer:*"));

        Arc::new(DefaultWebSecurityManager::from(realm))
    }

    fn permissions(permissions: &[&str]) -> Option<Object> {
        Some(Object::ListStr(
            permissions.iter().map(ToString::to_string).collect(),
        ))
    }

    #[tokio::test]
    async fn a_subject_that_holds_the_permission_is_allowed() {
        let filter = PermissionsAuthorizationFilter::default();
        let mut request = request_of_root(manager_with_printer());
        let mut response = axum::response::Response::default();

        assert!(filter
            .is_access_allowed(&mut request, &mut response, permissions(&["printer:print"]))
            .await);
        assert!(filter
            .is_access_allowed(
                &mut request,
                &mut response,
                permissions(&["printer:print", "printer:query"])
            )
            .await);
    }

    #[tokio::test]
    async fn a_permission_the_subject_does_not_hold_is_refused() {
        let filter = PermissionsAuthorizationFilter::default();
        let mut request = request_of_root(manager_with_printer());
        let mut response = axum::response::Response::default();

        assert!(!filter
            .is_access_allowed(&mut request, &mut response, permissions(&["user:delete"]))
            .await);
        assert!(!filter
            .is_access_allowed(
                &mut request,
                &mut response,
                permissions(&["printer:print", "user:delete"])
            )
            .await);
    }
}