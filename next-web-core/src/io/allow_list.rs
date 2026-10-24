//! The rule the policies that allow a fixed set of file names and extensions
//! share.

use std::path::Path;

/// Returns whether `location` names a file the policy allows.
///
/// A location is allowed when its file stem is one of `names` (for example,
/// `application` for `application.yml`), or when its extension is one of
/// `extensions`, compared case-insensitively.
///
/// A location that attempts to escape its base directory with `..` is refused,
/// and so is one whose name and extension are both unknown.
pub(crate) fn allows(location: &str, extensions: &[&str], names: &[&str]) -> bool {
    // Normalize separators and reject any path that attempts to escape its
    // base directory via `..`.
    let normalized = location.replace('\\', "/");
    if normalized.split('/').any(|segment| segment == "..") {
        return false;
    }

    let path = Path::new(&normalized);

    // 1. Check whether the file stem matches one of the allowed names. This
    //    covers files that either carry no extension at all (`config`) or are
    //    known by their base name regardless of extension (`application.yml`).
    if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
        if names.iter().any(|name| stem.eq_ignore_ascii_case(name)) {
            return true;
        }
    }

    // 2. Check whether the file extension matches one of the allowed
    //    extensions, compared case-insensitively.
    if let Some(extension) = path.extension().and_then(|extension| extension.to_str()) {
        return extensions
            .iter()
            .any(|allowed| extension.eq_ignore_ascii_case(allowed));
    }

    false
}

#[cfg(test)]
mod tests {
    use super::allows;

    #[test]
    fn an_allowed_extension_is_accepted_case_insensitively() {
        assert!(allows("app.css", &["css"], &[]));
        assert!(allows("static/logo.PNG", &["png"], &[]));
        assert!(!allows("app.css", &["js"], &[]));
    }

    #[test]
    fn an_allowed_name_is_accepted() {
        assert!(allows("application.yaml", &["yaml"], &["application"]));
        assert!(allows("config", &[], &["config"]));
        assert!(!allows("other.yaml", &[], &["application"]));
    }

    #[test]
    fn a_location_that_escapes_its_base_is_refused() {
        assert!(!allows("../app.css", &["css"], &[]));
        assert!(!allows("static/../../app.css", &["css"], &[]));
    }
}
