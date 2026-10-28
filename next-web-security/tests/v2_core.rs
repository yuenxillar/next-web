//! The core tree of the Apache Shiro style port, without the web integration.
//!
//! The tests here hold the path of an application that has no HTTP layer: it
//! builds a security manager from a realm, builds the subject of the call, logs
//! the subject in with a token, and asks the subject what it may do. The same
//! application is built with `--no-default-features --features v2`, which is the
//! build these tests exist for.

#![cfg(all(feature = "v2", not(feature = "web")))]

use std::sync::Arc;

use next_web_security::core::{
    authc::{AuthenticationError, username_password_token::UsernamePasswordToken},
    mgt::{default_security_manager::DefaultSecurityManager, security_manager::SecurityManager},
    realm::simple_account_realm::{SimpleAccount, SimpleAccountRealm},
    subject::{Subject, support::default_subject_context::DefaultSubjectContext},
};

/// The realm of the tests: one account with a role and a permission.
fn realm() -> SimpleAccountRealm {
    let mut realm = SimpleAccountRealm::with_name("iniRealm");
    realm.add_account(
        SimpleAccount::new("root")
            .with_password("secret")
            .with_role("admin")
            .with_permission("printer:print"),
    );

    realm
}

/// Returns the subject of a call, built the way an application without a web
/// layer builds it: from a context that names the manager it works through.
async fn subject(manager: &Arc<dyn SecurityManager>) -> Box<dyn Subject> {
    let mut context = DefaultSubjectContext::default();
    context.set_security_manager(manager.clone());

    manager.create_subject(Arc::new(context)).await
}

/// Returns the token of the account of the tests.
fn token(password: &str) -> UsernamePasswordToken {
    UsernamePasswordToken::new("root".to_string(), password.to_string(), false, None)
}

/// The non-web example of `src/v2/README.md`, as a function that can be run.
///
/// The example answers with the errors of the login and of the logout, so this
/// function does the same: `?` on the two error types of the core tree is part
/// of the example, and the compiler checks it here.
async fn readme_example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let manager: Arc<dyn SecurityManager> = Arc::new(DefaultSecurityManager::from(realm()));

    let mut context = DefaultSubjectContext::default();
    context.set_security_manager(manager.clone());
    let mut subject = manager.create_subject(Arc::new(context)).await;

    subject.login(&token("secret")).await?;

    assert!(subject.is_authenticated().await);
    assert!(subject.has_role("admin").await);
    assert!(subject.is_permitted("printer:print").await);

    subject.logout().await?;
    assert!(!subject.is_authenticated().await);

    Ok(())
}

#[tokio::test]
async fn the_core_example_of_the_readme_runs() {
    readme_example().await.expect("the example runs");
}

#[tokio::test]
async fn a_login_authenticates_the_subject_and_opens_its_session() {
    let manager: Arc<dyn SecurityManager> = Arc::new(DefaultSecurityManager::from(realm()));
    let mut subject = subject(&manager).await;

    assert!(
        !subject.is_authenticated().await,
        "a subject starts anonymous"
    );
    assert!(
        subject.get_session().is_none(),
        "a subject starts without a session"
    );

    subject
        .login(&token("secret"))
        .await
        .expect("the account is known and the password matches");

    assert!(subject.is_authenticated().await);
    assert_eq!(
        subject.get_principal().await.map(ToString::to_string),
        Some("root".to_string())
    );
    assert!(
        subject.get_session().is_some(),
        "the login opens the session of the identity"
    );
}

#[tokio::test]
async fn a_logged_in_subject_answers_with_the_roles_and_permissions_of_its_account() {
    let manager: Arc<dyn SecurityManager> = Arc::new(DefaultSecurityManager::from(realm()));
    let mut subject = subject(&manager).await;

    subject
        .login(&token("secret"))
        .await
        .expect("the account is known and the password matches");

    // The role and the permission the realm holds for the account answer both
    // the boolean questions and the checked ones.
    assert!(subject.has_role("admin").await);
    assert!(subject.has_all_roles(&["admin"]).await);
    assert!(!subject.has_role("auditor").await);
    assert!(subject.check_role("admin").await.is_ok());
    assert!(subject.check_roles(&["admin", "auditor"]).await.is_err());

    assert!(subject.is_permitted("printer:print").await);
    assert!(subject.is_permitted_all(&["printer:print"]).await);
    assert!(!subject.is_permitted("user:delete").await);
    assert!(subject.check_permission("printer:print").await.is_ok());
    assert!(subject.check_permission("user:delete").await.is_err());
}

#[tokio::test]
async fn a_wrong_password_leaves_the_subject_anonymous() {
    let manager: Arc<dyn SecurityManager> = Arc::new(DefaultSecurityManager::from(realm()));
    let mut subject = subject(&manager).await;

    let error = subject
        .login(&token("not-the-password"))
        .await
        .err()
        .expect("the password does not match the stored one");

    assert!(matches!(error, AuthenticationError::Custom(message) if message.contains("iniRealm")));
    assert!(!subject.is_authenticated().await);
    assert!(subject.get_principal().await.is_none());
    assert!(subject.get_session().is_none());
}

#[tokio::test]
async fn a_logout_returns_the_subject_to_anonymous() {
    let manager: Arc<dyn SecurityManager> = Arc::new(DefaultSecurityManager::from(realm()));
    let mut subject = subject(&manager).await;

    subject
        .login(&token("secret"))
        .await
        .expect("the account is known and the password matches");
    assert!(subject.is_authenticated().await);

    subject.logout().await.expect("the logout is answered");

    assert!(!subject.is_authenticated().await);
    assert!(subject.get_principal().await.is_none());
    assert!(
        subject.get_session().is_none(),
        "a logged out subject holds no session"
    );
}
