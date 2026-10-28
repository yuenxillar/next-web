//! The view a handler answers with, and the model it is rendered with.
//!
//! [`ModelAndView`] mirrors the holder of the Spring web MVC framework: it keeps
//! the name of a view and the model the view is rendered with, and it is what a
//! handler returns when the answer is a document rather than a body it wrote
//! itself.
//!
//! The view name is a template of the
//! [`ResourceLoader`](crate::io::ResourceLoader) of the application, so
//! `home.html` renders the resource `home.html` and `home` renders `home.html`
//! when that resource exists. The model is the [`Context`] of the template
//! engine, and its entries are the variables of the template.
//!
//! ```ignore
//! #[get_mapping("/home")]
//! async fn home() -> ModelAndView {
//!     ModelAndView::with_view_name("home")
//!         .add_object("title", "Home")
//! }
//! ```

use std::fmt;

use axum::{
    body::Body,
    http::{HeaderValue, header},
    response::{IntoResponse, Response},
};
use serde::Serialize;
use tera::Context;

use crate::{error::BoxError, http::StatusCode, web::template_engine::TemplateEngine};

/// The content type a rendered view answers with.
const VIEW_CONTENT_TYPE: &str = "text/html; charset=utf-8";

/// The content type of a view that could not be rendered.
const RENDER_ERROR_CONTENT_TYPE: &str = "text/plain; charset=utf-8";

/// The view a handler renders, and the model it is rendered with.
///
/// The two parts are independent: a view can be rendered with an empty model,
/// and a model can be filled before the view is chosen. A handler builds the
/// holder and returns it, and the response is rendered from the resources of
/// the application.
#[derive(Clone, Debug, Default)]
pub struct ModelAndView {
    /// The name of the view, which is the template to render.
    view: Option<String>,
    /// The model the view is rendered with.
    model: Context,
    /// The status the response is rendered with, when it is not `200 OK`.
    status: Option<StatusCode>,
    /// Whether the holder was cleared with [`clear`](Self::clear).
    cleared: bool,
}

impl ModelAndView {
    /// Creates an empty holder, without a view and without a model.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a holder that renders `view_name`.
    pub fn with_view_name(view_name: impl Into<String>) -> Self {
        let mut model_and_view = Self::new();
        model_and_view.set_view_name(view_name);

        model_and_view
    }

    /// Creates a holder that renders `view_name` with `model`.
    pub fn with_view_name_and_model(view_name: impl Into<String>, model: Context) -> Self {
        let mut model_and_view = Self::with_view_name(view_name);
        model_and_view.set_model(model);

        model_and_view
    }

    /// Creates a holder that renders `view_name` with `status`.
    pub fn with_view_name_and_status(view_name: impl Into<String>, status: StatusCode) -> Self {
        let mut model_and_view = Self::with_view_name(view_name);
        model_and_view.set_status(status);

        model_and_view
    }

    /// Creates a holder that renders `view_name` with `model` and `status`.
    pub fn with_view_name_model_and_status(
        view_name: impl Into<String>,
        model: Context,
        status: StatusCode,
    ) -> Self {
        let mut model_and_view = Self::with_view_name_and_model(view_name, model);
        model_and_view.set_status(status);

        model_and_view
    }

    /// Sets the name of the view, which is the template to render.
    ///
    /// The name replaces a view the holder already had.
    pub fn set_view_name(&mut self, view_name: impl Into<String>) {
        self.view = Some(view_name.into());
    }

    /// Removes the view of the holder, leaving the model in place.
    pub fn clear_view_name(&mut self) {
        self.view = None;
    }

    /// Returns the name of the view, if the holder has one.
    pub fn view_name(&self) -> Option<&str> {
        self.view.as_deref()
    }

    /// Returns whether the holder has a view to render.
    pub fn has_view(&self) -> bool {
        self.view.is_some()
    }

    /// Returns whether the view is named rather than rendered directly.
    ///
    /// A holder always refers to a view by its name, so this answers `true`
    /// whenever the holder has a view. It is kept for the code that is written
    /// against the Spring API.
    pub fn is_reference(&self) -> bool {
        self.view.is_some()
    }

    /// Returns the model the view is rendered with.
    pub fn model(&self) -> &Context {
        &self.model
    }

    /// Returns the model the view is rendered with, mutably.
    pub fn model_mut(&mut self) -> &mut Context {
        &mut self.model
    }

    /// Replaces the model the view is rendered with.
    pub fn set_model(&mut self, model: Context) {
        self.model = model;
    }

    /// Returns the status the response is rendered with, if it is not `200 OK`.
    pub fn status(&self) -> Option<StatusCode> {
        self.status
    }

    /// Sets the status the response is rendered with.
    pub fn set_status(&mut self, status: StatusCode) {
        self.status = Some(status);
    }

    /// Removes the status of the holder, which renders `200 OK` again.
    pub fn clear_status(&mut self) {
        self.status = None;
    }

    /// Adds `value` to the model under `name`.
    ///
    /// The holder is returned so a handler can build the model and the view in
    /// one expression:
    ///
    /// ```ignore
    /// ModelAndView::with_view_name("home").add_object("title", "Home")
    /// ```
    pub fn add_object(mut self, name: impl Into<String>, value: impl Serialize) -> Self {
        self.model.insert(name.into(), &value);

        self
    }

    /// Adds every `(name, value)` pair to the model.
    pub fn add_all_objects<I, K, V>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Serialize,
    {
        for (name, value) in values {
            self.model.insert(name.into(), &value);
        }

        self
    }

    /// Clears the holder: the view, the model, and the status are removed.
    ///
    /// [`was_cleared`](Self::was_cleared) answers `true` until something is
    /// added to the holder again, which is how a filter suppresses the
    /// rendering of a holder that a handler returned.
    pub fn clear(&mut self) {
        self.view = None;
        self.model = Context::new();
        self.status = None;
        self.cleared = true;
    }

    /// Returns whether the holder has neither a view nor a model.
    pub fn is_empty(&self) -> bool {
        self.view.is_none() && self.model == Context::new()
    }

    /// Returns whether the holder is empty because it was cleared.
    pub fn was_cleared(&self) -> bool {
        self.cleared && self.is_empty()
    }

    /// Renders the view with `engine`.
    ///
    /// # Errors
    ///
    /// Returns an error when the holder has no view, when the template of the
    /// view is not known to the engine, or when rendering it fails.
    pub fn render(&self, engine: &TemplateEngine) -> Result<String, BoxError> {
        let Some(view_name) = self.view.as_deref() else {
            return Err("the model and view has no view to render".into());
        };

        engine.render(view_name, &self.model)
    }
}

impl fmt::Display for ModelAndView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let view = match self.view.as_deref() {
            Some(view_name) => format!("\"{view_name}\""),
            None => "none".to_owned(),
        };

        write!(f, "ModelAndView [view={view}; model={:?}]", self.model)
    }
}

impl IntoResponse for ModelAndView {
    fn into_response(self) -> Response {
        let status = self.status.unwrap_or(StatusCode::OK);

        // A holder without a view has nothing to render, so the response is the
        // status alone. It is what a cleared holder answers with.
        if !self.has_view() {
            tracing::debug!("the model and view has no view to render");

            return status.into_response();
        }

        match self.render(TemplateEngine::shared()) {
            Ok(document) => {
                let mut response = Response::new(Body::from(document));
                *response.status_mut() = status;
                response.headers_mut().insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static(VIEW_CONTENT_TYPE),
                );

                response
            }
            Err(error) => {
                let view_name = self.view.as_deref().unwrap_or_default();
                tracing::error!("the view `{view_name}` could not be rendered: {error}");

                let mut response = Response::new(Body::from(format!(
                    "Failed to render `{view_name}`: {error}"
                )));
                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                response.headers_mut().insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static(RENDER_ERROR_CONTENT_TYPE),
                );

                response
            }
        }
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

    use axum::response::IntoResponse;
    use tera::Value;

    use super::ModelAndView;
    use crate::{
        http::StatusCode,
        io::{DefaultResourceLoader, ResourceLoader},
        web::template_engine::TemplateEngine,
    };

    /// The number of resources directories this process has created.
    static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

    /// Creates a loader over a temporary resources directory holding `files`.
    ///
    /// The caller removes the directory it is given when the test is done.
    fn resources(files: &[(&str, &str)]) -> (PathBuf, Arc<DefaultResourceLoader>) {
        let root = std::env::temp_dir().join(format!(
            "next-web-model-and-view-{}-{}",
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

    #[test]
    fn a_holder_holds_the_name_of_its_view() {
        let model_and_view = ModelAndView::with_view_name("home.html");

        assert!(model_and_view.has_view());
        assert!(model_and_view.is_reference());
        assert_eq!(model_and_view.view_name(), Some("home.html"));
        assert!(!model_and_view.is_empty());
        assert!(!model_and_view.was_cleared());
    }

    #[test]
    fn an_object_is_added_to_the_model() {
        let model_and_view = ModelAndView::with_view_name("home.html").add_object("title", "Home");

        assert_eq!(
            model_and_view.model().get("title").and_then(Value::as_str),
            Some("Home")
        );
    }

    #[test]
    fn every_object_of_a_map_is_added_to_the_model() {
        let model_and_view = ModelAndView::with_view_name("home.html")
            .add_all_objects([("first", 1i32), ("second", 2i32)]);

        assert_eq!(
            model_and_view.model().get("first").and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            model_and_view.model().get("second").and_then(Value::as_i64),
            Some(2)
        );
    }

    #[test]
    fn a_view_is_rendered_with_its_model() {
        let (root, loader) = resources(&[("home.html", "<h1>{{ title }}</h1>")]);
        let engine = TemplateEngine::from_loader(loader.as_ref()).unwrap();

        let model_and_view = ModelAndView::with_view_name("home.html").add_object("title", "Home");

        assert_eq!(model_and_view.render(&engine).unwrap(), "<h1>Home</h1>");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_holder_without_a_view_cannot_be_rendered() {
        let (root, loader) = resources(&[("home.html", "home")]);
        let engine = TemplateEngine::from_loader(loader.as_ref()).unwrap();

        assert!(ModelAndView::new().render(&engine).is_err());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_cleared_holder_is_empty() {
        let mut model_and_view =
            ModelAndView::with_view_name("home.html").add_object("title", "Home");

        model_and_view.clear();

        assert!(model_and_view.is_empty());
        assert!(model_and_view.was_cleared());
        assert!(!model_and_view.has_view());
        assert!(model_and_view.model().get("title").is_none());
    }

    #[test]
    fn a_status_is_carried_by_the_holder() {
        let mut model_and_view = ModelAndView::with_view_name("home.html");
        model_and_view.set_status(StatusCode::CREATED);

        assert_eq!(model_and_view.status(), Some(StatusCode::CREATED));
    }

    #[test]
    fn a_holder_without_a_view_answers_with_its_status() {
        let mut model_and_view = ModelAndView::new();
        model_and_view.set_status(StatusCode::NO_CONTENT);

        let response = model_and_view.into_response();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[test]
    fn a_view_that_cannot_be_rendered_answers_internal_server_error() {
        // The shared engine reads the resources of the crate, which does not
        // hold this template.
        let model_and_view = ModelAndView::with_view_name("next-web-missing-template.html");

        let response = model_and_view.into_response();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
