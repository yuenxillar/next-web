//! Renders views with Tera, from the resources of the application.
//!
//! Run it with `cargo run -p next-web-examples --bin template` and open one of
//! (the examples are served under the `/app` context path of
//! `resources/application.yaml`):
//!
//! * `http://127.0.0.1:11000/app/template/home` — a handler that returns a
//!   [`ModelAndView`]; the page is rendered from `resources/views/home.html`
//!   with the model the handler built.
//! * `http://127.0.0.1:11000/app/template/login` — a dispatch that forwards to
//!   `resources/views/login.html`; the model is the view model of the request,
//!   which is where `{{ error }}` comes from. Submitting the form answers with
//!   the home page of the user (POST to the same path).
//! * `http://127.0.0.1:11000/app/template/raw` — a dispatch that includes
//!   `resources/hello.json` as it is, without rendering it.
//! * `http://127.0.0.1:11000/app/template/app.css` — a static asset of
//!   `resources/static/`, served by the same dispatcher; the default policy of
//!   the resource loader allows the assets a page links to.
//!
//! The dispatcher the routes dispatch with is installed by the framework, which
//! adds it to the extensions of every request that matches a route, so the
//! example reads it from the request instead of installing one of its own.

use axum::{
    Form, Router,
    body::Body,
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use next_web::{
    Application, NextWebApplication,
    core::{ApplicationContext, traits::apply_router::ApplyRouter},
    macros::{bind::get_mapping, bind::singleton, order},
};
use next_web_core::{http::HttpRequest, tera::Value, web::ModelAndView};
use serde::Deserialize;

#[derive(Default)]
struct TestApplication;

impl Application for TestApplication {}

/// Answers with the home page, rendered by the handler itself.
///
/// The value a handler returns is rendered into the response, so a
/// [`ModelAndView`] answers with the document of its view.
#[get_mapping(path = "/template/home")]
async fn home() -> ModelAndView {
    ModelAndView::with_view_name("views/home.html")
        .add_object("title", "Template example")
        .add_object("user", "Alice")
}

/// Answers with the login page, rendered from the view model of the request.
async fn login(mut request: Request) -> Response {
    // The view model of the request is the model of the template, so this is
    // what `{{ error }}` renders: the attributes of the request are a store of
    // their own and are not rendered.
    if let Some(model) = request.view_model_mut() {
        model.insert("error", Value::from("Bad credentials"));
    }

    dispatch(&mut request, "/views/login.html", Dispatch::Forward)
}

/// The fields the login page submits.
#[derive(Deserialize)]
struct LoginForm {
    /// The name of the user that signed in.
    username: String,
    /// The password of the user, which the example does not read.
    #[allow(dead_code)]
    password: String,
}

/// Answers the submitted form with the home page of the user.
///
/// The login page asks for itself, so the form is submitted to the route that
/// rendered it, and the POST of the same path is what answers it. Without the
/// POST route the submission would be answered with
/// `405 Method Not Allowed`.
async fn submit(Form(form): Form<LoginForm>) -> ModelAndView {
    ModelAndView::with_view_name("views/home.html")
        .add_object("title", "Template example")
        .add_object("user", form.username)
}

/// Answers with a resource of the application, as it is.
async fn raw(mut request: Request) -> Response {
    dispatch(&mut request, "/hello.json", Dispatch::Include)
}

/// Answers with the stylesheet of the example, as a static asset.
async fn stylesheet(mut request: Request) -> Response {
    dispatch(&mut request, "/static/app.css", Dispatch::Include)
}

/// How the dispatcher writes the resource into the response.
enum Dispatch {
    /// Renders the resource as a template, with the view model of the request
    /// as its model.
    Forward,
    /// Writes the bytes of the resource as they are.
    Include,
}

/// Writes the resource named by `path` into a response.
fn dispatch(request: &mut Request, path: &str, dispatch: Dispatch) -> Response {
    let Some(dispatcher) = request.resource_dispatcher() else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "the request carries no resource dispatcher",
        )
            .into_response();
    };

    let mut response = Response::new(Body::empty());
    let result = match dispatch {
        Dispatch::Forward => dispatcher.forward(path, &*request, &mut response),
        Dispatch::Include => dispatcher.include(path, &*request, &mut response),
    };

    match result {
        Ok(()) => response,
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response(),
    }
}

/// Contributes the routes of the example.
#[order(0)]
#[singleton(binds = [Self::into_apply_router])]
#[derive(Clone)]
pub struct TemplateRouter;

impl TemplateRouter {
    fn into_apply_router(self) -> Box<dyn ApplyRouter> {
        Box::new(self)
    }
}

impl ApplyRouter for TemplateRouter {
    fn apply(&mut self, _ctx: &mut dyn ApplicationContext) -> Router {
        Router::new()
            .route("/template/login", get(login).post(submit))
            .route("/template/raw", get(raw))
            .route("/template/app.css", get(stylesheet))
    }
}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}
