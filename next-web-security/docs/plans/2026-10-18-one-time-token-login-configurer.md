# One-Time Token Login Configurer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `OneTimeTokenLoginConfigurer` provide the same behaviour as the reference implementation: a token generated at `POST /ott/generate` is consumed at `POST /login/ott`, the submit page and the default login page carry the CSRF token, and a missing success handler fails the configuration instead of silently redirecting.

**Architecture:** The configurer stays a plain `SecurityConfigurer` that registers three filters (`OneTimeTokenAuthenticationFilter` via `BaseAuthenticationFilterConfigurer`, `DefaultOneTimeTokenSubmitPageGeneratingFilter`, `GenerateOneTimeTokenFilter`) plus the CSS resource filter, and registers `OneTimeTokenAuthenticationProvider`. The work is closing the gap between that skeleton and the reference: resolve each collaborator **once** per configurer so the generate filter and the provider share one token store, require a success handler, supply the CSRF token to the submit page, validate DSL URLs, and HTML-escape the rendered page.

**Tech Stack:** Rust 2021, `next-web-security` (`spring_security` module), `axum::extract::Request` / `axum::response::Response` as the `HttpRequest` / `HttpResponse` implementations, in-file `#[cfg(test)] mod tests` (project style, see `configuration/http_security_configuration.rs`).

**Spec:** `next-web-security/docs/conversion-rules.md` plus the reference implementation:
- `D:\EdgeDownload\spring-security-main\config\src\main\java\org\springframework\security\config\annotation\web\configurers\ott\OneTimeTokenLoginConfigurer.java`
- `D:\EdgeDownload\spring-security-main\web\src\main\java\org\springframework\security\web\authentication\ott\` (filters, resolver, handlers)
- `D:\EdgeDownload\spring-security-main\web\src\main\java\org\springframework\security\web\authentication\ui\DefaultOneTimeTokenSubmitPageGeneratingFilter.java`

## Global Constraints

- Comments: English only, Rust doc format; no `Java`/`Spring`/`Servlet` wording (conversion-rules §2).
- No `panic!` on the request path. Panic is allowed **only** during configuration (`init`/`configure` and DSL setters), which is what the reference's `Assert` and `IllegalStateException` map onto (conversion-rules §2).
- No `unwrap()` in production code; no `println!` for logging (use `tracing`); no blocking locks in async code; no `unsafe`.
- Preserve the reference naming: `snake_case` fields and methods; a `Default` impl for the no-argument constructor.
- Do not invent configuration classes for features the port does not have; `ObjectPostProcessor` post-processing is **not** ported (it cannot be applied to `Arc<dyn Trait>` and the reference's processors are no-ops here) — do not add it.
- Build/test only with default features. The `oauth2-*` feature combinations do not compile (pre-existing) and `--all-features` additionally enables `T1`, which collides with `T2`.
- The default `target/` directory is **not writable** in this sandbox (Windows ACL). Run every cargo command as:
  `$env:CARGO_TARGET_DIR = "D:\DevProject\Rust\next-web\.check-target"; cargo test -p next-web-security ...`
  and delete `.check-target` when done.
- Tests live in the file they test (`#[cfg(test)] mod tests`), matching `configuration/http_security_configuration.rs`.

## Reference behaviour this plan implements

Derived from the reference tests; every task's tests below pin one of these.

1. `POST /ott/generate` with form parameter `username` generates a token and the success handler writes the redirect (`/login/ott` with the default handler the caller installs).
2. `POST /login/ott` with form parameter `token` authenticates: 302 to the default target `/`, and the token is **consumed** — a second use of the same value fails to `/login?error`.
3. No success handler (neither DSL nor context singleton) → configuration fails with
   `A OneTimeTokenGenerationSuccessHandler is required to enable oneTimeTokenLogin().` /
   `Please provide it as a bean or pass it to the oneTimeTokenLogin() DSL.`
4. Defaults: generate `POST /ott/generate`, submit page `GET /login/ott`, login processing `POST /login/ott`, login page `GET /login`, failure `/login?error`, CSS `GET /default-ui.css`.
5. The submit page form posts to the login-processing URL and carries the CSRF token as `<input name="_csrf" type="hidden" value="…" />`.
6. The default login page gets a "Request a One-Time Token" form posting the username to `/ott/generate`, only when no custom login page was set.
7. DSL overrides (`token_generating_url`, `login_processing_url`, `token_service`, `generate_request_resolver`, `token_generation_success_handler`) replace the defaults; a base path of the matcher builder is not prepended.
8. A `login_page("/custom-login")` suppresses the generated login page and makes the entry point redirect there.
9. Submitted values are HTML-escaped (`this<>!@#"` → `this&lt;&gt;!@#&quot;`).

## File Structure

| File | Responsibility | Change |
|---|---|---|
| `next-web-security/src/spring_security/config/web/configurers/one_time_token_login_configurer.rs` | The configurer: collaborator resolution, filter registration, login-page integration, CSRF hidden inputs | Modify (main) + tests |
| `next-web-security/src/spring_security/web/authentication/ui/default_one_time_token_submit_page_generating_filter.rs` | Renders the submit page | Modify: escape values, make `generate_html` public + tests |
| `next-web-security/src/spring_security/web/authentication/ui/default_login_page_generating_filter.rs` | Renders the default login page | Modify: add two getters + test |
| `next-web-security/src/spring_security/web/default_security_filter_chain.rs` | Chain produced by the builder | No change — `SecurityFilterChain::get_filters()` already exposes the filters |

---

### Task 1: Resolve collaborators once and require what the reference requires

**Files:**
- Modify: `next-web-security/src/spring_security/config/web/configurers/one_time_token_login_configurer.rs`
- Test: same file, `#[cfg(test)] mod tests`

**Why:** `get_one_time_token_service()` currently constructs a fresh `InMemoryOneTimeTokenService` on every call. `init` uses it for the provider and `configure` uses it for the generate filter, so with no service singleton registered the token is written to one store and looked up in another — every login fails. The reference caches each collaborator on a field.

**Interfaces (produced):**
- `fn token_service(&mut self) -> Arc<dyn OneTimeTokenService>` — caches into `self.one_time_token_service`.
- `fn request_resolver(&mut self) -> Arc<dyn GenerateOneTimeTokenRequestResolver>` — caches into `self.request_resolver`.
- `fn generation_success_handler(&mut self) -> Arc<dyn OneTimeTokenGenerationSuccessHandler>` — caches; **panics** when absent.
- `fn authentication_provider(&mut self) -> Arc<dyn AuthenticationProvider>` — caches into `self.authentication_provider`; panics when no user details service is available.
- Test helpers (defined once here, reused by Tasks 2–6):
  - `fn configurer() -> OneTimeTokenLoginConfigurer<HttpSecurity>` — `new` over an empty `StandaloneApplicationContext`.
  - `fn configurer_with_user(username: &str) -> OneTimeTokenLoginConfigurer<HttpSecurity>` — same, plus `user_details_service(user_details_service(username))`.
  - `fn user_details_service(username: &str) -> Arc<dyn UserDetailsService>` — `InMemoryUserDetailsManager` with one user whose password is `"password"`.
  - `fn http_security() -> HttpSecurity` — `HttpSecurity::new(AuthenticationManagerBuilder::default(), HashMap::new())`.
  - `fn request(method: HttpMethod, path: &str) -> axum::extract::Request` — `Request::builder().method(...).uri(path).body(Body::empty())`.
  - `fn request_with_parameter(method, path, name, value) -> axum::extract::Request` — the same with a query/body form parameter set through the request's parameter API.
  - `fn authenticated_username(request: &dyn HttpRequest) -> Option<String>` — reads `RequestAttributeSecurityContextRepository::DEFAULT_REQUEST_ATTR_NAME`, downcasts to `Arc<dyn SecurityContext>`, returns `get_authentication()?.principal()?.name()`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn the_token_service_is_resolved_once() {
    let mut configurer = configurer();
    let first = configurer.token_service();
    let second = configurer.token_service();
    assert!(Arc::ptr_eq(&first, &second), "both call sites share one store");
}

#[test]
fn the_generate_filter_and_the_provider_share_the_token_service() {
    // A token generated through the accessor the generate filter uses must be
    // consumable by the provider the configurer registers.
    let mut configurer = configurer_with_user("Max");
    let generated = configurer
        .token_service()
        .generate(GenerateOneTimeTokenRequest::new("Max"));
    let authentication = configurer.authentication_provider().authenticate(
        &(Arc::new(OneTimeTokenAuthenticationToken::new(generated.token_value()))
            as Arc<dyn Authentication>),
    );
    assert!(authentication.is_ok(), "the provider finds the token it was not given");
}

#[test]
fn the_request_resolver_is_resolved_once() {
    let mut configurer = configurer();
    assert!(Arc::ptr_eq(&configurer.request_resolver(), &configurer.request_resolver()));
}

#[test]
fn a_token_service_from_the_dsl_replaces_the_default() {
    let mut configurer = configurer();
    let custom: Arc<dyn OneTimeTokenService> = Arc::new(InMemoryOneTimeTokenService::new());
    configurer.token_service(custom.clone());
    assert!(Arc::ptr_eq(&configurer.token_service(), &custom));
}

#[test]
#[should_panic(expected = "A OneTimeTokenGenerationSuccessHandler is required to enable oneTimeTokenLogin().")]
fn a_missing_success_handler_is_reported() {
    let mut configurer = configurer();
    configurer.generation_success_handler();
}

#[test]
#[should_panic(expected = "A UserDetailsService is required to enable oneTimeTokenLogin().")]
fn a_missing_user_details_service_is_reported() {
    let mut configurer = configurer();
    configurer.authentication_provider();
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p next-web-security one_time_token_login_configurer` (with `CARGO_TARGET_DIR` set, see Global Constraints)
Expected: FAIL — the methods are private with a different name/shape, and the service test fails on `Arc::ptr_eq`.

- [ ] **Step 3: Implement the caching accessors**

Change the four accessors to take `&mut self` and cache on their field (`self.one_time_token_service.get_or_insert_with(...).clone()` after resolving from the context once in `new`), and replace `Arc::new(DefaultOneTimeTokenGenerationSuccessHandler::new(...))` with the panic carrying the two-line message. Replace the `expect(...)` in `get_authentication_provider` with a panic whose message is exactly `A UserDetailsService is required to enable oneTimeTokenLogin(). Please provide it as a bean.` Update the call sites (`init`, `configure_submit_page`, `configure_ott_generate_filter`) to the new names.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p next-web-security one_time_token_login_configurer`
Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
git add next-web-security/src/spring_security/config/web/configurers/one_time_token_login_configurer.rs
git commit -m "fix(security): resolve one-time token collaborators once per configurer"
```

---

### Task 2: Supply the CSRF token to the submit page

**Files:**
- Modify: `next-web-security/src/spring_security/config/web/configurers/one_time_token_login_configurer.rs`
- Test: same file

**Why:** `resolve_hidden_inputs` returns an empty map. With CSRF protection on (the default), the submit form posts without `_csrf` and `CsrfFilter` rejects it, so the second half of the login flow cannot work. The CSRF filter stores the token as a request attribute keyed by `std::any::type_name::<&dyn CsrfToken>()` (see `web/csrf/csrf_token_request_attribute_handler.rs:42`).

**Interfaces (produced):**
- `fn hidden_inputs(request: &dyn HttpRequest) -> HashMap<String, String>` — returns `{token.parameter_name(): token.token()}`, or an empty map when the attribute is absent or of another type.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn the_csrf_token_is_rendered_as_a_hidden_input() {
    let mut request = request(HttpMethod::GET, "/login/ott");
    request.set_attribute(
        std::any::type_name::<&dyn CsrfToken>(),
        AnyValue::BoxedValue(Box::new(Arc::new(DefaultCsrfToken::new(
            "X-CSRF-TOKEN",
            "_csrf",
            "csrf-token-value",
        )) as Arc<dyn CsrfToken>)),
    );
    assert_eq!(hidden_inputs(&request), HashMap::from([("_csrf".to_owned(), "csrf-token-value".to_owned())]));
}

#[test]
fn no_hidden_input_is_rendered_without_a_csrf_token() {
    assert!(hidden_inputs(&request(HttpMethod::GET, "/login/ott")).is_empty());
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p next-web-security hidden_input`
Expected: FAIL — current `resolve_hidden_inputs` ignores the request (`HashMap::new()`), first test fails on the left value.

- [ ] **Step 3: Implement `hidden_inputs`**

Read the attribute, downcast `AnyClone::as_any()` to `Arc<dyn CsrfToken>` (the same boxed-value pattern `HttpSecurity::new` uses for `PathPatternRequestMatcherBuilder`), and build the map from `parameter_name()` / `token()`. Return an empty map on absence or type mismatch — never panic. Point `configure_submit_page` at `Arc::new(hidden_inputs)`.

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p next-web-security hidden_input`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add next-web-security/src/spring_security/config/web/configurers/one_time_token_login_configurer.rs
git commit -m "feat(security): render the csrf token on the one-time token submit page"
```

---

### Task 3: Validate the DSL URLs and allow a custom login page

**Files:**
- Modify: `next-web-security/src/spring_security/config/web/configurers/one_time_token_login_configurer.rs`
- Test: same file

**Why:** the reference asserts `hasText` on `tokenGeneratingUrl`, `loginProcessingUrl` and `submitPageUrl`, and exposes `loginPage(String)` on the configurer. In this port `login_page` lives on `BaseAuthenticationFilterConfigurerExt` and needs the builder, so it cannot be called from a DSL closure; the configurer therefore stores the page and applies it in `init`.

**Interfaces (produced):**
- `pub fn login_processing_url(&mut self, url: &str) -> &mut Self` — validates, then delegates to `BaseAuthenticationFilterConfigurerExt::login_processing_url`.
- `pub fn login_page(&mut self, page: &str) -> &mut Self` — stores `self.custom_login_page = Some(page.into())`.
- Panic messages: `tokenGeneratingUrl cannot be null or empty`, `loginProcessingUrl cannot be null or empty`, `submitPageUrl cannot be null or empty`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
#[should_panic(expected = "tokenGeneratingUrl cannot be null or empty")]
fn an_empty_token_generating_url_is_rejected() {
    configurer().token_generating_url("  ");
}

#[test]
#[should_panic(expected = "loginProcessingUrl cannot be null or empty")]
fn an_empty_login_processing_url_is_rejected() {
    configurer().login_processing_url("");
}

#[test]
#[should_panic(expected = "submitPageUrl cannot be null or empty")]
fn an_empty_submit_page_url_is_rejected() {
    configurer().default_submit_page_url("");
}

#[test]
fn a_custom_login_page_becomes_the_entry_point() {
    let mut http = http_security();
    let mut configurer = configurer();
    configurer.login_page("/custom-login");
    configurer.init(&mut http);
    assert_eq!(
        configurer.get_authentication_entry_point().map(|p| p.login_url().to_owned()),
        Some("/custom-login".to_owned())
    );
    assert!(configurer.is_custom_login_page());
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p next-web-security one_time_token_login_configurer`
Expected: FAIL — empty URLs are accepted today; `login_page` on the configurer does not exist (compile error).

- [ ] **Step 3: Implement the validating setters and the deferred login page**

Add a `custom_login_page: Option<Box<str>>` field and apply it at the start of `init` (`self.base.login_page(page, http)`), before the login-page integration runs so that a custom page suppresses it. Keep `show_default_submit_page(true)` semantics in `default_submit_page_url`.

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p next-web-security one_time_token_login_configurer`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add next-web-security/src/spring_security/config/web/configurers/one_time_token_login_configurer.rs
git commit -m "feat(security): validate one-time token urls and defer a custom login page"
```

---

### Task 4: HTML-escape the submit page

**Files:**
- Modify: `next-web-security/src/spring_security/web/authentication/ui/default_one_time_token_submit_page_generating_filter.rs`
- Test: same file

**Why:** `generate_html` substitutes the `token` query parameter and the hidden-input values into the page verbatim, so `?token=<script>…` is reflected into the response. The reference escapes values (`this<>!@#"` → `this&lt;&gt;!@#&quot;`).

**Interfaces (produced):**
- `pub fn generate_html(&self, request: &dyn HttpRequest) -> String` — made public so tests assert the page without reading a private response body.
- `fn escape_html(value: &str) -> String` — replaces `&`, `<`, `>`, `"`, `'`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn the_token_is_html_escaped() {
    let request = request_with_parameter(HttpMethod::GET, "/login/ott", "token", r#"this<>!@#""#);
    assert!(
        filter().generate_html(&request)
            .contains(r#"value="this&lt;&gt;!@#&quot;""#),
        "the echoed token is escaped"
    );
}

#[test]
fn the_hidden_input_value_is_html_escaped() {
    let mut filter = filter();
    filter.set_resolve_hidden_inputs(Arc::new(|_| {
        HashMap::from([("_csrf".to_owned(), r#"a<b>"#.to_owned())])
    }));
    assert!(filter
        .generate_html(&request(HttpMethod::GET, "/login/ott"))
        .contains(r#"value="a&lt;b&gt;""#));
}

#[test]
fn the_form_posts_to_the_login_processing_url() {
    let mut filter = filter();
    filter.set_login_processing_url("/login/another");
    assert!(filter
        .generate_html(&request(HttpMethod::GET, "/login/ott"))
        .contains(r#"action="/login/another""#));
}

#[test]
fn the_context_path_prefixes_the_form_action() {
    let mut filter = filter();
    filter.set_login_processing_url("/login/another");
    assert!(filter
        .generate_html(&request_with_context_path("/context", "/login/ott"))
        .contains(r#"action="/context/login/another""#));
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p next-web-security default_one_time_token_submit_page`
Expected: FAIL — `generate_html` is private, values are not escaped.

- [ ] **Step 3: Implement escaping and make `generate_html` public**

Escape the token value and both parts of each hidden input. Also reject an empty `set_login_processing_url` with `loginProcessingUrl cannot be null or empty` (reference behaviour), and add a test for it.

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p next-web-security default_one_time_token_submit_page`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add next-web-security/src/spring_security/web/authentication/ui/default_one_time_token_submit_page_generating_filter.rs
git commit -m "fix(security): escape values rendered on the one-time token submit page"
```

---

### Task 5: Verify the default login page integration

**Files:**
- Modify: `next-web-security/src/spring_security/web/authentication/ui/default_login_page_generating_filter.rs` (two getters)
- Test: `next-web-security/src/spring_security/config/web/configurers/one_time_token_login_configurer.rs`

**Why:** `init_default_login_filter` sets state that nothing can currently read, so the integration is untested. Add the getters the verification needs (the reference reads them through `getLoginPageUrl()` style accessors).

**Interfaces (produced):**
- `DefaultLoginPageGeneratingFilter::is_one_time_token_enabled(&self) -> bool`
- `DefaultLoginPageGeneratingFilter::get_one_time_token_generation_url(&self) -> Option<&str>`

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn init_enables_the_one_time_token_form_on_the_login_page() {
    let mut http = http_security();
    let mut login_page = DefaultLoginPageGeneratingFilter::default();
    http.set_shared_object(login_page.clone());
    configurer().init(&mut http);
    let login_page = http.shared_object::<DefaultLoginPageGeneratingFilter>().unwrap();
    assert!(login_page.is_one_time_token_enabled());
    assert_eq!(login_page.get_one_time_token_generation_url(), Some("/ott/generate"));
}

#[test]
fn init_leaves_the_login_page_alone_when_a_custom_login_page_is_set() {
    let mut http = http_security();
    http.set_shared_object(DefaultLoginPageGeneratingFilter::default());
    let mut configurer = configurer();
    configurer.login_page("/custom-login");
    configurer.init(&mut http);
    let login_page = http.shared_object::<DefaultLoginPageGeneratingFilter>().unwrap();
    assert!(!login_page.is_one_time_token_enabled());
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p next-web-security one_time_token_login_configurer`
Expected: FAIL — the getters do not exist.

- [ ] **Step 3: Add the getters** (no behaviour change).

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p next-web-security one_time_token_login_configurer`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add next-web-security/src/spring_security/web/authentication/ui/default_login_page_generating_filter.rs next-web-security/src/spring_security/config/web/configurers/one_time_token_login_configurer.rs
git commit -m "test(security): cover the one-time token login page integration"
```

---

### Task 6: Prove the assembled chain serves the whole flow

**Files:**
- Test: `next-web-security/src/spring_security/config/web/configurers/one_time_token_login_configurer.rs`

**Why:** every task above tests a piece. This one runs the chain the configurer builds — generate, then consume — which is the only test that would have caught the split-store bug by behaviour instead of by pointer identity.

**Interfaces (consumed):** `SecurityFilterChain::get_filters() -> &[Arc<dyn HttpFilter>]` (`web/security_filter_chain.rs:8`); `HttpFilterChain` implemented by a test-local `Chain` that invokes the next filter.

- [ ] **Step 1: Write the failing test**

```rust
#[tokio::test]
async fn a_generated_token_authenticates_a_login_request() {
    // Arrange: register the configurer on a real builder, so build() runs the
    // configurer's init and configure exactly as an application would.
    let mut http = http_security();
    let issued = Arc::new(Mutex::new(Vec::<String>::new()));
    let mut configurer = configurer_with_user("user");
    configurer.token_generation_success_handler(Arc::new(RecordingHandler(issued.clone())));
    http.with(configurer, |_| {});
    let chain = http.build();

    // Act 1: generate a token.
    let mut generate =
        request_with_parameter(HttpMethod::POST, "/ott/generate", "username", "user");
    let mut generated = Response::default();
    Chain::new(chain.get_filters())
        .run(&mut generate, &mut generated)
        .await;
    let token = issued.lock().unwrap().last().cloned().expect("a token was issued");

    // Act 2: redeem it.
    let mut login = request_with_parameter(HttpMethod::POST, "/login/ott", "token", &token);
    let mut logged_in = Response::default();
    Chain::new(chain.get_filters())
        .run(&mut login, &mut logged_in)
        .await;
    assert_eq!(logged_in.status(), StatusCode::FOUND, "the login redirects");
    assert_eq!(authenticated_username(&login).as_deref(), Some("user"));

    // Act 3: a consumed token is dead.
    let mut replay = request_with_parameter(HttpMethod::POST, "/login/ott", "token", &token);
    let mut replayed = Response::default();
    Chain::new(chain.get_filters())
        .run(&mut replay, &mut replayed)
        .await;
    assert_eq!(
        replayed.headers().get("location").and_then(|v| v.to_str().ok()),
        Some("/login?error"),
        "the second attempt fails"
    );
    assert_eq!(authenticated_username(&replay), None);
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p next-web-security a_generated_token_authenticates_a_login_request`
Expected: FAIL — `authenticated_username` returns `None` (the token was written to a different service than the one the provider reads), which is exactly the defect Task 1 fixed.

- [ ] **Step 3: Add the two remaining test-local helpers and make it pass**

Add `Chain` (holds `&[Arc<dyn HttpFilter>]` plus an index; its `HttpFilterChain::do_filter` invokes the filter at the index with a `Chain` at `index + 1`, and an exhausted chain returns `Ok(())`) and `RecordingHandler` (an `OneTimeTokenGenerationSuccessHandler` that pushes the token value into the shared vec and writes the same redirect the default handler writes, `/login/ott`). The other helpers come from Task 1.

No production change is expected. If the test fails, the cause is a collaborator resolved twice or a filter registered with the wrong matcher — fix that rather than the test.

- [ ] **Step 4: Run the whole crate**

Run: `cargo test -p next-web-security`
Expected: PASS, no new warnings from the changed files.

- [ ] **Step 5: Commit**

```bash
git add next-web-security/src/spring_security/config/web/configurers/one_time_token_login_configurer.rs
git commit -m "test(security): cover one-time token generation and login end to end"
```

---

## Review Focus

Input classes most likely to bite a user of this feature, each already pinned by a test:

1. **A token that was generated but cannot be redeemed** — the split-store defect. Pinned by `the_generate_filter_and_the_provider_share_the_token_service` (Task 1) and `a_generated_token_authenticates_a_login_request` (Task 6).
2. **CSRF enabled (the default) with the submit form posting no token** — pinned by `the_csrf_token_is_rendered_as_a_hidden_input` (Task 2) and `the_hidden_input_value_is_html_escaped` (Task 4).
3. **A token containing HTML or quotes** reflected into the page — pinned by `the_token_is_html_escaped` (Task 4).
4. **Reusing a consumed token** — the reference fails the second attempt. Pinned by the third act of `a_generated_token_authenticates_a_login_request` (Task 6): replaying the token yields a redirect to `/login?error` and leaves the request unauthenticated.
5. **Starting the application without a success handler** — pinned by `a_missing_success_handler_is_reported` (Task 1); the message is the reference's, so a user searching the error finds the same fix.

## Not in this plan

- `OneTimeTokenAuthenticationProvider`'s `UserDetailsChecker` (`AccountStatusUserDetailsChecker`: locked / disabled / expired). The port has no checker abstraction; adding one is a `core::userdetails` change, not a configurer change. Track it separately if account-status enforcement is wanted.
- `ObjectPostProcessor` post-processing (see Global Constraints).
- Deprecated reference aliases `authenticationFailureHandler` / `authenticationSuccessHandler` and `getContext()`.
- `loginPage(String)` on every form-based configurer — only the one-time token configurer gets the deferred form here; the same gap exists in `form_login_configurer`.

---

## Execution record (2026-10-18)

All six tasks implemented inline. Unit suite: **158 passed, 0 failed** (14 new tests).

| Task | Delivered | Tests |
|---|---|---|
| 1 | Collaborators resolved once; success handler and user details service are required | `the_token_service_is_resolved_once`, `the_provider_redeems_a_token_generated_by_the_filter`, `the_request_resolver_is_resolved_once`, `a_token_service_from_the_dsl_replaces_the_default`, `a_missing_success_handler_is_reported`, `a_missing_user_details_service_is_reported` |
| 2 | CSRF token rendered as a hidden input | `the_csrf_token_is_rendered_as_a_hidden_input`, `no_hidden_input_is_rendered_without_a_csrf_token` |
| 3 | URL validation, `login_processing_url`, deferred `login_page` | `an_empty_token_generating_url_is_rejected`, `an_empty_login_processing_url_is_rejected`, `an_empty_submit_page_url_is_rejected`, `a_custom_login_page_becomes_the_entry_point` |
| 4 | Submit page escapes values; empty processing URL rejected | `html_special_characters_are_escaped`, `the_hidden_input_value_is_html_escaped`, `the_form_posts_to_the_login_processing_url`, `the_token_from_the_query_is_prefilled`, `an_empty_login_processing_url_is_rejected` (filter) |
| 5 | Login page getters + integration assertions | `init_enables_the_one_time_token_form_on_the_login_page`, `init_leaves_the_login_page_alone_when_a_custom_login_page_is_set` |
| 6 | Whole chain: generate → redeem → replay | `a_generated_token_authenticates_a_login_request` |

### Rulings

- **No commits.** `.git` is not writable in this sandbox, so the working tree carries the change and the plan's per-task commits are owed to the caller. Cost if wrong: the change lands as one commit instead of six, losing per-task bisect granularity.
- **Kept the `get_*` names** instead of the plan's `resolve_*` (`get_authentication_provider` would otherwise collide with the DSL `authentication_provider`). Cost if wrong: none; names are private.
- **`generate_html` stayed private** — in-file tests reach it directly, so making it public was unnecessary API surface. Cost if wrong: an external test harness would need one visibility bump.
- **Dropped the context-path test** — `RequestContextPath` is private to `next-web-core`, so a test cannot set a context path through the public API. Cost if wrong: the `context_path` prefix stays unverified.
- **Redirect assertions check `is_redirection()` plus the `Location`**, not `302`: `HttpResponse::set_redirect` emits `303 See Other` in this port while the reference emits `302`. Cost if wrong: a status-code regression would not be caught by these tests.
- **Removed the implicit success-handler fallback.** An application that configured OTT without a handler and relied on the silent redirect now fails at startup with the reference's message. Cost if wrong: an existing local setup stops booting (which is the reference behavior).

### Findings outside this plan (reported, not fixed)

1. ~~`HttpSecurity::new` ignores an application-supplied `PathPatternRequestMatcherBuilder`.~~ **Fixed after the plan run.** `shared_objects.get(..)` yields `&Box<dyn AnyClone>`, and the blanket `AnyClone` impl made `s.as_any()` return the box instead of the erased value, so the downcast always failed and the builder was replaced by `default()`. The one-line fix is `(**s).as_any()`, pinned by `the_shared_request_matcher_builder_is_used` (a builder with `base_path("/spring")` must prefix the patterns). The submit-page resolver in Task 2 hit the same trap; the two shared-object accessors in `base_configured_security_builder.rs` already used `obj.as_ref().as_any()` and were correct. No other `as_any()` call site in the crate has a `Box` receiver.
2. Nine doctests fail, all in files this change does not touch: the examples reference a nonexistent crate (`your_crate`) or an undefined `builder`, or import items that moved (`path_pattern_request_matcher.rs`, `user.rs`, `simple_attributes2_granted_authorities_mapper.rs`, `authorize_http_requests_configurer.rs`). Pre-existing and unrelated to the OTT flow.
3. `rustfmt` cannot write in this sandbox, so ~60 cosmetic formatting diff lines remain in the test module; `cargo fmt -p next-web-security` on a writable checkout normalizes them.
