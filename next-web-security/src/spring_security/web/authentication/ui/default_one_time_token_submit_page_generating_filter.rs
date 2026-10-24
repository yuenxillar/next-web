use std::collections::HashMap;
use std::sync::Arc;

use next_web_core::{
    async_trait,
    http::{
        filter::{FilterError, HttpFilter, HttpFilterChain},
        HttpMethod, HttpRequest, HttpResponse,
    },
    Named,
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
/// Ported from Next Security's `DefaultOneTimeTokenSubmitPageGeneratingFilter`.
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
                Some(HttpMethod::GET),
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
    ///
    /// # Panics
    ///
    /// Panics when the URL is empty or whitespace, because a form that posts
    /// nowhere cannot authenticate anything.
    pub fn set_login_processing_url(&mut self, login_processing_url: impl Into<Box<str>>) {
        let login_processing_url = login_processing_url.into();
        assert!(
            !login_processing_url.trim().is_empty(),
            "loginProcessingUrl cannot be null or empty"
        );

        self.login_processing_url = login_processing_url;
    }

    /// Sets the function used to resolve a Map of the hidden inputs where the
    /// key is the name of the input and the value is the value of the input.
    /// Typically this is used to resolve the CSRF token.
    pub fn set_resolve_hidden_inputs(&mut self, resolve_hidden_inputs: ResolveHiddenInputsFn) {
        self.resolve_hidden_inputs = resolve_hidden_inputs;
    }

    /// Generates the one-time token submit page HTML.
    fn generate_html(&self, request: &dyn HttpRequest) -> String {
        let context_path = request.context_path().unwrap_or_default();

        // Read the `token` query parameter to pre-fill the form
        let token_value = request.parameter("token").unwrap_or_default();

        let hidden_inputs_html = self.render_hidden_inputs(request);

        ONE_TIME_TOKEN_SUBMIT_PAGE_TEMPLATE
            .replace("{{contextPath}}", context_path)
            .replace("{{tokenValue}}", &escape_html(token_value))
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
                .replace("{{name}}", &escape_html(name))
                .replace("{{value}}", &escape_html(value));
            sb.push_str(&input_element);
        }
        sb
    }
}

/// Escapes the characters that would otherwise be read as HTML markup.
///
/// The page echoes a token and the hidden inputs of the form back into the
/// document, so a value that carries markup must not be able to change the page.
fn escape_html(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());

    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            other => escaped.push(other),
        }
    }

    escaped
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

#[cfg(test)]
mod tests {
    use axum::{body::Body, extract::Request};

    use super::*;

    /// Returns a submit page filter with the default settings.
    fn filter() -> DefaultOneTimeTokenSubmitPageGeneratingFilter {
        DefaultOneTimeTokenSubmitPageGeneratingFilter::new()
    }

    /// Returns a `GET` request for the given path.
    fn request(path: &str) -> Request {
        Request::builder()
            .method(HttpMethod::GET)
            .uri(path)
            .body(Body::empty())
            .expect("a valid request")
    }

    #[test]
    fn html_special_characters_are_escaped() {
        assert_eq!(escape_html(r#"this<>!@#""#), "this&lt;&gt;!@#&quot;");
    }

    #[test]
    fn the_hidden_input_value_is_html_escaped() {
        let mut filter = filter();
        filter.set_resolve_hidden_inputs(Arc::new(|_| {
            HashMap::from([("_csrf".to_owned(), "<b>".to_owned())])
        }));

        let html = filter.generate_html(&request("/login/ott"));

        assert!(
            html.contains(r#"<input name="_csrf" type="hidden" value="&lt;b&gt;" />"#),
            "the hidden input is escaped, page was: {html}"
        );
    }

    #[test]
    fn the_form_posts_to_the_login_processing_url() {
        let mut filter = filter();
        filter.set_login_processing_url("/login/another");

        let html = filter.generate_html(&request("/login/ott"));

        assert!(
            html.contains(r#"action="/login/another""#),
            "the form action is the processing url, page was: {html}"
        );
    }

    #[test]
    fn the_token_from_the_query_is_prefilled() {
        let html = filter().generate_html(&request("/login/ott?token=1234"));

        assert!(
            html.contains(r#"value="1234""#),
            "the token is prefilled, page was: {html}"
        );
    }

    #[test]
    #[should_panic(expected = "loginProcessingUrl cannot be null or empty")]
    fn an_empty_login_processing_url_is_rejected() {
        filter().set_login_processing_url("");
    }
}
