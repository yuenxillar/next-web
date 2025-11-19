use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use next_web_core::{
    async_trait,
    traits::{
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
        required::Required,
    },
};

use crate::{
    core::util::object::Object,
    web::filter::{
        access_control_filter::AccessControlFilterExt, advice_filter::AdviceFilterExt,
        authz::permissions_authorization_filter::PermissionsAuthorizationFilter,
        once_per_request_filter::OncePerRequestFilter,
    },
};

#[derive(Clone)]
pub struct HttpMethodPermissionFilter {
    http_method_actions: HashMap<String, String>,

    permissions_authorization_filter: PermissionsAuthorizationFilter,
}

impl HttpMethodPermissionFilter {
    pub(super) const CREATE_ACTION: &'static str = "create";
    pub(super) const READ_ACTION: &'static str = "read";
    pub(super) const UPDATE_ACTION: &'static str = "update";
    pub(super) const DELETE_ACTION: &'static str = "delete";

    pub fn get_http_method_actions(&self) -> &HashMap<String, String> {
        &self.http_method_actions
    }

    fn get_http_method_action(&self, req: &dyn HttpRequest) -> String {
        let method = req.method().to_string().to_lowercase();
        match self.http_method_actions.get(&method).map(|s| s.as_str()) {
            Some(action) => action.to_string(),
            None => method,
        }
    }

    fn build_permissions(&self, configured_perms: Vec<String>, action: &str) -> Vec<String> {
        if configured_perms.is_empty() || action.is_empty() {
            return configured_perms;
        }

        let mut mapped_perms = Vec::with_capacity(configured_perms.len());

        // loop and append :action
        configured_perms.into_iter().for_each(|perm| {
            mapped_perms.push(perm + ":" + action);
        });

        mapped_perms
    }
}

#[async_trait]
impl AccessControlFilterExt for HttpMethodPermissionFilter {
    async fn is_access_allowed(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        mapped_value: Option<Object>,
    ) -> bool {
        let perms = mapped_value
            .as_ref()
            .map(|value| value.as_list_str().unwrap_or_default())
            .unwrap_or_default()
            .iter()
            .map(ToString::to_string)
            .collect();

        // append the http action to the end of the permissions and then back to super
        let action = self.get_http_method_action(request);
        let resolved_perms = self.build_permissions(perms, &action);

        self.permissions_authorization_filter
            .is_access_allowed(request, response, Some(Object::ListStr(resolved_perms)))
            .await
    }
}

#[async_trait]
impl AdviceFilterExt for HttpMethodPermissionFilter {}

impl Required<OncePerRequestFilter> for HttpMethodPermissionFilter {
    fn get_object(&self) -> &OncePerRequestFilter {
        &self
            .permissions_authorization_filter
            .access_control_filter
            .path_matching_filter
            .advice_filter
            .once_per_request_filter
    }

    fn get_mut_object(&mut self) -> &mut OncePerRequestFilter {
        &mut self
            .permissions_authorization_filter
            .access_control_filter
            .path_matching_filter
            .advice_filter
            .once_per_request_filter
    }
}

impl Named for HttpMethodPermissionFilter {
    fn name(&self) -> &str {
        "HttpMethodPermissionFilter"
    }
}

impl Deref for HttpMethodPermissionFilter {
    type Target = PermissionsAuthorizationFilter;

    fn deref(&self) -> &Self::Target {
        &self.permissions_authorization_filter
    }
}

impl DerefMut for HttpMethodPermissionFilter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.permissions_authorization_filter
    }
}

impl Default for HttpMethodPermissionFilter {
    fn default() -> Self {
        let mut http_method_actions: HashMap<String, String> = Default::default();
        HttpMethodAction::get_values()
            .into_iter()
            .for_each(|action| {
                http_method_actions
                    .insert(action.name().to_string(), action.get_action().to_string());
            });

        Self {
            http_method_actions,
            permissions_authorization_filter: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HttpMethodAction {
    DELETE,
    GET,
    HEAD,
    MKCOL,
    OPTIONS,
    POST,
    PUT,
    TRACE,
}

impl HttpMethodAction {
    fn name(self) -> &'static str {
        match self {
            HttpMethodAction::DELETE => "delete",
            HttpMethodAction::GET => "get",
            HttpMethodAction::HEAD => "head",
            HttpMethodAction::MKCOL => "mkcol",
            HttpMethodAction::OPTIONS => "options",
            HttpMethodAction::POST => "post",
            HttpMethodAction::PUT => "put",
            HttpMethodAction::TRACE => "trace",
        }
    }

    fn get_values() -> Vec<Self> {
        vec![
            HttpMethodAction::DELETE,
            HttpMethodAction::GET,
            HttpMethodAction::HEAD,
            HttpMethodAction::MKCOL,
            HttpMethodAction::OPTIONS,
            HttpMethodAction::POST,
            HttpMethodAction::PUT,
            HttpMethodAction::TRACE,
        ]
    }

    fn get_action(self) -> &'static str {
        match self {
            HttpMethodAction::DELETE => HttpMethodPermissionFilter::DELETE_ACTION,
            HttpMethodAction::GET => HttpMethodPermissionFilter::READ_ACTION,
            HttpMethodAction::HEAD => HttpMethodPermissionFilter::READ_ACTION,
            HttpMethodAction::MKCOL => HttpMethodPermissionFilter::CREATE_ACTION,
            HttpMethodAction::OPTIONS => HttpMethodPermissionFilter::READ_ACTION,
            HttpMethodAction::POST => HttpMethodPermissionFilter::CREATE_ACTION,
            HttpMethodAction::PUT => HttpMethodPermissionFilter::UPDATE_ACTION,
            HttpMethodAction::TRACE => HttpMethodPermissionFilter::READ_ACTION,
        }
    }
}
