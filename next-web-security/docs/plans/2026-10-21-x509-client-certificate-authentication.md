# X.509 client certificate authentication: verifying it

Date: 2026-10-21
Status: done — 7 tests of the configuration and the name of a pre-authenticated
token, 5 of the bridge that turns the certificate of the connection into the
attribute the filter reads, and 2 end-to-end tests over TLS
(`tests/x509_over_tls.rs`): a TLS listener that requires a client certificate, a
client that presents the one the fixtures carry, and the chain behind it. The
defect the chain test found is in `docs/verification-matrix.md`.

## The certificate of the connection

A certificate is not part of an HTTP request, so the port needs a step between
the transport and the filter that authenticates:

- `next_web_core::http::PeerCertificate` is what the transport inserts into the
  request extensions once it has validated the certificate; `HttpRequest::
  peer_certificate` reads it. A request of any other transport has none.
- `ClientCertificateAttributeFilter`, installed by `x509()` ahead of the filter
  that authenticates, writes the subject of that certificate into the attribute
  the X.509 filter already reads. It parses the subject out of the DER encoding —
  RFC 4514 order, escaped values — and uses the subject a terminator rendered
  when the transport carries one.

The TLS server of `next-web` is the transport that inserts it: `TlsWebServer`
terminates the connection with rustls, reads the certificate the handshake
settled on, and inserts it into every request it serves, so an application that
serves over TLS and configures `x509()` needs nothing else.
`next-web/tests/tls_client_certificate.rs` holds that path.

## Where it stands

`HttpSecurity::x509` is marked `Unverified`. Everything it installs exists and
has tests of its own:

- `X509AuthenticationFilter` reads the client certificate from the request
  attribute the container (or a TLS terminator) writes, and falls back to a
  subject DN string; 12 tests, including the custom extractor, the custom
  attribute, already-authenticated, principal change, failure handling and the
  details source.
- `SubjectDnX509PrincipalExtractor` (deprecated) and
  `SubjectX500PrincipalExtractor` extract the principal from the subject DN.
- `PreAuthenticatedAuthenticationProvider` and
  `UserDetailsByNameServiceWrapper` are the provider and the bridge the
  configurer wires.
- `FilterOrderRegistration` already gives `X509AuthenticationFilter` a place in
  the chain, so `add_filter` does not panic.

What has no test behind it is the configuration: nothing asserts that `x509(…)`
puts the filter in the chain, registers the provider, installs the
`Http403ForbiddenEntryPoint`, and lets a request with a certificate
authenticate end to end. A status without a test behind it is an opinion, which
is why it says `Unverified`.

## What "verified" means here, and what it does not

The certificate's signature is **not** checked by this port and cannot be: the
container validates it when the client connects, and the framework only reads
what the container put in the request attribute. The chain test therefore
asserts the framework's half — extract, look up, grant `FACTOR_X509`, store the
authentication — and the documents must keep saying so.

## Tasks

### Task 1: the configuration installs the filter, the provider and the entry point

**Files:** `config/web/configurers/x509_configurer.rs` (test module, new).

Steps: write a test that builds an `HttpSecurity`, applies `x509(|x| x.user_details_service(…))`,
builds the chain, and asserts the filter names contain `X509AuthenticationFilter`;
assert `http.shared_object::<Arc<dyn AuthenticationEntryPoint>>()` is an
`Http403ForbiddenEntryPoint`. Run it, watch it pass or fix what it reveals.
A second test asserts a missing `UserDetailsService` panics with the message the
configurer already carries.

### Task 2: a request with a certificate authenticates through the chain

**Files:** the same test module; the `web/test_support.rs` runner.

Steps: write the chain test — `Chain::new(chain.get_filters().to_vec())`, a POST
to a protected path carrying `DEFAULT_CLIENT_CERTIFICATE_ATTRIBUTE` =
`"CN=Alice,O=Example"`, a user "Alice" in an `InMemoryUserDetailsManager`; assert
the stored authentication's principal is `Alice` and that its authorities contain
`FACTOR_X509`. Then the negative: the same request without the attribute
continues the chain and stores no authentication. Run, fix, keep green.

### Task 3: the documents stop saying `Unverified`

**Files:** `docs/verification-matrix.md`, `config/web/builders/http_security.rs`,
this file.

Steps: move the `x509` row into "Verified end to end" naming the chain test and
the file; change the `HttpSecurity::x509` marker to `Ok` and say in one sentence
that the certificate's own validation is the container's; record here what was
and was not proven. Fix the unresolved rustdoc link to
`SubjectDnX509PrincipalExtractor` (the type is deprecated; link
`SubjectX500PrincipalExtractor` instead) that `cargo doc` reports.

## Review Focus

- A request through a **TLS terminator** that writes the certificate in another
  form (PEM, or a header): the filter reads one attribute and one string form, so
  the application must put the right one there — worth a doc line.
- A certificate whose subject has **no CN**: the default expression fails the
  authentication with `BadCredentials` rather than falling back to the whole DN.
- **Already authenticated** in the same request (an earlier filter): the filter
  leaves the existing authentication alone, and a changed principal is only
  re-authenticated when `check_for_principal_changes(true)`.
- A user the certificate names but the application does not have: the provider
  refuses the authentication, and with the default entry point the request is
  403 rather than 401 (there is nothing a client certificate can retry).
- The chain continuing after a **failed** authentication is configurable; the
  default must not continue as an authenticated request.
