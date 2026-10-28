# Web Authentication, the browser half: implementation plan

Date: 2026-10-21
Status: Tasks 1-4 are done — 10 tests. Task 5's browser walk has not been done:
there is no browser in the environment this was built in, so the page and the
script are verified by their content and by the server-side tests, not by a
person clicking "Sign in with a passkey".

> **For agentic workers:** REQUIRED SUB-SKILL: use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make a passkey usable from the pages this port ships: the default
login page offers "Sign in with a passkey" and the script behind it talks to the
four endpoints that already exist, and a registration page registers a passkey
for the signed-in user.

**Architecture:** The server half is done (`webauthn/`, `web/webauthn/`,
`WebAuthnConfigurer`). What is missing is on the page: the shipped
`/login/webauthn.js` is a placeholder with no `setupLogin`, and no configurer
ever calls `set_passkeys_enabled(true)`, so the passkey form is never rendered.
This plan wires the configurer to the login page filter, hands the script the
values it cannot guess (username parameter, CSRF headers, context path), and
implements the two browser ceremonies against `navigator.credentials`.

**Tech Stack:** Rust (next-web-security, next-web-core), plain browser
JavaScript (no build step, no dependency), `navigator.credentials`.

**Spec:** `next-web-security/docs/plans/2026-10-20-webauthn.md` (Phases 0-5,
done) and `next-web-security/docs/verification-matrix.md`.

## Global Constraints

- Every public Rust item has an English doc comment; no comment references Java
  or Spring.
- No `unwrap()`/`panic!` in runtime code; startup configuration may panic and
  must say what to configure (`webAuthn() requires the identifier of the relying
  party` is the pattern already in place).
- Errors carry a kind and a message; conversions go through `From`.
- Only the `none` attestation format and ES256/RS256 are accepted; the browser
  side must not offer a ceremony the relying party will refuse.
- The script is served as data by `DefaultResourcesFilter::webauthn()`; it has no
  build step, no dependency, and no inline event handlers (the login page's CSP
  is out of scope, but keep it to `addEventListener`).
- The JS that talks to `navigator.credentials` is browser code: assert its
  content from Rust tests, and verify the ceremony by hand in a browser once
  (Task 5).

## Environment

- Builds must not write into the checkout: run them as
  `$env:CARGO_TARGET_DIR="$env:TEMP\dsh-target-nextweb"; cargo test -p next-web-security`.
- `git` cannot write `.git` inside the sandbox; commit with one escalated
  command at the end (`git add <paths>; git commit ...`), and never `git add -A`
  (the branch carries unrelated staged work).

## Review Focus

- The login page served when `formLogin()` is **not** configured: the passkey
  form must not appear for a page whose JavaScript is never loaded.
- A custom login page (`formLogin(|f| f.login_page("/auth"))`): the passkey
  script must not be injected into a page the application owns.
- CSRF enabled: the two POSTs must carry the token the page was rendered with,
  and a page rendered without CSRF headers must not send a stale header.
- A username the relying party has no credential for: the options call must
  still answer, and `navigator.credentials.get` failing must not break the form
  login that shares the page.
- A browser without `PublicKeyCredential` (or on a non-secure origin): the
  passkey form must hide itself rather than throw.

---

### Task 1: Enable the passkey form on the default login page

**Files:**
- Modify: `next-web-security/src/spring_security/config/web/configurers/web_authn_configurer.rs`
- Test: the same file, `mod tests`

**Interfaces:**
- Consumes: `DefaultLoginPageGeneratingFilter::set_passkeys_enabled(bool)` and
  `HttpSecurityBuilder::shared_object_mut::<DefaultLoginPageGeneratingFilter>()`
  (the mechanism `FormLoginConfigurer::init` already uses).
- Produces: nothing later tasks call; the login page starts rendering
  `PASSKEY_FORM_TEMPLATE` and `PASSKEY_SCRIPT_TEMPLATE`.

- [ ] **Step 1: Write the failing test**

In `web_authn_configurer.rs::tests`, add:

```rust
#[test]
fn the_configuration_enables_the_passkey_form_of_the_login_page() {
    let mut http = http_security();
    http.shared_object::<DefaultLoginPageGeneratingFilter>(); // see Step 3
    let mut configurer = configurer();
    configurer.init(&mut http);

    let page = http
        .shared_object::<DefaultLoginPageGeneratingFilter>()
        .expect("the login page is a shared object");
    assert!(page.passkeys_enabled(), "the form is rendered when webAuthn() is configured");
}
```

The exact accessor is added in Step 3; the test fails first with "no method named
`passkeys_enabled`" — that is the RED.

- [ ] **Step 2: Run it and watch it fail**

Run: `$env:CARGO_TARGET_DIR="$env:TEMP\dsh-target-nextweb"; cargo test -p next-web-security web_authn_configurer`
Expected: FAIL — `no method named passkeys_enabled`.

- [ ] **Step 3: Implement**

In `default_login_page_generating_filter.rs`, add
`pub fn passkeys_enabled(&self) -> bool { self.passkeys_enabled }` next to the
existing setter (documented). In `WebAuthnConfigurer::init`, after the manager is
built:

```rust
if let Some(login_page) = http.shared_object_mut::<DefaultLoginPageGeneratingFilter>() {
    login_page.set_passkeys_enabled(true);
}
```

- [ ] **Step 4: Run the test and the whole crate**

Run: `cargo test -p next-web-security`
Expected: PASS; 372 passed or more, 0 failed.

- [ ] **Step 5: Commit** (see Environment: one escalated command)

---

### Task 2: Hand the script the values it cannot guess

**Files:**
- Modify: `next-web-security/src/spring_security/web/authentication/ui/default_login_page_generating_filter.rs`
- Test: the same file, `mod tests`

**Interfaces:**
- Consumes: `PASSKEY_SCRIPT_TEMPLATE` (already rendered with `csrfHeaders`,
  `contextPath`; it also carries `loginPageUrl`, which the script does not use).
- Produces: the rendered page passes `usernameParameter` and the registration
  URL, so the script reads:

```js
setupLogin(csrfHeaders, contextPath, usernameParameter, registerUrl, button)
```

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn the_passkey_script_is_rendered_with_the_username_parameter() {
    let mut filter = DefaultLoginPageGeneratingFilter::default();
    filter.set_passkeys_enabled(true);
    filter.set_username_parameter("user");

    let html = filter.render_passkey_script_for_tests(); // Step 3

    assert!(
        html.contains(r#"setupLogin("#) && html.contains("user"),
        "the script is told which field holds the user name, was: {html}"
    );
}
```

- [ ] **Step 2: Run it and watch it fail**

Run: `cargo test -p next-web-security default_login_page`
Expected: FAIL — no method `render_passkey_script_for_tests`.

- [ ] **Step 3: Implement**

Make `render_java_script`'s passkey branch fill `usernameParameter` (the filter's
`username_parameter`) and `registerUrl` (`format!("{context_path}/webauthn/register")`)
in `PASSKEY_SCRIPT_TEMPLATE`, and change the template's call to pass them
through. Expose the branch to tests as
`pub(crate) fn render_passkey_script_for_tests(&self) -> String` over
`render_java_script(&Default::default(), "")`.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p next-web-security default_login_page`
Expected: PASS.

- [ ] **Step 5: Commit**

---

### Task 3: Implement the login ceremony in the shipped script

**Files:**
- Modify: `next-web-security/src/spring_security/web/authentication/ui/resources/spring-security-webauthn.js`
- Modify: `next-web-security/src/spring_security/web/authentication/ui/default_resources_filter.rs` (only if the served content type or path needs it — expected: no change)
- Test: `next-web-security/src/spring_security/web/authentication/ui/default_resources_filter.rs`, `mod tests`

**Interfaces:**
- Consumes: the four endpoints of
  `next-web-security/src/spring_security/config/web/configurers/web_authn_configurer.rs`,
  and the option field names of `webauthn/options.rs` (base64url strings for
  `challenge`, `allowCredentials[].id`, `user.id`).
- Produces: `window.setupLogin(csrfHeaders, contextPath, usernameParameter, registerUrl, button)`.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn the_served_script_runs_the_login_ceremony() {
    let script = crate::web::authentication::ui::DEFAULT_WEBAUTHN_JS;

    assert!(script.contains("/webauthn/authenticate/options"), "asks for the options");
    assert!(script.contains("/webauthn/authenticate\""), "answers them");
    assert!(script.contains("navigator.credentials.get"), "asks the authenticator");
    assert!(script.contains("setupLogin"), "the login page calls this");
    assert!(script.contains("PublicKeyCredential"), "hides itself where passkeys are absent");
}
```

- [ ] **Step 2: Run it and watch it fail**

Run: `cargo test -p next-web-security the_served_script`
Expected: FAIL — the placeholder contains none of these strings.

- [ ] **Step 3: Implement the script**

Replace the placeholder with a script that, per ceremony:

1. hides `button` when `window.PublicKeyCredential` is undefined;
2. reads the user name from the input named `usernameParameter`;
3. `POST ${contextPath}/webauthn/authenticate/options` with
   `{"username": name}` and the CSRF headers;
4. decodes `challenge` and `allowCredentials[].id` from base64url to
   `Uint8Array`, calls `navigator.credentials.get({publicKey})`;
5. encodes `rawId`, `clientDataJSON`, `authenticatorData`, `signature` and
   (`userHandle` when present) back to base64url, `POST`s
   `${contextPath}/webauthn/authenticate`, and follows the redirect
   (`window.location.assign(contextPath + "/")`) on 2xx;
6. on any failure, restores the form and appends the message the server wrote in
   `{"error": ...}` — never throws out of the handler.

Base64url helpers in ~10 lines each (`btoa`/`atob` with `-`/`_` translation and
padding removal); no dependency, no build step.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p next-web-security the_served_script`
Expected: PASS.

- [ ] **Step 5: Commit**

---

### Task 4: Serve a registration page and register a passkey from it

**Files:**
- Create: `next-web-security/src/spring_security/web/authentication/ui/default_webauthn_registration_page_generating_filter.rs`
- Modify: `next-web-security/src/spring_security/web/authentication/ui/mod.rs`,
  `next-web-security/src/spring_security/config/web/configurers/web_authn_configurer.rs`
- Modify: `.../resources/spring-security-webauthn.js` (add `setupRegistration`)
- Test: the new filter's file, `mod tests`; the configurer's `mod tests`

**Interfaces:**
- Consumes: `WebAuthnRegistrationFilter` (POST endpoints of Task 3's sibling),
  the `DefaultResourcesFilter::webauthn()` resource, and the manager the
  configurer already holds.
- Produces: `DefaultWebAuthnRegistrationPageGeneratingFilter` answering
  `GET /webauthn/register` with a page that calls
  `setupRegistration(csrfHeaders, contextPath, button)`; the JSON ceremony stays
  on `POST /webauthn/register`.

- [ ] **Step 1: Write the failing tests**

```rust
#[tokio::test]
async fn the_registration_page_is_served_on_a_get() {
    let filter = DefaultWebAuthnRegistrationPageGeneratingFilter::default();
    let mut request = get("https://sp.example.org/webauthn/register");
    let mut response = axum::response::Response::default();

    filter.do_filter(&mut request, &mut response, &ContinuingChain).await.unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(header(&response, "content-type"), Some("text/html;charset=UTF-8"));
}

#[tokio::test]
async fn a_post_of_the_registration_is_left_to_the_ceremony() {
    // the page filter matches GET only; a POST continues the chain
}
```

and in the configurer:

```rust
#[test]
fn the_configuration_serves_the_registration_page() {
    // build the chain; one of the filters is named "DefaultWebAuthnRegistrationPageGeneratingFilter"
}
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo test -p next-web-security registration_page`
Expected: FAIL — the type does not exist.

- [ ] **Step 3: Implement the page**

A filter over `PathPatternRequestMatcher::path_pattern(None, "/webauthn/register")`
that matches `GET` only, renders a page with one button
(`id="passkey-register"`), the CSRF headers the page was rendered with (the same
way `DefaultLoginPageGeneratingFilter` renders them), and the script tag for
`/login/webauthn.js`. Register it in `WebAuthnConfigurer::configure` unless
`disable_default_registration_page` is set — that flag currently suppresses only
the resource; make it suppress the page as its documentation says. Add
`setupRegistration` to the script: options → `navigator.credentials.create` →
`decode base64url of `clientDataJSON` and `attestationObject` → POST
`${contextPath}/webauthn/register` → report success by redirecting to `/`.

- [ ] **Step 4: Run the whole crate**

Run: `cargo test -p next-web-security`
Expected: PASS.

- [ ] **Step 5: Commit**

---

### Task 5: Verify by hand, and record it

**Files:**
- Modify: `next-web-security/docs/verification-matrix.md`,
  `next-web-security/docs/plans/2026-10-20-webauthn.md`

- [ ] **Step 1: Run the crate once more, and write down the numbers**

Run: `cargo test -p next-web-security`, `--features oauth2-client`, and the three
oauth2 features. Update the "Last verified with" block.

- [ ] **Step 2: Walk the ceremony in a browser**

Serve an application built on `web_authn(...)` and, in a browser: sign in with a
form, open `/webauthn/register`, register a passkey, sign out, then sign in with
it. Record in the matrix what was seen and what was not (a virtual authenticator
is not a physical one; which browser and version).

- [ ] **Step 3: Update the documents**

The matrix's web authn row gains the browser evidence (or says plainly that the
browser walk is what is missing). The plan's Phase 6 (this plan) is marked done,
including the three decisions to revisit: no discoverable-credential ceremony,
no session binding of the challenge, and the JSON endpoints not exempted from
CSRF (the page sends the header instead).

- [ ] **Step 4: Commit**

---

## What this plan deliberately leaves out

Each is a separate decision, not a task here:

1. **Discoverable credentials** (empty user name → `allowCredentials` absent).
   Needs `WebAuthnManager::create_discoverable_authentication_options` and a
   filter branch.
2. **`packed` / `tpm` attestation**: needs X.509 certificate-chain verification
   and a new dependency; the plan of record refuses them by name until then.
3. **Real browser or FIDO conformance vectors** committed as data: the crate's
   arithmetic is anchored by RFC 7515, the ceremony plumbing by a test-held key.
   A browser-produced attestation would test the parsers against real encodings.
4. **The deferred minors of the review**: `update_sign_count`'s ignored bool,
   credential-id length bounds, the RSA modulus floor, the STANDARD base64
   fallback in `parse_base64_url`, the lock-poison `expect`s and the blocking
   `RwLock` in async filters, and body-size caps.
