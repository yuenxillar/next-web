//! The lifecycle of a request: the per-request state it starts with, and the
//! session it leaves behind.
//!
//! A request is prepared before the filters and the routes of an application
//! run, and it is finished after the response was produced. The preparation
//! reads the attributes, the cookies, and the session of the client from the
//! request; the finish writes the session back to the session manager and tells
//! the client about a session it does not know yet.

use axum::{extract::Request, response::Response};

use crate::{
    http::{session::SESSION_COOKIE_NAME, Cookie},
    traits::http::{
        http_request::{is_request_prepared, HttpRequest},
        http_response::HttpResponse,
    },
};

/// The lifecycle of a request.
///
/// [`RequestLifecycle::begin`] is called with the request before the filters and
/// the routes run, and [`RequestLifecycle::end`] is called with the request and
/// the response afterwards. The session identifier the client presented is
/// remembered by `begin`, so `end` reports the session back to the client only
/// when the client does not know it yet.
///
/// The preparation of the request has no effect when the request was prepared
/// before, so a caller that runs the filters of an application inside the layer
/// that prepares every request of the application does not read the state of the
/// client twice.
///
/// A caller that runs the filters itself owns the request until the response is
/// produced, so it can finish the lifecycle. A caller that hands the request to
/// another service (such as the routes of a router) can only prepare it: the
/// request cannot be read after the service consumed it.
pub struct RequestLifecycle {
    presented_session_id: Option<String>,
}

impl RequestLifecycle {
    /// Prepares the per-request state of the request.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the layer.
    pub fn begin(request: &mut Request) -> Self {
        // The request was prepared by the layer that prepares every request of
        // an application, and the preparation has no effect when it already ran,
        // so it is skipped here.
        if !is_request_prepared(request) {
            HttpRequest::ready(request);
        }

        Self {
            presented_session_id: session_id_of(request),
        }
    }

    /// Finishes the request: the session it leaves behind is written back to
    /// the session manager, and the client is told about a session it does not
    /// know yet.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the layer.
    /// * `response` - The response the request produced.
    pub fn end(self, request: &mut Request, response: &mut Response) {
        // The session and the context path are read before the state of the
        // request is released, because releasing it drops both.
        let session_id = HttpRequest::session(request).map(|session| session.id().to_string());
        let context_path = HttpRequest::context_path(request)
            .unwrap_or("/")
            .to_string();

        HttpRequest::clean_up(request);

        let Some(session_id) = session_id else {
            return;
        };
        // The client already has the session, so it is not told about it again.
        // A session identifier that changed (the session was renewed) is not the
        // identifier the client presented, so it is reported.
        if self.presented_session_id.as_deref() == Some(session_id.as_str()) {
            return;
        }

        let cookie = Cookie::builder(SESSION_COOKIE_NAME, Some(session_id)).path(context_path);
        if let Ok(cookie) = cookie.build() {
            HttpResponse::add_cookie(response, cookie);
        }
    }
}

/// Returns the identifier of the session the request carries, if any.
///
/// # Arguments
///
/// * `request` - The request of the layer.
fn session_id_of(request: &Request) -> Option<String> {
    HttpRequest::cookies(request)
        .and_then(|cookies| {
            cookies
                .iter()
                .find(|cookie| cookie.name() == SESSION_COOKIE_NAME)
        })
        .map(|cookie| cookie.value().to_string())
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, extract::Request, http::header::COOKIE, response::Response};

    use super::{RequestLifecycle, SESSION_COOKIE_NAME};
    use crate::{
        any::AnyValue,
        http::session::{generate_session_id, global_session_manager, SessionManager},
        traits::http::http_request::{is_request_prepared, HttpRequest},
    };

    /// Returns the session cookie of a response, if the response carries one.
    ///
    /// # Arguments
    ///
    /// * `response` - The response of the request.
    fn session_cookie(response: &Response) -> Option<String> {
        response
            .headers()
            .get("set-cookie")
            .and_then(|cookie| cookie.to_str().ok())
            .map(ToString::to_string)
    }

    /// Returns a request that presents the given session.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The identifier of the session.
    fn request_with_session(session_id: &str) -> Request {
        Request::builder()
            .header(COOKIE, format!("{SESSION_COOKIE_NAME}={session_id}"))
            .body(Body::empty())
            .expect("the request is built")
    }

    #[test]
    fn a_session_the_request_created_is_reported_to_the_client() {
        let mut request = Request::builder()
            .body(Body::empty())
            .expect("the request is built");
        let lifecycle = RequestLifecycle::begin(&mut request);
        {
            let session =
                HttpRequest::session_mut(&mut request, true).expect("the session is created");
            session.set_attribute("user", AnyValue::from("alice"));
        }
        let mut response = Response::new(Body::empty());

        lifecycle.end(&mut request, &mut response);

        let cookie = session_cookie(&response).expect("the session is reported to the client");
        assert!(cookie.starts_with(SESSION_COOKIE_NAME), "{cookie}");
        assert!(cookie.contains("Path=/"), "{cookie}");
    }

    #[test]
    fn a_session_the_client_knows_is_not_reported_again() {
        let manager = global_session_manager();
        let session_id = generate_session_id();
        manager.create_session_with_id(session_id.clone());
        let mut request = request_with_session(&session_id);
        let lifecycle = RequestLifecycle::begin(&mut request);
        // The request ends with the session it presented.
        let _ = HttpRequest::session_mut(&mut request, false);
        let mut response = Response::new(Body::empty());

        lifecycle.end(&mut request, &mut response);

        assert!(session_cookie(&response).is_none(), "no cookie is repeated");
    }

    #[test]
    fn a_session_whose_identifier_changed_is_reported() {
        let manager = global_session_manager();
        let session_id = generate_session_id();
        manager.create_session_with_id(session_id.clone());
        let mut request = request_with_session(&session_id);
        let lifecycle = RequestLifecycle::begin(&mut request);
        let renewed_session_id = HttpRequest::change_session_id(&mut request);
        let mut response = Response::new(Body::empty());

        lifecycle.end(&mut request, &mut response);

        let cookie = session_cookie(&response).expect("the renewed session is reported");
        assert!(
            cookie.contains(&renewed_session_id),
            "the client is told about the renewed session: {cookie}"
        );
    }

    #[test]
    fn the_prepared_request_has_its_cookies_and_attributes() {
        let mut request = Request::builder()
            .header(COOKIE, "first=1; second=2")
            .body(Body::empty())
            .expect("the request is built");

        let _lifecycle = RequestLifecycle::begin(&mut request);

        assert_eq!(
            HttpRequest::cookies(&request).map(<[_]>::len),
            Some(2),
            "the cookies are read by the preparation"
        );
        HttpRequest::set_attribute(&mut request, "user", AnyValue::from("alice"));
        assert_eq!(
            HttpRequest::get_attribute(&request, "user").and_then(AnyValue::as_str),
            Some("alice")
        );
    }

    #[test]
    fn a_request_that_was_prepared_is_not_prepared_again() {
        let mut request = Request::builder()
            .header(COOKIE, format!("{SESSION_COOKIE_NAME}=the-session"))
            .body(Body::empty())
            .expect("the request is built");
        HttpRequest::ready(&mut request);
        assert!(is_request_prepared(&request));

        // The lifecycle of a request that was prepared by the outermost layer of
        // an application keeps the state it read, and it can still see the
        // session the client presented.
        let lifecycle = RequestLifecycle::begin(&mut request);
        assert!(is_request_prepared(&request));
        let mut response = Response::new(Body::empty());
        lifecycle.end(&mut request, &mut response);

        assert!(!is_request_prepared(&request), "the state was released");
        assert_eq!(
            HttpRequest::cookies(&request).map(<[_]>::len),
            None,
            "the cookies were released with the state"
        );
    }
}
