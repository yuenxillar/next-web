/// Represents the next version of the application.
pub struct NextVersion;

impl NextVersion {
    /// Returns the version of the application, if available.
    pub fn version() -> Option<&'static str> {
        option_env!("CARGO_PKG_VERSION")
    }
}
