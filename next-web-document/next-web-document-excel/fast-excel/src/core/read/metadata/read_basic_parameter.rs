use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use crate::core::{
    metadata::basic_parameter::BasicParameter, read::listener::read_listener::ReadListener,
};

#[derive(Clone)]
pub struct ReadBasicParameter<T> {
    /// Count the number of added heads when read sheet.
    head_row_number: Option<u32>,
    /// Custom type listener run after default
    pub(crate) custom_read_listener_list: Vec<Arc<dyn ReadListener<T>>>,

    pub(crate) basic_parameter: BasicParameter,
}

impl<T> ReadBasicParameter<T> {
    pub fn get_head_row_number(&self) -> Option<u32> {
        self.head_row_number
    }

    pub fn get_ref_custom_read_listener_list(&self) -> Vec<&dyn ReadListener<T>> {
        self.custom_read_listener_list
            .iter()
            .map(|s| s.as_ref())
            .collect()
    }

    pub fn get_custom_read_listener_list(&self) -> Vec<Arc<dyn ReadListener<T>>> {
        self.custom_read_listener_list.clone()
    }

    pub fn add_custom_read_listener(&mut self, listener: Arc<dyn ReadListener<T>>) {
        self.custom_read_listener_list.push(listener);
    }

    pub fn set_custom_read_listener(
        &mut self,
        custom_read_listener_list: Vec<Arc<dyn ReadListener<T>>>,
    ) {
        self.custom_read_listener_list = custom_read_listener_list;
    }

    pub fn set_head_row_number(&mut self, head_row_number: u32) {
        self.head_row_number = Some(head_row_number);
    }
}

impl<T> Default for ReadBasicParameter<T> {
    fn default() -> Self {
        Self {
            head_row_number: None,
            custom_read_listener_list: Vec::new(),
            basic_parameter: BasicParameter::default(),
        }
    }
}

impl<T> Deref for ReadBasicParameter<T> {
    type Target = BasicParameter;

    fn deref(&self) -> &Self::Target {
        &self.basic_parameter
    }
}

impl<T> DerefMut for ReadBasicParameter<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.basic_parameter
    }
}
