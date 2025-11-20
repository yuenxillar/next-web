use std::collections::HashSet;

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
pub struct RolesAuthorizationFilter {
    authorization_filter: AuthorizationFilter,
}

#[async_trait]
impl AccessControlFilterExt for RolesAuthorizationFilter {
    async fn is_access_allowed(
        &self,
        request: &mut dyn HttpRequest,
        _response: &mut dyn HttpResponse,
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

        let subject = WebUtils::get_subject(request).await;
        subject.has_all_roles(&deduped_roles).await
    }
}

#[async_trait]
impl AdviceFilterExt for RolesAuthorizationFilter {}

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
            .path_matching_filter
            .once_per_request_filter
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
                .map(|s| s.clone())
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
