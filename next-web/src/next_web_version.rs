/// Exposes the Next Web version.
pub struct NextWebVersion;

impl NextWebVersion {
    /// Return the full version string of the present Next Web codebase.
    pub fn get_version() -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
}
