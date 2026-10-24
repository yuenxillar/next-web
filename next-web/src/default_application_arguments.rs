//! Default implementation of [`ApplicationArguments`].

use std::collections::HashSet;

use next_web_core::env::{
    CommandLinePropertySourceExt, PropertySource, SimpleCommandLinePropertySource,
};

use crate::application_arguments::ApplicationArguments;

/// Default implementation of [`ApplicationArguments`].
///
/// Parses raw command line arguments in the form `--key=value` (option
/// arguments) and collects everything else as non-option arguments.
#[derive(Debug, Clone)]
pub struct DefaultApplicationArguments {
    source: SimpleCommandLinePropertySource,
    args: Vec<String>,
}

impl DefaultApplicationArguments {
    /// Creates a new [`DefaultApplicationArguments`] from the given raw arguments.
    ///
    /// # Arguments
    ///
    /// * `args` - The raw command line arguments. Must not be empty entries only;
    ///   the caller is responsible for passing a valid argument vector.
    ///
    /// # Returns
    ///
    /// A new instance wrapping the parsed arguments.
    pub fn new<I, E>(args: I) -> Self
    where
        I: IntoIterator<Item = E>,
        E: Into<String>,
    {
        let args: Vec<String> = args.into_iter().map(|a| a.into()).collect();
        let source = SimpleCommandLinePropertySource::new(&args);
        Self { source, args }
    }

    /// Returns the underlying parsed source.
    pub fn source(&self) -> &SimpleCommandLinePropertySource {
        &self.source
    }
}

impl ApplicationArguments for DefaultApplicationArguments {
    fn source_args(&self) -> &[String] {
        &self.args
    }

    fn option_names(&self) -> HashSet<&str> {
        self.source.property_names()
    }

    fn contains_option(&self, name: &str) -> bool {
        self.source.contains_property(name)
    }

    fn option_values(&self, name: &str) -> Option<&[String]> {
        self.source.option_values(name)
    }

    fn non_option_args(&self) -> &[String] {
        self.source.non_option_args()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_option_with_value() {
        let arguments = DefaultApplicationArguments::new(args(&["--foo=bar"]));
        assert!(arguments.contains_option("foo"));
        assert_eq!(
            arguments.option_values("foo"),
            Some(&["bar".to_string()][..])
        );
        assert!(arguments.non_option_args().is_empty());
    }

    #[test]
    fn parses_flag_option_without_value() {
        let arguments = DefaultApplicationArguments::new(args(&["--debug"]));
        assert!(arguments.contains_option("debug"));
        assert_eq!(arguments.option_values("debug"), Some(&[][..]));
    }

    #[test]
    fn parses_repeated_option_values() {
        let arguments = DefaultApplicationArguments::new(args(&["--foo=bar", "--foo=baz"]));
        assert_eq!(
            arguments.option_values("foo"),
            Some(&["bar".to_string(), "baz".to_string()][..])
        );
    }

    #[test]
    fn missing_option_returns_none() {
        let arguments = DefaultApplicationArguments::new(args(&["--foo=bar"]));
        assert!(!arguments.contains_option("missing"));
        assert_eq!(arguments.option_values("missing"), None);
    }

    #[test]
    fn collects_non_option_args() {
        let arguments = DefaultApplicationArguments::new(args(&["first", "--foo=bar", "second"]));
        assert_eq!(
            arguments.non_option_args(),
            &["first".to_string(), "second".to_string()][..]
        );
    }

    #[test]
    fn exposes_source_args() {
        let raw = args(&["--foo=bar", "file.txt"]);
        let arguments = DefaultApplicationArguments::new(raw.clone());
        assert_eq!(arguments.source_args(), raw.as_slice());
    }
}
