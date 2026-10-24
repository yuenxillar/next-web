use std::collections::{HashMap, HashSet};

use next_web_core::error::BoxError;

use crate::core::poi::ss::formula::function::{
    function_metadata::FunctionMetadata, function_metadata_registry::FunctionMetadataRegistry,
};

/// Temporarily collects `FunctionMetadata` instances for creation of a
/// `FunctionMetadataRegistry`.
pub(crate) struct FunctionDataBuilder {
    max_function_index: i32,
    function_data_by_name: HashMap<String, FunctionMetadata>,
    function_data_by_index: HashMap<i32, FunctionMetadata>,
    /// Stores indexes of all functions with footnotes (i.e. whose definitions might change)
    mutating_function_indexes: HashSet<i32>,
}

impl FunctionDataBuilder {
    pub(crate) fn new(size_estimate: usize) -> Self {
        Self {
            max_function_index: -1,
            function_data_by_name: HashMap::with_capacity(size_estimate * 3 / 2),
            function_data_by_index: HashMap::with_capacity(size_estimate * 3 / 2),
            mutating_function_indexes: HashSet::new(),
        }
    }

    pub(crate) fn add(
        &mut self,
        function_index: i32,
        function_name: &str,
        min_params: u16,
        max_params: u16,
        return_class_code: u8,
        parameter_class_codes: Vec<u8>,
        has_footnote: bool,
    ) -> Result<(), BoxError> {
        let fm = FunctionMetadata::new(
            function_index,
            function_name.to_string(),
            min_params,
            max_params,
            return_class_code,
            parameter_class_codes,
        );

        let index_key = function_index;

        if function_index > self.max_function_index {
            self.max_function_index = function_index;
        }

        // Allow function definitions to change only if both previous and the new items have footnotes
        if let Some(prev_fm) = self.function_data_by_name.get(function_name) {
            if !has_footnote || !self.mutating_function_indexes.contains(&index_key) {
                return Err(
                    format!("Multiple entries for function name '{}'", function_name).into(),
                );
            }
            self.function_data_by_index.remove(&prev_fm.get_index());
        }

        if let Some(prev_fm) = self.function_data_by_index.get(&index_key) {
            if !has_footnote || !self.mutating_function_indexes.contains(&index_key) {
                return Err(
                    format!("Multiple entries for function index ({})", function_index).into(),
                );
            }
            self.function_data_by_name.remove(prev_fm.get_name());
        }

        if has_footnote {
            self.mutating_function_indexes.insert(index_key);
        }

        self.function_data_by_index.insert(index_key, fm.clone());
        self.function_data_by_name
            .insert(function_name.to_string(), fm);

        Ok(())
    }

    pub(crate) fn build(self) -> FunctionMetadataRegistry {
        let mut fd_index_array = Vec::with_capacity((self.max_function_index + 1) as usize);

        for fd in self.function_data_by_name.values() {
            let index = fd.get_index() as usize;
            fd_index_array[index] = fd.clone();
        }

        FunctionMetadataRegistry::new(Some(fd_index_array), self.function_data_by_name)
    }
}
