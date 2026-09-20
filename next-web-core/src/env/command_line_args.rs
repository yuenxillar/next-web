use std::collections::{HashMap, HashSet};

/// A simple representation of command line arguments, broken into
/// option arguments and non-option arguments.
///
/// Option arguments are keyed by name and may have zero or more values
/// associated with each name. Non-option arguments are kept in the order
/// they appeared.
///
/// # See also
///
/// - [`SimpleCommandLineArgsParser`](crate::SimpleCommandLineArgsParser)
#[derive(Debug, Default, Clone)]
pub struct CommandLineArgs {
    option_args: HashMap<String, Vec<String>>,
    non_option_args: Vec<String>,
}

impl CommandLineArgs {
    /// Add an option argument for the given option name, and add the given value
    /// to the list of values associated with this option (of which there may be
    /// zero or more).
    ///
    /// The given value may be `None`, indicating that the option was specified
    /// without an associated value — for example, `--foo` vs. `--foo=bar`.
    pub fn add_option_arg(&mut self, option_name: String, option_value: Option<String>) {
        let values = self.option_args.entry(option_name).or_default();
        if let Some(value) = option_value {
            values.push(value);
        }
    }

    /// Return the set of the names of all option arguments present on the
    /// command line.
    pub fn option_names(&self) -> HashSet<&str> {
        self.option_args.keys().map(String::as_str).collect()
    }

    /// Return whether the option with the given name was present on the
    /// command line.
    pub fn contains_option(&self, option_name: &str) -> bool {
        self.option_args.contains_key(option_name)
    }

    /// Return the list of values associated with the given option.
    ///
    /// `None` signifies that the option was not present on the command line.
    /// An empty slice signifies that no values were associated with this option
    /// (i.e. the option was present but specified without a value, such as
    /// `--foo`).
    pub fn option_values(&self, option_name: &str) -> Option<&[String]> {
        self.option_args.get(option_name).map(Vec::as_slice)
    }

    /// Add the given value to the list of non-option arguments.
    pub fn add_non_option_arg(&mut self, value: String) {
        self.non_option_args.push(value);
    }

    /// Return the list of non-option arguments specified on the command line.
    pub fn non_option_args(&self) -> &[String] {
        &self.non_option_args
    }
}
