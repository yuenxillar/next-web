use std::collections::HashMap;
use std::sync::Arc;

use next_web_core::{
    async_trait,
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
    util::http_method::HttpMethod,
};

use crate::web::util::matcher::{PathPatternRequestMatcher, RequestMatcher};

/// A function that resolves hidden input fields from the request.
/// Typically used to resolve a CSRF token.
pub type ResolveHiddenInputsFn =
    Arc<dyn Fn(&dyn HttpRequest) -> HashMap<String, String> + Send + Sync>;

const DEFAULT_SUBMIT_PAGE_URL: &str = "/login/ott";

const DEFAULT_LOGIN_PROCESSING_URL: &str = "/login/ott";

const ONE_TIME_TOKEN_SUBMIT_PAGE_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <title>One-Time Token Login</title>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1, shrink-to-fit=no"/>
    <link href="{{contextPath}}/default-ui.css" rel="stylesheet" />
  </head>
  <body>
    <div class="container">
      <form class="login-form" action="{{loginProcessingUrl}}" method="post">
        <h2>Please input the token</h2>
        <p>
          <label for="token" class="screenreader">Token</label>
          <input type="text" id="token" name="token" value="{{tokenValue}}" placeholder="Token" required="true" autofocus="autofocus"/>
        </p>
        <button class="primary" type="submit">Sign in</button>
{{hiddenInputs}}
      </form>
    </div>
  </body>
</html>"#;

const HIDDEN_HTML_INPUT_TEMPLATE: &str =
    r#"<input name="{{name}}" type="hidden" value="{{value}}" />"#;

/// Creates a default one-time token submit page. If the request contains a
/// `token` query param the page will automatically fill the form with the
/// token value.
///
/// Ported from Spring Security's `DefaultOneTimeTokenSubmitPageGeneratingFilter`.
#[derive(Clone)]
pub struct DefaultOneTimeTokenSubmitPageGeneratingFilter {
    request_matcher: Arc<dyn RequestMatcher>,
    resolve_hidden_inputs: ResolveHiddenInputsFn,
    login_processing_url: Box<str>,
}

impl DefaultOneTimeTokenSubmitPageGeneratingFilter {
    /// Creates a new `DefaultOneTimeTokenSubmitPageGeneratingFilter` with a
    /// matcher for `GET /login/ott` and an empty hidden-inputs resolver.
    pub fn new() -> Self {
        Self {
            request_matcher: Arc::new(PathPatternRequestMatcher::path_pattern(
                Some(HttpMethod::Get),
                DEFAULT_SUBMIT_PAGE_URL,
            )),
            resolve_hidden_inputs: Arc::new(|_| HashMap::new()),
            login_processing_url: DEFAULT_LOGIN_PROCESSING_URL.into(),
        }
    }

    /// Use this `RequestMatcher` to choose whether this filter will handle the
    /// request. By default, it handles `/login/ott`.
    pub fn set_request_matcher(&mut self, request_matcher: Arc<dyn RequestMatcher>) {
        self.request_matcher = request_matcher;
    }

    /// Specifies the URL that the submit form should POST to. Defaults to
    /// `/login/ott`.
    pub fn set_login_processing_url(&mut self, login_processing_url: impl Into<Box<str>>) {
        self.login_processing_url = login_processing_url.into();
    }

    /// Sets the function used to resolve a Map of the hidden inputs where the
    /// key is the name of the input and the value is the value of the input.
    /// Typically this is used to resolve the CSRF token.
    pub fn set_resolve_hidden_inputs(
        &mut self,
        resolve_hidden_inputs: ResolveHiddenInputsFn,
    ) {
        self.resolve_hidden_inputs = resolve_hidden_inputs;
    }

    /// Generates the one-time token submit page HTML.
    fn generate_html(&self, request: &dyn HttpRequest) -> String {
        let context_path = request.context_path().unwrap_or_default();

        // Read the `token` query parameter to pre-fill the form
        let token_value = request
            .parameter("token")
            .unwrap_or_default();

        let hidden_inputs_html = self.render_hidden_inputs(request);

        ONE_TIME_TOKEN_SUBMIT_PAGE_TEMPLATE
            .replace("{{contextPath}}", context_path)
            .replace("{{tokenValue}}", token_value)
            .replace(
                "{{loginProcessingUrl}}",
                &format!("{}{}", context_path, self.login_processing_url.as_ref()),
            )
            .replace("{{hiddenInputs}}", &hidden_inputs_html)
    }

    /// Renders hidden input elements from the resolved hidden-inputs map.
    fn render_hidden_inputs(&self, request: &dyn HttpRequest) -> String {
        let inputs = (self.resolve_hidden_inputs)(request);
        let mut sb = String::new();
        for (name, value) in &inputs {
            let input_element = HIDDEN_HTML_INPUT_TEMPLATE
                .replace("{{name}}", name)
                .replace("{{value}}", value);
            sb.push_str(&input_element);
        }
        sb
    }
}

impl Default for DefaultOneTimeTokenSubmitPageGeneratingFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HttpFilter for DefaultOneTimeTokenSubmitPageGeneratingFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if !self.request_matcher.matches(request) {
            filter_chain.do_filter(request, response).await?;
            return Ok(());
        }

        let html = self.generate_html(request);
        response.insert_header("Content-Type", "text/html;charset=UTF-8");
        response.set_body(html.into_bytes());
        Ok(())
    }
}

impl Named for DefaultOneTimeTokenSubmitPageGeneratingFilter {
    fn name(&self) -> &str {
        "DefaultOneTimeTokenSubmitPageGeneratingFilter"
    }
}
