use std::ops::{Deref, DerefMut};

use next_web_core::{
    async_trait,
    traits::{
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
        required::Required,
    },
};

use crate::{
    core::util::{object::Object, web::WebUtils},
    web::filter::{
        access_control_filter::AccessControlFilterExt, advice_filter::AdviceFilterExt,
        authz::authorization_filter::AuthorizationFilter,
        once_per_request_filter::OncePerRequestFilter,
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
        _response: &mut dyn HttpResponse,
        mapped_value: Option<Object>,
    ) -> bool {
        let subject = WebUtils::get_subject(request).await;

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
}

#[async_trait]
impl AdviceFilterExt for PermissionsAuthorizationFilter {}

impl Required<OncePerRequestFilter> for PermissionsAuthorizationFilter {
    fn get_object(&self) -> &OncePerRequestFilter {
        &self
            .authorization_filter
            .access_control_filter
            .path_matching_filter
            .advice_filter
            .once_per_request_filter
    }

    fn get_mut_object(&mut self) -> &mut OncePerRequestFilter {
        &mut self
            .authorization_filter
            .access_control_filter
            .path_matching_filter
            .advice_filter
            .once_per_request_filter
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
