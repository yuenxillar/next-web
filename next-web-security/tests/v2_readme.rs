//! The web example of `src/v2/README.md`, compiled and run.
//!
//! The README shows how an application configures the filter chain of the `v2`
//! tree and runs a request through it. The test here is that example: if the
//! code in the README stops working, this test stops compiling.

#![cfg(all(feature = "v2", feature = "web"))]

use axum::{
    body::Body,
    http::{Request, Response, StatusCode},
};
use next_web_core::{
    async_trait,
    http::{
        HttpRequest, HttpResponse, RequestBody, RequestLifecycle,
        filter::{FilterError, HttpFilter, HttpFilterChain},
    },
    util::indexmap::IndexMap,
};
use next_web_security::{
    core::realm::simple_account_realm::{SimpleAccount, SimpleAccountRealm},
    web::{
        filter_proxy::FilterProxy, filter_proxy_configure::FilterProxyConfigure,
        mgt::default_web_security_manager::DefaultWebSecurityManager,
    },
};

/// The end of every chain: the routes of the application.
#[derive(Clone)]
struct Routes;

#[async_trait]
impl HttpFilterChain for Routes {
    async fn do_filter(
        &self,
        _request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        response.set_body(b"reached the application".to_vec());
        Ok(())
    }
}

fn proxy() -> FilterProxy {
    let mut realm = SimpleAccountRealm::with_name("iniRealm");
    realm.add_account(
        SimpleAccount::new("root")
            .with_password("secret")
            .with_role("admin"),
    );

    let mut chains = IndexMap::new();
    chains.insert("/login.jsp".to_string(), "authc".to_string());
    chains.insert("/admin/**".to_string(), "roles[admin]".to_string());
    chains.insert("/**".to_string(), "anon".to_string());

    let mut configure = FilterProxyConfigure::default();
    // The properties an application sets once, instead of on every filter of
    // every chain.
    configure.set_login_url("/login.jsp");
    configure.set_success_url("/");
    configure.set_unauthorized_url("/forbidden");
    configure.set_filter_chain_definition_map(chains);

    FilterProxy::new(DefaultWebSecurityManager::from(realm), configure)
}

/// Runs one request through the proxy, the way the web layer of Next Web does.
async fn call(
    proxy: &FilterProxy,
    method: &str,
    uri: &str,
    form: Option<&str>,
    session: Option<&str>,
) -> Response<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(session) = session {
        builder = builder.header("cookie", session);
    }

    let request = builder.body(Body::empty()).expect("the request is built");
    let (parts, body) = request.into_parts();
    let bytes = axum::body::to_bytes(body, 1024 * 1024)
        .await
        .expect("the body is read");
    let mut request = Request::from_parts(parts, Body::from(bytes));
    request.extensions_mut().insert(RequestBody::new(
        form.unwrap_or_default().as_bytes().to_vec(),
    ));

    let lifecycle = RequestLifecycle::begin(&mut request);
    let mut response = Response::default();
    proxy
        .do_filter(&mut request, &mut response, &Routes)
        .await
        .expect("the chain runs");
    lifecycle.end(&mut request, &mut response);

    response
}

#[tokio::test]
async fn the_web_example_of_the_readme_runs() {
    let proxy = proxy();

    // An anonymous visitor is not allowed into the administrative area: the
    // roles filter refuses the request and the application is not reached.
    let refused = call(&proxy, "GET", "/admin/reports", None, None).await;
    assert_eq!(refused.status(), StatusCode::FOUND);
    assert_eq!(refused.headers()["location"], "/forbidden");

    // The form login: the chain of `/login.jsp` builds the token from the body,
    // authenticates it, opens the session and returns the `Set-Cookie` of it.
    let login = call(
        &proxy,
        "POST",
        "/login.jsp",
        Some("username=root&password=secret"),
        None,
    )
    .await;
    assert_eq!(login.status(), StatusCode::FOUND);
    assert_eq!(login.headers()["location"], "/");
    let cookie = login
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| value.starts_with("JSESSIONID="))
        .expect("the login writes the session cookie")
        .split(';')
        .next()
        .unwrap()
        .to_string();

    // The next request presents the session; the roles filter finds the account
    // of the login and lets it through.
    let allowed = call(&proxy, "GET", "/admin/reports", None, Some(&cookie)).await;
    assert_eq!(allowed.status(), StatusCode::OK);
}
