use std::io;

use crate::io::{Resource, ResourceLoader};

#[derive(Debug)]
pub struct EmbedResourceLoader;

impl ResourceLoader for EmbedResourceLoader {
    fn load(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn get_resource(&self, location: &str) -> io::Result<&dyn Resource> {
        Ok(())
    }

    fn get_directory(&self, location: &str) -> Vec<Cow<'static, str>> {
        vec![]
    }

    fn paths(&self) -> Vec<Cow<'static, str>> {
        vec![]
    }

    fn exists(&self, location: &str) -> bool {
        false
    }
}
