//! The [`HttpRequest`] abstraction and its implementation for
//! [`axum::extract::Request`].
//!
//! `HttpRequest` is the framework's servlet-style view of an incoming request.
//! It exposes the request line, headers, cookies, query parameters, attributes,
//! and the [`HttpSession`] associated with the client, while hiding the
//! concrete transport.
//!
//! # Request lifecycle
//!
//! A request is prepared by calling [`HttpRequest::ready`] once, at the start of
//! the filter chain. That call initializes the per-request state (attributes,
//! the cookie cache, the [`SessionManager`], and the working session loaded from
//! the session cookie). At the end of the chain,
//! [`HttpRequest::clean_up`] persists the session and releases the per-request
//! state.

use std::{borrow::Cow, collections::HashMap, net::SocketAddr, sync::Arc};

use axum::{
    extract::Request,
    http::{
        HeaderMap, HeaderValue, Uri, header::ACCEPT_LANGUAGE, header::CONTENT_TYPE, uri::Scheme,
    },
};

use headers::{Cookie as HeaderCookie, HeaderMapExt, Host};
use next_web_context::Locale;

use crate::{
    any::AnyValue,
    http::{
        Cookie, HttpMethod, HttpRequestShare, HttpVersion, ResourceDispatcher,
        session::{
            HttpSession, SESSION_COOKIE_NAME, SessionManager, StandardSession,
            global_session_manager,
        },
    },
    web::ViewModel,
};

/// The request attributes of a servlet-style request.
///
/// Attributes live for the duration of a single request and are stored in the
/// request extensions. They are distinct from [`HttpSession`] attributes, which
/// outlive the request.
type OneMap = HashMap<String, AnyValue>;

/// The servlet-style view of an HTTP request.
///
/// The trait is implemented for [`axum::extract::Request`], so the transport
/// types of the web layer can be used directly by filters and handlers without
/// an adapter. Methods are grouped below by the part of the request they read
/// or mutate.
pub trait HttpRequest
where
    Self: Send,
{
    // ------------------------------------------------------------------
    // Session access.
    // ------------------------------------------------------------------

    /// Returns the session associated with the client, if one is present.
    ///
    /// A session is present once [`HttpRequest::ready`] has loaded one from the
    /// session cookie, or once [`HttpRequest::session_mut`] created one. Use
    /// [`HttpRequest::session_mut`] when the session must be created on demand.
    fn session(&self) -> Option<&dyn HttpSession>;

    /// Returns the session associated with the client, optionally creating it.
    ///
    /// When `create` is `true` and the request has no session yet, a fresh
    /// session is created, registered with the [`SessionManager`], and returned.
    /// When `create` is `false` this only returns an already existing session.
    fn session_mut(&mut self, create: bool) -> Option<&mut dyn HttpSession>;

    /// Assigns a new identifier to the request's session and returns it.
    ///
    /// This is used to defend against session-fixation attacks: the session
    /// state is migrated to a fresh identifier and the old identifier is
    /// dropped. Returns the new identifier, or an empty string when no session
    /// could be created.
    fn change_session_id(&mut self) -> String;

    /// Returns `true` when the session identifier requested by the client
    /// resolves to a live session.
    ///
    /// A request that carries a session cookie for an unknown or expired
    /// session reports `false`, which lets a filter react to the lost session.
    fn is_requested_session_id_valid(&self) -> bool;

    /// Returns the session identifier requested by the client, if any.
    ///
    /// This is the value from the session cookie, even when it does not resolve
    /// to a live session; pair it with
    /// [`HttpRequest::is_requested_session_id_valid`] to distinguish a valid
    /// session from a stale cookie.
    fn requested_session_id(&self) -> Option<&str>;

    // ------------------------------------------------------------------
    // Cookies.
    // ------------------------------------------------------------------

    /// Returns the first cookie of the request, if any.
    fn cookie(&self) -> Option<&Cookie>;

    /// Returns every cookie of the request.
    ///
    /// Cookies are parsed once, during [`HttpRequest::ready`], and cached for
    /// the rest of the request.
    fn cookies(&self) -> Option<&[Cookie]>;

    // ------------------------------------------------------------------
    // Resource rendering.
    // ------------------------------------------------------------------

    /// Returns the [`ResourceDispatcher`] the application installed, if there
    /// is one.
    ///
    /// The dispatcher is looked up from the request extensions, where the web
    /// layer (or a filter) stores it. It returns `None` when no dispatcher is
    /// configured, which lets callers fall back to a redirect. The resource to
    /// render is an argument of the dispatcher, so the lookup does not depend
    /// on the URL.
    fn resource_dispatcher(&self) -> Option<&dyn ResourceDispatcher>;

    // ------------------------------------------------------------------
    // Request line and headers.
    // ------------------------------------------------------------------

    /// Returns the request method.
    fn method(&self) -> HttpMethod;

    /// Returns the HTTP version of the request.
    fn version(&self) -> HttpVersion;

    /// Returns all request headers.
    fn headers(&self) -> &HeaderMap<HeaderValue>;

    /// Returns the first value of the named header, when it is valid UTF-8.
    fn header(&self, header_name: &str) -> Option<&str>;

    /// Returns every value of the named header, skipping invalid UTF-8 values.
    fn header_values(&self, header_name: &str) -> Vec<&str>;

    /// Returns the names of all request headers.
    fn header_names(&self) -> Vec<&str>;

    /// Returns the request URI.
    fn uri(&self) -> &Uri;

    /// Returns the raw query string of the request, without the `?`.
    fn query(&self) -> Option<&str>;

    /// Returns the first value of the named query parameter.
    fn parameter(&self, name: &str) -> Option<&str>;

    /// Returns every query parameter as percent-decoded name/value pairs.
    fn parameters(&self) -> Option<Vec<(Cow<'_, str>, Cow<'_, str>)>>;

    /// Returns every value of the named query parameter.
    fn parameter_values(&self, name: &str) -> Option<Vec<Cow<'_, str>>>;

    /// Returns the bytes of the request body when the transport buffered them.
    ///
    /// The body is buffered by the web layer for the requests whose filters
    /// have to read it, such as a url-encoded form post, and it is `None` when
    /// the body was not buffered.
    fn body_bytes(&self) -> Option<&[u8]> {
        None
    }

    /// Returns the path of the request URI.
    fn path(&self) -> &str;

    /// Returns the host of the request URI, if the URI is absolute.
    fn host(&self) -> Option<&str>;

    /// Returns the scheme of the request URI, if the URI is absolute.
    fn scheme(&self) -> Option<&str>;

    /// Returns the port of the request URI, if the URI carries one.
    fn server_port(&self) -> Option<u16>;

    /// Returns the server name, taken from the `Host` header.
    fn server_name(&self) -> Option<String>;

    /// Returns the context path the application is nested under, if the request
    /// carries one.
    ///
    /// The context path is an application setting, `next.server.context_path`, and
    /// the web layer strips it from the path before routing, so a request does not
    /// normally hold it. A filter that has to report it may store one in the
    /// request extensions, which is what this returns; it is `None` otherwise.
    fn context_path(&self) -> Option<&str>;

    // ------------------------------------------------------------------
    // Negotiated locale.
    // ------------------------------------------------------------------

    /// Returns the locale preferred by the client, from the `Accept-Language`
    /// header.
    fn locale(&self) -> Option<Locale>;

    /// Returns the locales preferred by the client, from the `Accept-Language`
    /// header.
    ///
    /// Falls back to the system locale when the header is present but empty.
    fn locales(&self) -> Option<Vec<Locale>>;

    /// Returns the media type of the request body, from the `Content-Type`
    /// header.
    fn content_type(&self) -> Option<&str>;

    // ------------------------------------------------------------------
    // Attributes.
    // ------------------------------------------------------------------

    /// Returns every attribute of the request, if the transport keeps them.
    ///
    /// An attribute is a value a filter or a handler passes around, and it may
    /// hold anything. The variables a view renders are the model of the request,
    /// see [`HttpRequest::view_model`], which is a store of its own.
    fn attributes(&self) -> Option<&HashMap<String, AnyValue>> {
        None
    }

    /// Returns the request attribute bound to `name`, if any.
    fn get_attribute(&self, name: &str) -> Option<&AnyValue>;

    /// Removes the request attribute bound to `name`.
    fn remove_attribute(&mut self, name: &str);

    /// Binds `value` to `name` as a request attribute.
    fn set_attribute(&mut self, name: &str, value: AnyValue);

    // ------------------------------------------------------------------
    // View model.
    // ------------------------------------------------------------------

    /// Returns the model the views of the request are rendered with, if the
    /// transport keeps one.
    ///
    /// The model holds the variables of a template as [`Value`]s, which is what
    /// a dispatch renders, see
    /// [`TemplateEngine::context_of`](crate::web::TemplateEngine::context_of).
    /// It is separate from the [`attributes`](HttpRequest::attributes) of the
    /// request, so what a filter passes around is not what a template renders.
    fn view_model(&self) -> Option<&ViewModel>;

    /// Returns the model the views of the request are rendered with, mutably,
    /// so a caller can bind the variables of a template.
    ///
    /// Returns `None` when the transport keeps no model, which is the case until
    /// [`ready`](HttpRequest::ready) prepared the request; use
    /// [`set_view_model`](HttpRequest::set_view_model) to give a request that
    /// was not prepared a model.
    fn view_model_mut(&mut self) -> Option<&mut ViewModel>;

    /// Replaces the model the views of the request are rendered with.
    fn set_view_model(&mut self, model: ViewModel);

    /// Removes the model the views of the request are rendered with.
    ///
    /// A view dispatched after this renders with an empty model, which is what
    /// a filter uses when it suppresses the model a previous filter built.
    fn remove_view_model(&mut self);

    // ------------------------------------------------------------------
    // Lifecycle and transport.
    // ------------------------------------------------------------------

    /// Prepares the request for the filter chain.
    ///
    /// Initializes the per-request state (attributes, cookie cache, session
    /// manager) and loads the working session from the session cookie. Calling
    /// it more than once has no effect.
    fn ready(&mut self);

    /// Finalizes the request at the end of the filter chain.
    ///
    /// Persists the working session and releases the per-request state.
    fn clean_up(&mut self);

    /// Returns `true` when the request was made over a secure transport.
    fn is_secure(&self) -> bool;

    /// Returns the address of the peer that sent the request, if known.
    fn remote_addr(&self) -> Option<&SocketAddr>;

    /// Returns the shared, transport-independent snapshot of the request.
    ///
    /// The snapshot is created on first use and cached for the request.
    fn shared(&mut self) -> &HttpRequestShare;
}

/// The session identifier requested by the client, parsed from the session
/// cookie during [`HttpRequest::ready`].
#[derive(Clone)]
struct RequestedSessionId(String);

/// A context path installed on a single request, overriding the global one.
#[derive(Clone)]
struct RequestContextPath(String);

/// Marks a request whose per-request state was prepared.
///
/// The marker is what makes the preparation of a request have no effect the
/// second time, and it is removed by [`HttpRequest::clean_up`], so a request
/// that is prepared again after it was released is prepared from its state
/// again.
#[derive(Clone, Copy, Debug)]
struct Prepared;

impl HttpRequest for Request {
    fn session(&self) -> Option<&dyn HttpSession> {
        self.extensions()
            .get::<StandardSession>()
            .map(|session| session as &dyn HttpSession)
    }

    fn session_mut(&mut self, create: bool) -> Option<&mut dyn HttpSession> {
        if create && self.extensions().get::<StandardSession>().is_none() {
            create_working_session(self);
        }

        self.extensions_mut()
            .get_mut::<StandardSession>()
            .map(|session| session as &mut dyn HttpSession)
    }

    fn change_session_id(&mut self) -> String {
        if self.extensions().get::<StandardSession>().is_none() {
            create_working_session(self);
        }

        let manager = session_manager(self);
        let Some(old_id) = self
            .extensions()
            .get::<StandardSession>()
            .map(|session| session.id().to_string())
        else {
            return String::new();
        };

        // Persist the working copy first so the migration sees the latest
        // attributes, then migrate the stored session onto a fresh id and load
        // the migrated copy back into the request.
        save_working_session(self, &manager);
        let new_id = manager.change_session_id(&old_id).unwrap_or(old_id);
        if let Some(session) = manager.load_session(&new_id) {
            self.extensions_mut().insert(session);
        }

        new_id
    }

    fn is_requested_session_id_valid(&self) -> bool {
        match self.extensions().get::<RequestedSessionId>() {
            Some(RequestedSessionId(id)) => session_manager(self).contains_session(id),
            None => false,
        }
    }

    fn requested_session_id(&self) -> Option<&str> {
        self.extensions()
            .get::<RequestedSessionId>()
            .map(|requested| requested.0.as_str())
    }

    fn cookie(&self) -> Option<&Cookie> {
        self.cookies()?.first()
    }

    fn cookies(&self) -> Option<&[Cookie]> {
        self.extensions().get::<Vec<Cookie>>().map(Vec::as_slice)
    }

    fn resource_dispatcher(&self) -> Option<&dyn ResourceDispatcher> {
        self.extensions()
            .get::<Arc<dyn ResourceDispatcher>>()
            .map(|dispatcher| &**dispatcher as &dyn ResourceDispatcher)
    }

    fn method(&self) -> HttpMethod {
        self.method().to_owned()
    }

    fn version(&self) -> HttpVersion {
        self.version()
    }

    fn headers(&self) -> &HeaderMap<HeaderValue> {
        self.headers()
    }

    fn content_type(&self) -> Option<&str> {
        self.headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
    }

    fn header(&self, header_name: &str) -> Option<&str> {
        self.headers()
            .get(header_name)
            .and_then(|value| value.to_str().ok())
    }

    fn header_values(&self, header_name: &str) -> Vec<&str> {
        self.headers()
            .get_all(header_name)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .collect()
    }

    fn header_names(&self) -> Vec<&str> {
        self.headers().keys().map(|name| name.as_str()).collect()
    }

    fn uri(&self) -> &Uri {
        self.uri()
    }

    fn query(&self) -> Option<&str> {
        self.uri().query()
    }

    fn parameter(&self, name: &str) -> Option<&str> {
        let query = self.query()?;
        query.split('&').find_map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (key == name).then_some(value)
        })
    }

    fn parameters(&self) -> Option<Vec<(Cow<'_, str>, Cow<'_, str>)>> {
        self.query()
            .map(|query| form_urlencoded::parse(query.as_bytes()).collect())
    }

    fn parameter_values(&self, name: &str) -> Option<Vec<Cow<'_, str>>> {
        self.parameters().map(|params| {
            params
                .into_iter()
                .filter(|(key, _)| *key == name)
                .map(|(_, value)| value)
                .collect()
        })
    }

    fn body_bytes(&self) -> Option<&[u8]> {
        self.extensions()
            .get::<crate::http::RequestBody>()
            .map(crate::http::RequestBody::as_bytes)
    }

    fn path(&self) -> &str {
        self.uri().path()
    }

    fn host(&self) -> Option<&str> {
        self.uri().host()
    }

    fn scheme(&self) -> Option<&str> {
        self.uri().scheme().map(|scheme| scheme.as_str())
    }

    fn server_port(&self) -> Option<u16> {
        self.uri().port_u16()
    }

    fn server_name(&self) -> Option<String> {
        self.headers()
            .typed_get::<Host>()
            .map(|host| host.hostname().to_string())
    }

    fn context_path(&self) -> Option<&str> {
        self.extensions()
            .get::<RequestContextPath>()
            .map(|s| s.0.as_str())
    }

    fn locale(&self) -> Option<Locale> {
        self.header(ACCEPT_LANGUAGE.as_str())
            .and_then(Locale::from_accept_language)
    }

    fn locales(&self) -> Option<Vec<Locale>> {
        let accept_language = self.header(ACCEPT_LANGUAGE.as_str())?;
        let locales = Locale::parse_accept_language(accept_language);

        if locales.is_empty() {
            Some(vec![Locale::system_locale()])
        } else {
            Some(locales)
        }
    }

    fn get_attribute(&self, name: &str) -> Option<&AnyValue> {
        self.extensions()
            .get::<OneMap>()
            .and_then(|attributes| attributes.get(name))
    }

    fn attributes(&self) -> Option<&HashMap<String, AnyValue>> {
        self.extensions().get::<OneMap>()
    }

    fn remove_attribute(&mut self, name: &str) {
        if let Some(attributes) = self.extensions_mut().get_mut::<OneMap>() {
            attributes.remove(name);
        }
    }

    fn set_attribute(&mut self, name: &str, value: AnyValue) {
        attribute_map(self).insert(name.to_owned(), value);
    }

    fn view_model(&self) -> Option<&ViewModel> {
        self.extensions().get::<ViewModel>()
    }

    fn view_model_mut(&mut self) -> Option<&mut ViewModel> {
        self.extensions_mut().get_mut::<ViewModel>()
    }

    fn set_view_model(&mut self, model: ViewModel) {
        let _ = self.extensions_mut().insert::<ViewModel>(model);
    }

    fn remove_view_model(&mut self) {
        let _ = self.extensions_mut().remove::<ViewModel>();
    }

    fn ready(&mut self) {
        // The preparation of a request has no effect when it already ran, which
        // is what keeps a layer that follows the outermost layer of an
        // application (the layer that prepares every request) from reading the
        // state of the client a second time. See [`is_request_prepared`].
        if self.extensions().get::<Prepared>().is_some() {
            return;
        }

        if self.extensions().get::<OneMap>().is_none() {
            self.extensions_mut().insert(OneMap::new());
        }

        if self.extensions().get::<ViewModel>().is_none() {
            self.extensions_mut().insert(ViewModel::new());
        }

        if self.extensions().get::<Vec<Cookie>>().is_none() {
            let cookies = parse_cookies(self);
            self.extensions_mut().insert(cookies);
        }

        if self.extensions().get::<Arc<dyn SessionManager>>().is_none() {
            self.extensions_mut().insert(global_session_manager());
        }

        if self.extensions().get::<StandardSession>().is_none() {
            load_working_session(self);
        }

        self.extensions_mut().insert(Prepared);
    }

    fn clean_up(&mut self) {
        let manager = session_manager(self);
        save_working_session(self, &manager);

        let extensions = self.extensions_mut();
        extensions.remove::<OneMap>();
        extensions.remove::<ViewModel>();
        extensions.remove::<Vec<Cookie>>();
        extensions.remove::<StandardSession>();
        extensions.remove::<RequestedSessionId>();
        extensions.remove::<Arc<dyn SessionManager>>();
        extensions.remove::<RequestContextPath>();
        extensions.remove::<Prepared>();
    }

    fn is_secure(&self) -> bool {
        self.uri().scheme() == Some(&Scheme::HTTPS)
    }

    fn remote_addr(&self) -> Option<&SocketAddr> {
        self.extensions().get::<SocketAddr>()
    }

    fn shared(&mut self) -> &HttpRequestShare {
        if self.extensions().get::<HttpRequestShare>().is_none() {
            let shared = HttpRequestShare::from(self as &dyn HttpRequest);
            self.extensions_mut().insert(shared);
        }

        self.extensions()
            .get::<HttpRequestShare>()
            .expect("HttpRequestShare was just inserted")
    }
}

/// Returns the session manager of the request, falling back to the
/// process-wide default.
fn session_manager(request: &Request) -> Arc<dyn SessionManager> {
    request
        .extensions()
        .get::<Arc<dyn SessionManager>>()
        .cloned()
        .unwrap_or_else(global_session_manager)
}

/// Returns the request attribute map, creating it when it is missing.
fn attribute_map(request: &mut Request) -> &mut OneMap {
    if request.extensions().get::<OneMap>().is_none() {
        request.extensions_mut().insert(OneMap::new());
    }

    request
        .extensions_mut()
        .get_mut::<OneMap>()
        .expect("attribute map was just inserted")
}

/// Loads the session named by the session cookie into the request, if it
/// resolves to a live session.
fn load_working_session(request: &mut Request) {
    let Some(id) = session_cookie_value(request) else {
        return;
    };

    let manager = session_manager(request);
    let Some(mut session) = manager.load_session(&id) else {
        // Keep the requested id so `is_requested_session_id_valid` and
        // `requested_session_id` can report the stale cookie.
        request.extensions_mut().insert(RequestedSessionId(id));
        return;
    };

    // The client echoed the cookie, so it knows about the session already.
    session.set_new(false);

    let extensions = request.extensions_mut();
    extensions.insert(RequestedSessionId(id));
    extensions.insert(session);
}

/// Creates a new session registered with the manager and stores a working copy
/// on the request.
fn create_working_session(request: &mut Request) {
    let manager = session_manager(request);
    let id = crate::http::session::generate_session_id();
    manager.create_session_with_id(id.clone());

    if let Some(session) = manager.load_session(&id) {
        request.extensions_mut().insert(session);
    }
}

/// Writes the working session of the request back to the manager.
fn save_working_session(request: &Request, manager: &Arc<dyn SessionManager>) {
    if let Some(session) = request.extensions().get::<StandardSession>() {
        manager.put_session(session.clone());
    }
}

/// Returns the value of the session cookie, if the request carries one.
fn session_cookie_value(request: &Request) -> Option<String> {
    request
        .extensions()
        .get::<Vec<Cookie>>()
        .and_then(|cookies| {
            cookies
                .iter()
                .find(|cookie| cookie.name() == SESSION_COOKIE_NAME)
        })
        .map(|cookie| cookie.value().to_string())
}

/// Parses the `Cookie` header of the request into [`Cookie`] values.
fn parse_cookies(request: &Request) -> Vec<Cookie> {
    let Some(cookie_header) = request.headers().typed_get::<HeaderCookie>() else {
        return Vec::new();
    };
    cookie_header
        .iter()
        .map(|(name, value)| Cookie::new(name, Some(value.to_owned())))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{borrow::Cow, sync::Arc};

    use axum::{
        body::Body,
        http::header::{COOKIE, HeaderName},
    };

    use super::{HttpRequest, Request, RequestContextPath};
    use crate::{
        error::BoxError,
        http::session::SESSION_COOKIE_NAME,
        http::{HttpResponse, ResourceDispatcher},
    };

    fn session_request(id: &str) -> Request {
        Request::builder()
            .header(
                HeaderName::from_static("cookie"),
                format!("{SESSION_COOKIE_NAME}={id}"),
            )
            .body(Body::empty())
            .unwrap()
    }

    #[test]
    fn attributes_work_without_an_explicit_ready() {
        let mut request = Request::builder().body(Body::empty()).unwrap();

        assert!(request.get_attribute("missing").is_none());
        request.set_attribute("user", "alice".into());
        assert_eq!(
            request
                .get_attribute("user")
                .and_then(|value| value.as_str()),
            Some("alice")
        );

        request.remove_attribute("user");
        assert!(request.get_attribute("user").is_none());
    }

    #[test]
    fn cookies_are_parsed_during_ready() {
        let mut request = Request::builder()
            .header(COOKIE, "first=1; second=2")
            .body(Body::empty())
            .unwrap();
        request.ready();

        let cookies = request.cookies().expect("cookies are parsed by ready");
        assert_eq!(cookies.len(), 2);
        assert_eq!(request.cookie().map(|cookie| cookie.name()), Some("first"));
        assert_eq!(request.cookie().map(|cookie| cookie.value()), Some("1"));
    }

    #[test]
    fn a_session_survives_between_requests() {
        let mut first = Request::builder().body(Body::empty()).unwrap();
        first.ready();

        let id = {
            let session = first.session_mut(true).expect("a session is created");
            session.set_attribute("user", "alice".into());
            session.id().to_string()
        };
        first.clean_up();

        let mut second = session_request(&id);
        second.ready();

        assert!(second.is_requested_session_id_valid());
        assert_eq!(second.requested_session_id(), Some(id.as_str()));
        let session = second.session().expect("the session was loaded");
        assert!(!session.is_new());
        assert_eq!(
            session.attribute("user").and_then(|value| value.as_str()),
            Some("alice")
        );
    }

    #[test]
    fn changing_the_session_id_moves_the_state() {
        let mut request = Request::builder().body(Body::empty()).unwrap();
        request.ready();

        let old_id = {
            let session = request.session_mut(true).expect("a session is created");
            session.set_attribute("user", "alice".into());
            session.id().to_string()
        };

        let new_id = request.change_session_id();
        assert_ne!(new_id, old_id);
        assert_eq!(
            request.session().map(|session| session.id()),
            Some(new_id.as_str())
        );
        assert_eq!(
            request
                .session()
                .and_then(|session| session.attribute("user"))
                .and_then(|value| value.as_str()),
            Some("alice")
        );
    }

    #[test]
    fn a_stale_session_cookie_is_reported_but_not_valid() {
        let mut request = session_request("does-not-exist");
        request.ready();

        assert_eq!(request.requested_session_id(), Some("does-not-exist"));
        assert!(!request.is_requested_session_id_valid());
        assert!(request.session().is_none());
    }

    #[test]
    fn a_request_may_override_the_context_path() {
        let mut request = Request::builder().body(Body::empty()).unwrap();
        request
            .extensions_mut()
            .insert(RequestContextPath("/custom".to_string()));

        assert_eq!(request.context_path(), Some("/custom"));
    }

    #[test]
    fn an_installed_dispatcher_is_returned() {
        struct NoopDispatcher;

        impl ResourceDispatcher for NoopDispatcher {
            fn include(
                &self,
                _path: &str,
                _request: &dyn HttpRequest,
                _response: &mut dyn HttpResponse,
            ) -> Result<(), BoxError> {
                Ok(())
            }
        }

        let mut request = Request::builder().body(Body::empty()).unwrap();
        assert!(request.resource_dispatcher().is_none());

        let dispatcher: Arc<dyn ResourceDispatcher> = Arc::new(NoopDispatcher);
        request.extensions_mut().insert(dispatcher);

        assert!(request.resource_dispatcher().is_some());
    }

    #[test]
    fn the_request_line_is_reported_the_way_a_servlet_reads_it() {
        let mut request = Request::builder()
            .uri("https://api.example.com:8443/orders?page=2&tag=a&tag=b")
            .header("host", "api.example.com")
            .header("content-type", "application/json")
            .body(Body::empty())
            .unwrap();
        request.ready();

        assert_eq!(request.path(), "/orders");
        assert_eq!(request.query(), Some("page=2&tag=a&tag=b"));
        assert_eq!(request.host(), Some("api.example.com"));
        assert_eq!(request.server_port(), Some(8443));
        assert_eq!(request.server_name(), Some("api.example.com".to_owned()));
        assert_eq!(request.content_type(), Some("application/json"));
        assert!(request.is_secure());

        assert_eq!(request.parameter("page"), Some("2"));
        assert_eq!(
            request.parameter_values("tag"),
            Some(vec![Cow::from("a"), Cow::from("b")])
        );
        assert_eq!(
            request.parameters().map(|pairs| pairs.len()),
            Some(3),
            "every query parameter is reported"
        );
    }

    #[test]
    fn an_empty_accept_language_falls_back_to_the_system_locale() {
        let mut request = Request::builder()
            .header("accept-language", "")
            .body(Body::empty())
            .unwrap();
        request.ready();

        // The header is present but empty, which is not the same as absent: the
        // application still has to answer in some language.
        let locales = request.locales().expect("an empty header still answers");
        assert_eq!(locales.len(), 1);
        assert_eq!(request.locale().is_some(), false);
    }

    #[test]
    fn the_shared_snapshot_can_be_read_more_than_once() {
        let mut request = Request::builder()
            .uri("https://example.com/orders?page=2")
            .body(Body::empty())
            .unwrap();
        request.ready();

        {
            let shared = request.shared();
            assert_eq!(shared.path(), "/orders");
            assert_eq!(shared.query(), Some("page=2"));
        }

        // The snapshot is cached on the request rather than rebuilt, and the
        // borrow is what keeps a caller from holding it across a second call: the
        // reference is tied to `&mut self`, so the two can never overlap.
        let shared = request.shared();
        assert_eq!(shared.path(), "/orders");
    }
}
