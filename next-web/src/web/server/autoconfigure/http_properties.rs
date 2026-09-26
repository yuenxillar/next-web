//! The HTTP configuration of the web server.
//!
//! The properties are part of [`ServerProperties`](super::ServerProperties) and
//! are bound from the `next.server.http` prefix of the environment of the
//! application:
//!
//! ```yaml
//! next:
//!   server:
//!     http:
//!       request:
//!         max_file_size: 10485760
//!         max_request_size: 10485760
//!         location: /var/lib/app/uploads
//!         trace: true
//!         timeout: 30
//! ```

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// The value of the request timeout when the environment configures none, in
/// seconds.
pub const DEFAULT_REQUEST_TIMEOUT_SECONDS: u64 = 5;

/// The HTTP configuration of the web server.
///
/// The configuration holds the settings of the requests the server accepts and
/// of the responses it writes, each of which is a group of its own, see
/// [`RequestProperties`] and [`ResponseProperties`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HttpProperties {
    /// The settings of the requests the server accepts.
    request: RequestProperties,

    /// The settings of the responses the server writes.
    response: ResponseProperties,
}

impl HttpProperties {
    /// Creates the HTTP configuration with the settings of its groups left at
    /// the defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the settings of the requests the server accepts.
    pub fn request(&self) -> &RequestProperties {
        &self.request
    }

    /// Returns the settings of the responses the server writes.
    pub fn response(&self) -> &ResponseProperties {
        &self.response
    }

    /// Replaces the settings of the requests the server accepts.
    pub fn set_request(&mut self, request: RequestProperties) {
        self.request = request;
    }

    /// Replaces the settings of the responses the server writes.
    pub fn set_response(&mut self, response: ResponseProperties) {
        self.response = response;
    }
}

/// The settings of the requests the web server accepts.
///
/// The sizes are byte counts, and the timeout is a number of seconds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RequestProperties {
    /// The largest size of the file of a request, in bytes.
    ///
    /// The setting limits the size of an uploaded file, and it defaults to
    /// `None`, which leaves the size of a file to the limit of the request.
    max_file_size: Option<usize>,

    /// The largest size of a request body, in bytes.
    ///
    /// The setting limits the size of the body the server reads, which is what
    /// keeps a large request from exhausting the memory of the application. It
    /// defaults to `None`, which leaves the size of a request to the server.
    max_request_size: Option<usize>,

    /// Whether every request the server serves is traced.
    ///
    /// The layer that traces the requests writes a span per request, which is
    /// the information the log of an application shows about the requests it
    /// served. It defaults to `false`.
    trace: bool,

    /// The directory the files of a request are written to.
    ///
    /// It defaults to `None`, which leaves the files to the location the
    /// application chooses itself, and it is normally a directory the process
    /// can write to, such as `/var/lib/app/uploads`.
    location: Option<String>,

    /// The time a request is given to complete, in seconds.
    ///
    /// The request is answered with the status `408 Request Timeout` when the
    /// time passes before its response is written. It defaults to
    /// [`DEFAULT_REQUEST_TIMEOUT_SECONDS`].
    timeout: u64,
}

impl RequestProperties {
    /// Creates the request settings with the given limits.
    ///
    /// # Arguments
    ///
    /// * `max_file_size` - The largest size of the file of a request, in bytes.
    /// * `max_request_size` - The largest size of a request body, in bytes.
    /// * `trace` - Whether every request is traced.
    /// * `location` - The directory the files of a request are written to.
    /// * `timeout` - The time a request is given to complete, in seconds.
    pub fn new(
        max_file_size: usize,
        max_request_size: usize,
        trace: bool,
        location: impl Into<String>,
        timeout: u64,
    ) -> Self {
        Self {
            max_file_size: Some(max_file_size),
            max_request_size: Some(max_request_size),
            trace,
            location: Some(location.into()),
            timeout,
        }
    }

    /// Returns the largest size of the file of a request, in bytes, when the
    /// environment configures one.
    pub fn max_file_size(&self) -> Option<usize> {
        self.max_file_size
    }

    /// Returns the largest size of a request body, in bytes, when the
    /// environment configures one.
    pub fn max_request_size(&self) -> Option<usize> {
        self.max_request_size
    }

    /// Returns whether every request the server serves is traced.
    pub fn trace(&self) -> bool {
        self.trace
    }

    /// Returns the directory the files of a request are written to, when the
    /// environment configures one.
    pub fn location(&self) -> Option<&str> {
        self.location.as_deref()
    }

    /// Returns the time a request is given to complete, in seconds.
    pub fn timeout(&self) -> u64 {
        self.timeout
    }

    /// Returns the time a request is given to complete, as a duration.
    pub fn timeout_duration(&self) -> Duration {
        Duration::from_secs(self.timeout)
    }

    /// Sets the largest size of the file of a request, in bytes.
    pub fn set_max_file_size(&mut self, max_file_size: usize) {
        self.max_file_size = Some(max_file_size);
    }

    /// Sets the largest size of a request body, in bytes.
    pub fn set_max_request_size(&mut self, max_request_size: usize) {
        self.max_request_size = Some(max_request_size);
    }

    /// Sets whether every request the server serves is traced.
    pub fn set_trace(&mut self, trace: bool) {
        self.trace = trace;
    }

    /// Sets the directory the files of a request are written to.
    pub fn set_location(&mut self, location: impl Into<String>) {
        self.location = Some(location.into());
    }

    /// Sets the time a request is given to complete, in seconds.
    pub fn set_timeout(&mut self, timeout: u64) {
        self.timeout = timeout;
    }
}

impl Default for RequestProperties {
    fn default() -> Self {
        Self {
            max_file_size: None,
            max_request_size: None,
            trace: false,
            location: None,
            timeout: DEFAULT_REQUEST_TIMEOUT_SECONDS,
        }
    }
}

/// The settings of the responses the web server writes.
///
/// The group is present so that the responses have a place of their own next to
/// the requests, and no setting is read from it yet.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ResponseProperties {}

impl ResponseProperties {
    /// Creates the response settings, which hold no value yet.
    pub fn new() -> Self {
        Self {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_leave_the_limits_to_the_server() {
        let properties = RequestProperties::default();

        assert_eq!(properties.max_file_size(), None);
        assert_eq!(properties.max_request_size(), None);
        assert_eq!(properties.location(), None);
        assert!(!properties.trace());
        assert_eq!(properties.timeout(), DEFAULT_REQUEST_TIMEOUT_SECONDS);
        assert_eq!(
            properties.timeout_duration(),
            Duration::from_secs(DEFAULT_REQUEST_TIMEOUT_SECONDS)
        );
    }

    #[test]
    fn the_settings_are_read_from_a_document() {
        let properties: HttpProperties = serde_yaml::from_str(
            r#"
            request:
              max_file_size: 1024
              max_request_size: 4096
              location: /tmp/uploads
              trace: true
              timeout: 30
            "#,
        )
        .expect("the document describes the HTTP configuration");

        assert_eq!(properties.request().max_file_size(), Some(1024));
        assert_eq!(properties.request().max_request_size(), Some(4096));
        assert_eq!(properties.request().location(), Some("/tmp/uploads"));
        assert!(properties.request().trace());
        assert_eq!(properties.request().timeout(), 30);
    }
}
