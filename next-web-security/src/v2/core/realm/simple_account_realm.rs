//! Shiro's in-memory realm: accounts, their roles, and their permissions.

use std::{any::Any, collections::HashMap, sync::Arc};

use next_web_core::async_trait;

use crate::core::{
    authc::{
        authentication_info::AuthenticationInfo, authentication_token::AuthenticationToken,
        credential::{
            credentials_matcher::CredentialsMatcher, simple_credentials_matcher::SimpleCredentialsMatcher,
        },
        simple_authentication_info::SimpleAuthenticationInfo,
        username_password_token::UsernamePasswordToken,
    },
    authz::{
        authorization_error::AuthorizationError, authorization_info::AuthorizationInfo,
        authorizer::Authorizer,
        permission::{
            permission_resolver::PermissionResolver, wildcard_permission_resolver::WildcardPermissionResolver,
            Permission,
        },
        simple_authorization_info::SimpleAuthorizationInfo,
    },
    cache::{
        cache_manager_aware::CacheManagerAware, default_cache_manager::DefaultCacheManager,
    },
    event::{event_bus_aware::EventBusAware, support::default_event_bus::DefaultEventBus},
    realm::Realm,
    subject::{
        principal_collection::PrincipalCollection,
        simple_principal_collection::SimplePrincipalCollection,
    },
    util::object::Object,
};
use tracing::debug;

/// The account of one user: the password a login is matched against, and the
/// roles and the permissions the account holds.
#[derive(Clone, Default)]
pub struct SimpleAccount {
    username: String,
    password: Option<Object>,
    password_salt: Option<Vec<u8>>,
    roles: Vec<String>,
    permissions: Vec<String>,
}

impl SimpleAccount {
    /// Creates an account with a user name.
    ///
    /// # Arguments
    ///
    /// * `username` - The name the account is looked up by.
    pub fn new(username: impl ToString) -> Self {
        Self {
            username: username.to_string(),
            ..Self::default()
        }
    }

    /// Sets the password of the account.
    ///
    /// # Arguments
    ///
    /// * `password` - The credentials a login is matched against.
    pub fn with_password(mut self, password: impl ToString) -> Self {
        self.password = Some(Object::Str(password.to_string()));
        self
    }

    /// Sets the salt the password of the account was hashed with.
    ///
    /// # Arguments
    ///
    /// * `salt` - The salt of the account.
    pub fn with_password_salt(mut self, salt: Vec<u8>) -> Self {
        self.password_salt = Some(salt);
        self
    }

    /// Returns the salt the password of the account was hashed with.
    pub fn password_salt(&self) -> Option<&[u8]> {
        self.password_salt.as_deref()
    }

    /// Adds a role to the account.
    ///
    /// # Arguments
    ///
    /// * `role` - The name of the role.
    pub fn with_role(mut self, role: impl ToString) -> Self {
        self.roles.push(role.to_string());
        self
    }

    /// Adds a permission to the account.
    ///
    /// # Arguments
    ///
    /// * `permission` - The wildcard string of the permission.
    pub fn with_permission(mut self, permission: impl ToString) -> Self {
        self.permissions.push(permission.to_string());
        self
    }

    /// Returns the name the account is looked up by.
    pub fn username(&self) -> &str {
        &self.username
    }

    /// Returns the password of the account.
    pub fn password(&self) -> Option<&Object> {
        self.password.as_ref()
    }

    /// Returns the names of the roles of the account.
    pub fn roles(&self) -> &[String] {
        &self.roles
    }

    /// Returns the wildcard strings of the permissions of the account.
    pub fn permissions(&self) -> &[String] {
        &self.permissions
    }
}

/// A realm that holds its accounts in memory.
///
/// It authenticates a user name and a password against the account it holds, and
/// it authorizes with the roles and the permissions of that account. It is the
/// realm of an application that keeps its users in a configuration file, and the
/// realm the tests of the crate use.
#[derive(Clone)]
pub struct SimpleAccountRealm {
    name: String,
    accounts: HashMap<String, SimpleAccount>,
    permission_resolver: Arc<dyn PermissionResolver>,
    credentials_matcher: Arc<dyn CredentialsMatcher>,
}

impl SimpleAccountRealm {
    /// Creates a realm with the given name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name the realm reports, and the name its principals are
    ///   grouped under.
    pub fn with_name(name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            accounts: HashMap::new(),
            permission_resolver: Arc::new(WildcardPermissionResolver::default()),
            // A realm that is given no matcher compares the credentials as they
            // are stored, which is what Shiro's `SimpleCredentialsMatcher` does.
            credentials_matcher: Arc::new(SimpleCredentialsMatcher::default()),
        }
    }

    /// Sets the resolver that turns a permission string into a permission.
    ///
    /// # Arguments
    ///
    /// * `permission_resolver` - The resolver of the realm.
    pub fn with_permission_resolver(
        mut self,
        permission_resolver: Arc<dyn PermissionResolver>,
    ) -> Self {
        self.permission_resolver = permission_resolver;
        self
    }

    /// Returns the resolver that turns a permission string into a permission.
    pub fn permission_resolver(&self) -> &dyn PermissionResolver {
        self.permission_resolver.as_ref()
    }

    /// Sets how the submitted credentials are compared with the stored ones.
    ///
    /// # Arguments
    ///
    /// * `credentials_matcher` - The matcher of the realm.
    pub fn with_credentials_matcher(
        mut self,
        credentials_matcher: Arc<dyn CredentialsMatcher>,
    ) -> Self {
        self.set_credentials_matcher(credentials_matcher);
        self
    }

    /// Sets how the submitted credentials are compared with the stored ones, on a
    /// realm that already exists.
    ///
    /// # Arguments
    ///
    /// * `credentials_matcher` - The matcher of the realm.
    pub fn set_credentials_matcher(&mut self, credentials_matcher: Arc<dyn CredentialsMatcher>) {
        self.credentials_matcher = credentials_matcher;
    }

    /// Returns the matcher of the realm.
    pub fn credentials_matcher(&self) -> &dyn CredentialsMatcher {
        self.credentials_matcher.as_ref()
    }

    /// Adds an account, replacing the account of the same name.
    ///
    /// # Arguments
    ///
    /// * `account` - The account to hold.
    pub fn add_account(&mut self, account: SimpleAccount) {
        self.accounts.insert(account.username.clone(), account);
    }

    /// Returns the account of the given user name.
    ///
    /// # Arguments
    ///
    /// * `username` - The name the account is looked up by.
    pub fn get_account(&self, username: &str) -> Option<&SimpleAccount> {
        self.accounts.get(username)
    }

    /// Returns the accounts of the realm.
    pub fn accounts(&self) -> &HashMap<String, SimpleAccount> {
        &self.accounts
    }

    /// Returns the authorization info of the account a principal names.
    ///
    /// # Arguments
    ///
    /// * `principals` - The principals of the subject.
    pub fn get_authorization_info(
        &self,
        principals: &dyn PrincipalCollection,
    ) -> Option<SimpleAuthorizationInfo> {
        let username = principals.get_primary_principal()?.to_string();
        let account = self.accounts.get(&username)?;

        Some(SimpleAuthorizationInfo::new(
            account.roles.clone(),
            account.permissions.clone(),
        ))
    }

    /// Returns whether the account of the principal holds the permission.
    fn account_is_permitted(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &dyn Permission,
    ) -> bool {
        let Some(principal) = principal else {
            return false;
        };
        let Some(info) = self.get_authorization_info(principal) else {
            return false;
        };

        info.get_dyn_permissions()
            .iter()
            .any(|granted| granted.implies(permission))
    }
}

impl Default for SimpleAccountRealm {
    fn default() -> Self {
        Self::with_name("simpleAccountRealm")
    }
}

impl CacheManagerAware<DefaultCacheManager> for SimpleAccountRealm {
    fn set_cache_manager(&mut self, _cache_manager: DefaultCacheManager) {}
}

impl EventBusAware<DefaultEventBus> for SimpleAccountRealm {
    fn set_event_bus(&mut self, _event_bus: DefaultEventBus) {}
}

#[async_trait]
impl Realm for SimpleAccountRealm {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn supports(&self, authentication_token: &dyn AuthenticationToken) -> bool {
        let token: &dyn Any = authentication_token;

        token.is::<UsernamePasswordToken>()
    }

    fn as_authorizer(&self) -> Option<Arc<dyn Authorizer>> {
        Some(Arc::new(self.clone()))
    }

    async fn get_authentication_info(
        &self,
        token: &dyn AuthenticationToken,
    ) -> Option<Box<dyn AuthenticationInfo>> {
        let username = token.get_principal().to_string();
        let account = self.accounts.get(&username)?;

        let principals: Arc<dyn PrincipalCollection> = Arc::new(
            SimplePrincipalCollection::from_primary(
                Object::Str(account.username.clone()),
                self.name.clone(),
            ),
        );

        let mut info = SimpleAuthenticationInfo::new(principals, account.password.clone());
        if let Some(salt) = account.password_salt.clone() {
            info = info.with_salt(salt);
        }

        // The realm looks the account up and the matcher decides whether the
        // submitted credentials are the ones of that account; a token that does
        // not match them authenticates nobody.
        if !self.credentials_matcher.do_credentials_match(token, &info) {
            debug!(
                "The submitted credentials did not match the stored ones for [{}].",
                account.username
            );
            return None;
        }

        Some(Box::new(info))
    }
}

impl Authorizer for SimpleAccountRealm {
    fn is_permitted(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &dyn Permission,
    ) -> bool {
        self.account_is_permitted(principal, permission)
    }

    fn is_permitted_from_str(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &str,
    ) -> bool {
        self.is_permitted(
            principal,
            self.permission_resolver.resolve_permission(permission).as_ref(),
        )
    }

    fn is_permitted_from_str_list(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[&str],
    ) -> Vec<bool> {
        permissions
            .iter()
            .map(|permission| self.is_permitted_from_str(principal, permission))
            .collect()
    }

    fn is_permitted_from_permission_list(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[Box<dyn Permission>],
    ) -> Vec<bool> {
        permissions
            .iter()
            .map(|permission| self.is_permitted(principal, permission.as_ref()))
            .collect()
    }

    fn is_permitted_all(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[Box<dyn Permission>],
    ) -> bool {
        permissions
            .iter()
            .all(|permission| self.is_permitted(principal, permission.as_ref()))
    }

    fn is_permitted_all_from_str(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permissions: &[&str],
    ) -> bool {
        permissions
            .iter()
            .all(|permission| self.is_permitted_from_str(principal, permission))
    }

    fn check_permission(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &dyn Permission,
    ) -> Result<(), AuthorizationError> {
        if self.is_permitted(principal, permission) {
            Ok(())
        } else {
            Err(AuthorizationError::forbidden(permission.identifier()))
        }
    }

    fn check_permission_from_str(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        permission: &str,
    ) -> Result<(), AuthorizationError> {
        if self.is_permitted_from_str(principal, permission) {
            Ok(())
        } else {
            Err(AuthorizationError::forbidden(permission))
        }
    }

    fn check_permissions(
        &self,
        principals: Option<&dyn PrincipalCollection>,
        permissions: &[Box<dyn Permission>],
    ) -> Result<(), AuthorizationError> {
        for permission in permissions {
            self.check_permission(principals, permission.as_ref())?;
        }

        Ok(())
    }

    fn check_permissions_from_str(
        &self,
        principals: Option<&dyn PrincipalCollection>,
        permissions: &[&str],
    ) -> Result<(), AuthorizationError> {
        for permission in permissions {
            self.check_permission_from_str(principals, permission)?;
        }

        Ok(())
    }

    fn has_role(&self, principal: Option<&dyn PrincipalCollection>, role_identifier: &str) -> bool {
        let Some(principal) = principal else {
            return false;
        };

        self.get_authorization_info(principal)
            .map(|info| info.has_role(role_identifier))
            .unwrap_or_default()
    }

    fn has_roles(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        role_identifiers: &[&str],
    ) -> Vec<bool> {
        role_identifiers
            .iter()
            .map(|role| self.has_role(principal, role))
            .collect()
    }

    fn has_all_roles(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        role_identifiers: &[&str],
    ) -> bool {
        role_identifiers
            .iter()
            .all(|role| self.has_role(principal, role))
    }

    fn check_role(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        role: &str,
    ) -> Result<(), AuthorizationError> {
        if self.has_role(principal, role) {
            Ok(())
        } else {
            Err(AuthorizationError::forbidden(role))
        }
    }

    fn check_roles(
        &self,
        principal: Option<&dyn PrincipalCollection>,
        roles: &[&str],
    ) -> Result<(), AuthorizationError> {
        for role in roles {
            self.check_role(principal, role)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{SimpleAccount, SimpleAccountRealm};
    use crate::core::{
        authc::{
            username_password_token::UsernamePasswordToken,
        },
        authz::{authorization_info::AuthorizationInfo, authorizer::Authorizer},
        realm::Realm,
        subject::{
            principal_collection::PrincipalCollection,
            simple_principal_collection::SimplePrincipalCollection,
        },
        util::object::Object,
    };

    fn realm() -> SimpleAccountRealm {
        let mut realm = SimpleAccountRealm::with_name("iniRealm");
        realm.add_account(
            SimpleAccount::new("root")
                .with_password("secret")
                .with_role("admin")
                .with_permission("printer:*"),
        );
        realm.add_account(SimpleAccount::new("guest").with_role("guest"));

        realm
    }

    fn principals(username: &str) -> Arc<dyn PrincipalCollection> {
        Arc::new(SimplePrincipalCollection::new(
            [Object::Str(username.to_string())],
            "iniRealm",
        ))
    }

    #[test]
    fn the_accounts_are_looked_up_by_name() {
        let realm = realm();

        assert_eq!(
            realm
                .get_account("root")
                .and_then(|account| account.password().map(ToString::to_string)),
            Some("secret".to_string())
        );
        assert_eq!(realm.accounts().len(), 2);
        assert!(realm.get_account("nobody").is_none());
    }

    #[test]
    fn the_realm_supports_a_username_and_a_password_token() {
        let realm = realm();
        let token =
            UsernamePasswordToken::new("root".to_string(), "secret".to_string(), false, None);

        assert!(realm.supports(&token));
    }

    #[tokio::test]
    async fn the_authentication_info_of_an_account_carries_its_password() {
        let realm = realm();
        let token =
            UsernamePasswordToken::new("root".to_string(), "secret".to_string(), false, None);

        let info = realm
            .get_authentication_info(&token)
            .await
            .expect("the account is known");

        assert_eq!(
            info.get_credentials().map(ToString::to_string),
            Some("secret".to_string())
        );
        assert_eq!(
            info.get_principals()
                .and_then(|principals| principals.get_primary_principal())
                .map(ToString::to_string),
            Some("root".to_string())
        );
    }

    #[tokio::test]
    async fn an_unknown_account_has_no_authentication_info() {
        let realm = realm();
        let token = UsernamePasswordToken::new("nobody".to_string(), String::new(), false, None);

        assert!(realm.get_authentication_info(&token).await.is_none());
    }

    #[test]
    fn the_roles_and_the_permissions_of_an_account_are_its_authorization_info() {
        let realm = realm();
        let root = principals("root");

        let info = realm
            .get_authorization_info(root.as_ref())
            .expect("the account is known");

        assert_eq!(info.get_roles(), vec!["admin".to_string()]);
        assert_eq!(info.get_permissions(), vec!["printer:*".to_string()]);
        assert!(realm
            .get_authorization_info(principals("nobody").as_ref())
            .is_none());
    }

    #[test]
    fn the_realm_authorizes_with_the_wildcard_permissions_of_the_account() {
        let realm = realm();
        let principals = principals("root");

        assert!(realm.is_permitted_from_str(Some(principals.as_ref()), "printer:print"));
        assert!(!realm.is_permitted_from_str(Some(principals.as_ref()), "user:read"));
        assert!(realm.has_role(Some(principals.as_ref()), "admin"));
        assert!(!realm.has_role(Some(principals.as_ref()), "guest"));
        assert!(!realm.is_permitted_from_str(None, "printer:print"));
    }

    #[test]
    fn a_refused_permission_names_the_right_it_asked_for() {
        let realm = realm();
        let principals = principals("guest");

        let error = realm
            .check_permission_from_str(Some(principals.as_ref()), "printer:print")
            .expect_err("the account holds no permission");

        assert_eq!(error.message(), "printer:print");
        assert!(realm
            .check_roles(Some(principals.as_ref()), &["guest"])
            .is_ok());
        assert!(realm
            .check_roles(Some(principals.as_ref()), &["guest", "admin"])
            .is_err());
    }
}
