use std::io::{self, Write};

use next_web_core::{clone_trait_object, env::Environment, DynClone};

/// writing a banner programmatically.
///
/// Banners are cloneable so that a boxed banner can be shared and printed more
/// than once.
pub trait Banner
where
    Self: DynClone,
{
    /// Print the banner to the specified print writer.
    fn print_banner(&self, environment: &dyn Environment, out: &mut dyn Write) -> io::Result<()>;
}

clone_trait_object!(Banner);

#[derive(Debug, Clone, PartialEq, Eq, Copy, Default)]
pub enum BannerMode {
    /// Disable printing of the banner.
    Off,

    /// Print the banner to System.out.
    #[default]
    Console,

    /// Print the banner to the log file.
    Log,
}

impl std::str::FromStr for BannerMode {
    type Err = String;

    /// Parses a banner mode from its textual representation.
    ///
    /// Matching is case-insensitive and ignores surrounding whitespace. The
    /// accepted values are `off`, `console`, and `log`.
    ///
    /// # Errors
    ///
    /// Returns the unrecognized value when it does not name a banner mode.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" => Ok(Self::Off),
            "console" => Ok(Self::Console),
            "log" => Ok(Self::Log),
            other => Err(format!("Invalid banner mode '{other}'")),
        }
    }
}
