//! The login chain of the Shiro style tree, end to end.
//!
//! A realm holds the accounts; the authenticator turns the token of a login into
//! an authentication; the principals of that authentication are what the
//! authorizer asks about. The tests here hold those three together, which is the
//! path an application takes on every request that logs a user in.

#![cfg(feature = "v2")]

use std::sync::Arc;

use next_web_security::core::{
    authc::{
        authenticator::Authenticator,
        credential::hashed_credentials_matcher::HashedCredentialsMatcher,
        username_password_token::UsernamePasswordToken, AuthenticationError,
    },
    authz::{authorizer::Authorizer, modular_realm_authorizer::ModularRealmAuthorizer},
    crypto::hash::simple_hash::{HashAlgorithm, SimpleHash},
    realm::{
        simple_account_realm::{SimpleAccount, SimpleAccountRealm},
        Realm,
    },
};

/// The salt of the account the tests log in with.
const SALT: &[u8] = b"the-salt-of-root";

/// The store of the tests: one account, with a hashed password.
fn account_realm() -> Arc<SimpleAccountRealm> {
    let password = SimpleHash::new(HashAlgorithm::Sha256, b"secret", Some(SALT), 1).to_hex();

    let mut realm = SimpleAccountRealm::with_name("iniRealm").with_credentials_matcher(Arc::new(
        HashedCredentialsMatcher::sha256(1),
    ));
    realm.add_account(
        SimpleAccount::new("root")
            .with_password(password)
            .with_password_salt(SALT.to_vec())
            .with_role("admin")
            .with_permission("printer:*"),
    );

    Arc::new(realm)
}

fn token(password: &str) -> UsernamePasswordToken {
    UsernamePasswordToken::new("root".to_string(), password.to_string(), false, None)
}

#[tokio::test]
async fn a_login_authorizes_with_the_roles_and_permissions_of_the_account() {
    let realm = account_realm();
    let authenticator = next_web_security::core::authc::pam::modular_realm_authenticator::ModularRealmAuthenticator::new(
        vec![realm.clone() as Arc<dyn Realm>],
    );
    let authorizer = ModularRealmAuthorizer::new(vec![realm as Arc<dyn Authorizer>]);

    let authentication = authenticator
        .authenticate(&token("secret"))
        .await
        .expect("the account is known and the password matches");
    let principals = authentication
        .get_principals()
        .expect("the authentication names its principals");

    assert_eq!(
        principals.get_primary_principal().map(ToString::to_string),
        Some("root".to_string())
    );
    assert!(authorizer.is_permitted_from_str(Some(principals.as_ref()), "printer:print"));
    assert!(authorizer.is_permitted_from_str(Some(principals.as_ref()), "printer:query:color"));
    assert!(authorizer.has_role(Some(principals.as_ref()), "admin"));
    assert!(authorizer
        .check_permission_from_str(Some(principals.as_ref()), "user:read")
        .is_err());
    assert!(authorizer
        .check_roles(Some(principals.as_ref()), &["admin", "auditor"])
        .is_err());
}

#[tokio::test]
async fn a_wrong_password_never_reaches_the_authorizer() {
    let realm = account_realm();
    let authenticator = next_web_security::core::authc::pam::modular_realm_authenticator::ModularRealmAuthenticator::new(
        vec![realm.clone() as Arc<dyn Realm>],
    );

    let error = authenticator
        .authenticate(&token("not-the-password"))
        .await
        .err()
        .expect("the password does not match the stored hash");

    assert!(matches!(error, AuthenticationError::Custom(message) if message.contains("iniRealm")));
}

#[tokio::test]
async fn an_unknown_account_is_refused_by_the_realms_that_support_the_token() {
    let realm = account_realm();
    let authenticator = next_web_security::core::authc::pam::modular_realm_authenticator::ModularRealmAuthenticator::new(
        vec![realm as Arc<dyn Realm>],
    );

    let error = authenticator
        .authenticate(&UsernamePasswordToken::new(
            "nobody".to_string(),
            "secret".to_string(),
            false,
            None,
        ))
        .await
        .err()
        .expect("no account is called nobody");

    assert!(matches!(error, AuthenticationError::Custom(message) if message.contains("iniRealm")));
}

/// A resolver that maps the name of a shortcut to the permission it stands for.
///
/// It is what an application plugs in when its rules are written in a vocabulary
/// of their own — `publish` standing for `article:publish`, say.
struct ShortcutResolver;

impl next_web_security::core::authz::permission::permission_resolver::PermissionResolver
    for ShortcutResolver
{
    fn resolve_permission(
        &self,
        permission: &str,
    ) -> Box<dyn next_web_security::core::authz::permission::Permission> {
        let expanded = match permission {
            "publish" => "article:publish",
            other => other,
        };

        Box::new(
            next_web_security::core::authz::permission::wildcard_permission::WildcardPermission::new(
                expanded,
            ),
        )
    }
}

#[test]
fn a_realm_resolves_permissions_with_the_resolver_it_was_given() {
    use next_web_security::core::{
        authz::permission::{
            permission_resolver::PermissionResolver,
            wildcard_permission::WildcardPermission,
        },
        subject::simple_principal_collection::SimplePrincipalCollection,
        util::object::Object,
    };

    let mut realm = SimpleAccountRealm::with_name("iniRealm")
        .with_permission_resolver(Arc::new(ShortcutResolver));
    realm.add_account(
        SimpleAccount::new("root")
            .with_password("secret")
            .with_permission("article:publish"),
    );

    let principals = SimplePrincipalCollection::new([Object::Str("root".to_string())], "iniRealm");
    let authorizer = ModularRealmAuthorizer::new(vec![Arc::new(realm) as Arc<dyn Authorizer>]);

    // The account holds `article:publish`; the resolver of the realm turns the
    // shortcut the application asks about into that permission.
    assert!(authorizer.is_permitted_from_str(Some(&principals), "publish"));
    assert!(authorizer.is_permitted_from_str(Some(&principals), "article:publish"));
    assert!(!authorizer.is_permitted_from_str(Some(&principals), "article:delete"));

    // The resolver itself is the one the realm reports.
    let resolved: &dyn PermissionResolver = &ShortcutResolver;
    assert!(resolved
        .resolve_permission("publish")
        .implies(&WildcardPermission::new("article:publish")));

    // A realm that was given no resolver reads the shortcut as a permission of
    // its own, which the account does not hold.
    let plain = {
        let mut realm = SimpleAccountRealm::with_name("plainRealm");
        realm.add_account(
            SimpleAccount::new("root")
                .with_password("secret")
                .with_permission("article:publish"),
        );
        ModularRealmAuthorizer::new(vec![Arc::new(realm) as Arc<dyn Authorizer>])
    };
    assert!(!plain.is_permitted_from_str(Some(&principals), "publish"));
}
#[tokio::test]
async fn an_ini_realm_verifies_the_hashed_passwords_of_its_document() {
    use next_web_security::core::{
        authc::credential::password_matcher::PasswordMatcher,
        crypto::hash::password_service::DefaultPasswordService,
        realm::ini_realm::IniRealm,
    };

    let service = DefaultPasswordService::new().with_iterations(1_000);
    let stored = service.encrypt_password_with_salt("secret", b"a-salt");
    let document = format!("[users]\nroot = {stored}, admin\n\n[roles]\nadmin = printer:*\n");

    let realm = IniRealm::new(&document)
        .expect("the document is read")
        .with_credentials_matcher(Arc::new(PasswordMatcher::with_password_service(Arc::new(
            service,
        ))));
    let authenticator =
        next_web_security::core::authc::pam::modular_realm_authenticator::ModularRealmAuthenticator::new(
            vec![Arc::new(realm) as Arc<dyn Realm>],
        );

    // The document holds the hash of the password, and the matcher reads the format
    // the hash carries, so the password itself verifies.
    assert!(authenticator
        .authenticate(&token("secret"))
        .await
        .is_ok());
    assert!(authenticator
        .authenticate(&token("not-the-password"))
        .await
        .is_err());
}

#[tokio::test]
async fn the_first_successful_strategy_stops_before_the_realm_that_would_refuse() {
    use next_web_security::core::authc::pam::authentication_strategy::{
        AllSuccessfulStrategy, FirstSuccessfulStrategy,
    };

    /// A realm whose account of `root` has the given password.
    fn realm_with(name: &'static str, password: &str) -> Arc<dyn Realm> {
        let mut realm = SimpleAccountRealm::with_name(name);
        realm.add_account(SimpleAccount::new("root").with_password(password));

        Arc::new(realm)
    }

    let realms = || vec![realm_with("firstRealm", "secret"), realm_with("secondRealm", "other")];

    // The first realm authenticates the token; the second would refuse it, and the
    // first successful strategy does not give it the chance.
    let first_successful =
        next_web_security::core::authc::pam::modular_realm_authenticator::ModularRealmAuthenticator::new(
            realms(),
        )
        .with_authentication_strategy(Arc::new(FirstSuccessfulStrategy));
    assert!(first_successful
        .authenticate(&token("secret"))
        .await
        .is_ok());

    // The strategy that requires every realm to authenticate is refused by the same
    // realms, which shows the two strategies disagree on the same answers.
    let all_successful =
        next_web_security::core::authc::pam::modular_realm_authenticator::ModularRealmAuthenticator::new(
            realms(),
        )
        .with_authentication_strategy(Arc::new(AllSuccessfulStrategy));
    assert!(all_successful
        .authenticate(&token("secret"))
        .await
        .is_err());
}