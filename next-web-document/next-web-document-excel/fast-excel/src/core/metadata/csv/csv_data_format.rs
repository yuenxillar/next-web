use std::collections::HashMap;

use next_web_core::util::locale::Locale;

use crate::core::constant::builtin_formats::BuiltinFormats;

#[derive(Debug, Clone)]
pub struct CsvDataFormat {
    /// It is stored in both map and list for easy retrieval
    format_map: HashMap<String, u16>,
    format_list: Vec<String>,

    /// Excel's built-in format conversion.
    builtin_formats_map: HashMap<String, u16>,
    builtin_formats: &'static [Option<&'static str>],
}

impl CsvDataFormat {
    pub fn new(locale: &Locale) -> Self {
        CsvDataFormat {
            format_map: HashMap::new(),
            format_list: Vec::new(),
            // Assuming BuiltinFormats has appropriate Rust implementation
            builtin_formats_map: BuiltinFormats::switch_builtin_formats_map(locale),
            builtin_formats: BuiltinFormats::switch_builtin_formats(locale),
        }
    }
}

impl DataFormat for CsvDataFormat {
    fn get_format(&mut self, format: &str) -> u16 {
        // Check built-in formats first
        if let Some(&index) = self.builtin_formats_map.get(format) {
            return index;
        }

        // Check custom formats
        if let Some(&index) = self.format_map.get(format) {
            return index;
        }

        // Create new custom format
        let index = (self.format_list.len() as u16) + BuiltinFormats::MIN_CUSTOM_DATA_FORMAT_INDEX;
        self.format_list.push(format.to_string());
        self.format_map.insert(format.to_string(), index);

        index
    }

    fn get_format_from_index(&self, index: u16) -> Option<&str> {
        // if index < BuiltinFormats::MIN_CUSTOM_DATA_FORMAT_INDEX {
        //     return self.builtin_formats.get(index as usize).map(|s| s.as_str());
        // }

        // let actual_index = (index - BuiltinFormats::MIN_CUSTOM_DATA_FORMAT_INDEX) as usize;
        // self.format_list.get(actual_index).map(|s| s.as_str())
        todo!()
    }
}

// Trait definition (as you mentioned you'll define it)
pub trait DataFormat {
    fn get_format(&mut self, format: &str) -> u16;
    fn get_format_from_index(&self, index: u16) -> Option<&str>;
}
