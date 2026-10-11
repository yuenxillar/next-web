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
//!   `resources/views/login.html`; the model is the attributes of the request,
//!   which is where `{{ error }}` comes from.
//! * `http://127.0.0.1:11000/app/template/raw` — a dispatch that includes
//!   `resources/hello.json` as it is, without rendering it.
//! * `http://127.0.0.1:11000/app/template/app.css` — a static asset of
//!   `resources/static/`, served by the same dispatcher; the default policy of
//!   the resource loader allows the assets a page links to.

use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use next_web::{
    Application, NextWebApplication,
    core::{ApplicationContext, traits::apply_router::ApplyRouter},
    macros::{bind::get_mapping, bind::singleton, order},
};
use next_web_core::{
    any::AnyValue,
    http::DefaultResourceDispatcher,
    traits::http::{http_request::HttpRequest, resource_dispatcher::ResourceDispatcher},
    web::ModelAndView,
};

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

/// Answers with the login page, rendered from the attributes of the request.
async fn login(mut request: Request) -> Response {
    // The attributes of the request are the model of the template, so this is
    // what `{{ error }}` renders.
    request.set_attribute("error", AnyValue::from("Bad credentials"));

    dispatch(&mut request, "/views/login.html", Dispatch::Forward)
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
    /// Renders the resource as a template, with the attributes of the request
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

/// Installs the dispatcher the routes of the example dispatch with.
///
/// An application installs the dispatcher once, where a request enters the
/// framework, so that every filter and every handler can reach it. The example
/// installs it in front of its own routes.
async fn install_resource_dispatcher(mut request: Request, next: Next) -> Response {
    request
        .extensions_mut()
        .insert(Arc::new(DefaultResourceDispatcher::new()) as Arc<dyn ResourceDispatcher>);

    next.run(request).await
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
            .route("/template/login", get(login))
            .route("/template/raw", get(raw))
            .route("/template/app.css", get(stylesheet))
            .layer(middleware::from_fn(install_resource_dispatcher))
    }
}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}
