//! Port of `org.springframework.core.io.support.PropertiesLoaderUtils`.
//!
//! The helpers read a [`Resource`] into a
//! [`Properties`](next_web_context::util::Properties) instance. A resource whose
//! name ends with [`.xml`](XML_FILE_EXTENSION) is read with the XML format of
//! `Properties.loadFromXML`, every other resource with the `.properties` format
//! of `Properties.load`. The content is decoded as UTF-8, since a Rust string is
//! UTF-8.
//!
//! # Examples
//!
//! ```ignore
//! let resource = resource_loader.get_resource("messages/messages.properties")?;
//! let properties = PropertiesLoaderUtils::load_properties(resource)?;
//!
//! assert_eq!(properties.get_property("hello"), Some("Hello, world!"));
//! ```

use std::io;

use next_web_context::util::Properties;

use crate::io::{Resource, ResourceLoader};

/// The extension of a properties file that is read with the XML format.
pub const XML_FILE_EXTENSION: &str = ".xml";

/// Reads properties files.
///
/// The type has no state: every helper is an associated function, exactly like
/// the static methods of the original.
pub struct PropertiesLoaderUtils;

impl PropertiesLoaderUtils {
    /// Fills the given instance with the properties of the given resource.
    ///
    /// Equivalent to `fillProperties(Properties, Resource)`: the content is
    /// read as UTF-8 and parsed with the format its name asks for.
    ///
    /// # Arguments
    ///
    /// * `properties` - The instance to fill. The entries that are already
    ///   there are kept, and the entries of the resource replace the values of
    ///   the keys they share.
    /// * `resource` - The resource to read.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] when the resource cannot be read, when its
    /// content is not valid UTF-8, or when it does not match the format of the
    /// resource name.
    pub fn fill_properties(properties: &mut Properties, resource: &dyn Resource) -> io::Result<()> {
        let content = resource.get_content()?;

        if is_xml(resource) {
            properties.load_from_xml(&content)?;
        } else {
            properties.load_from_bytes(&content)?;
        }

        Ok(())
    }

    /// Reads the properties of the given resource.
    ///
    /// Equivalent to `loadProperties(Resource)`.
    ///
    /// # Arguments
    ///
    /// * `resource` - The resource to read.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] when the resource cannot be read, when its
    /// content is not valid UTF-8, or when it does not match the format of the
    /// resource name.
    pub fn load_properties(resource: &dyn Resource) -> io::Result<Properties> {
        let mut properties = Properties::create_sorted_properties(false);
        Self::fill_properties(&mut properties, resource)?;
        Ok(properties)
    }

    /// Reads every resource the loader holds under the given name, and merges
    /// them into a single instance.
    ///
    /// Equivalent to `loadAllProperties(String, ClassLoader)`. The resources
    /// are read in the order the loader returns them, so a property that
    /// several of them define is taken from the last one.
    ///
    /// # Arguments
    ///
    /// * `resource_loader` - The loader the resources are looked up with.
    /// * `resource_name` - The name of the resources to read. A name that ends
    ///   with [`.xml`](XML_FILE_EXTENSION) is read with the XML format of
    ///   `Properties.loadFromXML`, every other name with the `.properties`
    ///   format of `Properties.load`.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] when a resource cannot be read or holds content
    /// that does not match the format of its name.
    pub fn load_all_properties(
        resource_loader: &dyn ResourceLoader,
        resource_name: &str,
    ) -> io::Result<Properties> {
        let mut properties = Properties::create_sorted_properties(false);

        for resource in resource_loader.get_resources(resource_name)? {
            Self::fill_properties(&mut properties, resource)?;
        }

        Ok(properties)
    }
}

impl std::fmt::Debug for PropertiesLoaderUtils {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PropertiesLoaderUtils")
    }
}

/// Returns whether the resource has to be read with the XML format.
fn is_xml(resource: &dyn Resource) -> bool {
    resource
        .filename()
        .is_some_and(|filename| filename.ends_with(XML_FILE_EXTENSION))
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use crate::io::BytesResource;

    use super::*;

    fn resource(name: &'static str, content: &'static str) -> BytesResource<'static> {
        BytesResource::new(name, Cow::Borrowed(content.as_bytes()))
    }

    #[test]
    fn fills_the_properties_of_a_properties_resource() {
        let resource = resource(
            "messages/messages.properties",
            "# comment\nhello=Hello, world!\nname=Your name is {0}\n",
        );
        let properties = PropertiesLoaderUtils::load_properties(&resource).unwrap();

        assert_eq!(properties.get_property("hello"), Some("Hello, world!"));
        assert_eq!(properties.get_property("name"), Some("Your name is {0}"));
        assert_eq!(properties.get_property("comment"), None);
    }

    #[test]
    fn fills_the_previous_entries_of_the_instance() {
        let mut properties = Properties::from_iter([("hello", "previous")]);
        properties.set_property("kept", "kept");
        let resource = resource("messages.properties", "hello=Hello, world!\n");

        PropertiesLoaderUtils::fill_properties(&mut properties, &resource).unwrap();

        assert_eq!(properties.get_property("hello"), Some("Hello, world!"));
        assert_eq!(properties.get_property("kept"), Some("kept"));
    }

    #[test]
    fn fills_the_properties_of_an_xml_resource() {
        let resource = resource(
            "messages/messages.xml",
            r#"<!DOCTYPE properties SYSTEM "http://java.sun.com/dtd/properties.dtd">
<properties>
  <entry key="hello">Hello, world!</entry>
</properties>
"#,
        );
        let properties = PropertiesLoaderUtils::load_properties(&resource).unwrap();

        assert_eq!(properties.get_property("hello"), Some("Hello, world!"));
    }

    #[test]
    fn reads_the_resource_as_utf_8() {
        let resource = resource("messages.properties", "hello=caf\u{E9}\n");
        let properties = PropertiesLoaderUtils::load_properties(&resource).unwrap();

        assert_eq!(properties.get_property("hello"), Some("caf\u{E9}"));
    }

    #[test]
    fn rejects_a_resource_that_is_not_utf_8() {
        let resource = BytesResource::new(
            "messages.properties",
            Cow::Borrowed(b"hello=caf\xE9\n" as &[u8]),
        );
        let error = PropertiesLoaderUtils::load_properties(&resource).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn rejects_a_malformed_xml_resource() {
        let resource = resource("messages.xml", "<properties><unknown/></properties>");
        let error = PropertiesLoaderUtils::load_properties(&resource).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn loads_the_resource_of_a_name() {
        let loader = TestResourceLoader::new([
            (
                "META-INF/messages.properties",
                "hello=First\nshared=first\n",
            ),
            ("META-INF/errors.properties", "hello=Ignored\n"),
        ]);

        let properties =
            PropertiesLoaderUtils::load_all_properties(&loader, "META-INF/messages.properties")
                .unwrap();

        assert_eq!(properties.get_property("hello"), Some("First"));
        assert_eq!(properties.get_property("shared"), Some("first"));
    }

    /// A loader that serves in-memory resources and reports them by name.
    struct TestResourceLoader {
        resources: Vec<(&'static str, BytesResource<'static>)>,
    }

    impl TestResourceLoader {
        fn new(entries: impl IntoIterator<Item = (&'static str, &'static str)>) -> Self {
            Self {
                resources: entries
                    .into_iter()
                    .map(|(name, content)| {
                        (
                            name,
                            BytesResource::new(name, Cow::Owned(content.as_bytes().to_vec())),
                        )
                    })
                    .collect(),
            }
        }
    }

    impl ResourceLoader for TestResourceLoader {
        fn load(&mut self) -> io::Result<()> {
            Ok(())
        }

        fn refresh(&mut self) -> io::Result<()> {
            Ok(())
        }

        fn clear(&mut self) {}

        fn get_resource(&self, location: &str) -> io::Result<&dyn Resource> {
            self.resources
                .iter()
                .position(|(name, _)| *name == location)
                .map(|index| &self.resources[index].1 as &dyn Resource)
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "resource not found"))
        }

        fn get_resources(&self, pattern: &str) -> io::Result<Vec<&dyn Resource>> {
            Ok(self
                .resources
                .iter()
                .filter(|(name, _)| *name == pattern)
                .map(|(_, resource)| resource as &dyn Resource)
                .collect())
        }

        fn get_directory(&self, location: &str) -> Vec<Cow<'_, str>> {
            self.resources
                .iter()
                .filter_map(|(name, _)| name.strip_prefix(location))
                .map(Cow::Borrowed)
                .collect()
        }

        fn paths(&self) -> Vec<Cow<'_, str>> {
            self.resources
                .iter()
                .map(|(name, _)| Cow::Borrowed(*name))
                .collect()
        }

        fn exists(&self, location: &str) -> bool {
            self.resources.iter().any(|(name, _)| *name == location)
        }
    }
}
