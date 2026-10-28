# The `v2` tree — Apache Shiro style

`src/v2` is a Rust translation of [Apache Shiro](https://shiro.apache.org/). It keeps Shiro's
layering: a `SecurityManager` behind the `Subject` the application talks to, realms that look the
accounts up, and realms again that answer the role and permission questions.

The tree is split the way Shiro is split:

| Layer | Module root | What it holds | Feature |
| --- | --- | --- | --- |
| core | `src/v2/core` | subjects, realms, authentication, authorization, sessions, caches, password hashing | `v2` |
| web | `src/v2/web` | the filter chain, the session cookie, the HTTP authentication and authorization filters | `v2-web` |

The core tree compiles without the web module: every reference to `crate::web` in `src/v2/core` sits
behind `feature = "web"`. That is what makes the tree usable by a program that is not an HTTP
application at all — a CLI, a job, a desktop application — the way `shiro-core` is.

```toml
# Core only: no filter chain, no cookies, no `web` module.
next-web-security = { version = "0.2", default-features = false, features = ["v2"] }

# Core plus the web integration. This is what the default features build.
next-web-security = { version = "0.2", features = ["v2-web"] }
```

| Feature | Modules it compiles | Extra crates |
| --- | --- | --- |
| `v2` | `src/v2/core` | `dashmap` (the caches of a realm) |
| `v2-web` *(default)* | `src/v2/core` and `src/v2/web` | `dashmap` |
| `v1` | `src/v1`, the Spring Security style tree | — |

`v1` and `v2` are mutually exclusive: each is re-exported at the crate root, so a build enables
exactly one of them. The web integration of `v2` is opt-in on top of the core tree, and `web` is the
marker the two trees share for it.

## Without the web integration

An application that has no HTTP layer builds the manager, builds the subject of the call, logs the
subject in with a token, and asks the subject what it may do. The subject of this build is the plain
`DelegatingSubject`; it is built from a `DefaultSubjectContext` that names the manager it works
through.

```rust
use std::sync::Arc;

use next_web_security::core::{
    authc::username_password_token::UsernamePasswordToken,
    mgt::{default_security_manager::DefaultSecurityManager, security_manager::SecurityManager},
    realm::simple_account_realm::{SimpleAccount, SimpleAccountRealm},
    subject::{support::default_subject_context::DefaultSubjectContext, Subject},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // The accounts, in code. `IniRealm` reads the same model from a document.
    let mut realm = SimpleAccountRealm::with_name("iniRealm");
    realm.add_account(
        SimpleAccount::new("root")
            .with_password("secret")
            .with_role("admin")
            .with_permission("printer:print"),
    );

    let manager: Arc<dyn SecurityManager> = Arc::new(DefaultSecurityManager::from(realm));

    // The context names the manager the subject works through.
    let mut context = DefaultSubjectContext::default();
    context.set_security_manager(manager.clone());
    let mut subject = manager.create_subject(Arc::new(context)).await;

    subject
        .login(&UsernamePasswordToken::new(
            "root".to_string(),
            "secret".to_string(),
            false,
            None,
        ))
        .await?;

    assert!(subject.is_authenticated().await);
    assert_eq!(
        subject.get_principal().await.map(ToString::to_string),
        Some("root".to_string())
    );

    // The realm of the account answers the authorization questions.
    assert!(subject.has_role("admin").await);
    assert!(subject.has_all_roles(&["admin"]).await);
    assert!(subject.is_permitted("printer:print").await);
    assert!(!subject.is_permitted("user:delete").await);
    assert!(subject.check_roles(&["admin", "auditor"]).await.is_err());

    // Logging out clears the identity and stops the session of the subject.
    subject.logout().await?;
    assert!(!subject.is_authenticated().await);

    Ok(())
}
```

The core tree answers the questions an application has about an identity:

* `Subject::login` / `Subject::logout` with the token of the authentication scheme;
* `has_role`, `has_all_roles`, `is_permitted`, `is_permitted_all`, and the `check_*` variants that
  return `AuthorizationError` instead of `false`;
* `get_session` / `get_session_or_create` — the session of a login is opened by the manager, and the
  state of the subject is written into it;
* realms: `SimpleAccountRealm` in code, `IniRealm` from the Shiro ini document, and any type that
  implements `Realm` for a directory, a database or a remote service;
* credentials: `SimpleCredentialsMatcher`, `HashedCredentialsMatcher`, the SHA matchers,
  `PasswordMatcher` for the `$shiro1$` format, and `AllowAllCredentialsMatcher`.

## With the web integration

The web build adds the filter chain of Shiro on top of the same manager. The application declares
the chains by the paths they serve, and the resolver picks the chain of the request:

```text
/login.jsp  = authc              the form login page
/admin/**   = roles[admin]       the administrative area
/**         = anon               everything else is public
```

```rust
use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, Response, StatusCode},
};
use next_web_core::{
    async_trait,
    http::{
        filter::{FilterError, HttpFilter, HttpFilterChain},
        HttpRequest, HttpResponse, RequestBody, RequestLifecycle,
    },
    util::indexmap::IndexMap,
};
use next_web_security::{
    core::realm::simple_account_realm::{SimpleAccount, SimpleAccountRealm},
    web::{
        filter_proxy::FilterProxy,
        filter_proxy_configure::FilterProxyConfigure,
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

#[tokio::main]
async fn main() {
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
```

A web application of Next Web registers the proxy as an `Arc<dyn HttpFilter>` in its application
context. The filter layer of `next-web` resolves the filters of the context and runs them in front
of the routes; `FilterProxy` reads the path of every request, resolves its chain, and runs the
filters of that chain, so the routes only see the requests a chain let through.

The filters of the chain are the Shiro ones, named the way Shiro names them:

| Name | Filter | What it does |
| --- | --- | --- |
| `anon` | `AnonymousFilter` | lets every request through |
| `authc` | `FormAuthenticationFilter` | the form login: reads `username` / `password`, redirects to the login URL |
| `authcBasic` | `BasicHttpAuthenticationFilter` | `Authorization: Basic ...`, answers `401` with a challenge |
| `authcBearer` | `BearerHttpAuthenticationFilter` | `Authorization: Bearer ...`, answers `401` with a challenge |
| `roles[...]` | `RolesAuthorizationFilter` | the subject must hold every role in the brackets |
| `perms[...]` | `PermissionsAuthorizationFilter` | the subject must hold every permission in the brackets |
| `rest[...]` | `HttpMethodPermissionFilter` | `perms` with the HTTP method appended as the action |
| `user` | `UserFilter` | the subject must be known (logged in or remembered) |
| `logout` | `LogoutFilter` | logs the subject out |
| `noSessionCreation` | `NoSessionCreationFilter` | the chain may not open a session |
| `ssl`, `port`, `ip` | `SslFilter`, `PortFilter`, `IpFilter` | the request must arrive over HTTPS, on a port, from an address |

`FilterProxyConfigure` applies the properties of the application — `loginUrl`, `successUrl`,
`unauthorizedUrl` — to the filters of every chain, whether the filter was registered by the
application or built by the chain manager, and a filter the application configured itself keeps its
own URL.

The web build names the managers of the web layer beside the core ones:

* `DefaultWebSecurityManager` — the manager of an application: it reads the session of the request,
  writes the session cookie, and answers the login with the request and the response at hand;
* `DefaultWebSessionManager` — the session manager that writes and reads `JSESSIONID`;
* `DefaultWebSubjectFactory` and `WebDelegatingSubject` — the subject of a web call, which also
  carries the request and the response;
* `CookieRememberMeManager` — the remember-me identity in a cookie.

## Running the tests

The core tree has its own tests, and they are only compiled without the web integration:

```bash
# The core tree: the login, the roles and permissions of a subject, its session.
cargo test -p next-web-security --no-default-features --features v2

# The default build: the core tree and the web integration.
cargo test -p next-web-security

# The Spring Security style tree.
cargo test -p next-web-security --no-default-features --features v1
```

The complete flow runs on a real Next Web application as well, on a real socket and
through the production filter layer of the framework, and it is driven by a real
HTTP client that keeps its cookies:

```bash
# The login form and its session cookie, the roles of the account, the
# unauthorized page, the logout, the HTTP authentication, remember-me, the
# invalid request filter, and several sessions served at once.
cargo test -p next-web --test v2_web_security
```
