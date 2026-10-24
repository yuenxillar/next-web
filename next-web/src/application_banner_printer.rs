//! Printer used by the application to print the application banner.

use std::io::{self, Write};

use next_web_core::clone_box;
use next_web_core::env::Environment;
use next_web_core::io::ResourceLoader;

use crate::{Banner, NextWebBanner, ResourceBanner};

/// Property key used to look up the banner location.
pub const BANNER_LOCATION_PROPERTY: &str = "next.banner.location";

/// Default banner location.
pub const DEFAULT_BANNER_LOCATION: &str = "banner.txt";

/// Printer used by the application to print the application banner.
///
/// The printer resolves the effective [`Banner`] from the environment, falling
/// back to a configurable fallback banner and finally to the built-in
/// [`NextWebBanner`], then prints it either to a logger or to a writer.
pub struct ApplicationBannerPrinter<'a, L>
where
    L: ResourceLoader + ?Sized,
{
    /// Loader used to resolve the banner resource.
    resource_loader: &'a L,
    /// Optional fallback banner used when no text banner is found.
    fallback_banner: Option<&'a dyn Banner>,
}

impl<'a, L> ApplicationBannerPrinter<'a, L>
where
    L: ResourceLoader + ?Sized,
{
    /// Creates a new [`ApplicationBannerPrinter`].
    ///
    /// # Arguments
    ///
    /// * `resource_loader` - The loader used to resolve the banner resource.
    /// * `fallback_banner` - An optional fallback banner.
    pub fn new(resource_loader: &'a L, fallback_banner: Option<&'a dyn Banner>) -> Self {
        Self {
            resource_loader,
            fallback_banner,
        }
    }

    /// Prints the banner to the logger and returns a [`PrintedBanner`]
    /// wrapping it, so that it can be printed again later.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment used to resolve banner properties.
    ///
    /// # Returns
    ///
    /// A [`PrintedBanner`] wrapping the banner that was printed.
    pub fn print_to_logger(&self, environment: &dyn Environment) -> PrintedBanner<'a> {
        let banner = self.get_banner(environment);
        match self.create_string_from_banner(banner.as_ref(), environment) {
            Ok(text) => tracing::info!("{}", text),
            Err(error) => tracing::warn!(%error, "failed to render banner"),
        }
        PrintedBanner::new(banner)
    }

    /// Prints the banner to the given writer and returns a [`PrintedBanner`]
    /// wrapping it, so that it can be printed again later.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment used to resolve banner properties.
    /// * `out` - The writer to print the banner to.
    ///
    /// # Returns
    ///
    /// A [`PrintedBanner`] wrapping the banner that was printed.
    pub fn print_to_writer<W>(
        &self,
        environment: &dyn Environment,
        out: &mut W,
    ) -> io::Result<PrintedBanner<'a>>
    where
        W: Write,
    {
        let banner = self.get_banner(environment);
        banner.print_banner(environment, out)?;
        Ok(PrintedBanner::new(banner))
    }

    /// Resolves the effective banner for the given environment.
    ///
    /// Resolution order:
    ///
    /// 1. A text banner resolved from the configured resource location.
    /// 2. The configured fallback banner, if any.
    /// 3. The built-in [`NextWebBanner`].
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment used to resolve banner properties.
    ///
    /// # Returns
    ///
    /// The resolved banner.
    fn get_banner(&self, environment: &dyn Environment) -> Box<dyn Banner + 'a> {
        if let Some(text_banner) = self.get_text_banner(environment) {
            return text_banner;
        }
        if let Some(fallback) = self.fallback_banner {
            return clone_box(fallback);
        }

        Box::new(NextWebBanner)
    }

    /// Attempts to resolve a text banner from the configured resource location.
    ///
    /// Returns `None` when the resource cannot be resolved.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment used to resolve banner properties.
    ///
    /// # Returns
    ///
    /// An optional [`ResourceBanner`].
    fn get_text_banner(&self, environment: &dyn Environment) -> Option<Box<dyn Banner + 'a>> {
        let location = environment
            .get_property(BANNER_LOCATION_PROPERTY)
            .unwrap_or_else(|| DEFAULT_BANNER_LOCATION.to_owned());
        // The loader outlives the printer, so the text banner is bound by the
        // lifetime of the loader rather than by the borrow of the printer.
        let loader: &'a L = self.resource_loader;
        let resource = loader.get_resource(&location).ok()?;
        Some(Box::new(ResourceBanner::new(resource)))
    }

    /// Renders the given banner into a string.
    ///
    /// # Arguments
    ///
    /// * `banner` - The banner to render.
    /// * `environment` - The environment used to resolve the charset.
    ///
    /// # Returns
    ///
    /// The rendered banner text.
    fn create_string_from_banner(
        &self,
        banner: &dyn Banner,
        environment: &dyn Environment,
    ) -> io::Result<String> {
        let mut buffer: Vec<u8> = Vec::new();
        banner.print_banner(environment, &mut buffer)?;
        String::from_utf8(buffer).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }
}

/// Decorator that allows a [`Banner`] to be printed again without needing to
#[derive(Clone)]
pub struct PrintedBanner<'a> {
    /// The wrapped banner.
    banner: Box<dyn Banner + 'a>,
}

impl<'a> PrintedBanner<'a> {
    /// Creates a new [`PrintedBanner`].
    ///
    /// # Arguments
    ///
    /// * `banner` - The wrapped banner.
    pub fn new(banner: Box<dyn Banner + 'a>) -> Self {
        Self { banner }
    }
}

impl Banner for PrintedBanner<'_> {
    fn print_banner(&self, environment: &dyn Environment, out: &mut dyn Write) -> io::Result<()> {
        self.banner.print_banner(environment, out)
    }
}
