use std::collections::HashMap;
use std::sync::Arc;

use next_web_core::{
    async_trait,
    filter::FilterError,
    http::HttpMethod,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};
use tracing::trace;

use crate::web::util::matcher::{PathPatternRequestMatcher, RequestMatcher};

/// A function that resolves hidden input fields from the request.
/// Typically used to resolve a CSRF token.
pub type ResolveHiddenInputsFn =
    Arc<dyn Fn(&dyn HttpRequest) -> HashMap<String, String> + Send + Sync>;

const LOGOUT_PAGE_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1, shrink-to-fit=no">
    <meta name="description" content="">
    <meta name="author" content="">
    <title>Confirm Log Out?</title>
    <link href="{{contextPath}}/default-ui.css" rel="stylesheet" />
  </head>
  <body>
    <div class="content">
      <form class="logout-form" method="post" action="{{contextPath}}/logout">
        <h2>Are you sure you want to log out?</h2>
{{hiddenInputs}}
        <button class="primary" type="submit">Log Out</button>
      </form>
    </div>
  </body>
</html>"#;

const HIDDEN_HTML_INPUT_TEMPLATE: &str =
    r#"<input name="{{name}}" type="hidden" value="{{value}}" />"#;

/// Generates a default log out page.
///
/// This filter renders a confirmation page when a user navigates to `/logout`
/// via a GET request. The page contains a form that POSTs to `/logout` to
/// complete the logout action.
///
/// Ported from Spring Security's `DefaultLogoutPageGeneratingFilter`.
#[derive(Clone)]
pub struct DefaultLogoutPageGeneratingFilter {
    matcher: Arc<dyn RequestMatcher>,
    resolve_hidden_inputs: ResolveHiddenInputsFn,
}

impl DefaultLogoutPageGeneratingFilter {
    /// Creates a new `DefaultLogoutPageGeneratingFilter` with a matcher for
    /// `GET /logout` and an empty hidden-inputs resolver.
    pub fn new() -> Self {
        Self {
            matcher: Arc::new(PathPatternRequestMatcher::path_pattern(
                Some(HttpMethod::GET),
                "/logout",
            )),
            resolve_hidden_inputs: Arc::new(|_| HashMap::new()),
        }
    }

    /// Creates a new `DefaultLogoutPageGeneratingFilter` with a custom
    /// hidden-inputs resolver.
    ///
    /// # Arguments
    ///
    /// * `resolve_hidden_inputs` - A function that receives the current request
    ///   and returns a map of hidden input names to values (typically used for
    ///   CSRF tokens).
    pub fn with_hidden_inputs_resolver(resolve_hidden_inputs: ResolveHiddenInputsFn) -> Self {
        Self {
            matcher: Arc::new(PathPatternRequestMatcher::path_pattern(
                Some(HttpMethod::GET),
                "/logout",
            )),
            resolve_hidden_inputs,
        }
    }

    /// Sets the function used to resolve a Map of the hidden inputs where the
    /// key is the name of the input and the value is the value of the input.
    /// Typically this is used to resolve the CSRF token.
    pub fn set_resolve_hidden_inputs(&mut self, resolve_hidden_inputs: ResolveHiddenInputsFn) {
        self.resolve_hidden_inputs = resolve_hidden_inputs;
    }

    /// Returns a reference to the hidden-inputs resolver.
    pub fn get_resolve_hidden_inputs(&self) -> &ResolveHiddenInputsFn {
        &self.resolve_hidden_inputs
    }

    /// Renders the logout confirmation page.
    fn render_logout(&self, request: &dyn HttpRequest, response: &mut dyn HttpResponse) {
        let context_path = request.context_path().unwrap_or_default();
        let hidden_inputs_html = self.render_hidden_inputs(request);

        let page = LOGOUT_PAGE_TEMPLATE
            .replace("{{contextPath}}", context_path)
            .replace("{{hiddenInputs}}", &format!("{:>8}", hidden_inputs_html));

        response.insert_header("Content-Type", "text/html;charset=UTF-8");
        response.set_body(page.into_bytes());
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

impl Default for DefaultLogoutPageGeneratingFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HttpFilter for DefaultLogoutPageGeneratingFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if self.matcher.matches(request) {
            self.render_logout(request, response);
        } else {
            trace!(
                "Did not render default logout page since request did not match [{:?}]",
                self.matcher
            );
            filter_chain.do_filter(request, response).await?;
        }
        Ok(())
    }
}

impl Named for DefaultLogoutPageGeneratingFilter {
    fn name(&self) -> &str {
        "DefaultLogoutPageGeneratingFilter"
    }
}
