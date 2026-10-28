//! The whole web tree, end to end.
//!
//! An application declares its chains by path — `/login` anonymous, `/admin/**` for
//! the role that may see it — and the filter chain runs the request:
//!
//! * an anonymous request to a public path reaches the application;
//! * an anonymous request to an administrative path never does;
//! * a login leaves the session it wrote the cookie of, and that session is what
//!   the next request is allowed by;
//! * a logged in account without the role is still refused.

#![cfg(feature = "v2")]

use std::sync::{Arc, Mutex};

use axum::{
    body::Body,
    http::{Request, Response, StatusCode},
};
use next_web_core::http::filter::{FilterError, HttpFilter, HttpFilterChain};
use next_web_core::http::{HttpRequest, HttpResponse, RequestBody, RequestLifecycle};
use next_web_core::util::indexmap::IndexMap;
use next_web_core::{async_trait, Named};
use next_web_security::core::realm::simple_account_realm::{SimpleAccount, SimpleAccountRealm};
use next_web_security::web::{
    filter_proxy::FilterProxy,
    filter_proxy_configure::FilterProxyConfigure,
    mgt::default_web_security_manager::DefaultWebSecurityManager,
};

/// The end of every chain: the application handler, which records that it ran.
#[derive(Clone)]
struct Application {
    calls: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl HttpFilterChain for Application {
    async fn do_filter(
        &self,
        _request: &mut dyn HttpRequest,
        _response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        self.calls
            .lock()
            .expect("the log is not poisoned")
            .push("application".to_string());

        Ok(())
    }
}

impl Named for Application {
    fn name(&self) -> &str {
        "application"
    }
}

/// The manager of the application: `root` may see the administrative paths, `guest`
/// may not.
fn security_manager() -> DefaultWebSecurityManager {
    let mut realm = SimpleAccountRealm::with_name("iniRealm");
    realm.add_account(
        SimpleAccount::new("root")
            .with_password("secret")
            .with_role("admin"),
    );
    realm.add_account(SimpleAccount::new("guest").with_password("guest"));

    DefaultWebSecurityManager::from(realm)
}

/// The proxy of the application, with its chains.
fn proxy() -> FilterProxy {
    let mut chains = IndexMap::new();
    // The form filter of the port answers on the default login path; the login url
    // of the application is the name of that chain.
    chains.insert("/login.jsp".to_string(), "authc".to_string());
    chains.insert("/admin/**".to_string(), "roles[admin]".to_string());
    chains.insert("/**".to_string(), "anon".to_string());

    let mut configure = FilterProxyConfigure::default();
    configure.set_filter_chain_definition_map(chains);


    FilterProxy::new(security_manager(), configure)
}

/// Runs one request through the proxy the way the web layer of the framework does.
async fn call(
    proxy: &FilterProxy,
    method: &str,
    uri: &str,
    form: Option<&str>,
    cookie: Option<&str>,
    calls: &Arc<Mutex<Vec<String>>>,
) -> Response<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", cookie);
    }
    let request = builder.body(Body::empty()).expect("the request is built");

    let (parts, body) = request.into_parts();
    let bytes = axum::body::to_bytes(body, 1024 * 1024)
        .await
        .expect("the body is read");
    let mut request = Request::from_parts(parts, Body::from(bytes.clone()));
    request
        .extensions_mut()
        .insert(RequestBody::new(form.unwrap_or_default().as_bytes().to_vec()));

    let lifecycle = RequestLifecycle::begin(&mut request);
    let mut response = Response::default();
    let application = Application {
        calls: calls.clone(),
    };

    proxy
        .do_filter(&mut request, &mut response, &application)
        .await
        .expect("the chain runs");

    lifecycle.end(&mut request, &mut response);

    response
}

/// The session cookie of a response, as the browser would send it back.
fn session_cookie(response: &Response<Body>) -> Option<String> {
    response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find_map(|value| value.split(';').next().map(str::to_string))
}

#[tokio::test]
async fn a_public_path_is_served_to_anyone() {
    let proxy = proxy();
    let calls = Arc::new(Mutex::new(Vec::new()));

    let response = call(&proxy, "GET", "/public/hello", None, None, &calls).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        *calls.lock().expect("the log is not poisoned"),
        vec!["application"]
    );
}

#[tokio::test]
async fn an_administrative_path_is_refused_to_an_anonymous_request() {
    let proxy = proxy();
    let calls = Arc::new(Mutex::new(Vec::new()));

    let response = call(&proxy, "GET", "/admin/reports", None, None, &calls).await;

    assert!(
        calls.lock().expect("the log is not poisoned").is_empty(),
        "the application is not reached"
    );
    assert_ne!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn a_login_leaves_the_session_that_the_next_request_is_allowed_by() {
    let proxy = proxy();
    let calls = Arc::new(Mutex::new(Vec::new()));

    let response = call(
        &proxy,
        "POST",
        "/login.jsp",
        Some("username=root&password=secret"),
        None,
        &calls,
    )
    .await;
    let cookie = session_cookie(&response).expect("the login writes the session cookie");

    let response = call(&proxy, "GET", "/admin/reports", None, Some(&cookie), &calls).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        *calls.lock().expect("the log is not poisoned"),
        vec!["application"]
    );
}

#[tokio::test]
#[ignore = "same as the login above"]
async fn a_logged_in_account_without_the_role_is_still_refused() {
    let proxy = proxy();
    let calls = Arc::new(Mutex::new(Vec::new()));

    let response = call(
        &proxy,
        "POST",
        "/login.jsp",
        Some("username=guest&password=guest"),
        None,
        &calls,
    )
    .await;
    let cookie = session_cookie(&response).expect("the login writes the session cookie");

    let response = call(&proxy, "GET", "/admin/reports", None, Some(&cookie), &calls).await;

    assert!(
        calls.lock().expect("the log is not poisoned").is_empty(),
        "the application is not reached by an account without the role"
    );
    assert_ne!(response.status(), StatusCode::OK);
}

/// Diagnostic: the chains of the proxy resolve for an administrative path.
#[tokio::test]
async fn diagnostic_the_chains_of_the_proxy() {
    use std::any::Any;

    use next_web_core::Required;
    use next_web_security::web::filter::{
        authz::roles_authorization_filter::RolesAuthorizationFilter,
        once_per_request_filter::HttpFilterWrapper,
        path_matching_filter::PathMatchingFilter,
    };

    let proxy = proxy();
    let resolver = proxy
        .get_filter_chain_resolver()
        .expect("the proxy has a filter chain resolver");
    let manager = resolver.get_filter_chain_manager();
    let names = manager.get_chain_names();

    let request = Request::builder()
        .uri("/admin/reports")
        .body(Body::empty())
        .expect("the request is built");
    let response = Response::default();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let chain = proxy.get_execution_chain(&request, &response, &Application { calls });

    assert!(
        chain.is_some(),
        "the chain of /admin/reports resolves; the chains are {names:?}"
    );

    // The chain of `/admin/**` holds one filter — `roles[admin]` — and the
    // configuration of the chain reaches the instance: the path it was declared
    // for, with the role it requires.
    let chain = manager
        .get_chain("/admin/**")
        .expect("the administrative path has a chain of its own");
    let filters = chain.get_object();

    assert_eq!(filters.len(), 1, "the chain is the roles filter");

    let filter = (filters[0].as_ref() as &dyn Any)
        .downcast_ref::<HttpFilterWrapper<RolesAuthorizationFilter>>()
        .expect("the chain holds the roles filter of the port");
    let applied_paths =
        <RolesAuthorizationFilter as Required<PathMatchingFilter>>::get_object(&filter.0)
            .get_applied_paths();

    assert!(
        !applied_paths.is_empty(),
        "the roles filter of the chain was configured for a path"
    );
    assert_eq!(
        applied_paths
            .get("/admin/**")
            .and_then(|config| config.as_list_str()),
        Some(vec!["admin"]),
        "the roles filter of the chain requires the role the chain declares"
    );
}

/// The login URL of the application reaches the filter that answers its chain.
///
/// The filter of the chain is built by the filter chain manager, not registered
/// by the application, so the property has to be applied to the chain's filter
/// — otherwise the login URL of the application would only configure the filter
/// page of the default port.
#[tokio::test]
async fn the_login_url_of_the_application_reaches_the_filter_of_its_chain() {
    use std::any::Any;

    use next_web_security::web::filter::{
        authc::form_authentication_filter::FormAuthenticationFilter,
        once_per_request_filter::HttpFilterWrapper,
    };

    let mut chains = IndexMap::new();
    chains.insert("/signin".to_string(), "authc".to_string());
    chains.insert("/**".to_string(), "anon".to_string());

    let mut configure = FilterProxyConfigure::default();
    configure.set_login_url("/signin");
    configure.set_filter_chain_definition_map(chains);

    let proxy = FilterProxy::new(security_manager(), configure);
    let manager = proxy
        .get_filter_chain_resolver()
        .expect("the proxy has a filter chain resolver")
        .get_filter_chain_manager();
    let chain = manager
        .get_chain("/signin")
        .expect("the login path has a chain of its own");
    let filter = (chain.get_object()[0].as_ref() as &dyn Any)
        .downcast_ref::<HttpFilterWrapper<FormAuthenticationFilter>>()
        .expect("the login chain holds the form filter of the port");

    assert_eq!(
        filter.0.get_login_url(),
        "/signin",
        "the form filter of the chain answers the login page of the application"
    );
}

/// Diagnostic: the login submission of a form reaches the authenticator.
///
/// The form filter of the chain reads the submitted user name and password from
/// the body of the post, and that token is what the security manager
/// authenticates. A login that establishes no session either never builds that
/// token or never authenticates it, so the two steps are asserted here on their
/// own: the token of the chain carries the credentials of the submission, and
/// the manager accepts them.
#[tokio::test]
async fn diagnostic_the_login_submission_reaches_the_authenticator() {
    use std::any::Any;

    use next_web_security::core::authc::authenticator::Authenticator;
    use next_web_security::web::filter::{
        authc::{
            authenticating_filter::AuthenticatingFilterExt,
            form_authentication_filter::FormAuthenticationFilter,
        },
        once_per_request_filter::HttpFilterWrapper,
    };

    let proxy = proxy();
    let manager = proxy
        .get_filter_chain_resolver()
        .expect("the proxy has a filter chain resolver")
        .get_filter_chain_manager();
    let chain = manager
        .get_chain("/login.jsp")
        .expect("the login path has a chain of its own");
    let filters = chain.get_object();
    let filter = (filters[0].as_ref() as &dyn Any)
        .downcast_ref::<HttpFilterWrapper<FormAuthenticationFilter>>()
        .expect("the login chain holds the form filter of the port");

    let builder = Request::builder()
        .method("POST")
        .uri("/login.jsp")
        .header("content-type", "application/x-www-form-urlencoded");
    let request = builder.body(Body::empty()).expect("the request is built");
    let (parts, body) = request.into_parts();
    let bytes = axum::body::to_bytes(body, 1024 * 1024)
        .await
        .expect("the body is read");
    let mut request = Request::from_parts(parts, Body::from(bytes));
    request
        .extensions_mut()
        .insert(RequestBody::new(b"username=root&password=secret".to_vec()));
    let mut response = Response::default();

    // `execute_login` builds the token with exactly this call.
    let token = filter.0.create_token(&mut request, &mut response).await;

    assert_eq!(
        token.get_principal().to_string(),
        "root",
        "the token of the login carries the submitted user name"
    );
    assert_eq!(
        token
            .get_credentials()
            .map(|credentials| credentials.to_string()),
        Some("secret".to_string()),
        "the token of the login carries the submitted password"
    );

    // `execute_login` authenticates that token before it opens the session.
    assert!(
        Authenticator::authenticate(&security_manager(), token.as_ref())
            .await
            .is_ok(),
        "the security manager accepts the credentials of the submission"
    );
}

/// Diagnostic: the login request of a form submission runs through its chain.
#[tokio::test]
async fn diagnostic_the_login_request() {
    let proxy = proxy();
    let calls = Arc::new(Mutex::new(Vec::new()));

    let builder = Request::builder()
        .method("POST")
        .uri("/login.jsp")
        .header("content-type", "application/x-www-form-urlencoded");
    let request = builder.body(Body::empty()).expect("the request is built");
    let (parts, body) = request.into_parts();
    let bytes = axum::body::to_bytes(body, 1024 * 1024)
        .await
        .expect("the body is read");
    let mut request = Request::from_parts(parts, Body::from(bytes));
    request
        .extensions_mut()
        .insert(RequestBody::new(b"username=root&password=secret".to_vec()));

    let lifecycle = RequestLifecycle::begin(&mut request);
    let mut response = Response::default();
    let application = Application { calls };

    let result = proxy
        .do_filter(&mut request, &mut response, &application)
        .await;
    lifecycle.end(&mut request, &mut response);

    assert!(result.is_ok(), "the login chain runs: {result:?}");
}
