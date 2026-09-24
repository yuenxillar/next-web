use std::borrow::Cow;
use std::io;
use std::path::Path;
use std::sync::Arc;
use std::{error::Error, time::Duration};

use next_web_context::support::{BundleLoader, ResourceBundleMessageSource};
use next_web_context::util::Properties;
use next_web_context::{
    APPLICATION_ENVIRONMENT_SINGLETON_NAME, ApplicationContext, ApplicationContextExt, Locale,
    MESSAGE_SOURCE_SINGLETON_NAME, MessageSource, RESOURCE_LOADER_SINGLETON_NAME,
};
use next_web_core::env::ConfigurableEnvironment;
use next_web_core::{
    Ordered, async_trait,
    io::support::PropertiesLoaderUtils,
    io::{BytesResource, Resource, ResourceLoader},
    traits::config::auto_configuration::AutoConfiguration,
};

#[cfg(feature = "embed-resources")]
use next_web_core::context::application_resources::RESOURCE_LOADER;
use next_web_macros::singleton;

use crate::autoconfigure::condition::{ConditionMessage, NextWebCondition};
use crate::autoconfigure::{
    condition::{ConditionError, ConditionOutcome, NextWebConditionExt},
    context::message_source_properties::MessageSourceProperties,
};

/// Auto-configuration for MessageSource.
#[singleton(binds =[Self::into_auto_configuration])]
#[derive(Clone)]
pub struct MessageSourceAutoConfiguration {
    message_source_properties: MessageSourceProperties,
}

#[async_trait]
impl AutoConfiguration for MessageSourceAutoConfiguration {
    async fn configure(&mut self, ctx: &mut dyn ApplicationContext) -> Result<(), Box<dyn Error>> {
        #[cfg(feature = "embed-resources")]
        let bundle_loader: Arc<dyn BundleLoader> = match RESOURCE_LOADER.get().map(Clone::clone) {
            Some(loader) => Arc::new(EmbeddedBundleLoader(loader)),
            None => {
                return Err("Please add the macro #[next_application] to the main function".into());
            }
        };

        #[cfg(not(feature = "embed-resources"))]
        let bundle_loader: Arc<dyn BundleLoader> = {
            let resource_loader = ctx
                .get_singleton_option_with_name::<Arc<dyn ResourceLoader>>(
                    RESOURCE_LOADER_SINGLETON_NAME,
                )
                .map(Clone::clone)
                .ok_or(format!(
                    "No ResourceLoader found: {RESOURCE_LOADER_SINGLETON_NAME}"
                ))?;

            Arc::new(ResourceLoaderBundleLoader(resource_loader))
        };

        let mut message_source = ResourceBundleMessageSource::default();
        // The message source reads its bundles with the same loader the common
        // messages are read with.
        message_source.set_bundle_loader(Arc::clone(&bundle_loader));

        let base_name = self.message_source_properties.base_name();
        if base_name.iter().any(|s| !s.is_empty()) {
            message_source.set_basenames(base_name);
        }
        if let Some(locale) = self.message_source_properties.default_local() {
            let locale = Locale::for_language_tag(locale).map_err(|error| {
                format!(
                    "Invalid default locale [{locale}] configured for the message source: {error}"
                )
            })?;
            message_source.set_default_locale(Some(locale));
        }
        message_source.set_fallback_to_system_locale(
            self.message_source_properties.fallback_to_system_locale(),
        );
        if let Some(cache_duration) = self.message_source_properties.cache_duration() {
            message_source.set_cache_duration(Some(Duration::from_secs(cache_duration)));
        }
        message_source.set_always_use_message_format(
            self.message_source_properties
                .is_always_use_message_format(),
        );
        message_source.set_use_code_as_default_message(
            self.message_source_properties
                .is_use_code_as_default_message(),
        );

        if let Some(common_messages) = self.load_common_messages(
            bundle_loader.as_ref(),
            self.message_source_properties
                .common_messages()
                .unwrap_or_default(),
        )? {
            message_source.set_common_messages(common_messages);
        }
        ctx.insert_singleton_with_name::<Arc<dyn MessageSource>>(
            Arc::new(message_source),
            MESSAGE_SOURCE_SINGLETON_NAME,
        );

        Ok(())
    }

    fn matches(&self, ctx: &dyn ApplicationContext) -> bool {
        !ctx.contains_singleton_with_name::<Arc<dyn MessageSource>>(MESSAGE_SOURCE_SINGLETON_NAME)
            && ResourceBundleCondition
                .matches(ctx)
                .ok()
                .unwrap_or_default()
    }
}

impl Ordered for MessageSourceAutoConfiguration {
    fn order(&self) -> i32 {
        i32::MIN
    }
}

impl MessageSourceAutoConfiguration {
    pub fn into_auto_configuration(self: Self) -> Box<dyn AutoConfiguration> {
        Box::new(self)
    }

    /// Loads common messages from the given resources.
    ///
    /// Equivalent to `loadCommonMessages(List<Resource> resources)`.
    ///
    /// # Arguments
    ///
    /// * `bundle_loader` - The loader the resources are read with.
    /// * `resources` - The locations of the resources holding the common
    ///   messages.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] when a resource does not exist or cannot be
    /// read.
    fn load_common_messages(
        &self,
        bundle_loader: &dyn BundleLoader,
        resources: &[String],
    ) -> io::Result<Option<Arc<Properties>>> {
        if resources.is_empty() {
            return Ok(None);
        }

        // `CollectionFactory.createSortedProperties(false)` is a properties
        // instance that keeps the insertion order of its entries.
        let mut properties = Properties::create_sorted_properties(false);

        for location in resources {
            let content = bundle_loader.load(location)?.ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Failed to load common messages from '{location}': resource not found"),
                )
            })?;

            // Only the content of the resource is read here; the format of the
            // resource decides how `fill_properties` reads it.
            let resource = BytesResource::new(Path::new(location), Cow::Owned(content));
            PropertiesLoaderUtils::fill_properties(&mut properties, &resource).map_err(
                |error| {
                    io::Error::new(
                        error.kind(),
                        format!("Failed to load common messages from '{location}': {error}"),
                    )
                },
            )?;
        }

        Ok(Some(Arc::new(properties)))
    }
}

/// Adapts the [`ResourceLoader`] of the context to the loader the message source
/// reads its bundles with.
struct ResourceLoaderBundleLoader(Arc<dyn ResourceLoader>);

impl BundleLoader for ResourceLoaderBundleLoader {
    fn load(&self, location: &str) -> io::Result<Option<Vec<u8>>> {
        if !self.0.exists(location) {
            return Ok(None);
        }

        let resource = self.0.get_resource(location)?;
        Ok(Some(resource.get_content()?.into_owned()))
    }
}

/// Adapts the resource loader of the embedded resources to the loader the
/// message source reads its bundles with.
#[cfg(feature = "embed-resources")]
struct EmbeddedBundleLoader(Arc<dyn next_web_core::context::application_resources::ResourceLoader>);

#[cfg(feature = "embed-resources")]
impl BundleLoader for EmbeddedBundleLoader {
    fn load(&self, location: &str) -> io::Result<Option<Vec<u8>>> {
        Ok(self.0.load(location).map(Cow::into_owned))
    }
}

struct ResourceBundleCondition;

impl NextWebConditionExt for ResourceBundleCondition {
    fn get_match_outcome(
        &self,
        context: &dyn ApplicationContext,
    ) -> Result<ConditionOutcome, ConditionError> {
        let basename = context
            .get_singleton_with_name::<Arc<dyn ConfigurableEnvironment>>(
                APPLICATION_ENVIRONMENT_SINGLETON_NAME,
            )
            .get_property_or_default("next.messages.basename", "messages");

        let outcome = self.get_matches_for_basename(context, &basename)?;
        Ok(outcome)
    }
}

impl ResourceBundleCondition {
    fn get_matches_for_basename(
        &self,
        context: &dyn ApplicationContext,
        basename: &str,
    ) -> Result<ConditionOutcome, ConditionError> {
        let message = ConditionMessage::for_condition("ResourceBundle", &[]);

        let trimmed: String = basename.chars().filter(|c| !c.is_whitespace()).collect();
        let names: Vec<&str> = trimmed.split(',').collect();

        let resoure_load = context
            .get_singleton_option_with_name::<Arc<dyn ResourceLoader>>(
                RESOURCE_LOADER_SINGLETON_NAME,
            )
            .ok_or_else(|| ConditionError::Evaluation("ResourceLoader not found".to_string()))?;
        for name in names {
            let resources = self.get_resources(resoure_load.as_ref(), name);
            if resources.len() > 0 {
                let items = resources
                    .into_iter()
                    .filter_map(|s| s.filename())
                    .collect::<Vec<_>>();
                return Ok(ConditionOutcome::match_message(
                    message.found("bundle", "bundle").items(&items),
                ));
            }
        }

        let s = format!("bundle with basename {}", basename);
        Ok(ConditionOutcome::no_match_message(
            message.did_not_find(&s, &s).at_all(),
        ))
    }

    /// Returns the resources for the given name using the given resource loader.
    fn get_resources<'a>(
        &'a self,
        resource_loader: &'a dyn ResourceLoader,
        name: &str,
    ) -> Vec<&'a dyn Resource> {
        let target = name.replace('.', "/");
        let pattern = format!("**/{target}.properties");
        resource_loader.get_resources(&pattern).unwrap_or_default()
    }
}
