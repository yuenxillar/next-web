use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use crate::core::poi::ss::formula::function::function_metadata::FunctionMetadata;
use crate::core::poi::ss::formula::function::function_metadata_reader::FunctionMetadataReader;

// Global registry instances
static REGISTRY: OnceLock<Arc<FunctionMetadataRegistry>> = OnceLock::new();
static REGISTRY_CETAB: OnceLock<Arc<FunctionMetadataRegistry>> = OnceLock::new();

/// Allows clients to get FunctionMetadata instances for any built-in function of Excel.
pub struct FunctionMetadataRegistry {
    function_data_by_index: Option<Vec<FunctionMetadata>>,
    function_data_by_name: HashMap<String, FunctionMetadata>,
}

impl FunctionMetadataRegistry {
    // Common function names and indices
    /// The name of the IF function (i.e. "IF"). Extracted as a constant for clarity.
    pub const FUNCTION_NAME_IF: &'static str = "IF";
    pub const FUNCTION_INDEX_IF: u16 = 1;
    pub const FUNCTION_INDEX_SUM: u16 = 4;
    pub const FUNCTION_INDEX_CHOOSE: u16 = 100;
    pub const FUNCTION_INDEX_INDIRECT: u16 = 148;
    pub const FUNCTION_INDEX_EXTERNAL: u16 = 255;

    /// Creates a new registry with the provided data
    pub fn new(
        function_data_by_index: Option<Vec<FunctionMetadata>>,
        function_data_by_name: HashMap<String, FunctionMetadata>,
    ) -> Self {
        Self {
            function_data_by_index,
            function_data_by_name,
        }
    }

    fn get_instance() -> &'static Arc<FunctionMetadataRegistry> {
        REGISTRY.get_or_init(|| Arc::new(FunctionMetadataReader::create_registry().unwrap()))
    }

    fn get_instance_cetab() -> &'static Arc<FunctionMetadataRegistry> {
        REGISTRY_CETAB
            .get_or_init(|| Arc::new(FunctionMetadataReader::create_registry_cetab().unwrap()))
    }

    /// Gets all function names from both registries
    pub fn get_all_function_names(&self) -> Vec<String> {
        self.function_data_by_name
            .keys()
            .map(Clone::clone)
            .collect::<Vec<_>>()
    }

    /// Gets a function by index from the main registry
    pub fn get_function_by_index(index: u16) -> Option<&'static FunctionMetadata> {
        Self::get_instance().get_function_by_index_internal(index)
    }

    /// Gets a function by index from the CETAB registry
    pub fn get_cetab_function_by_index(index: u16) -> Option<&'static FunctionMetadata> {
        Self::get_instance_cetab().get_function_by_index_internal(index)
    }

    fn get_function_by_index_internal(&self, index: u16) -> Option<&FunctionMetadata> {
        self.function_data_by_index
            .as_ref()
            .map(|f| f.get(index as usize))
            .unwrap_or_default()
    }

    /// Resolves a built-in function index from either registry.
    /// Returns None if the function name is not found.
    pub fn lookup_index_by_name(name: &str) -> i16 {
        let fd = Self::get_instance().get_function_by_name_internal(name);
        if fd.is_none() {
            if Self::get_instance_cetab()
                .get_function_by_name_internal(name)
                .is_none()
            {
                return -1;
            }
        }

        fd.unwrap().get_index() as i16
    }

    fn get_function_by_name_internal(&self, name: &str) -> Option<&FunctionMetadata> {
        self.function_data_by_name.get(name)
    }

    pub fn get_function_by_name(&self, name: &str) -> Option<&'static FunctionMetadata> {
        let fm = Self::get_instance().get_function_by_name_internal(name);

        if fm.is_none() {
            return Self::get_instance_cetab().get_function_by_name_internal(name);
        }

        fm
    }
}
