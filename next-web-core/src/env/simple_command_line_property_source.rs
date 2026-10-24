//! A [`CommandLinePropertySource`] implementation backed by an instance of
//! [`CommandLineArgs`].

use std::collections::HashSet;
use std::fmt;

use crate::env::CommandLineArgs;
use crate::env::CommandLinePropertySource;
use crate::env::CommandLinePropertySourceExt;
use crate::env::PropertySource;
use crate::env::SimpleCommandLineArgsParser;

/// A [`CommandLinePropertySource`] implementation backed by an instance of
/// [`CommandLineArgs`].
///
/// # Purpose
///
/// This implementation aims to provide the simplest possible approach to
/// parsing command line arguments. Command line arguments are broken into two
/// distinct groups: *option arguments* and *non-option arguments*.
///
/// # Working with option arguments
///
/// Option arguments must adhere to the exact syntax:
///
/// ```text
/// --optName[=optValue]
/// ```
///
/// Options must be prefixed with `--` and may or may not specify a value. If a
/// value is specified, the name and value must be separated *without spaces* by
/// an equals sign (`=`). The value may optionally be an empty string. If an
/// option is present multiple times with different values — for example,
/// `--foo=bar --foo=baz` — all supplied values will be stored for the option.
///
/// ## Valid examples of option arguments
///
/// ```text
/// --foo
/// --foo=
/// --foo=""
/// --foo=bar
/// --foo="bar then baz"
/// --foo=bar,baz,biz
/// --foo=bar --foo=baz --foo=biz
/// ```
///
/// ## Invalid examples of option arguments
///
/// ```text
/// -foo
/// --foo bar
/// --foo = bar
/// ```
///
/// # End of option arguments
///
/// The underlying parser supports the POSIX "end of options" delimiter, meaning
/// that any `--` (empty option name) in the command line signals that all
/// remaining arguments are non-option arguments. For example, `--opt1=ignored`,
/// `--opt2`, and `filename` in the following command line are considered
/// non-option arguments:
///
/// ```text
/// --foo=bar -- --opt1=ignored -opt2 filename
/// ```
///
/// # Working with non-option arguments
///
/// Any arguments following the "end of options" delimiter (`--`) or specified
/// without the `--` option prefix will be considered as "non-option arguments"
/// and made available through [`CommandLineArgs::non_option_args`].
///
/// # Typical usage
///
/// ```ignore
/// let ps = SimpleCommandLinePropertySource::new(vec!["--foo=bar".to_string()]);
/// // ...
/// ```
#[derive(Clone)]
pub struct SimpleCommandLinePropertySource {
    base: CommandLinePropertySource<CommandLineArgs>,
}

impl SimpleCommandLinePropertySource {
    /// Creates a new [`SimpleCommandLinePropertySource`] having the default name
    /// and backed by the given command line arguments.
    ///
    /// # Arguments
    ///
    /// * `args` - The raw command line arguments.
    pub fn new(args: &[String]) -> Self {
        Self {
            base: CommandLinePropertySource::with_source(SimpleCommandLineArgsParser::parse(args)),
        }
    }

    /// Creates a new [`SimpleCommandLinePropertySource`] having the given name
    /// and backed by the given command line arguments.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of this property source.
    /// * `args` - The raw command line arguments.
    pub fn with_name(name: impl Into<String>, args: &[String]) -> Self {
        Self {
            base: CommandLinePropertySource::new(name, SimpleCommandLineArgsParser::parse(args)),
        }
    }

    /// Get the property names for the option arguments.
    pub fn property_names(&self) -> HashSet<&str> {
        self.source().option_names()
    }
}

impl PropertySource<CommandLineArgs> for SimpleCommandLinePropertySource {
    fn name(&self) -> &str {
        self.base.name()
    }

    fn contains_property(&self, name: &str) -> bool {
        self.base.contains_property(name, self)
    }

    fn property(&self, name: &str) -> Option<String> {
        self.base.property(name, self)
    }

    fn source(&self) -> &CommandLineArgs {
        self.base.source()
    }
}

impl CommandLinePropertySourceExt for SimpleCommandLinePropertySource {
    fn contains_option(&self, name: &str) -> bool {
        self.source().contains_option(name)
    }

    fn option_values(&self, name: &str) -> Option<&[String]> {
        self.source().option_values(name)
    }

    fn non_option_args(&self) -> &[String] {
        self.source().non_option_args()
    }
}

impl fmt::Debug for SimpleCommandLinePropertySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SimpleCommandLinePropertySource(name={})", self.name())
    }
}
