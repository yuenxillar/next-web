use std::{
    collections::HashMap,
    sync::Arc,
};

use next_web_core::{
    async_trait,
    http::{
        filter::{FilterError, HttpFilter, HttpFilterChain},
        HttpMethod, HttpRequest, HttpResponse,
    },
    Named,
};

use crate::web::{
    authentication::ui::HtmlTemplates,
    csrf::CsrfToken,
    util::matcher::{PathPatternRequestMatcher, RequestMatcher},
};

/// Serves the page a signed-in user registers a passkey from.
///
/// The page carries one button and the script of the login page: the script
/// asks `/webauthn/register/options` for a challenge, has the authenticator
/// create the credential, and submits it to `POST /webauthn/register`. A `GET`
/// is answered with the page; any other method is left to the ceremony.
#[derive(Clone)]
pub struct DefaultWebAuthnRegistrationPageGeneratingFilter {
    matcher: Arc<dyn RequestMatcher>,
}

impl DefaultWebAuthnRegistrationPageGeneratingFilter {
    /// The path the page is served on.
    pub const DEFAULT_REGISTRATION_PAGE_URL: &'static str = "/webauthn/register";

    /// Creates the page.
    pub fn new() -> Self {
        Self {
            matcher: Arc::new(PathPatternRequestMatcher::path_pattern(
                None,
                Self::DEFAULT_REGISTRATION_PAGE_URL,
            )),
        }
    }

    /// Renders the page, with the headers a submission has to carry.
    fn render(&self, request: &dyn HttpRequest, context_path: &str) -> String {
        HtmlTemplates::from_template(REGISTRATION_PAGE_TEMPLATE)
            .with_value("contextPath", context_path)
            .with_raw_html("csrfHeaders", &Self::csrf_headers(request))
            .render()
    }

    /// Returns the CSRF headers of the request, as a JSON object.
    ///
    /// The token is the request attribute the CSRF filter writes, which is where
    /// the default login page reads it from as well. A request without one (CSRF
    /// protection disabled) renders an empty object, and the script sends no
    /// header.
    fn csrf_headers(request: &dyn HttpRequest) -> String {
        let token = request
            .get_attribute(std::any::type_name::<&dyn CsrfToken>())
            .and_then(|value| value.as_ref_value::<Arc<dyn CsrfToken>>());

        let headers = match token {
            Some(token) => HashMap::from([(
                token.header_name().to_string(),
                token.token().to_string(),
            )]),
            None => HashMap::new(),
        };

        serde_json::to_string(&headers).unwrap_or_else(|_| "{}".to_string())
    }
}

impl Default for DefaultWebAuthnRegistrationPageGeneratingFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl Named for DefaultWebAuthnRegistrationPageGeneratingFilter {
    fn name(&self) -> &str {
        "DefaultWebAuthnRegistrationPageGeneratingFilter"
    }
}

#[async_trait]
impl HttpFilter for DefaultWebAuthnRegistrationPageGeneratingFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if request.method() != HttpMethod::GET || !self.matcher.matcher(request).is_match() {
            return filter_chain.do_filter(request, response).await;
        }

        let context_path = request.context_path().unwrap_or("").to_string();
        let page = self.render(request, &context_path);
        response.insert_header("Content-Type", "text/html;charset=UTF-8");
        response.set_body(page.into_bytes());

        Ok(())
    }
}

const REGISTRATION_PAGE_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1, shrink-to-fit=no">
    <title>Register a passkey</title>
    <link href="{{contextPath}}/default-ui.css" rel="stylesheet" />
    <script type="text/javascript" src="{{contextPath}}/login/webauthn.js"></script>
    <script type="text/javascript">
      document.addEventListener("DOMContentLoaded",() => setupRegistration({{csrfHeaders}}, "{{contextPath}}", document.getElementById('passkey-register')));
    </script>
  </head>
  <body>
    <div class="content">
      <div class="login-form">
        <h2>Register a passkey</h2>
        <p>Register this device as a way to sign in without a password.</p>
        <button id="passkey-register" type="button" class="primary">Register a passkey</button>
      </div>
    </div>
  </body>
</html>"#;

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    use axum::{body::Body, extract::Request, response::Response};
    use next_web_core::{
        any::AnyValue,
        async_trait,
        http::{
            filter::{FilterError, HttpFilterChain},
            HttpRequest, HttpResponse,
        },
    };

    use super::*;
    use crate::web::csrf::DefaultCsrfToken;

    /// Returns a request of the given method for the registration page.
    fn request(method: &str) -> Request {
        Request::builder()
            .method(method)
            .uri("https://sp.example.org/webauthn/register")
            .body(Body::empty())
            .expect("a valid request")
    }

    /// A chain that records whether it ran.
    #[derive(Clone, Default)]
    struct RecordingChain {
        ran: Arc<AtomicBool>,
    }

    impl RecordingChain {
        fn ran(&self) -> bool {
            self.ran.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl HttpFilterChain for RecordingChain {
        async fn do_filter(
            &self,
            _request: &mut dyn HttpRequest,
            _response: &mut dyn HttpResponse,
        ) -> Result<(), FilterError> {
            self.ran.store(true, Ordering::SeqCst);

            Ok(())
        }
    }

    /// Runs the filter against the request.
    async fn run(filter: &DefaultWebAuthnRegistrationPageGeneratingFilter, request: &mut Request) -> (Response, bool) {
        let mut response = Response::default();
        let chain = RecordingChain::default();
        filter
            .do_filter(request, &mut response, &chain)
            .await
            .expect("the filter runs");

        (response, chain.ran())
    }

    /// Returns the body of a response.
    async fn body(response: Response) -> String {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("the body is collected");

        String::from_utf8(bytes.to_vec()).expect("the page is UTF-8")
    }

    #[tokio::test]
    async fn the_page_is_served_on_a_get() {
        let filter = DefaultWebAuthnRegistrationPageGeneratingFilter::new();
        let mut request = request("GET");

        let (response, ran) = run(&filter, &mut request).await;
        let status = response.status();
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let page = body(response).await;

        assert!(!ran, "the page is served here");
        assert_eq!(status, 200);
        assert_eq!(content_type.as_deref(), Some("text/html;charset=UTF-8"));
        assert!(page.contains("setupRegistration("), "the page wires the script");
        assert!(page.contains("/login/webauthn.js"), "the page loads the script");
        assert!(
            page.contains(r#"id="passkey-register""#),
            "the page offers the button"
        );
    }

    #[tokio::test]
    async fn the_submission_of_the_ceremony_is_left_to_the_endpoint() {
        let filter = DefaultWebAuthnRegistrationPageGeneratingFilter::new();
        let mut request = request("POST");

        let (_, ran) = run(&filter, &mut request).await;

        assert!(ran, "a POST is the ceremony, not the page");
    }

    #[tokio::test]
    async fn the_page_carries_the_csrf_header_of_the_request() {
        let filter = DefaultWebAuthnRegistrationPageGeneratingFilter::new();
        let mut request = request("GET");
        let token: Arc<dyn CsrfToken> = Arc::new(DefaultCsrfToken::new(
            "X-CSRF-TOKEN",
            "_csrf",
            "the-token",
        ));
        request.set_attribute(
            std::any::type_name::<&dyn CsrfToken>(),
            AnyValue::BoxedValue(Box::new(token)),
        );

        let (response, _) = run(&filter, &mut request).await;
        let page = body(response).await;

        assert!(
            page.contains(r#"{"X-CSRF-TOKEN":"the-token"}"#),
            "the submission carries the token the page was rendered with, was: {page}"
        );
    }
}
