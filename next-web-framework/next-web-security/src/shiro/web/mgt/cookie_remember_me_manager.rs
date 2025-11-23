use std::{ops::Deref, sync::Arc};

use base64::{prelude::BASE64_STANDARD, Engine};
use next_web_core::{
    error::BoxError,
    traits::http::{
        http_request::{HttpRequest, IDENTITY_REMOVED_KEY},
        http_response::HttpResponse,
    },
};
use tracing::{debug, error, trace, warn};

use crate::{
    core::{
        authc::{
            authentication_error::AuthenticationError, authentication_info::AuthenticationInfo,
            authentication_token::AuthenticationToken,
        },
        mgt::{
            default_remember_me_manager::{DefaultRememberMeManager, DefaultRememberMeManagerExt},
            remember_me_manager::RememberMeManager,
        },
        subject::principal_collection::PrincipalCollection,
    },
    web::{
        subject::{web_subject::WebSubject, web_subject_context::WebSubjectContext},
        Cookie, SimpleCookie,
    },
};

#[derive(Clone)]
pub struct CookieRememberMeManager {
    cookie: Arc<dyn Cookie>,
    default_remember_me_manager: DefaultRememberMeManager,
}

impl CookieRememberMeManager {
    pub const DEFAULT_REMEMBER_ME_COOKIE_NAME: &'static str = "rememberMe";

    pub fn new(key: Vec<u8>) -> Self {
        Self {
            cookie: Arc::new(Self::create_default_cookie()),
            default_remember_me_manager: DefaultRememberMeManager::with_key(key),
        }
    }

    pub fn get_cookie(&self) -> &dyn Cookie {
        self.cookie.as_ref()
    }

    pub fn set_cookie<T: Cookie + 'static>(&mut self, cookie: T) {
        self.cookie = Arc::new(cookie);
    }

    fn is_identity_removed(&self, req: &dyn HttpRequest) -> bool {
        if let Some(removed) = req
            .get_attribute(IDENTITY_REMOVED_KEY)
            .map(|val| val.as_boolean().unwrap_or_default())
        {
            return removed;
        }

        false
    }

    fn create_default_cookie() -> SimpleCookie {
        let mut cookie = SimpleCookie::new(Self::DEFAULT_REMEMBER_ME_COOKIE_NAME);
        cookie.set_http_only(true);
        // One year should be long enough - most sites won't object to requiring a user to log in if they haven't visited
        // in a year:
        cookie.set_max_age(SimpleCookie::ONE_YEAR);

        cookie
    }

    fn ensure_padding(&self, base64: &str) -> String {
        let length = base64.len();
        if length % 4 != 0 {
            let mut sb = String::from(base64);
            while sb.len() % 4 != 0 {
                sb.push('=');
            }
            sb
        } else {
            base64.to_string()
        }
    }
}

impl CookieRememberMeManager {
    fn get_remembered_serialized_identity(
        &self,
        _subject_context: &dyn WebSubjectContext,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Option<Vec<u8>> {
        if self.is_identity_removed(req) {
            return None;
        }

        let base64 = self.get_cookie().read_value(req, resp);

        // Browsers do not always remove cookies immediately (SHIRO-183)
        // ignore cookies that are scheduled for removal
        if let Some(base64) = base64.as_ref() {
            if SimpleCookie::DELETED_COOKIE_VALUE.eq(base64) {
                return None;
            }
        }

        if let Some(mut base64) = base64 {
            base64 = self.ensure_padding(&base64);
            trace!("Acquired Base64 encoded identity [{}]", base64);

            let decoded = match BASE64_STANDARD.decode(&base64.as_bytes()) {
                Ok(decoded) => decoded,
                Err(error) => {
                    // https://issues.apache.org/jira/browse/SHIRO-766:
                    // If the base64 string cannot be decoded, just assume there is no valid cookie value.
                    self.get_cookie().remove_from(req, resp);

                    warn!(
                        "Unable to decode existing base64 encoded entity: [{}], error: {}",
                        base64, error
                    );
                    return None;
                }
            };

            return Some(decoded);
        }

        None
    }

    fn _forget_identity(&self, req: &dyn HttpRequest, resp: &mut dyn HttpResponse) {
        self.get_cookie().remove_from(req, resp);
    }
}

impl RememberMeManager for CookieRememberMeManager {
    fn on_successful_login(
        &self,
        subject: &dyn WebSubject,
        token: &dyn AuthenticationToken,
        info: &dyn AuthenticationInfo,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) {
        // always clear any previous identity:
        self._forget_identity(req, resp);

        // now save the new identity:
        if self.is_remember_me(token) {
            if let Err(err) = self.remember_identity(subject, token, info, self, resp) {
                error!("Unable to remember identity: {}", err);
            }
        } else {
            debug!(
                "AuthenticationToken did not indicate RememberMe is requested.
                RememberMe functionality will not be executed for corresponding account."
            );
        }
    }

    fn on_failed_login(
        &self,
        _subject: &dyn WebSubject,
        _token: &dyn AuthenticationToken,
        _ae: &AuthenticationError,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) {
        self._forget_identity(req, resp);
    }

    fn on_logout(
        &self,
        _subject: &dyn WebSubject,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) {
        self._forget_identity(req, resp);
    }

    fn get_remembered_principals(
        &self,
        subject_context: &dyn WebSubjectContext,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Option<Arc<dyn PrincipalCollection>> {
        let bytes = self.get_remembered_serialized_identity(subject_context, req, resp);

        let mut principals: Option<Arc<dyn PrincipalCollection>> = None;
        // SHIRO-138 - only call convert_bytes_to_principals if bytes exist:
        if let Some(mut bytes) = bytes {
            if !bytes.is_empty() {
                match self
                    .default_remember_me_manager
                    .convert_bytes_to_principals(&bytes, subject_context)
                {
                    Ok(principal_collection) => {
                        principals = Some(principal_collection);
                    }
                    Err(err) => {
                        warn!("Unable to deserialize principals: {}", err);
                        self._forget_identity(req, resp);

                        return None;
                    }
                };

                bytes.clear();
            }
        }

        principals
    }

    fn forget_identity(
        &self,
        _subject_context: &dyn WebSubjectContext,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) {
        self._forget_identity(req, resp);
    }
}

impl DefaultRememberMeManagerExt for CookieRememberMeManager {
    fn remember_serialized_identity(
        &self,
        _subject: &dyn WebSubject,
        serialized: &[u8],
        resp: &mut dyn HttpResponse,
    ) -> Result<(), BoxError> {
        // base 64 encode it and store as a cookie:
        let base64 = BASE64_STANDARD.decode(serialized)?;

        let template = self.get_cookie();
        let mut cookie = SimpleCookie::from(template);
        cookie.set_value(String::from_utf8(base64)?);

        cookie.save_to(None, resp);

        Ok(())
    }
}

impl Deref for CookieRememberMeManager {
    type Target = DefaultRememberMeManager;

    fn deref(&self) -> &Self::Target {
        &self.default_remember_me_manager
    }
}

impl Default for CookieRememberMeManager {
    fn default() -> Self {
        Self {
            cookie: Arc::new(Self::create_default_cookie()),
            default_remember_me_manager: Default::default(),
        }
    }
}
