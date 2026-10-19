# SAML 2.0 Login Slice (part 1) — Execution Record

**Goal:** let the service provider start a login at an asserting party.

**Scope chosen by the caller:** the full login chain, with XML signature verification behind a fail-closed boundary. This record covers the half that can be built and tested in this environment: everything up to the assertion consumer service. The verifier seam and the consumer are specified at the end as the next slice.

**Why the boundary exists:** verifying a SAML response needs XML-DSig — exclusive canonicalization, the enveloped-signature transform, and reference/digest checks. `samael` and `xmlsec` are not in the offline crate cache and crates.io is unreachable from this sandbox (`cargo search` fails with a TLS credential error), so no trustworthy implementation can be added. Hand-writing canonicalization is how signature-wrapping vulnerabilities get shipped; it is deliberately not attempted.

## What was built

| File | Responsibility |
|---|---|
| `src/spring_security/saml2/error.rs` | `Saml2Error` (kind + message, `Display`, `Error`, `From<io::Error>`) and `Saml2ErrorKind`: `MissingConfiguration`, `RegistrationNotFound`, `InvalidResponse`, `VerificationFailed`, `UserNotFound`, `Encoding` |
| `src/spring_security/saml2/xml.rs` | Shared `resolve_placeholders` and `escape_xml` (the metadata writer was moved onto them) |
| `src/spring_security/saml2/login/saml2_authentication_request.rs` | `Saml2AuthenticationRequest::create(registration, base_url)`, `to_xml()`, `to_redirect_url(relay_state)`, `id()` |
| `src/spring_security/web/saml2/login/saml2_authentication_request_filter.rs` | `Saml2AuthenticationRequestFilter`: `GET /saml2/authenticate/{registrationId}` → redirect to the asserting party; unknown registration → 401; unmatched → chain |
| `src/spring_security/web/saml2/mod.rs` | `request_base_url` (scheme, host, non-default port, context path), shared by the metadata and login filters |
| `next-web-security/src/spring_security/saml2/registration/relying_party_registration.rs` | IdP side of the registration: asserting party entity id, single sign-on location and binding (default `Redirect`), verification public key |

**Dependency:** `flate2` added to the workspace and to `next-web-security` (the HTTP-Redirect binding deflates the request). It is already in `Cargo.lock` and the crate cache, so no new download is involved.

## Contract implemented

- Authentication request: `<saml2p:AuthnRequest …>` with `ID="_<32 hex>"`, `Version="2.0"`, `IssueInstant` (UTC, seconds), `Destination` = asserting party SSO location, `ProtocolBinding` = the registration's assertion consumer binding, `AssertionConsumerServiceURL` = the resolved ACS location, and `<saml2:Issuer>` = the service provider entity id.
- Redirect binding: raw DEFLATE, standard base64, percent-encoded into `?SAMLRequest=`; `&RelayState=` when one is given.
- The endpoint answers 401 for an unknown registration id and for a registration that cannot start a login (no single sign-on location), and leaves unmatched requests to the rest of the chain.

## Tests

9 new, each seen failing first. Whole crate: **202 passed, 0 failed**, no new warnings, no broken intra-doc links.

- Registration: asserting party defaults (no IdP configured, `Redirect` binding), builder overrides.
- Authentication request: XML carries the service provider and the destination; the redirect URL carries the deflated request (decoded and compared to the XML); `RelayState` round-trips; a registration without an SSO location fails with `MissingConfiguration`.
- Filter: redirect to the asserting party, 401 for an unknown registration, passthrough for an unmatched path.

## Rulings

- **Only the HTTP-Redirect binding.** `POST` binding (an auto-submitting form) is not implemented. Cost if wrong: an asserting party configured for POST binding cannot be started against.
- **The IdP key is a PEM public key, not a certificate.** The port has no X.509 parser, so a registration carries `verification_public_key` (SPKI PEM). Cost if wrong: operators paste the certificate body instead of the key and get a parse failure later, once a verifier exists.
- **`Saml2AuthenticationRequest::id()` is exposed but unused so far** — it is what the response's `InResponseTo` will be checked against. Cost if wrong: none; it is a getter.
- **A registration that cannot start a login answers 401**, the same status an unknown registration gets, instead of a distinct 500. Cost if wrong: a misconfiguration looks like an unknown registration to the caller (the reason is logged).

## Next slice (specified, not implemented)

1. ~~**`saml2/login/saml2_response.rs`**~~ **Done** — `Saml2Response::parse` with `quick-xml`: destination, `InResponseTo`, issuer, status code, name id, audience, recipient, `NotBefore`/`NotOnOrAfter`, attribute values, `is_signed`.
2. ~~**`saml2/login/assertion_verifier.rs`**~~ **Done** — `AssertionVerifier` trait plus `RejectingAssertionVerifier`, which refuses every response with `VerificationFailed` and a message telling the operator to supply one.
3. ~~**`saml2/login/saml2_authentication_token.rs`**~~ **Done** — the token the converter produces: registration id, response XML, relay state, unauthenticated, `of()` = its own `TypeId` so the provider can downcast it.
4. ~~**`web/saml2/login/saml2_authentication_converter.rs`**~~ **Done** — reads the buffered form body, takes `SAMLResponse` and `RelayState`, base64-decodes the response, and derives the registration id from the endpoint's last path segment. A form without a response is not an authentication attempt (`Ok(None)`); a response that is not base64 is an `AuthenticationError`.

Still open:

5. ~~**`saml2/login/saml2_authentication.rs`**~~ **Done** — the authenticated result: the name id as principal, the authorities of the mapped user, the response it came from, `is_authenticated()` true.
6. ~~**`saml2/login/saml2_authentication_provider.rs`**~~ **Done** — token → `Saml2Response::parse` → `AssertionVerifier::verify` → `UserDetailsService::load_user_by_username(name_id)` → `Saml2Authentication`; unknown registration, unparsable response, rejected response and unknown user all fail with a message that names the cause.
7. ~~**`web/saml2/login/saml2_authentication_filter.rs`**~~ **Done** — the assertion consumer endpoint over `BaseAuthenticationProcessingFilter`, named `Saml2AuthenticationFilter`, with `Saml2AuthenticationConverter` as its converter and `POST /login/saml2/sso/{registrationId}` as its pattern.
8. ~~**`configurers/saml2_login_configurer.rs`**~~ **Done** — repository and user details service from DSL → shared object → context (both fail fast), optional `assertion_verifier` (defaults to the rejecting one), registers the provider and both endpoints ahead of `BasicAuthenticationFilter`.

## Known gaps in this slice

- ~~**The assertion consumer filter has no success or failure handler configured.**~~ **Fixed.** `configure` installs the chain's `AuthenticationManager` on the filter and a `SimpleUrlAuthenticationFailureHandler("/login?error")`.
- ~~**A believed response cannot be run through the built chain.**~~ **Fixed, and the cause was not where the first guess pointed.** `ProviderAuthenticationManager::authenticate` rebuilds the authentication with `authentication.to_builder().build()` before any provider sees it, and `erase_credentials` does the same to the result. Both SAML types delegated `to_builder()` to `SimpleAuthentication`, so the rebuilt object was a plain authentication: the provider never downcast to the token (`Ok(None)` → "No AuthenticationProvider accepted the authentication"), and the stored authentication lost its SAML type. Each type now has its own builder that round-trips the SAML payload. Pinned by `a_believed_response_authenticates_through_the_chain`, which asserts the redirect, the principal and that the stored authentication's `of()` is `Saml2Authentication`.
- **The provider collapses every failure into `BadCredentials`.** The precise kind lives in `Saml2Error` up to the verifier; `AuthenticationError` has no SAML-specific kind. A caller that needs to distinguish "bad signature" from "unknown user" has to read the message.
- **The authentication request endpoint only supports the HTTP-Redirect binding** (see the ruling above).
