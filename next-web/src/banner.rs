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
