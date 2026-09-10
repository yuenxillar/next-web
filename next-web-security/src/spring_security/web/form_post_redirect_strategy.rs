#![allow(dead_code)]

use std::borrow::Cow;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use next_web_core::traits::http::http_request::HttpRequest;
use next_web_core::traits::http::http_response::HttpResponse;
use next_web_core::{error::BoxError, util::HtmlUtils};
use rand::RngCore;
use std::fmt::Write;

use crate::web::RedirectStrategy;
use next_web_core::http::{header::CONTENT_TYPE, StatusCode, Uri};
use next_web_core::util::form_urlencoded;

/// Content-Security-Policy header name.
const CONTENT_SECURITY_POLICY_HEADER: &str = "Content-Security-Policy";

/// HTML page template that auto-submits a POST form.
const REDIRECT_PAGE_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1, shrink-to-fit=no">
    <meta name="description" content="">
    <meta name="author" content="">
    <title>Redirect</title>
  </head>
  <body>
    <form id="redirect-form" method="POST" action="{{action}}">
      {{params}}
      <noscript>
        <p>JavaScript is not enabled for this page.</p>
        <button type="submit">Click to continue</button>
      </noscript>
    </form>
    <script nonce="{{nonce}}">
      document.getElementById("redirect-form").submit();
    </script>
  </body>
</html>
"#;

/// Hidden input template for a single query parameter.
const HIDDEN_INPUT_TEMPLATE: &str = r#"<input name="{{name}}" type="hidden" value="{{value}}" />"#;

/// Number of random bytes used to generate the CSP nonce.
const NONCE_BYTES: usize = 96;

/// Redirect strategy that renders an auto-submitting HTML form using the POST method.
///
/// All query parameters in the URL are converted into hidden form inputs, so they are
/// submitted as POST body data instead of query string data.
#[derive(Clone, Default)]
pub struct FormPostRedirectStrategy;

impl FormPostRedirectStrategy {
    /// Generates a URL-safe, unpadded base64 nonce from 96 random bytes.
    fn generate_nonce() -> String {
        let mut bytes = [0u8; NONCE_BYTES];
        rand::thread_rng().fill_bytes(&mut bytes);
        URL_SAFE_NO_PAD.encode(bytes)
    }
}

impl RedirectStrategy for FormPostRedirectStrategy {
    fn send_redirect(
        &self,
        _request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
        url: &str,
    ) -> Result<(), BoxError> {
        let uri = match url.parse::<Uri>() {
            Ok(uri) => uri,
            Err(err) => return Err(Box::new(err)),
        };

        // Build the hidden inputs HTML.
        let mut hidden_inputs_html = String::new();
        let grouped = get_parameters(&uri);
        for (name, values) in grouped {
            for value in values {
                let hidden_input = HIDDEN_INPUT_TEMPLATE
                    .replace("{{name}}", &HtmlUtils::html_escape(&name))
                    .replace("{{value}}", &HtmlUtils::html_escape(&value));
                hidden_inputs_html.push_str(hidden_input.trim());
            }
        }

        // Create the script-src policy directive for the Content-Security-Policy header.
        let nonce = Self::generate_nonce();
        let policy_directive = format!("script-src 'nonce-{}'", nonce);

        // Clear the query string as we don't want that to be part of the form action URL.
        let action_url = {
            let mut strf = String::new();
            if let Some(scheme) = uri.scheme() {
                write!(strf, "{}://", scheme)?;
            }

            if let Some(authority) = uri.authority() {
                write!(strf, "{}", authority)?;
            }

            write!(strf, "{}", uri.path())?;

            strf
        };

        let html = REDIRECT_PAGE_TEMPLATE
            .replace("{{action}}", &HtmlUtils::html_escape(action_url.as_str()))
            .replace("{{params}}", &hidden_inputs_html)
            .replace("{{nonce}}", &HtmlUtils::html_escape(&nonce));

        response.set_status_code(StatusCode::OK);
        response.insert_header(CONTENT_TYPE.as_str(), "text/html;charset=UTF-8");
        response.insert_header(CONTENT_SECURITY_POLICY_HEADER, &policy_directive);
        response.set_body(html.into_bytes());
        response.commit();

        Ok(())
    }
}

fn get_parameters(uri: &Uri) -> Vec<(Cow<'_, str>, Vec<Cow<'_, str>>)> {
    let query_pairs = match uri
        .query()
        .map(|query| form_urlencoded::parse(query.as_bytes()).collect::<Vec<_>>())
    {
        Some(pairs) => pairs,
        None => Vec::new(),
    };

    let mut grouped: Vec<(Cow<'_, str>, Vec<Cow<'_, str>>)> = Vec::new();
    for (name, value) in query_pairs {
        if let Some((_, values)) = grouped.iter_mut().find(|(key, _)| key == &name) {
            values.push(value);
        } else {
            grouped.push((name, vec![value]));
        }
    }

    grouped
}
