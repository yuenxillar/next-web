use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use crate::core::util::object::Object;

#[derive(Default, Clone)]
pub struct MapContext {
    backing_map: HashMap<String, Object>,
}

impl MapContext {
    pub fn new(backing_map: HashMap<String, Object>) -> Self {
        Self { backing_map }
    }
}

impl Deref for MapContext {
    type Target = HashMap<String, Object>;

    fn deref(&self) -> &Self::Target {
        &self.backing_map
    }
}

impl DerefMut for MapContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.backing_map
    }
}
