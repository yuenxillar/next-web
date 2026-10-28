use crate::{
    error::BoxError,
    http::{HttpRequest, HttpResponse},
};

/// Writes a resource of the application into a response.
///
/// A dispatcher renders a resource — a page, a stylesheet, an error document —
/// in the middle of a request, so a filter can answer with a file of the
/// application without sending the client somewhere else.
///
/// A dispatcher cannot run another route. Running one means awaiting the router
/// of the application, which the filters that ask for a dispatch cannot do,
/// because the dispatch of the servlet API they mirror is synchronous. What a
/// dispatcher can do without awaiting anything is read a resource and write it
/// into the response, which is all this trait allows.
pub trait ResourceDispatcher
where
    Self: Send + Sync,
{
    /// Renders the resource named by `path` into `response`.
    ///
    /// This is the dispatch a filter uses when the resource is a template: the
    /// resource names the template, the request provides the model, and the
    /// rendered result is written into the response. The status line of the
    /// response is left to the caller.
    ///
    /// The default does nothing and answers `Ok(())`, so a dispatcher that has
    /// no template to render does not have to implement it.
    /// [`DefaultResourceDispatcher`](crate::http::DefaultResourceDispatcher)
    /// overrides it and renders the resource with the template engine of its
    /// loader.
    ///
    /// # Errors
    ///
    /// Returns an error when the resource does not exist, cannot be read, or
    /// cannot be rendered.
    fn forward(
        &self,
        path: &str,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), BoxError> {
        let _ = (path, request, response);

        Ok(())
    }

    /// Writes the resource named by `path` into `response`.
    ///
    /// `path` names the resource the way a request names it, such as
    /// `"/login.html"`, so the caller passes the URL it wants to render instead
    /// of holding a dispatcher per resource. Unlike
    /// [`forward`](Self::forward), which renders a template, an include writes
    /// the bytes of the resource as they are. The request is handed in so an
    /// implementation can read the state of the request it writes for.
    ///
    /// # Errors
    ///
    /// Returns an error when the resource does not exist or cannot be read. The
    /// response is left as it was in that case.
    fn include(
        &self,
        path: &str,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), BoxError>;
}
