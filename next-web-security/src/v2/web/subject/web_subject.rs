use std::sync::{Arc, Mutex};

use next_web_core::http::{HttpRequestShare, HttpResponseShare};

use crate::core::subject::Subject;

/// The subject of a web call.
///
/// Apache Shiro's `WebSubject` also gives the subject the request and the response
/// it serves. The port hands them over as shared snapshots 鈥?`Arc<HttpRequestShare>`
/// and `Arc<Mutex<HttpResponseShare>>` 鈥?because a request is borrowed mutably by
/// the filter chain for as long as it runs, while the subject, a filter and a task
/// spawned for the call all have to read the same request and write the same
/// response.
pub trait WebSubject: Subject {
    /// Returns the request the subject was built for, when it has one.
    fn request_share(&self) -> Option<&Arc<HttpRequestShare>> {
        None
    }

    /// Returns the response the subject writes to, when it has one.
    fn response_share(&self) -> Option<&Arc<Mutex<HttpResponseShare>>> {
        None
    }

    /// Gives the subject the request and the response of the call it serves.
    ///
    /// # Arguments
    ///
    /// * `request` - The snapshot of the request of the call.
    /// * `response` - The response of the call, shared so the subject and the
    ///   filter chain write the same one.
    fn attach_shared_context(
        &mut self,
        _request: Arc<HttpRequestShare>,
        _response: Arc<Mutex<HttpResponseShare>>,
    ) {
    }
}
next_web_core::clone_trait_object!(WebSubject);
