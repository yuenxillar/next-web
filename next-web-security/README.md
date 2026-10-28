# next-web-security

Security, authentication, and authorization for [Next Web](https://github.com/yuenxillar/next-web).

The crate is a Rust port of the [Spring Security](https://spring.io/projects/spring-security)
architecture. An application declares the chain it wants through an `HttpSecurity` DSL, the
configurers contribute the filters of that chain, and the chain runs in front of the application on
the `HttpFilter` pipeline of `next-web-core`.

Two trees live in the crate:

| Tree | Module root | Shape | Status |
| --- | --- | --- | --- |
| `v2` | `src/v2` | Apache Shiro style: security manager, realms, subjects, sessions, and the filter chain | supported, and the default build; documented in [`src/v2/README.md`](src/v2/README.md) |
| `v1` | `src/v1` | Spring Security 6 style: `HttpSecurity`, configurers, `WebSecurityConfigurer`, the security context | supported; documented in this README |

The two trees are mutually exclusive. The default features build the `v2` tree with its web
integration (`v2-web`); the examples below configure the `v1` tree, which is built with
`--no-default-features --features v1`.

## Installation

```toml
[dependencies]
next-web-security = "0.2"
```

A default build compiles the `v2` tree with its web integration: the security manager, realms,
subjects, sessions, the filter chain, and the HTTP authentication and authorization filters. The
`v1` tree below is built with `--no-default-features --features v1`.

The protocol subsystems are opt-in, and each one brings only its own third-party dependencies with
it:

```toml
[dependencies]
next-web-security = { version = "0.2", features = ["saml2", "webauthn", "oauth2"] }
```

## Features

| Feature | What it enables | Third-party crates it adds |
| --- | --- | --- |
| `v2-web` *(default)* | the `v2` core tree and its web integration: `src/v2/core` and `src/v2/web` | `dashmap` |
| `v2` | the `v2` core tree alone: `src/v2/core`, without any dependency on the web integration | `dashmap` |
| `v1` | the whole `v1` tree below, minus the opt-in subsystems | — |
| `saml2` | `saml2`, `web::saml2`, and the `saml2_login()`, `saml2_metadata()`, `saml2_logout()` configurers | `quick-xml`, `flate2`, `rsa` |
| `webauthn` | `webauthn`, `web::webauthn`, and the `web_authn()` configurer | `ciborium`, `coset`, `p256`, `rsa` |
| `oauth2-client` | `oauth2::client`, the `oauth2_login()` and `oauth2_client()` configurers | `reqwest`, `jsonwebtoken`, `rsa` |
| `oauth2-resource-server` | `oauth2::server::resource` and the `oauth2_resource_server()` configurer | `reqwest`, `jsonwebtoken`, `rsa` |
| `oauth2-authorization-server` | `oauth2::server::authorization` and the `oauth2_authorization_server()` configurer | `jsonwebtoken`, `hmac`, `rsa` |
| `oauth2` | all three OAuth 2.0 sides at once | the union of the three above |
| `web` | the marker the two trees share for the web integration | — |

`saml2`, `webauthn` and every `oauth2-*` feature pull `rsa`, because each of them signs or verifies
with it; enabling two of them costs it once. The protocol features extend the `v1` tree, so they are
built with `--no-default-features --features v1,saml2` — a build that enables `v1` together with the
default `v2-web` is refused at compile time.

Gated code is gated as a whole: with `saml2` off, the `saml2` module is not compiled, the
`saml2_*()` methods are not on `HttpSecurity`, and `quick-xml` and `flate2` are not built.

### What a build costs

`cargo tree -e normal` resolves, for the whole graph of this crate:

| Build | Crates |
| --- | --- |
| default (feature `v2-web`) | 129 |
| `saml2` | 148 |
| `webauthn` | 165 |
| `oauth2-authorization-server` | 147 |
| `oauth2-client` | 195 |
| `saml2,webauthn,oauth2` | 224 |
| before the split — every dependency was unconditional, whatever the features | 228 |

The OAuth 2.0 features pull `reqwest`, which builds a TLS backend (`aws-lc-sys` or `ring`); those
builds need a C toolchain.

### What is not behind a feature

A few crates stay unconditional, because the core chain itself needs them:

- `bcrypt` — `BCryptPasswordEncoder` and the `DaoAuthenticationProvider`
- `md5` — the remember-me token digest and HTTP Digest authentication
- `sha2`, `base64`, `hex`, `urlencoding`, `rand`, `chrono` — one-time tokens, remember-me tokens,
  CSRF tokens, and the X.509 subject mapping
- `simple_asn1` — the X.509 client-certificate pre-authentication, which is part of the chain
  rather than an opt-in subsystem

`futures` used to be listed here and was not referenced by the crate at all; it is gone.

## Quick start

```rust
use std::sync::Arc;

use next_web_security::{
    authentication::dao::DaoAuthenticationProvider,
    config::web::{builders::HttpSecurity, HttpSecurityBuilder, WebSecurityConfigurer},
    core::userdetails::{User, UserDetailsService},
    crypto::{bcrypt::BCryptPasswordEncoder, password::PasswordEncoder},
    provisioning::InMemoryUserDetailsManager,
};

/// The security configuration of an application: a form login over users in memory.
#[derive(Clone)]
struct SecurityConfig;

impl WebSecurityConfigurer for SecurityConfig {
    fn configure(&mut self, http: &mut HttpSecurity) {
        // A password is stored the way a password is stored: as a bcrypt hash.
        let password = BCryptPasswordEncoder::new(10)
            .encode(Some("password"))
            .expect("the password is encoded")
            .expect("the encoder returns a hash");
        let users: Arc<dyn UserDetailsService> =
            Arc::new(InMemoryUserDetailsManager::with_users(vec![Arc::new(
                User::with_username("user")
                    .password(Some(password))
                    .authorities_with_strings(&["ROLE_USER"])
                    .build(),
            )]));

        // The provider checks the submitted password against the hash of the user.
        let mut provider = DaoAuthenticationProvider::new(users);
        provider.set_password_encoder(Arc::new(BCryptPasswordEncoder::new(10)));
        http.authentication_provider(Arc::new(provider));

        http.form_login(|_| {})
            .logout(|_| {})
            // Every request that is not one of the login endpoints needs an
            // authentication; the generated login page is served to the rest.
            .authorize_http_requests(|requests| {
                requests.any_request().authenticated();
            });
    }
}
```

An application binds the implementation as a `WebSecurityConfigurer`; a Next Web application does
that with the `#[singleton(binds = [...])]` attribute of `next-web-macros` — see
`examples/security.rs` in the workspace for a complete application.

Every configuration takes a closure that receives the configurer of that configuration. A closure
that does nothing keeps the defaults of the configurer, so `.form_login(|_| {})` is the whole of
the default form login.

## The `v1` tree

| Module | Contents | Feature |
| --- | --- | --- |
| `access` | authorization managers, `AccessDeniedHandler`s, and the expression evaluators behind `@pre_authorize` / `@post_authorize` | `v1` |
| `acl` | the access control list model: `Acl`, `ObjectIdentity`, `Sid`, `Permission` | `v1` |
| `authentication` | the providers and tokens of a login: `DaoAuthenticationProvider`, one-time tokens, authentication events | `v1` |
| `authorization` | `AuthorizationFilter`, the authorization managers and events, the required-factor rules | `v1` |
| `autoconfigure` | the auto configuration that installs the default chain: properties, user details service, filter registration, request matchers | `v1` |
| `config` | the DSL itself: `HttpSecurity`, `SecurityBuilder`, the configurers, `WebSecurityConfigurer` | `v1` |
| `context` | `DelegatingApplicationListener` and the glue to the application context | `v1` |
| `core` | `Authentication`, `UserDetails`, authorities, `AuthenticationError` | `v1` |
| `crypto` | `BCryptPasswordEncoder` and `DelegatingPasswordEncoder` | `v1` |
| `ldap` | LDAP user details, authority populators and `LdapAuthenticationProvider` (no directory client yet) | `v1` |
| `oauth2` | OAuth 2.0 and OpenID Connect: the core messages, the client, the resource server, the authorization server, and the JWK / JWS keys | `oauth2-*` |
| `permission` | the permission model and its authorization manager | `v1` |
| `provisioning` | `UserDetailsManager` and `InMemoryUserDetailsManager` | `v1` |
| `saml2` | the SAML 2.0 relying party: registrations, assertions, XML DSig, metadata, single logout | `saml2` |
| `web` | the chain and its filters: CSRF, headers, sessions, firewall, request cache, X.509, the generated pages, the request matchers | `v1`, with `web::saml2` and `web::webauthn` in their features |
| `webauthn` | the Web Authentication ceremonies: COSE keys, attestation, `WebAuthnManager` | `webauthn` |
| `cas`, `data`, `kerberos`, `messaging`, `rsocket` | translation entry points; nothing is implemented in them yet | `v1` |

## The `v2` tree (Apache Shiro style)

The `v2` feature is the Apache Shiro 3.0.1 model, ported class by class: the
`Subject` as the application-facing handle, `SecurityManager` behind it, realms
that look accounts up, and the credentials matchers and authorization that decide
what a login may do. The two trees are mutually exclusive — each is re-exported at
the crate root — so a build enables exactly one of them.

The tree is layered the way Shiro is: `v2` is the core — no HTTP at all — and
`v2-web` adds the filter chain and the session cookie on top of it. The default
features build `v2-web`. The two builds, the module map, and a complete example of
each are documented in [`src/v2/README.md`](src/v2/README.md).

```toml
# The core tree, without the web integration.
next-web-security = { version = "0.2", default-features = false, features = ["v2"] }

# The core tree and the web integration (the default build).
next-web-security = { version = "0.2", features = ["v2-web"] }
```

```rust,ignore
use std::sync::Arc;

use next_web_security::core::{
    authc::{
        authenticator::Authenticator,
        pam::modular_realm_authenticator::ModularRealmAuthenticator,
        username_password_token::UsernamePasswordToken,
    },
    authz::{authorizer::Authorizer, modular_realm_authorizer::ModularRealmAuthorizer},
    realm::{
        simple_account_realm::{SimpleAccount, SimpleAccountRealm},
        Realm,
    },
};

let mut realm = SimpleAccountRealm::with_name("iniRealm");
realm.add_account(
    SimpleAccount::new("root")
        .with_password("secret")
        .with_role("admin")
        .with_permission("printer:*"),
);
let realm = Arc::new(realm);

let authenticator = ModularRealmAuthenticator::new(vec![realm.clone() as Arc<dyn Realm>]);
let authorizer = ModularRealmAuthorizer::new(vec![realm as Arc<dyn Authorizer>]);

let token = UsernamePasswordToken::new("root".into(), "secret".into(), false, None);
let authentication = authenticator.authenticate(&token).await?;
let principals = authentication.get_principals().unwrap();

assert!(authorizer.is_permitted_from_str(Some(principals.as_ref()), "printer:print"));
assert!(authorizer.has_role(Some(principals.as_ref()), "admin"));
```

Stored passwords are hashed, never kept as text. The port has the whole chain
Shiro uses for that:

* `SimpleHash` — the salted, iterated digest (MD5, SHA-1, SHA-256, SHA-384, SHA-512).
* `HashedCredentialsMatcher` and `Sha256/384/512CredentialsMatcher` — a stored hash
  compared with the hash of what the user submitted.
* `DefaultPasswordService` and `PasswordMatcher` — the modular crypt format
  `$shiro1$ALGORITHM$ITERATIONS$salt$hash`, where the parameters travel with the
  stored value, so a stored password keeps verifying after the configuration of
  the application changes.
* `AllowAllCredentialsMatcher` — for a realm that verified the credential already.

The web side is the filter chain of the `v2` tree: `DefaultFilterChainManager` and
`PathMatchingFilterChainResolver` pick the chain of a request path (`/login.jsp`
with `anon`, `/**` with `authc`), and the filters of that chain run in order.

Accounts can come from a document instead of code, with the sections Shiro uses:

```ini
[users]
root = secret, admin

[roles]
admin = printer:*
```

```rust,ignore
let realm = IniRealm::new(document)?;
```

Sessions are created by the manager a subject asks, swept for expiry by
`ExecutorServiceSessionValidationScheduler`, and cached by `CachingSessionDAO`.
The scheduler takes a `SessionSweeper` — the same store, without a reference back
to the manager that owns the scheduler — so periodic sweeping can be wired up
before the manager is wrapped in an `Arc`.

Authentication strategies decide what a configuration of several realms requires:
`AtLeastOneSuccessfulStrategy` (the default), `AllSuccessfulStrategy`, and
`FirstSuccessfulStrategy`, which stops asking as soon as one realm authenticated.

### What the `v2` tree deliberately does not port

The Shiro classes that are tied to a runtime the port does not target are left
out rather than translated into something that only looks the same:

* the servlet and JSP surfaces — `ShiroHttpServletRequest`, `ShiroHttpSession`, the
  `*Tag` taglib classes, `AbstractShiroFilter` — because the port runs on axum and
  a filter chain, not on a servlet container;
* the annotation interceptors — `@RequiresPermissions` and friends — because Rust
  has no annotation-driven AOP, so the checks are called explicitly;
* JNDI, LDAP and Active Directory realms — the port has no directory client, so a
  deployment registers its own `Realm`;
* the Spring and OSGi bootstrap — `EnvironmentUser`, the ini factories,
  `EnvironmentLoader` — because an application builds the manager in code.

The Java exception hierarchy is collapsed into the error enums of the port
(`AuthenticationError`, `AuthorizationError`, `SessionError`), which keeps the
cases an HTTP layer has to tell apart and drops the ones it does not.

## SAML 2.0

```rust
use std::sync::Arc;

use next_web_security::saml2::registration::{
    InMemoryRelyingPartyRegistrationRepository, RelyingPartyRegistration,
    RelyingPartyRegistrationRepository,
};

// `http: &mut HttpSecurity` and `users: Arc<dyn UserDetailsService>` of the
// application, as in the quick start above.
let registrations: Arc<dyn RelyingPartyRegistrationRepository> = Arc::new(
    InMemoryRelyingPartyRegistrationRepository::new(vec![
        RelyingPartyRegistration::with_registration_id("okta")
            .entity_id("sp-entity-id")
            .assertion_consumer_service_location("https://sp.example.org/login/saml2/sso/okta")
            .asserting_party_entity_id("https://idp.example.org")
            .single_sign_on_service_location("https://idp.example.org/sso")
            .single_logout_service_location("https://sp.example.org/logout/saml2/slo")
            .single_logout_service_response_location("https://idp.example.org/slo")
            .build(),
    ]),
);

http.saml2_login(|saml2| {
    saml2
        .relying_party_registration_repository(registrations.clone())
        .user_details_service(users.clone());
})
.saml2_metadata(|metadata| {
    metadata.relying_party_registration_repository(registrations.clone());
})
.saml2_logout(|logout| {
    logout.relying_party_registration_repository(registrations.clone());
});
```

A response is believed only when its XML signature verifies against the key of the registration
(exclusive canonicalization, RSA-SHA256, SHA-256 digest) and the conditions of the web browser SSO
profile hold. A message that names another algorithm, carries no signature, or changed after it was
signed is refused.

## WebAuthn (passkeys)

```rust
// `http: &mut HttpSecurity` and `users: Arc<dyn UserDetailsService>` of the
// application, as in the quick start above.
http.web_authn(|web_authn| {
    web_authn
        .rp_id("example.org")
        .rp_name("The application")
        .allowed_origins(["https://example.org"])
        .user_details_service(users.clone());
});
```

The chain serves `POST /webauthn/register/options`, `POST /webauthn/register`,
`POST /webauthn/authenticate/options` and `POST /webauthn/authenticate`, plus the page a passkey is
registered from and the script both pages call (`/login/webauthn.js`). Only the `none` attestation
format is believed.

## OAuth 2.0

The three sides are independent features; `oauth2` enables all of them.

```rust
// A client: the authorization code flow against a registration of the application.
use std::sync::Arc;

use next_web_security::oauth2::{
    ClientRegistration, ClientRegistrationRepository, InMemoryClientRegistrationRepository,
};

let registrations: Arc<dyn ClientRegistrationRepository> = Arc::new(
    InMemoryClientRegistrationRepository::new(vec![
        ClientRegistration::new("test", "Test", "https://idp.example.org/authorize")
            .set_client_id("client")
            .set_client_secret("secret")
            .set_redirect_uri("https://sp.example.org/login/oauth2/code/test")
            .set_token_uri("https://idp.example.org/token")
            .set_scopes(["openid", "read"]),
    ]),
);

http.oauth2_login(|login| {
    login.client_registration_repository(registrations.clone());
})
.oauth2_client(|client| {
    client.client_registration_repository(registrations.clone());
});
```

```rust
// A resource server: the decoder of the application verifies the access token.
// It is an `Arc<dyn JwtDecoder>`: `oauth2::server::resource::jwt::JwtDecoder`.
http.oauth2_resource_server(|resource_server| {
    resource_server.jwt(|jwt| {
        jwt.decoder(Arc::new(my_jwt_decoder));
    });
});
```

```rust
// An authorization server: the registered clients of the application.
use next_web_security::oauth2::server::authorization::InMemoryRegisteredClientRepository;

http.oauth2_authorization_server(|server| {
    server.registered_client_repository(Arc::new(
        InMemoryRegisteredClientRepository::default(),
    ));
});
```

## Testing

The tests of the crate are split the same way the features are:

```text
cargo test -p next-web-security                              # the core tree
cargo test -p next-web-security --features saml2,webauthn    # the two protocol modules
cargo test -p next-web-security --features oauth2            # the three OAuth 2.0 sides
```

The `end_to_end_flow` test drives a real chain over a real socket; its passkey test is part of the
`webauthn` build. `--all-features` is not usable yet, because it enables `v2`.

## License

MIT OR Apache-2.0.
