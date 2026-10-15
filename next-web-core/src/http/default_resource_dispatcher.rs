//! The [`DefaultResourceDispatcher`], which renders a resource of the
//! application into a response.
//!
//! The dispatcher reads a resource through the [`ResourceLoader`] of the
//! application, so it can answer with a login page, an error page, or any other
//! file of the resources. It implements [`ResourceDispatcher`], the trait the
//! filters of the application use to answer with a resource, and it does not
//! reach the routes of the application: a dispatch is synchronous, and running
//! a route means awaiting the router.
//!
//! [`ResourceDispatcher::include`] writes the bytes of the resource as they
//! are, and [`ResourceDispatcher::forward`] renders the resource as a template
//! with the [template engine](crate::web::TemplateEngine) of the loader, which
//! is what a page that contains template syntax needs.
//!
//! # The path of the resource
//!
//! The path given to [`ResourceDispatcher::include`] is processed the way an
//! incoming request URI is: the query string is removed, the path parameters of
//! every segment (`;jsessionid=...`) are removed, the path is normalized (empty
//! segments and `.` are dropped, `..` removes the segment before it), and the
//! result is percent-decoded. The decoded path is normalized again, and a path
//! that the second normalization changes is refused, which is what keeps a
//! `%2e%2e` from escaping the root through its encoding. What is left, relative
//! to the root of the loader, is the key of the resource.
//!
//! ```ignore
//! let dispatcher = DefaultResourceDispatcher::new();
//! dispatcher.include("/login.html", &request, &mut response)?;
//! ```

use std::{
    path::Path,
    sync::{Arc, OnceLock},
};

use crate::{
    error::BoxError,
    http::{HttpRequest, HttpResponse, ResourceDispatcher},
    io::{ResourceLoader, web_resource_loader},
    web::TemplateEngine,
};

/// The [`ResourceDispatcher`] backed by the [`ResourceLoader`] of the
/// application.
///
/// The dispatcher is bound to a loader, not to a resource: the path of the
/// resource is an argument of [`include`](ResourceDispatcher::include), so one
/// dispatcher serves every resource of the application.
///
/// [`new`](Self::new) uses the shared loader, which reads the `resources`
/// directory of the application, and [`with_loader`](Self::with_loader) uses a
/// loader the caller owns. [`include`](ResourceDispatcher::include) writes the
/// resource as it is, and [`forward`](ResourceDispatcher::forward) renders it
/// with the templates compiled from the same loader.
pub struct DefaultResourceDispatcher {
    /// The loader the resources are read from.
    loader: Arc<dyn ResourceLoader>,
    /// The templates of the loader, compiled the first time a forward runs.
    ///
    /// A loader whose templates do not compile keeps the reason it failed, so
    /// every forward reports the same message instead of compiling again.
    templates: OnceLock<Result<TemplateEngine, String>>,
}

impl DefaultResourceDispatcher {
    /// Creates a dispatcher that reads from the loader of the web layer.
    ///
    /// An application installs the loader the web layer reads from while it
    /// starts, which is the loader built from the configuration of the
    /// resources; without one, the shared loader of the framework is used. Either
    /// loader reads the `resources` directory of the application, so this is the
    /// dispatcher of an application that serves the files of that directory.
    pub fn new() -> Self {
        Self::with_loader(web_resource_loader())
    }

    /// Creates a dispatcher that reads from `loader`.
    pub fn with_loader(loader: Arc<dyn ResourceLoader>) -> Self {
        Self {
            loader,
            templates: OnceLock::new(),
        }
    }

    /// Returns the loader the resources are read from.
    pub fn loader(&self) -> &Arc<dyn ResourceLoader> {
        &self.loader
    }

    /// Returns the templates of the loader, compiled on first use.
    fn templates(&self) -> Result<&TemplateEngine, BoxError> {
        match self.templates.get_or_init(|| {
            TemplateEngine::from_loader(self.loader.as_ref()).map_err(|error| error.to_string())
        }) {
            Ok(templates) => Ok(templates),
            Err(message) => Err(message.clone().into()),
        }
    }
}

impl Default for DefaultResourceDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceDispatcher for DefaultResourceDispatcher {
    fn forward(
        &self,
        path: &str,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), BoxError> {
        let location = resource_location(path)?;
        // The attributes of the request are the model of the template, which is
        // how an error page renders what a filter stored on the request.
        let context = TemplateEngine::context_of(request);
        let document = self.templates()?.render(&location, &context)?;

        // The status line of the response belongs to the caller of a forward,
        // so it is left alone; the body is the rendered document.
        let _ = response.insert_header("content-type", "text/html; charset=utf-8");
        let _ = response.insert_header("content-length", &document.len().to_string());
        response.set_body(document.into_bytes());

        Ok(())
    }

    fn include(
        &self,
        path: &str,
        _request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), BoxError> {
        let location = resource_location(path)?;

        let resource = self
            .loader
            .get_resource(&location)
            .map_err(|error| format!("the resource `{path}` is not available: {error}"))?;
        let content = resource
            .get_content()
            .map_err(|error| format!("the resource `{path}` could not be read: {error}"))?
            .into_owned();

        // The status line of the response belongs to the caller of an include,
        // so it is left alone. The content type and the length are those of the
        // resource, because the body is.
        let _ = response.insert_header("content-type", content_type(&location));
        let _ = response.insert_header("content-length", &content.len().to_string());
        response.set_body(content);

        Ok(())
    }
}

/// Turns the path of a resource into the key the loader knows it by.
///
/// The path is processed the way an incoming request URI is, and a path that
/// names something outside the root, or whose encoding hides such a path, is
/// refused. The result is relative to the root of the loader and never starts
/// with a separator.
///
/// # Errors
///
/// Returns an error when `path` is not an absolute path, when it names the
/// root itself, when it escapes the root, or when its encoding is invalid or
/// hides an escape.
fn resource_location(path: &str) -> Result<String, BoxError> {
    if !path.starts_with('/') {
        return Err(format!("the resource path must start with a forward slash: {path}").into());
    }

    // The query string of the path is removed first, the way an incoming
    // request URI is processed.
    let uri = path.split_once('?').map_or(path, |(uri, _)| uri);
    // Path parameters belong to a segment, not to the name of the resource, so
    // `/app;jsessionid=7f3c/main.js` names `app/main.js`.
    let uri = strip_path_params(uri);
    let normalized = normalize(&uri).ok_or_else(|| format!("invalid path: {path}"))?;

    // The decoded path is normalized again. A path that the second
    // normalization changes, such as `/..%2f`, is an attempt to escape the root
    // through its encoding and is refused.
    let decoded = percent_decode(&normalized)
        .ok_or_else(|| format!("the path is not valid percent-encoding: {path}"))?;
    match normalize(&decoded) {
        Some(re_normalized) if re_normalized == decoded => {}
        _ => return Err(format!("invalid path: {path}").into()),
    }

    let location = decoded.trim_start_matches('/').to_owned();
    if location.is_empty() {
        return Err(format!("the path does not name a resource: {path}").into());
    }

    Ok(location)
}

/// Removes the path parameters of every segment.
///
/// A parameter starts at a `;` and reaches the separator that follows it or the
/// end of the path, so `/app;jsessionid=7f3c/main.js` becomes `/app/main.js`.
fn strip_path_params(path: &str) -> String {
    if !path.contains(';') {
        return path.to_owned();
    }

    let mut result = String::with_capacity(path.len());
    let mut chars = path.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == ';' {
            while let Some(&next_ch) = chars.peek() {
                if next_ch == '/' {
                    break;
                }
                chars.next();
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Replaces every run of separators with a single separator.
fn collapse_slashes(path: &str) -> String {
    let mut result = String::with_capacity(path.len());
    let mut last_was_slash = false;

    for ch in path.chars() {
        if ch == '/' {
            if !last_was_slash {
                result.push(ch);
                last_was_slash = true;
            }
        } else {
            result.push(ch);
            last_was_slash = false;
        }
    }

    result
}

/// Normalizes a path.
///
/// Empty segments and `.` are dropped, `..` removes the segment before it, and
/// a path that would move above the root is rejected. The trailing separator of
/// the input is kept, so the caller can tell a directory from a file.
fn normalize(path: &str) -> Option<String> {
    let collapsed = collapse_slashes(path);
    let absolute = collapsed.starts_with('/');

    let mut segments: Vec<&str> = Vec::new();
    for segment in collapsed.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop()?;
            }
            _ => segments.push(segment),
        }
    }

    let mut normalized = String::with_capacity(collapsed.len());
    if absolute {
        normalized.push('/');
    }
    normalized.push_str(&segments.join("/"));

    if collapsed.ends_with('/') && !normalized.ends_with('/') {
        normalized.push('/');
    }

    Some(normalized)
}

/// Percent-decodes a path, keeping every character that is not an escape.
///
/// Only `%XX` is decoded, so a `+` stays a `+` instead of becoming a space.
/// Returns `None` when an escape is malformed or when the decoded bytes are not
/// valid UTF-8.
fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' {
            let escape = bytes.get(index + 1..index + 3)?;
            let high = (escape[0] as char).to_digit(16)?;
            let low = (escape[1] as char).to_digit(16)?;
            decoded.push((high * 16 + low) as u8);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }

    String::from_utf8(decoded).ok()
}

/// Returns the content type of a resource, guessed from its extension.
fn content_type(location: &str) -> &'static str {
    let extension = Path::new(location)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    match extension.as_str() {
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" | "mjs" => "text/javascript",
        "json" | "map" => "application/json",
        "xml" => "application/xml",
        "txt" => "text/plain",
        "csv" => "text/csv",
        "md" => "text/markdown",
        "yaml" | "yml" => "application/yaml",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "pdf" => "application/pdf",
        "wasm" => "application/wasm",
        "zip" => "application/zip",
        "gz" => "application/gzip",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use axum::{body::Body, extract::Request};

    use super::{DefaultResourceDispatcher, content_type, percent_decode, resource_location};
    use crate::{
        http::{HttpRequest, HttpResponse, ResourceDispatcher},
        http::{HttpResponseShare, StatusCode},
        io::{DefaultResourceLoader, ResourceLoader},
        web::ViewModel,
    };

    /// The number of resources directories this process has created.
    static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

    /// Creates a loader over a temporary resources directory holding `files`.
    ///
    /// The caller removes the directory it is given when the test is done.
    fn resources(files: &[(&str, &str)]) -> (PathBuf, Arc<DefaultResourceLoader>) {
        let root = std::env::temp_dir().join(format!(
            "next-web-default-resource-dispatcher-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        for (name, content) in files {
            let path = root.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        }

        let mut loader = DefaultResourceLoader::new(&root, None);
        loader.load().unwrap();

        (root, Arc::new(loader))
    }

    fn request() -> Request {
        Request::new(Body::empty())
    }

    #[test]
    fn a_path_is_reduced_to_the_key_of_a_resource() {
        assert_eq!(resource_location("/login.html").unwrap(), "login.html");
        assert_eq!(
            resource_location("/static/app.css").unwrap(),
            "static/app.css"
        );
        assert_eq!(
            resource_location("/login.html?next=%2Fhome").unwrap(),
            "login.html"
        );
        assert_eq!(
            resource_location("/app;jsessionid=7f3c/main.js").unwrap(),
            "app/main.js"
        );
        // The decoded path is normalized before it becomes a key.
        assert_eq!(resource_location("/a%20b/c.txt").unwrap(), "a b/c.txt");
        assert_eq!(resource_location("/a/./b/../c.txt").unwrap(), "a/c.txt");
        assert_eq!(resource_location("//a///b.txt").unwrap(), "a/b.txt");
    }

    #[test]
    fn a_path_that_escapes_the_root_is_refused() {
        assert!(resource_location("/../secret").is_err());
        assert!(resource_location("/a/../../secret").is_err());
        // An encoded `..` would escape the root once it is decoded.
        assert!(resource_location("/%2e%2e/secret").is_err());
        assert!(resource_location("/..%2fsecret").is_err());
        // Only an absolute path names a resource, and the root names none.
        assert!(resource_location("login.html").is_err());
        assert!(resource_location("/").is_err());
    }

    #[test]
    fn an_encoded_path_is_decoded() {
        assert_eq!(percent_decode("/a%20b").as_deref(), Some("/a b"));
        // A `+` of a path is a `+`, not a space.
        assert_eq!(percent_decode("/a+b").as_deref(), Some("/a+b"));
        assert!(percent_decode("/a%zz").is_none());
        assert!(percent_decode("/%ff").is_none());
    }

    #[test]
    fn the_content_type_follows_the_extension() {
        assert_eq!(content_type("login.html"), "text/html");
        assert_eq!(content_type("static/app.css"), "text/css");
        assert_eq!(content_type("static/app.min.js"), "text/javascript");
        assert_eq!(content_type("logo.PNG"), "image/png");
        assert_eq!(content_type("data.bin"), "application/octet-stream");
    }

    #[test]
    fn an_include_writes_the_resource_and_keeps_the_status_line() {
        let (root, loader) = resources(&[("login.html", "<h1>login</h1>")]);
        let dispatcher = DefaultResourceDispatcher::with_loader(loader);

        let mut response = HttpResponseShare::with_status(StatusCode::UNAUTHORIZED);
        let _ = response.insert_header("x-keep", "1");

        dispatcher
            .include("/login.html", &request(), &mut response)
            .unwrap();

        assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(response.header("x-keep"), Some("1"));
        assert_eq!(response.header("content-type"), Some("text/html"));
        assert_eq!(response.header("content-length"), Some("14"));
        assert_eq!(response.body(), b"<h1>login</h1>");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn one_dispatcher_serves_every_resource() {
        let (root, loader) = resources(&[("login.html", "login"), ("error.html", "error")]);
        let dispatcher = DefaultResourceDispatcher::with_loader(loader);

        let mut login = HttpResponseShare::new();
        dispatcher
            .include("/login.html", &request(), &mut login)
            .unwrap();
        assert_eq!(login.body(), b"login");

        let mut error = HttpResponseShare::new();
        dispatcher
            .include("/error.html", &request(), &mut error)
            .unwrap();
        assert_eq!(error.body(), b"error");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_include_answers_with_a_static_asset() {
        let (root, loader) = resources(&[
            ("static/app.css", "body { color: red }"),
            ("static/logo.png", "png-bytes"),
        ]);
        let dispatcher = DefaultResourceDispatcher::with_loader(loader);

        let mut stylesheet = HttpResponseShare::new();
        dispatcher
            .include("/static/app.css", &request(), &mut stylesheet)
            .unwrap();
        assert_eq!(stylesheet.header("content-type"), Some("text/css"));
        assert_eq!(stylesheet.body(), b"body { color: red }");

        let mut image = HttpResponseShare::new();
        dispatcher
            .include("/static/logo.png", &request(), &mut image)
            .unwrap();
        assert_eq!(image.header("content-type"), Some("image/png"));
        assert_eq!(image.body(), b"png-bytes");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_forward_renders_the_template_of_the_loader() {
        let (root, loader) = resources(&[("login.html", "{% if true %}login{% endif %}")]);
        let dispatcher = DefaultResourceDispatcher::with_loader(loader);

        let mut response = HttpResponseShare::with_status(StatusCode::UNAUTHORIZED);

        dispatcher
            .forward("/login.html", &request(), &mut response)
            .unwrap();

        assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            response.header("content-type"),
            Some("text/html; charset=utf-8")
        );
        assert_eq!(response.body(), b"login");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_forward_of_a_template_that_does_not_exist_is_reported() {
        let (root, loader) = resources(&[("login.html", "login")]);
        let dispatcher = DefaultResourceDispatcher::with_loader(loader);

        let mut response = HttpResponseShare::new();

        assert!(
            dispatcher
                .forward("/missing.html", &request(), &mut response)
                .is_err()
        );
        assert!(response.body().is_empty());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_forward_renders_the_template_with_the_view_model_of_the_request() {
        let (root, loader) = resources(&[("error.html", "<p>{{ error }}</p>")]);
        let dispatcher = DefaultResourceDispatcher::with_loader(loader);

        let mut request = request();
        let mut model = ViewModel::new();
        model.insert("error", tera::Value::from("Bad credentials"));
        request.set_view_model(model);

        let mut response = HttpResponseShare::new();
        dispatcher
            .forward("/error.html", &request, &mut response)
            .unwrap();

        assert_eq!(response.body(), b"<p>Bad credentials</p>");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_missing_resource_is_reported() {
        let (root, loader) = resources(&[("present.html", "present")]);
        let dispatcher = DefaultResourceDispatcher::with_loader(loader);

        let mut response = HttpResponseShare::new();
        assert!(
            dispatcher
                .include("/missing.html", &request(), &mut response)
                .is_err()
        );
        assert!(response.body().is_empty());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_relative_path_is_refused() {
        let dispatcher = DefaultResourceDispatcher::new();
        let mut response = HttpResponseShare::new();

        assert!(
            dispatcher
                .include("login.html", &request(), &mut response)
                .is_err()
        );
    }
}
