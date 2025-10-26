use std::{any::Any, sync::{
    atomic::{AtomicUsize, Ordering}, Arc
}};

use dashmap::DashMap;
use next_web_core::{async_trait, traits::required::Required};
use tracing::{debug, trace};

use crate::core::{
    authc::{
        authentication_error::AuthenticationError, authentication_info::AuthenticationInfo,
        authentication_token::AuthenticationToken,
        credential::credentials_matcher::CredentialsMatcher, logout_aware::LogoutAware,
    },
    cache::{cache_manager::CacheManager, cache_manager_aware::CacheManagerAware, default_cache_manager::DefaultCacheManager},
    util::object::Object,
    realm::{caching_realm::{CachingRealm, CachingRealmSupport}, Realm},
    subject::principal_collection::PrincipalCollection,
    util::nameable::Nameable,
};

type AuthenticationCache = DashMap<Object, Box<dyn AuthenticationInfo>>;
pub struct AuthenticatingRealm
where
    Self: Required<CachingRealm>,
{
    credentials_matcher: Option<Arc<dyn CredentialsMatcher>>,

    authentication_caching_enabled: bool,
    authentication_cache_name: String,

    authentication_cache: Option<AuthenticationCache>,
    caching_realm: CachingRealm,

    authenticating_realm_support: Option<&'static dyn AuthenticatingRealmSupport>,

    authentication_token_type: Option<&'static str>,
}

impl AuthenticatingRealm {
    const DEFAULT_AUTHENTICATION_CACHE_SUFFIX: &str = ".authenticationCache";
    const INSTANCE_COUNT: AtomicUsize = AtomicUsize::new(0);

    pub fn new(
        cache_manager: Option<DefaultCacheManager>,
        credentials_matcher: Option<Arc<dyn CredentialsMatcher>>,
    ) -> Self {
        let instance_number = Self::INSTANCE_COUNT.fetch_add(1, Ordering::Relaxed);
        let mut authenticating_realm = Self {
            credentials_matcher: None,
            authentication_caching_enabled: false,
            authentication_cache_name: format!(
                "{}{}.{}",
                std::any::type_name::<Self>(),
                Self::DEFAULT_AUTHENTICATION_CACHE_SUFFIX,
                if instance_number > 0 {
                    instance_number.to_string()
                } else {
                    "".to_string()
                }
            ),
            authentication_cache: None,
            authenticating_realm_support: None,
            caching_realm: Default::default(),
            authentication_token_type: Default::default(),
        };

        if let Some(cache_manager) = cache_manager {
            authenticating_realm
                .caching_realm
                .set_cache_manager(cache_manager);
        }

        if let Some(credentials_matcher) = credentials_matcher {
            authenticating_realm.set_credentials_matcher(credentials_matcher);
        }
        authenticating_realm
    }

    pub fn from_credentials_matcher<T: CredentialsMatcher + 'static>(
        credentials_matcher: T,
    ) -> Self {
        Self::new(None, Some(Arc::new(credentials_matcher)))
    }

    pub fn get_credentials_matcher(&self) -> Option<&dyn CredentialsMatcher> {
        self.credentials_matcher.as_deref()
    }

    pub fn set_credentials_matcher(&mut self, credentials_matcher: Arc<dyn CredentialsMatcher>) {
        self.credentials_matcher = Some(credentials_matcher);
    }

    pub fn get_authentication_token_type(&self) -> Option<&'static str> {
        self.authentication_token_type
    }

    pub fn set_authentication_token_type(&mut self, authentication_token_type: &'static str) {
        self.authentication_token_type = Some(authentication_token_type);
    }

    pub fn get_authentication_cache(&self) -> Option<&AuthenticationCache> {
        self.authentication_cache.as_ref()
    }

    pub fn set_authentication_cache(&mut self, authentication_cache: AuthenticationCache) {
        self.authentication_cache = Some(authentication_cache);
    }

    pub fn get_authentication_cache_name(&self) -> &str {
        &self.authentication_cache_name
    }

    pub fn set_authentication_cache_name(&mut self, authentication_cache_name: &str) {
        self.authentication_cache_name = authentication_cache_name.to_string();
    }

    pub fn is_authentication_caching_enabled(&self) -> bool {
        self.authentication_caching_enabled && self.caching_realm.is_caching_enabled()
    }

    pub fn set_authentication_caching_enabled(&mut self, authentication_caching_enabled: bool) {
        self.authentication_caching_enabled = authentication_caching_enabled;
        if authentication_caching_enabled {
            self.caching_realm.set_caching_enabled(true);
        }
    }

    pub fn set_name(&mut self, name: &str) {
        self.caching_realm.set_name(name);

        if self
            .authentication_cache_name
            .starts_with(std::any::type_name::<Self>())
        {
            self.authentication_cache_name =
                format!("{}{}", name, Self::DEFAULT_AUTHENTICATION_CACHE_SUFFIX);
        }
    }
   
    pub(crate) fn get_cached_authentication_info(
        &self,
        token: &dyn AuthenticationToken,
    ) -> Option<Box<dyn AuthenticationInfo>> {
        let mut info = None;
        let cache = self.get_available_authentication_cache();
        if let Some(cache) = cache {
            trace!("Attempting to retrieve the AuthenticationInfo from cache.");

            let key = self.get_authentication_cache_key_from_token(token);
            info = cache.get(&key).map(|a| a.value().clone());

            if let Some(info) = info.as_deref() {
                trace!("Found cached AuthenticationInfo for key [{}]", info);
            } else {
                trace!("No AuthenticationInfo found in cache for key [{}]", "null");
            }
        }

        info
    }

    pub(crate) fn cache_authentication_info_if_possible(
        &self,
        token: &dyn AuthenticationToken,
        info: Box<dyn AuthenticationInfo>,
    ) {
        if !self.is_authentication_caching_enabled() {
            debug!(
                "AuthenticationInfo caching is disabled for info [{}].  Submitted token: [{}].",
                info, token
            );
            return;
        }

        let cache = self.get_available_authentication_cache();
        let key = self.get_authentication_cache_key_from_token(token);

        trace!(
            "Cached AuthenticationInfo for continued authentication.  key=[{}], value=[{}].",
            key, info
        );

        if let Some(cache) = cache {
            cache.insert(key, info);
        }
    }

    fn get_authentication_cache_key_from_token(&self, token: &dyn AuthenticationToken) -> Object {
        token.get_principal().clone()
    }

    fn get_authentication_cache_key_from_principals(
        &self,
        principals: &dyn PrincipalCollection,
    ) -> Object {
        self.caching_realm.get_available_principal(principals)
    }

    fn get_available_authentication_cache(&self) -> Option<&AuthenticationCache> {
        let mut cache = self.get_authentication_cache();
        let authc_caching_enabled = self.is_authentication_caching_enabled();

        if cache.is_none() && authc_caching_enabled {
            cache = self.get_authentication_cache_lazy();
        }

        cache
    }

    fn get_authentication_cache_lazy(&self) -> Option<&AuthenticationCache> {
        match self.authentication_cache.as_ref() {
            Some(_) => todo!(),
            None => {
                trace!("No authenticationCache instance set.  Checking for a cacheManager...");

                let cache_manager = self.caching_realm.get_cache_manager();

                if let Some(cache_manager) = cache_manager {
                    let cache_name = self.get_authentication_cache_name();
                    debug!(
                        "CacheManager [{}] configured.  Building authentication cache '{}'",
                        cache_manager, cache_name
                    );

                    let authorization_cache = DashMap::new();

                    if let Some(caches) = cache_manager.get_cache(cache_name).map(|item| {
                        item.iter()
                            .filter_map(|(key, val)| match val {
                                Object::Obj(obj) => {
                                    if let Some(obj) = (obj.as_ref() as &dyn Any)
                                        .downcast_ref::<Box<dyn AuthenticationInfo>>()
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

                    if let Some(cache) = self.authentication_cache.as_ref() {
                        cache.clear();
                        for (key, value) in authorization_cache.into_iter() {
                            cache.insert(key, value);
                        }
                    }
                }
            }
        };

        self.authentication_cache.as_ref()
    }

    fn assert_credentials_match(
        &self,
        token: &dyn AuthenticationToken,
        info: &dyn AuthenticationInfo,
    ) -> Result<(), AuthenticationError> {
        let cm = self.get_credentials_matcher();
        if let Some(cm) = cm {
            if !cm.do_credentials_match(token, info) {
                let msg = format!(
                    "Submitted credentials for token [{token}] did not match the expected credentials."
                );
                return Err(AuthenticationError::Custom(msg));
            }
        } else {
            return Err(AuthenticationError::Custom(
                "A CredentialsMatcher must be configured in order to verify 
                    credentials during authentication.  If you do not wish for credentials to be examined, you 
                    can configure an AllowAllCredentialsMatcher instance.".to_string() )
            );
        };

        Ok(())
    }

    pub async fn init(&self, authenticating_realm_support:  Option<&mut dyn AuthenticatingRealmSupport>) {
        self.get_available_authentication_cache();

        if let Some(authenticating_realm_support) =  authenticating_realm_support {
            authenticating_realm_support.on_init().await;
        }
    }

    pub fn clear_cached_authentication_info(&mut self, principals: &dyn PrincipalCollection) {
        if !principals.is_empty() {
            let key = self.get_authentication_cache_key_from_principals(principals);
            match self.get_available_authentication_cache() {
                Some(cache) => {
                    cache.remove(&key);
                }
                None => return,
            }
        }
    }

    // fn is_authentication_caching_enabled(&self, token: &dyn AuthenticationToken, info: &dyn AuthenticationInfo) -> _ {
    //     todo!()
    // }
}


impl Default for AuthenticatingRealm {
    fn default() -> Self {
        Self::new(None, None)
    }
}

#[async_trait]
impl Realm for AuthenticatingRealm {
    fn get_name(&self) -> &str {
        ""
    }

    fn supports(&self, token: &dyn AuthenticationToken) -> bool {
        true
    }

    async fn get_authentication_info(
        &self,
        token: &dyn AuthenticationToken,
    ) -> Option<Box<dyn AuthenticationInfo>> {
        let mut info = self.get_cached_authentication_info(token);

        if let Some(info) = info.as_ref() {
            debug!(
                "Using cached authentication info [{}] to perform credentials matching.",
                info
            );
        } else {
            if let Some(authenticating_realm_support) = self.authenticating_realm_support {
                info = authenticating_realm_support.do_get_authentication_info(token).await;
            }
            if let Some(info) = info.as_ref() {
                debug!(
                    "Looked up AuthenticationInfo [{}] from doGetAuthenticationInfo",
                    info
                );
            }

            if let Some(info) = info.as_ref() {
                self.cache_authentication_info_if_possible(token, info.clone());
            }
        }

        if let Some(info) = info.as_deref() {
            self.assert_credentials_match(token, info).unwrap();
        } else {
            debug!(
                "No AuthenticationInfo found for submitted AuthenticationToken [{}].  Returning null.",
                token
            );
        }

        info
    }
}
impl Required<CachingRealm> for AuthenticatingRealm {
    fn get_object(&self) -> &CachingRealm {
        &self.caching_realm
    }

    fn get_mut_object(&mut self) -> &mut CachingRealm {
        &mut self.caching_realm
    }
}

impl LogoutAware for AuthenticatingRealm {
    fn on_logout(&mut self, principals: &dyn PrincipalCollection) {
        if self.caching_realm.clear_cache(principals) {
            self.do_clear_cache(principals);
        }
    }
}

impl CachingRealmSupport for AuthenticatingRealm{
    fn after_cache_manager_set(&mut self) {
        self.get_available_authentication_cache();
    }

    fn do_clear_cache(&mut self, principals: &dyn PrincipalCollection) {
        self.clear_cached_authentication_info(principals);
    }
}

#[async_trait]
pub trait AuthenticatingRealmSupport: Send + Sync {
    async fn on_init(&self) {

    }

    async fn do_get_authentication_info(
        &self,
        token: &dyn AuthenticationToken,
    ) -> Option<Box<dyn AuthenticationInfo>>;
}
