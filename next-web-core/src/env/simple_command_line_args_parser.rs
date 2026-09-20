use crate::env::CommandLineArgs;

/// Parses a slice of command line arguments in order to populate a
/// `CommandLineArgs` object.
///
/// ### Working with option arguments
///
/// Option arguments must adhere to the exact syntax:
///
/// ```text
/// --optName[=optValue]
/// ```
///
/// That is, options must be prefixed with `--` and may or may not
/// specify a value. If a value is specified, the name and value must be
/// separated *without spaces* by an equals sign (`=`). The value may
/// optionally be an empty string. If an option is present multiple times
/// with different values — for example, `--foo=bar --foo=baz` — all
/// supplied values will be stored for the option.
///
/// #### Valid examples of option arguments
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
/// #### Invalid examples of option arguments
///
/// ```text
/// -foo
/// --foo bar
/// --foo = bar
/// ```
///
/// ### End of option arguments
///
/// This parser supports the POSIX "end of options" delimiter, meaning that
/// any `--` (empty option name) in the command line signals that all
/// remaining arguments are non-option arguments. For example,
/// `--opt1=ignored`, `--opt2`, and `filename` in the following command line
/// are considered non-option arguments.
///
/// ```text
/// --foo=bar -- --opt1=ignored -opt2 filename
/// ```
///
/// ### Working with non-option arguments
///
/// Any arguments following the "end of options" delimiter (`--`) or
/// specified without the `--` option prefix will be considered as
/// "non-option arguments" and made available through the
/// `CommandLineArgs::non_option_args()` method.
pub struct SimpleCommandLineArgsParser;

impl SimpleCommandLineArgsParser {
    /// Parse the given slice based on the rules described above, returning a
    /// fully-populated [`CommandLineArgs`] object.
    ///
    /// `args` are command line arguments, typically from a `main()` function.
    ///
    /// # Panics
    ///
    /// Panics if an argument has the form `--=value` (an empty option name
    /// followed by an equals sign), since that is invalid syntax.
    pub fn parse(args: &[String]) -> CommandLineArgs {
        let mut command_line_args = CommandLineArgs::default();
        let mut end_of_options = false;

        for arg in args {
            if !end_of_options && arg.starts_with("--") {
                let option_text = &arg[2..];
                match option_text.find('=') {
                    Some(index_of_equals_sign) => {
                        let option_name = &option_text[..index_of_equals_sign];
                        let option_value = &option_text[index_of_equals_sign + 1..];
                        if option_name.is_empty() {
                            panic!("Invalid argument syntax: {}", arg);
                        }
                        command_line_args.add_option_arg(
                            option_name.to_string(),
                            Some(option_value.to_string()),
                        );
                    }
                    None if !option_text.is_empty() => {
                        command_line_args.add_option_arg(option_text.to_string(), None);
                    }
                    None => {
                        // '--' End of options delimiter; all remaining args
                        // are non-option arguments.
                        end_of_options = true;
                    }
                }
            } else {
                command_line_args.add_non_option_arg(arg.clone());
            }
        }

        command_line_args
    }
}
