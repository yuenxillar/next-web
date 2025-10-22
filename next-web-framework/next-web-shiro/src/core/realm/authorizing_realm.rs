use std::{
    any::Any,
    collections::HashSet,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use dashmap::DashMap;
use next_web_core::{async_trait, traits::required::Required};
use tracing::{debug, trace};

use crate::core::{
    authc::{
        authentication_token::AuthenticationToken,
        credential::{
            credentials_matcher::CredentialsMatcher,
            simple_credentials_matcher::SimpleCredentialsMatcher,
        },
    },
    authz::{
        authorization_error::AuthorizationError,
        authorization_info::AuthorizationInfo,
        authorizer::Authorizer,
        permission::{
            Permission, permission_resolver::PermissionResolver,
            permission_resolver_aware::PermissionResolverAware,
            role_permission_resolver::RolePermissionResolver,
            role_permission_resolver_aware::RolePermissionResolverAware,
            wildcard_permission_resolver::WildcardPermissionResolver,
        },
    },
    cache::{cache_manager::CacheManager, cache_manager_aware::CacheManagerAware},
    object::Object,
    realm::{
        authenticating_realm::{AuthenticatingRealm, AuthenticatingRealmSupport},
        caching_realm::CachingRealmSupport,
    },
    subject::principal_collection::PrincipalCollection,
    util::nameable::Nameable,
};

type AuthorizationCache = DashMap<Object, Box<dyn AuthorizationInfo>>;

pub struct AuthorizingRealm {
    authorization_caching_enabled: bool,
    authorization_cache_name: String,

    authorization_cache: Option<AuthorizationCache>,
    authenticating_realm: AuthenticatingRealm,
    permission_resolver: Arc<dyn PermissionResolver>,
    permission_role_resolver: Option<Arc<dyn RolePermissionResolver>>,

    authorizing_realm_support: Option<&'static dyn AuthorizingRealmSupport>,
}

impl AuthorizingRealm {
    const DEFAULT_AUTHORIZATION_CACHE_SUFFIX: &str = ".authorizationCache";
    const INSTANCE_COUNT: AtomicUsize = AtomicUsize::new(0);

    pub fn new(
        cache_manager: Option<Arc<dyn CacheManager>>,
        matcher: Option<Arc<dyn CredentialsMatcher>>,
    ) -> Self {
        let mut authenticating_realm = AuthenticatingRealm::default();
        if let Some(cm) = cache_manager {
            authenticating_realm.get_mut_object().set_cache_manager(cm);
        }

        if let Some(mt) = matcher {
            authenticating_realm.set_credentials_matcher(mt);
        }

        let instance_number = Self::INSTANCE_COUNT.fetch_add(1, Ordering::Relaxed);

        Self {
            authorization_caching_enabled: true,
            authorization_cache_name: format!(
                "{}{}.{}",
                std::any::type_name::<Self>(),
                Self::DEFAULT_AUTHORIZATION_CACHE_SUFFIX,
                if instance_number > 0 {
                    instance_number.to_string()
                } else {
                    "".to_string()
                }
            ),
            permission_resolver: Arc::new(WildcardPermissionResolver::default()),
            permission_role_resolver: Default::default(),
            authorization_cache: None,
            authorizing_realm_support: None,
            authenticating_realm,
        }
    }

    pub fn set_authorization_cache(&mut self, authorization_cache: AuthorizationCache) {
        self.authorization_cache = Some(authorization_cache);
    }

    pub fn get_authorization_cache(&self) -> Option<&AuthorizationCache> {
        self.authorization_cache.as_ref()
    }
    pub fn get_authorization_cache_name(&self) -> &str {
        &self.authorization_cache_name
    }

    pub fn set_authorization_cache_name(&mut self, authorization_cache_name: &str) {
        self.authorization_cache_name = authorization_cache_name.to_string();
    }

    pub fn is_authorization_caching_enabled(&self) -> bool {
        self.authorization_caching_enabled
    }

    pub fn set_authorization_caching_enabled(&mut self, authorization_caching_enabled: bool) {
        self.authorization_caching_enabled = authorization_caching_enabled;
        if authorization_caching_enabled {
            self.authenticating_realm
                .get_mut_object()
                .set_caching_enabled(true);
        }
    }

    pub fn get_permission_resolver(&self) -> &dyn PermissionResolver {
        self.permission_resolver.as_ref()
    }

    pub fn set_permission_resolver(&mut self, resolver: impl PermissionResolver + 'static) {
        self.permission_resolver = Arc::new(resolver);
    }

    pub fn get_role_permission_resolver(&self) -> Option<&dyn RolePermissionResolver> {
        self.permission_role_resolver.as_deref()
    }

    pub fn set_role_permission_resolver(
        &mut self,
        resolver: impl RolePermissionResolver + 'static,
    ) {
        self.permission_role_resolver = Some(Arc::new(resolver));
    }

    fn clear_cached_authorization_info(&mut self, principals: &dyn PrincipalCollection) {
        let cache = self.get_available_authorization_cache();

        if let Some(cache) = cache {
            let key = self.get_authorization_cache_key(principals);
            cache.remove(&Object::Str(key.id()));
        }
    }

    fn get_available_authorization_cache(&self) -> Option<&AuthorizationCache> {
        let val = self.is_authorization_caching_enabled();

        if !val {
            // 如果缓存未启用，直接返回现有缓存（如果有）
            return self.get_authorization_cache();
        }

        // 缓存启用时，使用懒加载获取或创建缓存
        self.get_authorization_cache_lazy()
    }

    fn get_authorization_cache_lazy(&self) -> Option<&AuthorizationCache> {
        if self.authorization_cache.is_none() {
            debug!("No authorizationCache instance set.  Checking for a cacheManager...");

            let cache_manager = self.authenticating_realm.get_object().get_cache_manager();
            if let Some(cache_manager) = cache_manager {
                let cache_name = self.get_authorization_cache_name();
                debug!(
                    "CacheManager [{}] has been configured.  Building authorization cache named [{}]",
                    cache_manager, cache_name
                );

                let authorization_cache = DashMap::new();

                if let Some(caches) = cache_manager.get_cache(cache_name).map(|item| {
                    item.iter()
                        .filter_map(|(key, val)| match val {
                            Object::Obj(obj) => {
                                if let Some(obj) = (obj.as_ref() as &dyn Any)
                                    .downcast_ref::<Box<dyn AuthorizationInfo>>()
                                {
                                    Some((key.clone(), obj.clone()))
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                }) {
                    for (key, value) in caches {
                        authorization_cache.insert(Object::Str(key), value);
                    }
                }

                if let Some(cache) = self.authorization_cache.as_ref() {
                    cache.clear();
                    for (key, value) in authorization_cache.into_iter() {
                        cache.insert(key, value);
                    }
                }
            } else {
                debug!(
                    "No cache or cacheManager properties have been set.  Authorization cache cannot be obtained."
                );
            }
        }

        self.authorization_cache.as_ref()
    }

    fn get_authorization_cache_key<'a>(
        &self,
        principals: &'a dyn PrincipalCollection,
    ) -> &'a dyn PrincipalCollection {
        principals
    }

    fn get_authorization_info(
        &self,
        principals: Option<&dyn PrincipalCollection>,
    ) -> Option<Box<dyn AuthorizationInfo>> {
        if principals.is_none() {
            return None;
        }

        let principals = principals.unwrap();
        let mut info: Option<Box<dyn AuthorizationInfo>> = None;

        trace!(
            "Retrieving AuthorizationInfo for principals [{}]",
            principals
        );

        if let Some(cache) = self.get_available_authorization_cache() {
            trace!("Attempting to retrieve the AuthorizationInfo from cache.");
            info = cache
                .get(&Object::Str(principals.id()))
                .map(|val| val.value().clone());

            if info.is_none() {
                trace!(
                    "No AuthorizationInfo found in cache for principals [{}]",
                    principals
                );
            } else {
                trace!(
                    "AuthorizationInfo found in cache for principals [{}]",
                    principals
                );
            }
        }

        if info.is_none() {
            if let Some(authorizing_realm_support) = self.authorizing_realm_support.as_ref() {
                info = authorizing_realm_support.do_get_authorization_info(principals);
            }
            if let Some(info) = info.as_ref() {
                if let Some(cache) = self.get_available_authorization_cache() {
                    trace!(
                        "Caching authorization info for principals: [{}].",
                        principals
                    );
                    cache.insert(Object::Str(principals.id()), info.clone());
                }
            }
        }

        info
    }

    fn _is_permitted(
        &self,
        permission: &dyn Permission,
        info: Option<&dyn AuthorizationInfo>,
    ) -> bool {
        let perms = self.get_permissions(info);
        if !perms.is_empty() {
            for perm in perms {
                if perm.implies(permission) {
                    return true;
                }
            }
        }

        false
    }

    fn _is_permitted_from_list(
        &self,
        permissions: &[Box<dyn Permission>],
        info: Option<&dyn AuthorizationInfo>,
    ) -> Vec<bool> {
        permissions
            .iter()
            .map(|p| self._is_permitted(p.as_ref(), info))
            .collect()
    }

    fn _is_permitted_all(
        &self,
        info: Option<&dyn AuthorizationInfo>,
        permissions: &[Box<dyn Permission>],
    ) -> bool {
        if !permissions.is_empty() {
            for p in permissions {
                if !self._is_permitted(p.as_ref(), info) {
                    return false;
                }
            }
        }

        true
    }

    fn _check_permission(
        &self,
        permission: &dyn Permission,
        info: Option<&dyn AuthorizationInfo>,
    ) -> Result<(), AuthorizationError> {
        if self._is_permitted(permission, info) {
            Ok(())
        } else {
            Err(AuthorizationError::Unauthorized(format!(
                "User is not permitted [{}]",
                permission
            )))
        }
    }

    fn _check_permissions(
        &self,
        permissions: &[Box<dyn Permission>],
        info: Option<&dyn AuthorizationInfo>,
    ) -> Result<(), AuthorizationError> {
        if !permissions.is_empty() {
            for p in permissions {
                self._check_permission(p.as_ref(), info).ok();
            }
        }

        Ok(())
    }

    fn _has_role(&self, role_name: &str, info: Option<&dyn AuthorizationInfo>) -> bool {
        if let Some(info) = info {
            let roles = info.get_roles();
            if !roles.is_empty() && roles.contains(&role_name.to_string()) {
                return true;
            }
        }

        false
    }

    fn _has_roles(
        &self,
        role_identifiers: &[&str],
        info: Option<&dyn AuthorizationInfo>,
    ) -> Vec<bool> {
        if !role_identifiers.is_empty() {
            return role_identifiers
                .into_iter()
                .map(|r| self._has_role(r, info))
                .collect::<Vec<_>>();
        }
        vec![]
    }

    fn _has_all_roles(
        &self,
        role_identifiers: &[&str],
        info: Option<&dyn AuthorizationInfo>,
    ) -> bool {
        if !role_identifiers.is_empty() {
            for role_name in role_identifiers {
                if !self._has_role(role_name, info) {
                    return false;
                }
            }
        }
        true
    }

    fn _check_role(
        &self,
        role: &str,
        info: Option<&dyn AuthorizationInfo>,
    ) -> Result<(), AuthorizationError> {
        if !self._has_role(role, info) {
            return Err(AuthorizationError::Unauthorized(format!(
                "User does not have role [{}]",
                role
            )));
        }
        Ok(())
    }

    fn _check_roles(
        &self,
        roles: &[&str],
        info: Option<&dyn AuthorizationInfo>,
    ) -> Result<(), AuthorizationError> {
        if !roles.is_empty() {
            for role_name in roles {
                self._check_role(role_name, info)?;
            }
        }

        Ok(())
    }

    fn get_permissions(&self, info: Option<&dyn AuthorizationInfo>) -> Vec<Box<dyn Permission>> {
        let mut permissions: HashSet<Box<dyn Permission>> = HashSet::new();

        if let Some(info) = info {
            let mut perms = info.get_dyn_permissions();
            if !perms.is_empty() {
                permissions.extend(perms);
            }

            perms = self.resolve_permissions(info.get_permissions());
            if !perms.is_empty() {
                permissions.extend(perms);
            }

            perms = self.resolve_role_permissions(info.get_roles());
            if !perms.is_empty() {
                permissions.extend(perms);
            }
        }

        if permissions.is_empty() {
            return Vec::with_capacity(0);
        } else {
            return permissions.into_iter().collect();
        }
    }

    fn resolve_permissions(&self, string_perms: Vec<String>) -> Vec<Box<dyn Permission>> {
        let mut perms = HashSet::new();
        let resolver = self.get_permission_resolver();

        if !string_perms.is_empty() {
            for str_permission in string_perms {
                let str_permission = str_permission.trim();
                if !str_permission.is_empty() && !str_permission.eq("") {
                    perms.insert(resolver.resolve_permission(str_permission));
                }
            }
        }

        perms.into_iter().collect()
    }

    fn resolve_role_permissions(&self, role_names: Vec<String>) -> Vec<Box<dyn Permission>> {
        let mut perms: HashSet<Box<dyn Permission>> = HashSet::new();
        let resolver = self.get_role_permission_resolver();

        match resolver {
            Some(resolver) => {
                if !role_names.is_empty() {
                    for role_name in role_names {
                        let role_name = role_name.trim();
                        if !role_name.is_empty() && !role_name.eq("") {
                            perms.extend(resolver.resolve_permissions_in_role(role_name));
                        }
                    }
                }

                perms.into_iter().collect()
            }
            None => return Vec::with_capacity(0),
        }
    }
}

#[async_trait]
impl AuthenticatingRealmSupport for AuthorizingRealm {
    async fn on_init(&self) {
        // self.authenticating_realm.on_init().await;
        self.get_available_authorization_cache();
    }

    async fn do_get_authentication_info(
        &self,
        _token: &dyn AuthenticationToken,
    ) -> Option<Box<dyn crate::core::authc::authentication_info::AuthenticationInfo>> {
        None
    }
}

impl CachingRealmSupport for AuthorizingRealm {
    fn after_cache_manager_set(&mut self) {
        self.authenticating_realm.after_cache_manager_set();
        self.get_available_authorization_cache();
    }

    fn do_clear_cache(&mut self, principals: &dyn PrincipalCollection) {
        self.authenticating_realm.do_clear_cache(principals);

        self.clear_cached_authorization_info(principals);
    }
}

impl Default for AuthorizingRealm {
    fn default() -> Self {
        Self::new(None, Some(Arc::new(SimpleCredentialsMatcher::default())))
    }
}

impl Authorizer for AuthorizingRealm {
    fn is_permitted(
        &self,
        principals: Option<&dyn PrincipalCollection>,
        permission: &dyn Permission,
    ) -> bool {
        let info = self.get_authorization_info(principals);
        self._is_permitted(permission, info.as_deref())
    }

    fn is_permitted_from_str(
        &self,
        principals: Option<&dyn PrincipalCollection>,
        permission: &str,
    ) -> bool {
        let permission = self
            .get_permission_resolver()
            .resolve_permission(permission);

        self.is_permitted(principals, permission.as_ref())
    }

    fn is_permitted_from_str_list(
        &self,
        principals: Option<&dyn PrincipalCollection>,
        permissions: &[&str],
    ) -> Vec<bool> {
        let perms = permissions
            .iter()
            .map(|p| self.get_permission_resolver().resolve_permission(p))
            .collect::<Vec<Box<dyn Permission>>>();
        self.is_permitted_from_permission_list(principals, &*perms)
    }
    fn is_permitted_from_permission_list(
        &self,
        principals: Option<&dyn PrincipalCollection>,
        permissions: &[Box<dyn Permission>],
    ) -> Vec<bool> {
        let info = self.get_authorization_info(principals);
        self._is_permitted_from_list(permissions, info.as_deref())
    }

    fn is_permitted_all(
        &self,
        principals: Option<&dyn PrincipalCollection>,
        permissions: &[Box<dyn Permission>],
    ) -> bool {
        let info = self.get_authorization_info(principals);
        self._is_permitted_all(info.as_deref(), permissions)
    }
    fn is_permitted_all_from_str(
        &self,
        principals: Option<&dyn PrincipalCollection>,
        permissions: &[&str],
    ) -> bool {
        if !permissions.is_empty() {
            let perms = permissions
                .into_iter()
                .map(|p| self.get_permission_resolver().resolve_permission(p))
                .collect::<Vec<Box<dyn Permission>>>();

            return self.is_permitted_all(principals, &*perms);
        }

        false
    }
    fn check_permission(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &dyn Permission,
    ) -> Result<(), AuthorizationError> {
        let info = self.get_authorization_info(principal);

        self._check_permission(permission, info.as_deref())
    }

    fn check_permission_from_str(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &str,
    ) -> Result<(), AuthorizationError> {
        self.check_permission(
            principal,
            self.get_permission_resolver()
                .resolve_permission(permissions)
                .as_ref(),
        )
    }

    fn check_permissions(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[Box<dyn Permission>],
    ) -> Result<(), AuthorizationError> {
        self._check_permissions(
            permissions,
            self.get_authorization_info(principal).as_deref(),
        )
    }
    fn check_permissions_from_str(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[&str],
    ) -> Result<(), AuthorizationError> {
        if !permissions.is_empty() {
            for perm_string in permissions {
                self.check_permission_from_str(principal, perm_string)?;
            }
        }
        Ok(())
    }

    // === 角色检查 ===
    fn has_role(&self, principal: Option<&dyn PrincipalCollection>, role_identifier: &str) -> bool {
        self._has_role(
            role_identifier,
            self.get_authorization_info(principal).as_deref(),
        )
    }
    fn has_roles(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        role_identifiers: &[&str],
    ) -> Vec<bool> {
        let info = self.get_authorization_info(principal);
        self._has_roles(role_identifiers, info.as_deref())
    }
    fn has_all_roles(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        role_identifiers: &[&str],
    ) -> bool {
        self._has_all_roles(
            role_identifiers,
            self.get_authorization_info(principal).as_deref(),
        )
    }

    // === 角色断言 ===
    fn check_role(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        role: &str,
    ) -> Result<(), AuthorizationError> {
        self._check_role(role, self.get_authorization_info(principal).as_deref())
    }

    fn check_roles(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        roles: &[&str],
    ) -> Result<(), AuthorizationError> {
        self._check_roles(roles, self.get_authorization_info(principal).as_deref())
    }
}

impl PermissionResolverAware for AuthorizingRealm {
    fn set_permission_resolver(&mut self, permission_resolver: impl PermissionResolver + 'static) {
        self.permission_resolver = Arc::new(permission_resolver);
    }
}

impl RolePermissionResolverAware for AuthorizingRealm {
    fn set_role_permission_resolver(
        &mut self,
        permission_role_resolver: impl RolePermissionResolver + 'static,
    ) {
        self.permission_role_resolver = Some(Arc::new(permission_role_resolver));
    }
}

impl Nameable for AuthorizingRealm {
    fn set_name(&mut self, name: &str) {
        self.authenticating_realm.set_name(name);
        if self
            .authorization_cache_name
            .starts_with(std::any::type_name::<Self>())
        {
            self.authorization_cache_name =
                format!("{}{}", name, Self::DEFAULT_AUTHORIZATION_CACHE_SUFFIX);
        }
    }
}

impl Required<AuthenticatingRealm> for AuthorizingRealm {
    fn get_object(&self) -> &AuthenticatingRealm {
        &self.authenticating_realm
    }

    fn get_mut_object(&mut self) -> &mut AuthenticatingRealm {
        &mut self.authenticating_realm
    }
}

pub trait AuthorizingRealmSupport
where
    Self: Send + Sync,
    Self: AuthenticatingRealmSupport,
{
    fn do_get_authorization_info(
        &self,
        principals: &dyn PrincipalCollection,
    ) -> Option<Box<dyn AuthorizationInfo>>;
}
