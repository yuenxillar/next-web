//! The session of a subject, end to end.
//!
//! A subject that needs a session asks the security manager for one; the manager
//! creates it in its store and hands back a handle that reads the store. The
//! tests here hold that path: a session created through a subject is a session a
//! later call finds, with what was written into it.

#![cfg(feature = "v2")]

use std::sync::Arc;

use next_web_security::core::{
    session::SessionValue,
    subject::Subject,
};
use next_web_security::web::{
    mgt::{
        default_web_security_manager::DefaultWebSecurityManager,
        web_security_manager::WebSecurityManager,
    },
    subject::support::web_delegating_subject::WebDelegatingSubject,
};

/// Returns a subject whose sessions are created in the store of its manager.
fn subject(manager: Arc<dyn WebSecurityManager>) -> WebDelegatingSubject {
    WebDelegatingSubject::new(None, false, Some("example.org".to_string()), None, true, manager)
}

#[tokio::test]
async fn a_subject_creates_its_session_through_its_manager() {
    let manager: Arc<dyn WebSecurityManager> = Arc::new(DefaultWebSecurityManager::default());
    let mut subject = subject(manager);

    assert!(
        subject.get_session().is_none(),
        "a subject starts without a session"
    );

    let session = subject
        .get_session_or_create(true)
        .await
        .expect("the manager creates a session");

    session
        .set_attribute("authenticated", SessionValue::Boolean(true))
        .await
        .expect("the attribute is written");
    assert_eq!(
        session
            .get_attribute("authenticated")
            .await
            .and_then(|value| value.as_boolean()),
        Some(true)
    );
    assert_eq!(subject.get_session().map(|session| session.id()), Some(session.id()));
}

#[tokio::test]
async fn a_subject_that_may_not_create_a_session_gets_none() {
    let manager: Arc<dyn WebSecurityManager> = Arc::new(DefaultWebSecurityManager::default());
    let mut subject = WebDelegatingSubject::new(None, false, None, None, false, manager);

    assert!(
        subject.get_session_or_create(true).await.is_none(),
        "session creation is disabled for the subject"
    );
}

#[tokio::test]
async fn asking_without_creating_never_makes_a_session() {
    let manager: Arc<dyn WebSecurityManager> = Arc::new(DefaultWebSecurityManager::default());
    let mut subject = subject(manager);

    assert!(subject.get_session_or_create(false).await.is_none());
    assert!(subject.get_session().is_none());
}

#[tokio::test]
async fn a_subject_with_the_request_of_its_call_gets_the_session_cookie_written() {
    use std::sync::Mutex;

    use axum::{body::Body, http::Request};
    use next_web_core::http::{HttpRequest, HttpResponseShare};
    use next_web_security::web::subject::web_subject::WebSubject;
    use next_web_security::web::subject::support::web_delegating_subject::WebDelegatingSubject;

    // The subject is handed the request of the call, as the filter chain does, and
    // a response it shares with whoever asked for the session.
    let mut live = Request::builder()
        .uri("/orders")
        .body(Body::empty())
        .expect("the request is built");
    let request = live.shared().clone();

    let manager: Arc<dyn WebSecurityManager> = Arc::new(DefaultWebSecurityManager::default());
    let response = Arc::new(Mutex::new(HttpResponseShare::default()));
    let mut subject = WebDelegatingSubject::new(None, false, None, None, true, manager)
        .with_shared_context(Arc::new(request), response.clone());

    let session = subject
        .get_session_or_create(true)
        .await
        .expect("the manager creates the session");
    let session_id = session.id().to_string();

    // The web session manager writes the identifier of the new session into the
    // response, which is what makes the login outlive its request.
    let shares = response.lock().expect("the response is not poisoned");
    let cookie = shares
        .header_map()
        .iter()
        .find(|(name, _)| name.as_str().eq_ignore_ascii_case("set-cookie"))
        .and_then(|(_, value)| value.to_str().ok())
        .map(str::to_string)
        .expect("the response holds the session id cookie");

    assert!(
        cookie.contains(&session_id),
        "the cookie names the session: {cookie}"
    );
    assert_eq!(
        subject.request_share().map(|share| share.path().to_string()),
        Some("/orders".to_string()),
        "the subject reads the request of its call"
    );
}

/// The subject of a call is reachable from the request, which is what the
/// authorization filters ask for by name.
#[tokio::test]
async fn the_subject_of_a_call_is_reachable_from_its_request() {
    use axum::{body::Body, http::Request};
    use next_web_core::any::AnyValue;
    use next_web_core::http::HttpRequest;
    use next_web_security::core::util::web::WebUtils;
    use next_web_security::web::subject::support::web_delegating_subject::WebDelegatingSubject;
    use next_web_security::web::subject::web_subject::WebSubject;

    let manager: Arc<dyn WebSecurityManager> = Arc::new(DefaultWebSecurityManager::default());

    // A subject of a logged in account: authenticated, serving `/admin`.
    let principals: Arc<dyn next_web_security::core::subject::principal_collection::PrincipalCollection> =
        Arc::new(
            next_web_security::core::subject::simple_principal_collection::SimplePrincipalCollection::new(
                [next_web_security::core::util::object::Object::Str("root".to_string())],
                "iniRealm",
            ),
        );
    let subject = WebDelegatingSubject::new(
        Some(principals),
        true,
        Some("example.org".to_string()),
        None,
        false,
        manager,
    );
    let boxed: Box<dyn WebSubject> = Box::new(subject);
    let bound: Arc<dyn WebSubject> = Arc::from(boxed);

    // The filter chain binds the subject to the request of the call.
    let mut request = Request::builder()
        .uri("/admin")
        .body(Body::empty())
        .expect("the request is built");
    request.set_attribute(
        WebUtils::SHARED_SUBJECT_KEY,
        AnyValue::BoxedValue(Box::new(bound)),
    );

    let mut response = axum::response::Response::default();
    let subject = WebUtils::get_subject(&mut request, &mut response).await;

    // A request without the binding would give a fresh subject, which is not
    // authenticated; this one is the subject of the call.
    assert!(
        subject.is_authenticated().await,
        "the bound subject is the one the filters are given"
    );

    // The binding is cleaned up by the proxy once the chain is done.
    request.remove_attribute(WebUtils::SHARED_SUBJECT_KEY);
    assert!(request.get_attribute(WebUtils::SHARED_SUBJECT_KEY).is_none());
}
/// The role of a subject, on the same realm the manager was built from.
#[tokio::test]
async fn the_subject_of_an_account_holds_the_role_of_that_account() {
    use next_web_security::core::{
        realm::simple_account_realm::{SimpleAccount, SimpleAccountRealm},
        subject::{
            principal_collection::PrincipalCollection,
            simple_principal_collection::SimplePrincipalCollection,
            Subject,
        },
        util::object::Object,
    };
    use next_web_security::web::{
        mgt::{
            default_web_security_manager::DefaultWebSecurityManager,
            web_security_manager::WebSecurityManager,
        },
        subject::support::web_delegating_subject::WebDelegatingSubject,
    };

    let mut realm = SimpleAccountRealm::with_name("iniRealm");
    realm.add_account(SimpleAccount::new("root").with_role("admin"));
    let manager: Arc<dyn WebSecurityManager> = Arc::new(DefaultWebSecurityManager::from(realm));

    let principals: Arc<dyn PrincipalCollection> =
        Arc::new(SimplePrincipalCollection::new([Object::Str("root".to_string())], "iniRealm"));

    // The manager authorizes the principals it was given a realm for.
    assert!(
        manager.has_role(Some(principals.as_ref()), "admin"),
        "the manager authorizes the role of the account"
    );

    // The subject of the same principals answers the same way.
    let subject = WebDelegatingSubject::new(
        Some(principals),
        true,
        None,
        None,
        false,
        manager,
    );

    assert!(
        subject.get_principals().await.is_some(),
        "the subject has the principals it was given"
    );
    assert!(subject.has_role("admin").await, "the subject holds the role of the account");
}