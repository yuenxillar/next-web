use std::collections::HashSet;

use next_web_core::{
    Named, Required, async_trait,
    http::{HttpRequest, HttpResponse},
};

use crate::{
    core::util::{object::Object, web::WebUtils},
    web::filter::{
        access_control_filter::AccessControlFilterExt,
        advice_filter::AdviceFilterExt,
        authz::authorization_filter::AuthorizationFilter,
        once_per_request_filter::OncePerRequestFilter,
        path_matching_filter::{PathMatchingFilter, PathMatchingFilterExt},
    },
};

#[derive(Clone)]
pub struct RolesAuthorizationFilter {
    authorization_filter: AuthorizationFilter,
}

#[async_trait]
impl AccessControlFilterExt for RolesAuthorizationFilter {
    async fn is_access_allowed(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        mapped_value: Option<Object>,
    ) -> bool {
        let Some(value) = mapped_value else {
            return true;
        };

        let roles = match value.as_list_str() {
            Some(roles) => roles,
            None => return true,
        };

        let deduped_roles = smart_deduplicate(roles);

        if deduped_roles.is_empty() {
            return true;
        }

        let subject = WebUtils::get_subject(request, response).await;
        subject.has_all_roles(&deduped_roles).await
    }

    async fn on_access_denied(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        mapped_value: Option<Object>,
    ) -> bool {
        // The chain requires a role the subject does not hold: the answer of an
        // authorization filter — a redirect to the page of the application, or
        // 401 — is the answer of this filter.
        self.authorization_filter
            .on_access_denied(request, response, mapped_value)
            .await
    }
}

#[async_trait]
impl AdviceFilterExt for RolesAuthorizationFilter {
    async fn pre_handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        ext: Option<&dyn PathMatchingFilterExt>,
    ) -> bool {
        // The role check is a path-configuration check: it runs for the path the
        // filter was configured for, and only then is the role requirement of
        // that path consulted.
        <Self as Required<PathMatchingFilter>>::get_object(self)
            .pre_handle(request, response, ext)
            .await
    }

    fn apply_unauthorized_url(&mut self, unauthorized_url: &str) -> bool {
        self.authorization_filter
            .apply_unauthorized_url_if_absent(unauthorized_url)
    }
}

impl Required<OncePerRequestFilter> for RolesAuthorizationFilter {
    fn get_object(&self) -> &OncePerRequestFilter {
        &self
            .authorization_filter
            .access_control_filter
            .once_per_request_filter
    }

    fn get_mut_object(&mut self) -> &mut OncePerRequestFilter {
        &mut self
            .authorization_filter
            .access_control_filter
            .once_per_request_filter
    }
}

impl Required<PathMatchingFilter> for RolesAuthorizationFilter {
    fn get_object(&self) -> &PathMatchingFilter {
        &self
            .authorization_filter
            .access_control_filter
            .path_matching_filter
    }

    fn get_mut_object(&mut self) -> &mut PathMatchingFilter {
        &mut self
            .authorization_filter
            .access_control_filter
            .path_matching_filter
    }
}

impl Named for RolesAuthorizationFilter {
    fn name(&self) -> &str {
        "RolesAuthorizationFilter"
    }
}

impl Default for RolesAuthorizationFilter {
    fn default() -> Self {
        Self {
            authorization_filter: Default::default(),
        }
    }
}

/// 根据数据特征选择最优去重策略
fn smart_deduplicate(mut roles: Vec<&str>) -> Vec<&str> {
    match roles.len() {
        0 | 1 => roles,
        2 => {
            if roles[0] == roles[1] {
                vec![roles[0]]
            } else {
                roles
            }
        }
        len if len <= 5 => {
            // 小数据集：使用简单的 Vec 去重
            let mut result: Vec<&str> = Vec::with_capacity(len);
            for role in roles {
                if !result.contains(&role) {
                    result.push(role);
                }
            }
            result
        }
        len if len <= 20 => {
            // 中等数据集：使用 HashSet（保持插入顺序）
            let mut seen = HashSet::with_capacity(len);
            roles
                .iter()
                .filter(|role| seen.insert(*role))
                .map(|s| *s)
                .collect()
        }
        _ => {
            // 大数据集：排序去重（不保持顺序但内存效率高）
            roles.sort_unstable();
            roles.dedup();
            roles
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{body::Body, http::Request};
    use next_web_core::any::AnyValue;
    use next_web_core::http::HttpRequest;

    use super::RolesAuthorizationFilter;
    use crate::{
        core::util::{object::Object, web::WebUtils},
        web::{
            mgt::{
                default_web_security_manager::DefaultWebSecurityManager,
                web_security_manager::WebSecurityManager,
            },
            subject::{
                support::web_delegating_subject::WebDelegatingSubject, web_subject::WebSubject,
            },
        },
    };

    /// Returns a request that carries the subject of the call, the way the filter
    /// chain hands it over.
    fn request_of_an_anonymous_call() -> Request<Body> {
        let manager: Arc<dyn WebSecurityManager> = Arc::new(DefaultWebSecurityManager::default());
        let subject: Box<dyn WebSubject> = Box::new(WebDelegatingSubject::new(
            None, false, None, None, false, manager,
        ));

        let mut request = Request::builder()
            .uri("/admin/reports")
            .body(Body::empty())
            .expect("the request is built");
        request.set_attribute(
            WebUtils::SHARED_SUBJECT_KEY,
            AnyValue::BoxedValue(Box::new(Arc::<dyn WebSubject>::from(subject))),
        );

        request
    }

    fn roles(roles: &[&str]) -> Option<Object> {
        Some(Object::ListStr(
            roles.iter().map(ToString::to_string).collect(),
        ))
    }

    #[tokio::test]
    async fn a_chain_that_requires_no_role_lets_the_request_through() {
        let filter = RolesAuthorizationFilter::default();
        let mut request = request_of_an_anonymous_call();
        let mut response = axum::response::Response::default();

        use crate::web::filter::access_control_filter::AccessControlFilterExt;

        assert!(
            filter
                .is_access_allowed(&mut request, &mut response, None)
                .await
        );
        assert!(
            filter
                .is_access_allowed(&mut request, &mut response, Some(Object::ListStr(vec![])))
                .await
        );
    }

    #[tokio::test]
    async fn a_subject_without_the_role_is_refused() {
        use crate::web::filter::access_control_filter::AccessControlFilterExt;

        let filter = RolesAuthorizationFilter::default();
        let mut request = request_of_an_anonymous_call();
        let mut response = axum::response::Response::default();

        // The subject of the call is anonymous: it holds no role, so a chain that
        // requires one refuses the request.
        assert!(
            !filter
                .is_access_allowed(&mut request, &mut response, roles(&["admin"]))
                .await
        );
        assert!(
            !filter
                .is_access_allowed(&mut request, &mut response, roles(&["admin", "auditor"]))
                .await
        );
        // The filter refuses the request; what the client sees — a redirect to the
        // login page, or 401 — is the answer of the filter chain around it.
        let _ = &response;
    }
}

/// The allow branch: a logged in subject that holds the role the chain requires.
#[cfg(test)]
mod allow_branch_tests {
    use std::sync::Arc;

    use axum::{body::Body, http::Request};
    use next_web_core::any::AnyValue;
    use next_web_core::http::HttpRequest;

    use super::RolesAuthorizationFilter;
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
            .uri("/admin")
            .body(Body::empty())
            .expect("the request is built");
        request.set_attribute(
            WebUtils::SHARED_SUBJECT_KEY,
            AnyValue::BoxedValue(Box::new(Arc::<dyn WebSubject>::from(subject))),
        );

        request
    }

    fn manager_with_admin() -> Arc<dyn WebSecurityManager> {
        let mut realm = SimpleAccountRealm::with_name("iniRealm");
        realm.add_account(SimpleAccount::new("root").with_role("admin"));

        Arc::new(DefaultWebSecurityManager::from(realm))
    }

    fn roles(roles: &[&str]) -> Option<Object> {
        Some(Object::ListStr(
            roles.iter().map(ToString::to_string).collect(),
        ))
    }

    #[tokio::test]
    async fn a_subject_that_holds_the_role_is_allowed() {
        let filter = RolesAuthorizationFilter::default();
        let mut request = request_of_root(manager_with_admin());
        let mut response = axum::response::Response::default();

        assert!(
            filter
                .is_access_allowed(&mut request, &mut response, roles(&["admin"]))
                .await
        );
    }

    #[tokio::test]
    async fn a_role_the_subject_does_not_hold_is_refused() {
        let filter = RolesAuthorizationFilter::default();
        let mut request = request_of_root(manager_with_admin());
        let mut response = axum::response::Response::default();

        assert!(
            !filter
                .is_access_allowed(&mut request, &mut response, roles(&["auditor"]))
                .await
        );
        assert!(
            !filter
                .is_access_allowed(&mut request, &mut response, roles(&["admin", "auditor"]))
                .await
        );
    }
}
