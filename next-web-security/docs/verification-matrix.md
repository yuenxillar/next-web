# What is verified, and what is not

This file answers one question: for each configuration the builder exposes, what
is the evidence that it works? A status without a test behind it is an opinion,
so every `Ok` below names the tests that hold it up.

Last verified with:

```
cargo test -p next-web-security
    420 passed, 0 failed, 3 over HTTP, 2 over TLS; 8 doctests passed, 9 ignored

cargo test -p next-web-security --features oauth2-client
    474 passed, 0 failed

cargo test -p next-web-security --features oauth2-resource-server
    430 passed, 0 failed

cargo test -p next-web-security --features oauth2-authorization-server
    478 passed, 0 failed

cargo test -p next-web-security \
  --features oauth2-client,oauth2-resource-server,oauth2-authorization-server
    563 passed, 0 failed, 3 over HTTP, 2 over TLS; 8 doctests passed, 10 ignored
```

The eight end-to-end tests are `tests/end_to_end_flow.rs` (a real socket, HTTP),
`tests/x509_over_tls.rs` (a real TLS listener that asks for a client certificate)
and `tests/saml2_signature_interop.rs` (a response signed by the `SignedXml` of
.NET, so the canonicalization of this crate is held against another XML signature
implementation).

## The configurations of the builder, and the tests behind each `Ok`

Every configuration `HttpSecurity` exposes is `Ok`: the ones that were `Pending`
or `Unverified` when this round started are now held by a chain-level test that
runs the real filters of a built chain.

| Configuration | Test that holds it |
|---|---|
| `security_matchers` | `the_matchers_of_several_calls_are_added`, `the_built_chain_only_serves_the_requests_the_matchers_declare`, `a_single_matcher_replaces_the_matchers_declared_before_it`, `an_empty_closure_keeps_the_matcher_of_the_builder`, `the_shared_request_matcher_builder_is_used` |
| `get_authentication_registry` | `the_registry_builds_the_manager_the_chain_authenticates_with`, `a_manager_of_its_own_replaces_the_one_the_registry_builds` |
| `oauth2_resource_server` | `the_configuration_installs_the_filter`, `a_bearer_token_authenticates_the_request`, `a_request_without_a_bearer_token_is_not_authenticated`, `a_token_that_is_refused_is_answered_with_unauthorized` |
| `oauth2_client` | `the_configuration_shares_the_repositories`, `a_request_for_the_authorization_endpoint_is_redirected_to_the_provider`, `the_callback_exchanges_the_code_and_stores_the_authorized_client`, `a_callback_without_a_code_is_not_matched_by_the_grant` |
| `oauth2_login` | `the_callback_signs_in_as_the_user_the_profile_names`, `the_authorities_of_the_login_are_mapped_by_the_configuration`, `a_profile_that_cannot_be_loaded_does_not_sign_the_user_in`, `the_login_page_links_the_login_of_the_registration` |
| `oidc_logout` | `the_configuration_installs_the_filter_before_the_csrf_filter`, `the_configuration_shares_the_repositories`, `a_logout_token_ends_the_sessions_of_the_provider`, `a_request_that_is_not_a_logout_is_left_alone`, `a_logout_token_that_cannot_be_verified_is_refused` |
| `saml2_login` | `a_response_signed_by_the_asserting_party_logs_the_user_in`, `a_believed_response_authenticates_through_the_chain`, `a_rejected_response_redirects_back_to_the_login_page`, and the six tests of `XmlSignatureVerifier` |
| `saml2_logout` | `a_signed_logout_request_ends_the_session_and_is_answered`, `a_logout_request_without_a_signature_is_refused`, `a_signed_logout_response_completes_the_logout`, `the_configuration_installs_the_logout_filters` |
| `oauth2_authorization_server` | `an_authorization_request_is_answered_with_a_code`, `the_code_is_exchanged_for_a_token`, `the_metadata_of_the_server_is_published`, `a_request_without_a_registered_client_is_refused` |

The runtime code of this round is two things the port did not have: an XML
signature verifier (`saml2/xml_dsig.rs`, exclusive canonicalization over a parsed
tree, RSA-SHA256 and SHA-256, fail-closed on every other algorithm) and the two
filters of the logout profile (`web/saml2/logout/`). Everything else that was
`Pending` was a configuration whose filters existed but whose behaviour no test
ran.

## Verified over HTTP and TLS

A request that goes through the chain the way an application receives it: a real
`TcpListener`, the request lifecycle of the web layer (the one that prepares the
request and tells the client about its session), the body buffered for the
filters, cookies, redirects — and, for the client certificate, a real TLS
handshake. This is the strongest evidence in this file.

| Configuration | Test | What it proves |
|---|---|---|
| the login page | `the_login_page_and_the_passkey_script_are_served` | the page carries the passkey form and loads `/login/webauthn.js`, and the script talks to the endpoints |
| form login | `a_form_login_signs_the_user_in` | the submission is accepted with the CSRF token the page rendered, the session is started, and the next request is authenticated |
| web authn | `a_passkey_is_registered_and_then_signs_the_user_in` | a passkey is registered for the signed-in user through the two registration endpoints, and a **new client with no password** signs in through the two authentication endpoints and reaches the application as that user |
| x509 client certificates | `a_client_certificate_authenticates_over_tls`, `a_client_without_a_certificate_is_refused_by_the_handshake` | a TLS handshake that requires a client certificate reaches the chain, and the application sees the user the certificate names; a client that presents none never reaches the chain |

What the x509 test does **not** prove is the certificate's own validation: rustls
validates it in the test, as a container would in production, and the request
carries the certificate itself — `next_web_core::http::PeerCertificate`, which
only a transport that validated it may insert. `ClientCertificateAttributeFilter`,
which `x509()` installs before the filter that authenticates, reads the subject
out of the DER encoding and writes it into the attribute the X.509 filter reads;
the test therefore walks the same path a TLS server or a terminator takes, and
what it holds is the subject parsing, the attribute, the user lookup,
`FACTOR_X509` and the session.

A transport of an application inserts `PeerCertificate` itself, and the TLS
server of `next-web` does: `TlsWebServer` terminates the connection with rustls,
reads the certificate the handshake settled on, and inserts it into every request
it serves (`next-web/tests/tls_client_certificate.rs`, which serves a
client-auth-only listener and checks the request carries the certificate, and
that a client without one is refused). A terminator that reports a subject as
text can use `PeerCertificate::with_subject` instead, and the filter uses that
subject as it is.

## Verified end to end

The test runs a chain that a real `HttpSecurity::build()` produced, with the
filters in order, the way the chain proxy runs them.

| Configuration | Evidence | File |
|---|---|---|
| form login | `a_form_login_authenticates_through_the_chain`, `a_wrong_password_does_not_authenticate` | `config/web/configurers/form_login_configurer.rs` |
| custom login page | `a_custom_login_page_is_installed_as_the_entry_point` | same |
| custom processing URL | `a_custom_login_processing_url_is_kept` | same |
| logout | `a_logged_in_user_can_be_logged_out` | same |
| http basic | `http_basic_authenticates_through_the_chain`, `http_basic_without_credentials_does_not_authenticate` | same |
| oauth2 authorization redirect | `the_authorization_endpoint_redirects_to_the_provider`, `the_redirect_remembers_the_authorization_request` | `oauth2_login_configurer.rs` |
| oauth2 callback (refusal) | `a_callback_with_an_unknown_state_is_refused` | same |
| oauth2 callback (success) | `a_callback_that_answers_the_issued_request_logs_the_user_in` | same |
| oauth2 authorization server | `an_authorization_request_is_answered_with_a_code`, `the_code_is_exchanged_for_a_token`, `the_metadata_of_the_server_is_published`, `a_request_without_a_registered_client_is_refused` | `config/web/configurers/oauth2_authorization_server_configurer.rs` |
| saml2 metadata | the filter and configurer tests, `Saml2MetadataFilter` | `web/saml2/metadata/`, `config/web/configurers/saml2_metadata_configurer.rs` |
| saml2 login | `a_response_signed_by_the_asserting_party_logs_the_user_in`, `a_believed_response_authenticates_through_the_chain`, `a_rejected_response_redirects_back_to_the_login_page` | `config/web/configurers/saml2_login_configurer.rs` |
| saml2 logout | `a_signed_logout_request_ends_the_session_and_is_answered`, `a_logout_request_without_a_signature_is_refused`, `a_signed_logout_response_completes_the_logout`, `the_configuration_installs_the_logout_filters` | `config/web/configurers/saml2_logout_configurer.rs` |
| saml2 signature | the six tests of `XmlSignatureVerifier` — the signature of a response verifies, a changed assertion is refused, another key is refused, no signature is refused, a registration without a key is refused, SHA-1 is refused | `saml2/login/xml_signature_verifier.rs` |
| saml2 signature (another implementation) | `the_signature_of_a_response_signed_by_another_implementation_is_verified`, `a_response_of_the_other_implementation_that_changed_is_refused`, `a_response_of_the_other_implementation_is_refused_for_another_key` — the fixture in `tests/fixtures/saml2/` is signed by the `SignedXml` of .NET, so the canonicalization of this crate is held against another XML signature implementation | `tests/saml2_signature_interop.rs` |
| one-time token submit | `the_token_is_read_from_the_form_body`, `a_form_without_a_token_converts_to_nothing` | `web/authentication/ott/one_time_token_authentication_converter.rs` |
| csrf token resolution | `the_token_is_resolved_from_a_form_body`, `a_submission_without_the_token_resolves_to_nothing` | `web/csrf/csrf_token_request_resolver.rs` |
| web authn | `a_passkey_registers_and_then_logs_the_user_in_through_the_chain` | `config/web/configurers/web_authn_configurer.rs` |
| x509 client certificates (chain, without TLS) | `a_request_with_a_certificate_authenticates_through_the_chain`, `a_request_without_a_certificate_does_not_authenticate` | `config/web/configurers/x509_configurer.rs` |

The form login, logout, http basic, the oauth2 cases, the web authn case and the
x509 case above share one chain runner: `web/test_support::Chain`. The web authn
case runs the two filters a built chain produced, registering a credential through
`/webauthn/register/options` and `/webauthn/register` and then logging in through
`/webauthn/authenticate/options` and `/webauthn/authenticate`.

The x509 case is the framework's half of that configuration only: the certificate
itself is validated by the container, or by the terminator in front of it, before
the request reaches the chain. What the test proves is that the attribute the
container writes is read, the user is looked up, the authentication carries
`FACTOR_X509`, and a request without a certificate authenticates nobody.

## Verified in parts

| Configuration | What holds, what does not |
|---|---|
| oauth2 client | The authorization code flow is verified from the redirect to the authenticated callback. The authorize-server side of the subsystem has filter-level tests but no chain test. |
| oauth2 authorization server | The authorization code grant is verified end to end: the authorization endpoint issues a code to a registered client and the token endpoint exchanges it for an access token. The consent page, the device flow, the pushed authorization request, the introspection and revocation endpoints and the user info endpoint are part of the chain but are not walked end to end. |

The SAML 2.0 login is no longer here: it verifies the signature of a response and
completes, and the tests above hold that from the endpoint to the stored
authentication.

## The half a configuration does not cover

| Configuration | What is missing |
|---|---|
| saml2 logout | A logout started at the application is not redirected to the asserting party: the registration of this port carries no signing key of the service provider, so the `LogoutRequest` that would start it cannot be signed. The half where the asserting party starts the logout, and the answer to it, are served and verified (`Saml2LogoutRequestFilter`, `Saml2LogoutResponseFilter`). |

It is marked `Ok` in the documentation of `HttpSecurity`, and the documentation
says which half of the profile is served rather than only that it is served.

## The one defect class this round found

Four places read a parameter from the query string only, while the endpoint they
served received a url-encoded form. Each is fixed and each has a test:

| Place | What it meant in production |
|---|---|
| `UsernamePasswordAuthenticationFilter` | a form login could never read the credentials |
| `oauth2::request_parameter` | the oauth2 callback never found the code |
| `OneTimeTokenAuthenticationConverter` | a submitted one-time token was never found |
| `CsrfTokenRequestResolver` | with csrf enabled, every form submission failed it |

A fifth suspect turned out to be correct already:
`oauth2/server/authorization/client_authentication.rs` parses the body itself as a
fallback. The lesson: a grep hit is a candidate, not a finding — read the function
before calling it a defect.

The last suspect was examined and fixed as well: the consent submission of
`oauth2/client/authorized_client.rs` read its `scope` from the query only, so the
scopes the resource owner chose were never seen.

## Web Authentication, the ceremony

The ceremony is the two methods of `WebAuthnManager`: `register` turns the
attestation a browser submits into a credential the relying party keeps, and
`authenticate` turns the assertion it submits into the credential that made it.
Both run the same chain of checks — the ceremony the client data names, the
challenge this relying party issued for that ceremony, the origin it allows, the
relying party the authenticator data binds to, and the user who took part — and
the refused cases are tests:

| Case | Test |
|---|---|
| origin of another site | `a_registration_of_another_origin_is_refused`, `an_authentication_of_another_origin_is_refused` |
| `rpIdHash` of another relying party | `a_registration_of_another_relying_party_is_refused`, `an_authentication_of_another_relying_party_is_refused` |
| replayed challenge | `a_registration_that_answers_the_same_challenge_twice_is_refused`, `an_authentication_that_answers_a_replayed_challenge_is_refused` |
| challenge never issued | `a_registration_that_answers_a_challenge_that_was_never_issued_is_refused` |
| challenge of the other ceremony | `a_challenge_issued_to_an_authentication_does_not_answer_a_registration` |
| expired challenge | `a_ceremony_whose_challenge_expired_is_refused` |
| tampered signature | `a_tampered_signature_is_refused` |
| sign count going backwards | `an_authentication_whose_sign_count_went_backwards_is_refused` |
| `USER_PRESENT` flag clear | `a_registration_without_the_user_present_flag_is_refused` |
| user verification required and not done | `a_registration_of_a_user_verification_that_was_not_performed_is_refused`, `an_authentication_of_an_unverified_user_is_refused_when_verification_is_required` |
| credential id of another user | `an_authentication_of_another_user_is_refused`, `a_registration_of_a_credential_id_of_another_user_is_refused` |

`authenticate` reports the user of the credential it verified; it does not take a
user to compare against. A caller that started a ceremony for one named user
compares `Credential::user_name` itself — `allowCredentials` is a hint to the
browser, not a check the relying party can rely on.

| Piece | Tests | Module |
|---|---|---|
| the options handed to the browser | 6 | `webauthn/options.rs` |
| the client data of a ceremony | 9 | `webauthn/client_data.rs` |
| the authenticator data and the attested credential | 9 | `webauthn/authenticator_data.rs` |
| the challenges a ceremony answers | 3 | `webauthn/challenge.rs` |
| the credentials of a relying party | 6 | `webauthn/credential.rs` |
| the requests the browser submits | 9 | `webauthn/request.rs` |
| the COSE key and the two signatures | 12 | `webauthn/cose.rs` |
| the attestation object | 12 | `webauthn/attestation.rs` |
| the two ceremonies | 30 | `webauthn/manager.rs` |
| the assertion as an authentication | 11 | `web/webauthn/web_authn_authentication*.rs` |
| the two endpoints of a registration | 5 | `web/webauthn/web_authn_registration_filter.rs` |
| the two endpoints of an authentication | 3 | `web/webauthn/web_authn_authentication_filter.rs` |
| the configuration of both ceremonies | 6 | `config/web/configurers/web_authn_configurer.rs` |
| the page a passkey is registered from | 3 | `web/authentication/ui/default_webauthn_registration_page_generating_filter.rs` |
| the login page's passkey form and script | 2 | `web/authentication/ui/default_login_page_generating_filter.rs` |
| the browser script, by its content | 2 | `web/authentication/ui/default_resources_filter.rs` |

What is **not** verified: the browser itself. `spring-security-webauthn.js`
calls `navigator.credentials.get` and `create`, and the two pages call into it,
but no test in this crate runs a browser, and nobody has walked a passkey
registration and a passkey sign-in on a real origin yet. That walk — one browser,
one virtual authenticator, one real one — is the first thing to do before this is
called done end to end.

Two things that table does not say on its own:

- The signatures the ceremony tests sign are made with a key the test holds, so
  they prove the plumbing and not the arithmetic. The arithmetic is anchored by
  the two signature vectors of RFC 7515 — appendix A.2 for RS256 and A.3 for
  ES256 — which another implementation produced. They are the only signatures in
  the crate that this port did not make.
- EdDSA, P-384 and P-521 are refused by name rather than accepted untested.
  ES256 and RS256 are the two algorithms the specification makes every
  authenticator support; the rest would need arithmetic the port does not carry.

## The defects verifying over HTTP found

Three defects stood between "the unit tests pass" and "a browser can use this",
and none of them had a test:

| Where | What it meant in production |
|---|---|
| `web/csrf/repository_deferred_csrf_token.rs` | the token was loaded, generated and saved against a **snapshot** of the request instead of the request itself, and a snapshot does not carry the session: no token was ever loaded back, so with CSRF enabled **every form submission was refused** (403); and `is_generated` reported false for a generated token, so a refused submission was logged as invalid rather than missing |
| `config/web/configurers/web_authn_configurer.rs` | the configurer enabled the passkey form but never wired `resolve_headers` on the login page — the reference implementation's `WebAuthnConfigurer` does — so the script sent **no** CSRF header and the JSON endpoints answered 403 |
| `config/web/configurers/{web_authn,saml2_login}_configurer.rs` | the login filter was added with `add_filter_before`, which bypasses the configurer that hands a filter the chain's security context repository, so the login was saved to the request attribute only: **the next request was anonymous** |

The first is one defect, not two: the reference implementation's
`RepositoryDeferredCsrfToken.init()` loads, generates and saves with the live
request and response, and this port had replaced them with snapshots. The fix
keeps that shape — the deferred token does the same three steps with the live
request and response it is handed — and the filter is back to the reference's
own body. Because a Rust borrow cannot be held across the rest of the filter,
the three steps run when the token is loaded rather than lazily on first use;
the filter reads a token from its first moment, so nothing observes the
difference.

Each has a test now: the CSRF one in `csrf_filter.rs`
(`the_token_of_a_page_is_stored_in_the_session_of_the_client`,
`a_submission_that_carries_the_rendered_token_is_accepted`), the other two in
`tests/end_to_end_flow.rs`.

## The defect verifying x509 found

A pre-authenticated token whose principal is a `User` reported
`User(username=Alice)` as its name, where the name of an authentication is the
user name — Spring's `AbstractAuthenticationToken#getName()` returns the username
of a `UserDetails` principal and only falls back to the display form for other
principals. The chain test found it because it asserts the name of the stored
authentication; `Identity::principal_name` now carries the rule, `User` answers
with its user name, and `SimpleAuthentication` and
`PreAuthenticatedAuthenticationToken` ask it. Every path whose provider puts a
`UserDetails` in the principal — x509, CAS and the other pre-authenticated ones —
reports the user name now.


