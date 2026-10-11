use std::borrow::Cow;
use std::io;
use std::path::Path;
use std::sync::Arc;
use std::{error::Error, time::Duration};

use next_web_context::support::{BundleLoader, ResourceBundleMessageSource};
use next_web_context::util::Properties;
use next_web_context::{
    APPLICATION_ENVIRONMENT_SINGLETON_NAME, ApplicationContext, ApplicationContextExt,
    MESSAGE_SOURCE_SINGLETON_NAME, MessageSource, RESOURCE_LOADER_SINGLETON_NAME,
};
use next_web_core::env::ConfigurableEnvironment;
use next_web_core::{
    Ordered, async_trait,
    io::support::PropertiesLoaderUtils,
    io::{BytesResource, Resource, ResourceLoader},
    traits::config::auto_configuration::AutoConfiguration,
};

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
        let resource_loader = Self::resource_loader(ctx)?;
        let bundle_loader: Arc<dyn BundleLoader> =
            Arc::new(ResourceLoaderBundleLoader(resource_loader));

        let mut message_source = ResourceBundleMessageSource::default();
        // The message source reads its bundles with the same loader the common
        // messages are read with.
        message_source.set_bundle_loader(Arc::clone(&bundle_loader));

        let base_name = self.message_source_properties.base_name();
        if base_name.iter().any(|s| !s.is_empty()) {
            message_source.set_basenames(base_name);
        }
        message_source.set_extensions(self.message_source_properties.extensions());
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
    #[cfg(feature = "embed-resources")]
    fn embedded_resource_loader() -> Option<Arc<dyn ResourceLoader>> {
        crate::resources::embedded_resource_loader()
    }

    #[cfg(not(feature = "embed-resources"))]
    fn embedded_resource_loader() -> Option<Arc<dyn ResourceLoader>> {
        None
    }

    /// Returns the loader the message bundles are read with.
    ///
    /// The loader of the application context wins, which is where an
    /// application configures its resource loader explicitly. An embedded
    /// loader is the fallback for a context that does not register one of its
    /// own.
    fn resource_loader(ctx: &dyn ApplicationContext) -> Result<Arc<dyn ResourceLoader>, String> {
        if let Some(loader) = ctx
            .get_singleton_option_with_name::<Arc<dyn ResourceLoader>>(
                RESOURCE_LOADER_SINGLETON_NAME,
            )
            .map(Clone::clone)
        {
            return Ok(loader);
        }

        if let Some(loader) = Self::embedded_resource_loader() {
            return Ok(loader);
        }

        Err(format!(
            "No ResourceLoader found: {RESOURCE_LOADER_SINGLETON_NAME}"
        ))
    }

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

struct ResourceBundleCondition;

impl NextWebConditionExt for ResourceBundleCondition {
    fn get_match_outcome(
        &self,
        context: &dyn ApplicationContext,
    ) -> Result<ConditionOutcome, ConditionError> {
        // The properties of the message source were bound before the conditions
        // are evaluated, so the condition sees the same basenames and extensions
        // the message source is built with.
        let (basenames, extensions) =
            match context.get_singleton_option_with_default_name::<MessageSourceProperties>() {
                Some(properties) => (properties.base_name(), properties.extensions()),
                None => (
                    vec![
                        context
                            .get_singleton_with_name::<Arc<dyn ConfigurableEnvironment>>(
                                APPLICATION_ENVIRONMENT_SINGLETON_NAME,
                            )
                            .get_property_or_default("next.messages.basename", "messages"),
                    ],
                    vec!["properties".to_owned()],
                ),
            };

        self.get_matches_for_bundles(context, &basenames, &extensions)
    }
}

impl ResourceBundleCondition {
    fn get_matches_for_bundles(
        &self,
        context: &dyn ApplicationContext,
        basenames: &[String],
        extensions: &[String],
    ) -> Result<ConditionOutcome, ConditionError> {
        let message = ConditionMessage::for_condition("ResourceBundle", &[]);

        let resource_loader = context
            .get_singleton_option_with_name::<Arc<dyn ResourceLoader>>(
                RESOURCE_LOADER_SINGLETON_NAME,
            )
            .ok_or_else(|| ConditionError::Evaluation("ResourceLoader not found".to_string()))?;

        for basename in basenames
            .iter()
            .map(|basename| basename.trim())
            .filter(|basename| !basename.is_empty())
        {
            for extension in extensions {
                let resources = self.get_resources(resource_loader.as_ref(), basename, extension);
                if !resources.is_empty() {
                    let items = resources
                        .into_iter()
                        .filter_map(|resource| resource.filename())
                        .collect::<Vec<_>>();

                    return Ok(ConditionOutcome::match_message(
                        message.found("bundle", "bundle").items(&items),
                    ));
                }
            }
        }

        let s = format!("bundle with basename {basenames:?} and extension {extensions:?}");
        Ok(ConditionOutcome::no_match_message(
            message.did_not_find(&s, &s).at_all(),
        ))
    }

    /// Returns the resources for the given name using the given resource loader.
    fn get_resources<'a>(
        &'a self,
        resource_loader: &'a dyn ResourceLoader,
        name: &str,
        extension: &str,
    ) -> Vec<&'a dyn Resource> {
        let target = name.replace('.', "/");
        let pattern = format!("**/{target}.{extension}");
        resource_loader.get_resources(&pattern).unwrap_or_default()
    }
}
