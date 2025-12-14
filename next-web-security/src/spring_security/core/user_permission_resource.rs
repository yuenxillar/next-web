use axum::http::Method;
use matchit::Router;

use super::auth_group::{AuthGroup, CombinationGroup, CombinationMode};

#[derive(Debug, serde::Deserialize)]
pub struct UserPermissionResourceBuilder {
    pub method: String,
    pub path: String,
    pub role: Vec<String>,
    pub permission: Vec<String>,
    pub mode: Option<CombinationMode>,
}

impl UserPermissionResourceBuilder {
    pub fn method(&self) -> Method {
        let method = self.method.to_uppercase();
        match method.as_str() {
            "POST" => Method::POST,
            "GET" => Method::GET,
            "PUT" => Method::PUT,
            "DELETE" => Method::DELETE,
            _ => Method::POST,
        }
    }
}

impl Into<UserPermissionResource> for Vec<UserPermissionResourceBuilder> {
    fn into(self) -> UserPermissionResource {
        let mut user_permission_resource = UserPermissionResource::new();
        for user in self {
            let method: Method = user.method();
            let path = user.path;
            let role_group = user.role;
            let permission_group = user.permission;
            let mode = user.mode;
            let auth_group = if let Some(mode) = mode {
                AuthGroup::new(
                    None,
                    None,
                    Some(CombinationGroup::new(
                        Some(role_group),
                        Some(permission_group),
                        mode,
                    )),
                )
            } else {
                AuthGroup::new(
                    if role_group.is_empty() {
                        None
                    } else {
                        Some(role_group)
                    },
                    if permission_group.is_empty() {
                        None
                    } else {
                        Some(permission_group)
                    },
                    None,
                )
            };

            user_permission_resource.add_permission(method, path.as_str(), auth_group);
        }
        user_permission_resource
    }
}

#[derive(Clone)]
pub struct UserPermissionResource {
    pub post_permission: Router<AuthGroup>,
    pub get_permission: Router<AuthGroup>,
    pub put_permission: Router<AuthGroup>,
    pub delete_permission: Router<AuthGroup>,
}

impl UserPermissionResource {
    pub fn new() -> Self {
        Self {
            post_permission: Router::new(),
            get_permission: Router::new(),
            put_permission: Router::new(),
            delete_permission: Router::new(),
        }
    }

    pub fn add_permission(&mut self, method: Method, path: &str, mut auth_group: AuthGroup) {
        if auth_group.is_combination() {
            auth_group.set_combination(true);
        }
        let _ = match method {
            Method::POST => self.post_permission.insert(path, auth_group),
            Method::GET => self.get_permission.insert(path, auth_group),
            Method::PUT => self.put_permission.insert(path, auth_group),
            Method::DELETE => self.delete_permission.insert(path, auth_group),
            _ => Ok(()),
        };
    }

    pub fn get_permission(&self, method: &Method, path: &str) -> Option<&AuthGroup> {
        let auth_group = match method {
            &Method::POST => self.post_permission.at(path),
            &Method::GET => self.get_permission.at(path),
            &Method::PUT => self.put_permission.at(path),
            &Method::DELETE => self.delete_permission.at(path),
            _ => return None,
        };
        if let Ok(auth_group) = auth_group {
            return Some(auth_group.value);
        }
        None
    }
}
