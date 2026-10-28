//! Shiro's `IniRealm`: the accounts of an application read from an ini document.
//!
//! The document of Apache Shiro has two sections:
//!
//! ```ini
//! [users]
//! root = secret, admin
//! guest = guest
//!
//! [roles]
//! admin = printer:*, user:*
//! guest = printer:print
//! ```
//!
//! A line of `[users]` is a user name, its password and the roles of the account;
//! a line of `[roles]` is a role and the permissions it grants. A `#` or `;`
//! starts a comment.

use std::{collections::HashMap, fmt::Display, sync::Arc};

use next_web_core::async_trait;

use crate::core::{
    authc::{
        authentication_info::AuthenticationInfo, authentication_token::AuthenticationToken,
        credential::credentials_matcher::CredentialsMatcher,
    },
    authz::authorizer::Authorizer,
    realm::{
        simple_account_realm::{SimpleAccount, SimpleAccountRealm},
        Realm,
    },
};

/// Why an ini document could not be read as the accounts of a realm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IniRealmError {
    /// A line of the document is not a `name = value` pair.
    MalformedLine {
        /// The line number of the document, starting at 1.
        line: usize,
        /// The text of the line.
        text: String,
    },
    /// The document names a section the realm does not know.
    UnknownSection {
        /// The line number of the document, starting at 1.
        line: usize,
        /// The name of the section.
        name: String,
    },
    /// A line of `[users]` or `[roles]` appears before its section.
    ValueOutsideSection {
        /// The line number of the document, starting at 1.
        line: usize,
    },
}

impl std::error::Error for IniRealmError {}

impl Display for IniRealmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MalformedLine { line, text } => {
                write!(f, "Line {line} of the document is not `name = value`: {text}")
            }
            Self::UnknownSection { line, name } => {
                write!(f, "Line {line} of the document opens the unknown section [{name}]")
            }
            Self::ValueOutsideSection { line } => {
                write!(f, "Line {line} of the document is outside a section")
            }
        }
    }
}

/// A realm whose accounts, roles and permissions are read from an ini document.
///
/// It holds an [`SimpleAccountRealm`], so an application that reads its accounts
/// from a document authenticates and authorizes exactly like one that builds the
/// realm in code.
#[derive(Clone)]
pub struct IniRealm {
    account_realm: SimpleAccountRealm,
}

impl IniRealm {
    /// Reads the accounts of the given document.
    ///
    /// # Arguments
    ///
    /// * `document` - The ini document with the `[users]` and `[roles]` sections.
    ///
    /// # Errors
    ///
    /// Returns [`IniRealmError`] for a document the realm cannot read.
    pub fn new(document: &str) -> Result<Self, IniRealmError> {
        Self::with_name("iniRealm", document)
    }

    /// Reads the accounts of the given document under the given realm name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name the realm reports, and the name its principals are
    ///   grouped under.
    /// * `document` - The ini document with the `[users]` and `[roles]` sections.
    ///
    /// # Errors
    ///
    /// Returns [`IniRealmError`] for a document the realm cannot read.
    pub fn with_name(name: impl ToString, document: &str) -> Result<Self, IniRealmError> {
        let mut roles: HashMap<String, Vec<String>> = HashMap::new();
        let mut users: Vec<(String, Vec<String>)> = Vec::new();
        let mut section: Option<&str> = None;

        for (index, raw) in document.lines().enumerate() {
            let line = index + 1;
            let text = raw.trim();

            if text.is_empty() || text.starts_with('#') || text.starts_with(';') {
                continue;
            }

            if let Some(name) = text.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
                let name = name.trim();
                match name.to_ascii_lowercase().as_str() {
                    "users" => section = Some("users"),
                    "roles" => section = Some("roles"),
                    "main" => section = Some("main"),
                    _ => {
                        return Err(IniRealmError::UnknownSection {
                            line,
                            name: name.to_string(),
                        });
                    }
                }
                continue;
            }

            let Some((key, value)) = text.split_once('=') else {
                return Err(IniRealmError::MalformedLine {
                    line,
                    text: text.to_string(),
                });
            };

            let key = key.trim();
            let values: Vec<String> = value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect();

            match section {
                Some("users") => users.push((key.to_string(), values)),
                Some("roles") => {
                    roles.insert(key.to_string(), values);
                }
                Some("main") => {}
                _ => return Err(IniRealmError::ValueOutsideSection { line }),
            }
        }

        let mut account_realm = SimpleAccountRealm::with_name(name);

        for (username, values) in users {
            let mut account = SimpleAccount::new(&username);

            // The first value is the password of the account, which may be empty;
            // the ones after it are the roles it holds.
            if let Some((password, roles_of_user)) = values.split_first() {
                account = account.with_password(password);
                for role in roles_of_user {
                    account = account.with_role(role);
                    for permission in roles.get(role).into_iter().flatten() {
                        account = account.with_permission(permission);
                    }
                }
            }

            account_realm.add_account(account);
        }

        Ok(Self { account_realm })
    }

    /// Sets how the submitted credentials are compared with the stored ones.
    ///
    /// A document that stores hashed passwords 鈥?the modular crypt format of the
    /// password service 鈥?needs the matcher that reads that format: without one the
    /// realm compares the stored value as text, which no password can match.
    ///
    /// # Arguments
    ///
    /// * `credentials_matcher` - The matcher of the realm.
    pub fn with_credentials_matcher(
        mut self,
        credentials_matcher: Arc<dyn CredentialsMatcher>,
    ) -> Self {
        self.account_realm
            .set_credentials_matcher(credentials_matcher);
        self
    }

    /// Returns the realm that holds the accounts of the document.
    pub fn account_realm(&self) -> &SimpleAccountRealm {
        &self.account_realm
    }
}

impl Display for IniRealm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.account_realm.get_name())
    }
}

#[async_trait]
impl Realm for IniRealm {
    fn get_name(&self) -> &str {
        self.account_realm.get_name()
    }

    fn supports(&self, authentication_token: &dyn AuthenticationToken) -> bool {
        self.account_realm.supports(authentication_token)
    }

    fn as_authorizer(&self) -> Option<Arc<dyn Authorizer>> {
        Some(Arc::new(self.account_realm.clone()))
    }

    async fn get_authentication_info(
        &self,
        token: &dyn AuthenticationToken,
    ) -> Option<Box<dyn AuthenticationInfo>> {
        self.account_realm.get_authentication_info(token).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{IniRealm, IniRealmError};
    use crate::core::{
        authc::{
            authenticator::Authenticator, username_password_token::UsernamePasswordToken,
        },
        authz::{authorizer::Authorizer, modular_realm_authorizer::ModularRealmAuthorizer},
        realm::Realm,
        util::object::Object,
    };

    const DOCUMENT: &str = r#"
# The accounts of the example application.
[users]
root = secret, admin
guest = guest
# A user may hold several roles.
max = max, admin, auditor

[roles]
admin = printer:*, user:*
guest = printer:print
auditor = audit:read
"#;

    fn token(username: &str, password: &str) -> UsernamePasswordToken {
        UsernamePasswordToken::new(username.to_string(), password.to_string(), false, None)
    }

    #[test]
    fn the_accounts_and_the_roles_of_the_document_are_read() {
        let realm = IniRealm::new(DOCUMENT).expect("the document is read");

        assert_eq!(realm.get_name(), "iniRealm");
        assert_eq!(realm.account_realm().accounts().len(), 3);
        assert_eq!(
            realm.account_realm().get_account("root").map(|account| account.roles().len()),
            Some(1)
        );
        assert_eq!(
            realm
                .account_realm()
                .get_account("max")
                .map(|account| account.permissions().len()),
            // admin grants two permissions and auditor one.
            Some(3)
        );
        // A user without a role holds no permission.
        assert!(realm
            .account_realm()
            .get_account("guest")
            .map(|account| account.permissions().is_empty())
            .unwrap_or_default());
    }

    #[tokio::test]
    async fn a_login_of_the_document_authorizes_with_its_roles() {
        let realm = Arc::new(IniRealm::new(DOCUMENT).expect("the document is read"));
        let authenticator = crate::core::authc::pam::modular_realm_authenticator::ModularRealmAuthenticator::new(
            vec![realm.clone() as Arc<dyn Realm>],
        );
        let authorizer = ModularRealmAuthorizer::new(vec![realm
            .as_authorizer()
            .expect("the realm authorizes")]);

        let info = authenticator
            .authenticate(&token("root", "secret"))
            .await
            .expect("the account is in the document");
        let principals = info.get_principals().expect("the info names its principals");

        assert!(authorizer.is_permitted_from_str(Some(principals.as_ref()), "user:delete"));
        assert!(authorizer.has_role(Some(principals.as_ref()), "admin"));
        assert!(authorizer
            .check_permission_from_str(Some(principals.as_ref()), "audit:read")
            .is_err());
    }

    #[tokio::test]
    async fn a_password_the_document_does_not_hold_is_refused() {
        let realm = IniRealm::new(DOCUMENT).expect("the document is read");
        let authenticator = crate::core::authc::pam::modular_realm_authenticator::ModularRealmAuthenticator::new(
            vec![Arc::new(realm) as Arc<dyn Realm>],
        );

        assert!(authenticator
            .authenticate(&token("root", "not-the-secret"))
            .await
            .err()
            .is_some());
    }

    #[test]
    fn a_line_that_is_not_a_pair_is_refused() {
        let error = IniRealm::new("[users]\nroot secret\n").err().expect("the line has no `=`");

        assert!(matches!(error, IniRealmError::MalformedLine { line: 2, .. }));
        assert!(error.to_string().contains("line 2") || error.to_string().contains("Line 2"));
    }

    #[test]
    fn a_section_the_realm_does_not_know_is_refused() {
        let error = IniRealm::new("[accounts]\nroot = secret\n").err().expect("the section is unknown");

        assert!(matches!(error, IniRealmError::UnknownSection { line: 1, .. }));
    }

    #[test]
    fn a_value_outside_a_section_is_refused() {
        let error = IniRealm::new("root = secret\n").err().expect("there is no section");

        assert!(matches!(error, IniRealmError::ValueOutsideSection { line: 1 }));
    }

    #[tokio::test]
    async fn the_principal_of_a_login_is_the_name_of_the_account() {
        let realm = IniRealm::new(DOCUMENT).expect("the document is read");
        let token = token("root", "secret");

        let info = realm
            .get_authentication_info(&token)
            .await
            .expect("the account is in the document");

        assert_eq!(
            info.get_principals()
                .and_then(|principals| principals.get_primary_principal())
                .map(ToString::to_string),
            Some("root".to_string())
        );
        assert_eq!(
            info.get_credentials().map(ToString::to_string),
            Some("secret".to_string())
        );
        assert_eq!(Object::Str("root".to_string()).to_string(), "root");
    }
}
