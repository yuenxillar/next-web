# Unblocking the OAuth 2.0 subsystem

Date: 2026-10-19
Status: the subsystem compiles and its tests run; the end-to-end test of the
authorization code flow is still open.

## What was broken

`cargo check -p next-web-security --features oauth2-client,oauth2-resource-server,oauth2-authorization-server`
failed with 35 errors, all of them one of two module paths:

| Loaded path | Where the item lives now |
|---|---|
| `next_web_core::filter::{FilterError, FilterChainError, …}` | `next_web_core::http::filter::…` |
| `next_web_core::traits::{filter::…, http::…, named::Named, required::Required, any_clone::AnyClone}` | `next_web_core::http::filter::…`, `next_web_core::http::…`, `next_web_core::Named`, `next_web_core::Required`, `next_web_core::any::AnyClone` |

The crate was reorganized and the code that only compiles behind a feature flag
was not updated with it. `next_web_core::config` also moved behind
`next_web_core::traits::config` in the old layout.

## What was done

1. Thirteen direct paths were renamed — the six modules of the shiro side, a doc
   link in `web_security.rs`, and six sites of the oauth2 side.
2. The rest are grouped imports of the shape

   ```rust
   use next_web_core::{
       filter::FilterError,
       traits::{
           filter::{HttpFilter, HttpFilterChain},
           http::{HttpRequest, HttpResponse},
           named::Named,
       },
   };
   ```

   which cannot be rewritten by one textual substitution: the brace group has to
   be taken apart. Seventeen files are affected. Instead of rewriting them by
   hand, `next_web_core` gained an explicit compatibility module:

   ```rust
   pub mod traits { /* aliases of the old layout */ }
   pub use crate::http::filter;
   ```

   Every item in it is an alias, and its documentation says so.
3. One real error surfaced once the paths resolved: the test support of the
   authorization endpoints imported `filter::application_filter_chain::ApplicationFilterChain`,
   a private module. It now names the re-export `filter::ApplicationFilterChain`.

## Result

```
default features        248 passed
oauth2 features         365 passed   (+117 tests that could not run before)
```

Both runs include eight doctests. No failures.

## Debt taken on

`next_web_core::traits` and `next_web_core::filter` are now two names for what
`http` and `any` hold. Delete the module once the seventeen files name the new
paths: `next-web-security/src/spring_security/oauth2/**` and
`next-web-security/src/spring_security/config/web/configuration/oauth2_client_configuration.rs`.
A `#[deprecated]` attribute is deliberately absent — it would bury the build in
warnings instead of pointing at the files.

One warning remains: `HttpMethod` is unused in
`oauth2/client/web/oauth2_authorization_code_grant_filter.rs`.

## Still open: the end-to-end test of the authorization code flow

The 119 tests of the subsystem work at the level of a single filter. Nothing runs
the flow through a chain built by `HttpSecurity`, so "the four configurers work"
is still unproven.

*Done:* `the_authorization_endpoint_redirects_to_the_provider`
(`config/web/configurers/oauth2_login_configurer.rs`) builds a real chain and
asserts that `GET /oauth2/authorization/okta` answers 302 with the authorization
uri of the registration, carrying `response_type=code`, `client_id` and `state`.

*Open:* the callback. Both injection points it needs already exist, which was the
thing worth checking:

- `OAuth2LoginConfigurer::authorization_request_repository` (line 497) — the
  default is session based, so a test supplies a repository that remembers the
  request in memory instead of in a session.
- `TokenEndpointConfig::access_token_response_client` (line 535) — reached through
  the `token_endpoint(|endpoint| …)` DSL of the configurer.

What a callback test still has to provide, in the order the flow needs it:

1. An `AuthorizationRequestRepository` that keeps the request in memory. The trait
   is in `oauth2/client/web/authorization_request_repository.rs`; the session
   implementation next to it shows the three methods to implement.
2. A fake `OAuth2AccessTokenResponseClient` that answers a fixed
   `OAuth2AccessTokenResponse` for any request — this is what keeps the test off
   the network.
3. A fake user service through `user_info_endpoint(|endpoint| endpoint.user_service(…))`,
   otherwise the user info of the profile is fetched over HTTP.
4. The two steps in one test: run the redirect, take `state` and (if the
   registration sets it) `code_challenge` from the redirect, then post
   `code` and `state` to `/login/oauth2/code/okta` and assert the security context
   holds the name of the user the fake service returns.

### The two trait signatures the test doubles implement

Verbatim from the source, so the next session does not have to look them up.

```rust
// oauth2/client/web/authorization_request_repository.rs
pub trait AuthorizationRequestRepository: Send + Sync {
    fn load_authorization_request(
        &self,
        request: &dyn HttpRequest,
    ) -> Option<OAuth2AuthorizationRequest>;

    fn save_authorization_request(
        &self,
        authorization_request: Option<OAuth2AuthorizationRequest>,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    );

    fn remove_authorization_request(
        &self,
        request: &mut dyn HttpRequest,
    ) -> Option<OAuth2AuthorizationRequest>;
}

// oauth2/client/endpoint/mod.rs
#[async_trait]
pub trait OAuth2AccessTokenResponseClient<R>: Send + Sync {
    async fn get_token_response(
        &self,
        request: &R,
    ) -> Result<OAuth2AccessTokenResponse, AuthenticationError>;
}
```

The in-memory repository is a `Mutex<Option<OAuth2AuthorizationRequest>>` behind
those three methods; the fake token client answers a fixed
`OAuth2AccessTokenResponse` for any request. The first test worth writing on top
of them needs no token client at all: inject the in-memory repository, run the
redirect, and assert that it stored a request whose `client_id` is the one of the
registration. That is the precondition of the callback, and it holds only because
the default repository writes to a session the test does not have.

The paths the test needs, confirmed:

| Item | Path | Note |
|---|---|---|
| `AuthorizationRequestRepository` | `crate::oauth2::client::web::AuthorizationRequestRepository` | re-exported by `web/mod.rs`; the module itself is not what you import |
| `OAuth2AuthorizationRequest` | `crate::oauth2::core::endpoint::OAuth2AuthorizationRequest` | it is a core type, not a client one — this is the lookup that costs time |
| `OAuth2AccessTokenResponse` | `crate::oauth2::core::endpoint::OAuth2AccessTokenResponse` | same module |

### What the callback test has now

`mod callback_tests` in `config/web/configurers/oauth2_login_configurer.rs` holds
three things the rest of the test builds on:

- `InMemoryAuthorizationRequestRepository`, a full implementation of the trait
  (the request type is `Clone`, so `load` hands out a copy the way the session
  repository does);
- `http_security(repository)`, a chain over one registration and that repository;
- two tests: the redirect remembers a request of the right registration, and a
  callback that answers a state the application never issued is refused.

### The remaining fakes, and what is known about them

```rust
// oauth2/client/userinfo/mod.rs
pub trait OAuth2UserService<R, U>: Send + Sync {
    async fn load_user(&self, request: &R) -> Result<U, AuthenticationError>;
}
```

The login provider is `Arc<dyn OAuth2UserService<OAuth2UserRequest, OAuth2User>>`,
and the configurer reaches it through
`user_info_endpoint(|endpoint| endpoint.user_service(…))`.

Still to look up before writing the success case:

1. ~~Where `OAuth2User` is defined and how it is built.~~ Found:
   `crate::oauth2::core::user::OAuth2User` (re-exported by `core/mod.rs`), built with
   `OAuth2User::new(name, attributes: HashMap<String, String>)`.
2. ~~The shape of `OAuth2UserRequest`.~~ It is
   `crate::oauth2::client::OAuth2UserRequest` (re-exported by `client/mod.rs:63`
   together with `OAuth2UserService` and `DefaultOAuth2UserService`).
3. `OAuth2AccessTokenResponse::new(access_token: impl Into<String>)` — confirmed at
   `oauth2/core/endpoint/mod.rs:222`, and its defaults are enough for the test.

One path is still unchecked: the `R` of the token client,
`OAuth2AuthorizationCodeGrantRequest` — the provider names it as
`OAuth2AccessTokenResponseClient<OAuth2AuthorizationCodeGrantRequest>`, and its
import is the first thing to read in `oauth2/client/authentication/oauth2_login_authentication_provider.rs`.

The fakes then look like this:

```rust
// The token exchange answers a fixed token; nothing else in it is read here.
#[async_trait]
impl OAuth2AccessTokenResponseClient<OAuth2AuthorizationCodeGrantRequest> for FakeTokenClient {
    async fn get_token_response(
        &self,
        _request: &OAuth2AuthorizationCodeGrantRequest,
    ) -> Result<OAuth2AccessTokenResponse, AuthenticationError> {
        Ok(OAuth2AccessTokenResponse::new("an-access-token"))
    }
}

// The profile comes from the test, not from the user-info endpoint.
impl OAuth2UserService<OAuth2UserRequest, OAuth2User> for FakeUserService {
    async fn load_user(&self, _request: &OAuth2UserRequest)
        -> Result<OAuth2User, AuthenticationError> {
        Ok(OAuth2User::new("Max", HashMap::new()))
    }
}
```

Injected through `token_endpoint(|e| e.access_token_response_client(…))` and
`user_info_endpoint(|e| e.user_service(…))`. The test then: run the redirect,
read the stored request from the in-memory repository (it is `Clone`, so `load`
works), post `code` and that `state` to `/login/oauth2/code/okta`, and assert the
security context holds `Max`.

The token client is the easy half: `impl OAuth2AccessTokenResponseClient<OAuth2AuthorizationCodeGrantRequest>`
with `async fn get_token_response(&self, request: &OAuth2AuthorizationCodeGrantRequest)
-> Result<OAuth2AccessTokenResponse, AuthenticationError>`, answering
`OAuth2AccessTokenResponse::new("an-access-token")`.

### A feature-flag defect worth fixing

`cargo test --features oauth2-client` alone does not compile: something in the
subsystem names `crate::oauth2::jose::JwtSigner` and `SecretJwtSigner`
unconditionally, and those live behind `oauth2-authorization-server`. Enabling the
client should not require the server side. The three features together compile and
pass, which is why the gap has not been noticed.


The pieces an end-to-end test needs, from the configurer's own code:

- `ClientRegistrationRepository` — through the DSL
  (`OAuth2LoginConfigurer::client_registration_repository`, resolved in `init`),
  or as a shared object; the configurer panics with
  `"ClientRegistrationRepository is required for oauth2_login"` without it.
- `OAuth2AuthorizedClientRepository` — shared object or DSL; absent, the
  configurer builds its own.
- A chain runner: `crate::web::test_support::Chain`, extracted for exactly this.

The first assertion worth writing is the redirect of
`GET /oauth2/authorization/{registrationId}`: it has to answer 302 with the
authorization uri of the registration, carrying `response_type=code`,
`client_id`, `state` and the PKCE `code_challenge`. That step needs no token
endpoint, so it needs no HTTP client. The callback step does — it exchanges the
code — and cannot be tested until the token response client is injectable.

### The names the test needs

Collected from the source, so the test does not start with a hunt:

```rust
// oauth2/client/registration/mod.rs
ClientRegistration::new("okta", "Okta", "https://idp.example.org/oauth2/authorize")
// takes (registrationId, clientName, authorizationUri); the rest are setters that
// follow the field names: set_client_id, set_client_secret, set_redirect_uri,
// set_token_uri, set_require_proof_key, set_scopes, …

// oauth2/client/registration/mod.rs
InMemoryClientRegistrationRepository::new(vec![registration])

// config/web/configurers/oauth2_login_configurer.rs
http.oauth2_login(|login| {
    login.client_registration_repository(Arc::new(repository));
});
// further setters: authorized_client_repository, authorized_client_service,
// security_context_repository, login_page, login_processing_url,
// authorization_endpoint, token_endpoint, redirection_endpoint,
// user_info_endpoint, base_uri, authorization_request_resolver
```

The chain needs what every hand-assembled chain needs: an
`AuthenticationManagerBuilder`, and the shared `PathPatternRequestMatcherBuilder`
(see the logout note — without it the configurer panics on a missing shared
object). The runner is `crate::web::test_support::Chain`.

### Unconfirmed before writing it

All three are confirmed now, and two of them change the shape of the test:

1. **The redirect does not need the manager.** `OAuth2LoginConfigurer::init` gives
   the redirect filter an `AuthorizationRequestRepository` and the client
   registrations; the authorized client manager belongs to the login filter at
   the callback. So a test of the redirect needs no HTTP client, and no manager.
2. **The default repository is session based.**
   `HttpSessionOAuth2AuthorizationRequestRepository` is the default
   (`get_authorization_request_repository`, `oauth2_login_configurer.rs:201`), and
   its `save_authorization_request` writes through `request.session()`. With no
   session in the test the write is a no-op: the redirect can still be asserted,
   but the `state` is not actually remembered — a test of the callback has to seed
   a session first.
3. **OAuth 2.0 does not resolve `{baseUrl}`.** The only placeholder in the
   subsystem is a registration field of the back-channel logout, hardcoded in its
   `Default`. Unlike the SAML side, a registration here has to carry absolute
   endpoints, so the redirection uri
   (`/login/oauth2/code/{registrationId}`) has to be given in full.


