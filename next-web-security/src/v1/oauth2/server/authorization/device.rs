//! The authorizations of the device authorization grant.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

/// The state of a device authorization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceAuthorizationStatus {
    /// The resource owner has not answered yet.
    Pending,
    /// The resource owner approved the request.
    Approved,
    /// The resource owner denied the request.
    Denied,
}

/// A device authorization of the device authorization grant.
///
/// A device asks the authorization server for a code and shows the user code
/// to the resource owner. The resource owner enters the user code on the
/// verification page of the application and approves the request, and the
/// device polls the token endpoint with its device code until it receives the
/// tokens.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OAuth2DeviceAuthorization {
    device_code: String,
    user_code: String,
    registered_client_id: String,
    scopes: Vec<String>,
    expires_at: u64,
    interval: u64,
    status: DeviceAuthorizationStatus,
    principal_name: Option<String>,
    last_polled_at: Option<u64>,
}

impl OAuth2DeviceAuthorization {
    /// Creates a device authorization.
    ///
    /// # Arguments
    ///
    /// * `device_code` - The code the device presents at the token endpoint.
    /// * `user_code` - The code the resource owner enters on the verification
    ///   page.
    /// * `registered_client_id` - The registration the request belongs to.
    /// * `scopes` - The scopes of the request.
    /// * `expires_at` - The time the authorization expires, in seconds since
    ///   the epoch.
    /// * `interval` - The number of seconds between the polls of the device.
    pub fn new(
        device_code: impl Into<String>,
        user_code: impl Into<String>,
        registered_client_id: impl Into<String>,
        scopes: Vec<String>,
        expires_at: u64,
        interval: u64,
    ) -> Self {
        Self {
            device_code: device_code.into(),
            user_code: user_code.into(),
            registered_client_id: registered_client_id.into(),
            scopes,
            expires_at,
            interval,
            status: DeviceAuthorizationStatus::Pending,
            principal_name: None,
            last_polled_at: None,
        }
    }

    /// Returns the code the device presents at the token endpoint.
    pub fn device_code(&self) -> &str {
        &self.device_code
    }

    /// Returns the code the resource owner enters on the verification page.
    pub fn user_code(&self) -> &str {
        &self.user_code
    }

    /// Returns the registration the request belongs to.
    pub fn registered_client_id(&self) -> &str {
        &self.registered_client_id
    }

    /// Returns the scopes of the request.
    pub fn scopes(&self) -> &[String] {
        &self.scopes
    }

    /// Returns the time the authorization expires, in seconds since the epoch.
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }

    /// Returns the number of seconds between the polls of the device.
    pub fn interval(&self) -> u64 {
        self.interval
    }

    /// Returns the state of the authorization.
    pub fn status(&self) -> DeviceAuthorizationStatus {
        self.status
    }

    /// Returns the name of the resource owner that answered, if one did.
    pub fn principal_name(&self) -> Option<&str> {
        self.principal_name.as_deref()
    }

    /// Returns the time of the last poll of the device, if it polled.
    pub fn last_polled_at(&self) -> Option<u64> {
        self.last_polled_at
    }

    /// Records that the resource owner approved the request.
    ///
    /// # Arguments
    ///
    /// * `principal_name` - The name of the resource owner.
    pub fn approve(&mut self, principal_name: impl Into<String>) {
        self.status = DeviceAuthorizationStatus::Approved;
        self.principal_name = Some(principal_name.into());
    }

    /// Records that the resource owner denied the request.
    pub fn deny(&mut self) {
        self.status = DeviceAuthorizationStatus::Denied;
    }

    /// Records that the device polled.
    ///
    /// # Arguments
    ///
    /// * `now` - The current time, in seconds since the epoch.
    pub fn record_poll(&mut self, now: u64) {
        self.last_polled_at = Some(now);
    }

    /// Returns whether the authorization is expired.
    ///
    /// # Arguments
    ///
    /// * `now` - The current time, in seconds since the epoch.
    pub fn is_expired(&self, now: u64) -> bool {
        now >= self.expires_at
    }
}

/// Stores the authorizations of the device authorization grant.
pub trait OAuth2DeviceAuthorizationService: Send + Sync {
    /// Stores a device authorization.
    ///
    /// # Arguments
    ///
    /// * `authorization` - The authorization to store.
    fn save(&self, authorization: OAuth2DeviceAuthorization);

    /// Returns the authorization of a device code.
    ///
    /// # Arguments
    ///
    /// * `device_code` - The code of the device.
    fn find_by_device_code(&self, device_code: &str) -> Option<OAuth2DeviceAuthorization>;

    /// Returns the authorization of a user code.
    ///
    /// # Arguments
    ///
    /// * `user_code` - The code the resource owner entered.
    fn find_by_user_code(&self, user_code: &str) -> Option<OAuth2DeviceAuthorization>;
}

/// A [`OAuth2DeviceAuthorizationService`] that keeps its authorizations in
/// memory.
#[derive(Clone, Debug, Default)]
pub struct InMemoryOAuth2DeviceAuthorizationService {
    authorizations: Arc<RwLock<HashMap<String, OAuth2DeviceAuthorization>>>,
}

impl InMemoryOAuth2DeviceAuthorizationService {
    /// Creates a service with the given authorizations.
    ///
    /// # Arguments
    ///
    /// * `authorizations` - The authorizations of the service.
    pub fn new<I>(authorizations: I) -> Self
    where
        I: IntoIterator<Item = OAuth2DeviceAuthorization>,
    {
        let authorizations = authorizations
            .into_iter()
            .map(|authorization| (authorization.device_code().to_string(), authorization))
            .collect();

        Self {
            authorizations: Arc::new(RwLock::new(authorizations)),
        }
    }
}

impl OAuth2DeviceAuthorizationService for InMemoryOAuth2DeviceAuthorizationService {
    fn save(&self, authorization: OAuth2DeviceAuthorization) {
        self.authorizations
            .write()
            .expect("the device authorizations lock is not poisoned")
            .insert(authorization.device_code().to_string(), authorization);
    }

    fn find_by_device_code(&self, device_code: &str) -> Option<OAuth2DeviceAuthorization> {
        self.authorizations
            .read()
            .expect("the device authorizations lock is not poisoned")
            .get(device_code)
            .cloned()
    }

    fn find_by_user_code(&self, user_code: &str) -> Option<OAuth2DeviceAuthorization> {
        self.authorizations
            .read()
            .expect("the device authorizations lock is not poisoned")
            .values()
            .find(|authorization| authorization.user_code().eq_ignore_ascii_case(user_code))
            .cloned()
    }
}

/// Returns the error a device that polls too quickly reports.
pub fn slow_down_error() -> &'static str {
    "slow_down"
}

#[cfg(test)]
mod tests {
    use super::{
        DeviceAuthorizationStatus, InMemoryOAuth2DeviceAuthorizationService,
        OAuth2DeviceAuthorization, OAuth2DeviceAuthorizationService,
    };

    fn authorization() -> OAuth2DeviceAuthorization {
        OAuth2DeviceAuthorization::new(
            "the-device-code",
            "ABCD-EFGH",
            "demo-client",
            vec!["read".to_string()],
            4_102_444_800,
            5,
        )
    }

    #[test]
    fn a_device_authorization_is_found_by_both_of_its_codes() {
        let service = InMemoryOAuth2DeviceAuthorizationService::default();
        service.save(authorization());

        assert!(service.find_by_device_code("the-device-code").is_some());
        assert!(service
            .find_by_user_code("abcd-efgh")
            .is_some_and(|authorization| authorization.registered_client_id() == "demo-client"));
    }

    #[test]
    fn the_resource_owner_answers_the_request() {
        let mut authorization = authorization();

        assert_eq!(authorization.status(), DeviceAuthorizationStatus::Pending);
        authorization.approve("user");
        assert_eq!(authorization.status(), DeviceAuthorizationStatus::Approved);
        assert_eq!(authorization.principal_name(), Some("user"));
    }
}
