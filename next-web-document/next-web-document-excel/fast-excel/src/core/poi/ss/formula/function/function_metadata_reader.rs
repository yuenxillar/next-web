use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use next_web_core::error::BoxError;
use once_cell::sync::Lazy;

use crate::core::poi::ss::formula::function::function_data_builder::FunctionDataBuilder;
use crate::core::poi::ss::formula::function::function_metadata_registry::FunctionMetadataRegistry;
use crate::core::poi::ss::formula::ptg::Ptg;

static MAX_RECORD_LENGTH: Lazy<Arc<AtomicUsize>> = Lazy::new(|| {
    Arc::new(AtomicUsize::new(
        FunctionMetadataReader::DEFAULT_MAX_RECORD_LENGTH,
    ))
});

pub(super) struct FunctionMetadataReader;

impl FunctionMetadataReader {
    // arbitrarily selected; may need to increase
    const DEFAULT_MAX_RECORD_LENGTH: usize = 100_000;

    const METADATA_FILE_NAME: &str = "functionMetadata.txt";
    const METADATA_FILE_NAME_CETAB: &str = "functionMetadataCetab.txt";

    /** Plain ASCII text metadata file uses three dots for ellipsis */
    const ELLIPSIS: &str = "...";

    const EMPTY_BYTE_ARRAY: &[u8] = &[];

    const DIGIT_ENDING_FUNCTION_NAMES: [&str; 7] = [
        // Digits at the end of a function might be due to a left-over footnote marker.
        // except in these cases
        "LOG10", "ATAN2", "DAYS360", "SUMXMY2", "SUMX2MY2", "SUMX2PY2", "A1.R1C1",
    ];

    /// Sets the maximum record length allowed for FunctionMetadataReader
    pub fn set_max_record_length(length: usize) {
        MAX_RECORD_LENGTH.store(length, Ordering::Relaxed);
    }

    ///  Gets the maximum record length allowed for FunctionMetadataReader
    pub fn get_max_record_length() -> usize {
        MAX_RECORD_LENGTH.load(Ordering::Relaxed)
    }

    /// Creates the standard function metadata registry
    pub fn create_registry() -> Result<FunctionMetadataRegistry, BoxError> {
        let mut fdb = FunctionDataBuilder::new(800);
        Self::read_resource_file(&mut fdb, Self::METADATA_FILE_NAME)?;
        Ok(fdb.build())
    }

    /// Creates the CETAB function metadata registry
    pub fn create_registry_cetab() -> Result<FunctionMetadataRegistry, BoxError> {
        let mut fdb = FunctionDataBuilder::new(800);
        Self::read_resource_file(&mut fdb, Self::METADATA_FILE_NAME_CETAB)?;
        Ok(fdb.build())
    }

    ///  Reads and processes the resource file
    fn read_resource_file(
        fdb: &mut FunctionDataBuilder,
        resource_file: &str,
    ) -> Result<(), BoxError> {
        let path = Path::new(resource_file);
        let file = File::open(&path)
            .map_err(|e| format!("Resource '{}' not found: {}", resource_file, e))?;

        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            if line.len() < 1 || line.starts_with('#') {
                continue;
            }

            let trim_line = line.trim();
            if trim_line.len() < 1 {
                continue;
            }

            Self::process_line(fdb, &line)?;
        }

        Ok(())
    }

    /// Processes a single line from the metadata file
    fn process_line(fdb: &mut FunctionDataBuilder, line: &str) -> Result<(), BoxError> {
        let parts: Vec<&str> = line.split('\t').collect();

        if parts.len() != 8 {
            return Err(format!(
                "Bad line format '{}' - expected 8 data fields delimited by tab, but had {}: {:?}",
                line,
                parts.len(),
                parts
            )
            .into());
        }

        let function_index = Self::parse_int(parts[0])?;
        let function_name = parts[1];
        let min_params = Self::parse_int(parts[2])?;
        let max_params = Self::parse_int(parts[3])?;
        let return_class_code = Self::parse_return_type_code(parts[4])?;
        let parameter_class_codes = Self::parse_operand_type_codes(parts[5])?;
        // parts[6] is isVolatile - currently not used by POI
        let has_note = !parts[7].is_empty();

        Self::validate_function_name(function_name)?;

        // TODO - make POI use isVolatile
        fdb.add(
            function_index,
            function_name,
            min_params as u16,
            max_params as u16,
            return_class_code,
            parameter_class_codes,
            has_note,
        )?;

        Ok(())
    }

    /// Parses the return type code
    fn parse_return_type_code(code: &str) -> Result<u8, BoxError> {
        if code.is_empty() {
            // happens for GETPIVOTDATA
            return Ok(Ptg::CLASS_REF);
        }
        Self::parse_operand_type_code(code)
    }

    /// Parses operand type codes from a space-separated string
    fn parse_operand_type_codes(codes: &str) -> Result<Vec<u8>, BoxError> {
        if codes.is_empty() {
            // happens for GETPIVOTDATA
            return Ok(Self::EMPTY_BYTE_ARRAY.to_vec());
        }

        if Self::is_dash(codes) {
            // '-' means empty
            return Ok(Self::EMPTY_BYTE_ARRAY.to_vec());
        }

        let array: Vec<&str> = codes.split(' ').collect();
        let mut n_items = array.len();

        if let Some(last) = array.last() {
            if *last == Self::ELLIPSIS {
                // final ellipsis is optional, and ignored
                // (all unspecified params are assumed to be the same as the last)
                n_items -= 1;
            }
        }

        let max_len = Self::get_max_record_length();
        if n_items > max_len {
            return Err(format!(
                "Requested allocation size {} exceeds maximum {}",
                n_items, max_len
            )
            .into());
        }

        let mut result = Vec::with_capacity(n_items);
        for i in 0..n_items {
            result.push(Self::parse_operand_type_code(array[i])?);
        }

        Ok(result)
    }

    ///Checks if the string is a single dash
    fn is_dash(codes: &str) -> bool {
        codes.len() == 1 && codes.starts_with('-')
    }

    /// Parses a single operand type code character
    fn parse_operand_type_code(code: &str) -> Result<u8, BoxError> {
        if code.len() != 1 {
            return Err(format!(
                "Bad operand type code format '{}' expected single char",
                code
            )
            .into());
        }

        match code.chars().next().unwrap() {
            'V' => Ok(Ptg::CLASS_VALUE),
            'R' => Ok(Ptg::CLASS_REF),
            'A' => Ok(Ptg::CLASS_ARRAY),
            _ => Err(format!("Unexpected operand type code '{}'", code).into()),
        }
    }

    /// Validates that footnote digits from the original OOO document have not been accidentally left behind
    fn validate_function_name(function_name: &str) -> Result<(), BoxError> {
        let len = function_name.len();
        if len == 0 {
            return Ok(());
        }

        if let Some(last_char) = function_name.chars().last() {
            if !last_char.is_ascii_digit() {
                return Ok(());
            }
        }

        let mut ix = len - 1;
        let chars: Vec<char> = function_name.chars().collect();

        while ix > 0 {
            if let Some(prev_char) = chars.get(ix) {
                if !prev_char.is_ascii_digit() {
                    break;
                }
            }
            ix -= 1;
        }

        if Self::DIGIT_ENDING_FUNCTION_NAMES.contains(&function_name) {
            return Ok(());
        }

        Err(format!(
            "Invalid function name '{}' (is footnote number incorrectly appended)",
            function_name
        )
        .into())
    }

    /// Parses an integer from string with error handling
    fn parse_int(val_str: &str) -> Result<i32, BoxError> {
        i32::from_str(val_str).map_err(|e| {
            format!(
                "Value '{}' could not be parsed as an integer: {}",
                val_str, e
            )
            .into()
        })
    }
}
