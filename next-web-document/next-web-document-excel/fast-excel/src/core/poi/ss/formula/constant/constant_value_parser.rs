use std::panic;

use next_web_core::anys::any_value::AnyValue;
use tracing::warn;

use crate::core::poi::{
    ss::{formula::constant::error_constant::ErrorConstant, util::string_util::StringUtil},
    util::{little_endian_input::LittleEndianInput, little_endian_output::LittleEndianOutput},
};

/// To support Constant Values (2.5.7) as required by the CRN record.
/// This class is also used for two dimensional arrays which are encoded by
/// EXTERNALNAME (5.39) records and Array tokens.
pub struct ConstantValueParser;

impl ConstantValueParser {
    // note - these (non-combinable) enum values are sparse.
    const TYPE_EMPTY: u8 = 0;
    const TYPE_NUMBER: u8 = 1;
    const TYPE_STRING: u8 = 2;
    const TYPE_BOOLEAN: u8 = 4;
    // TODO - update OOO document to include this value
    const TYPE_ERROR_CODE: u8 = 16;

    const TRUE_ENCODING: u8 = 1;
    const FALSE_ENCODING: u8 = 0;

    // TODO - is this the best way to represent 'EMPTY'?
    const EMPTY_REPRESENTATION: AnyValue = AnyValue::Null;

    /// Parse constant values from a little endian input
    ///
    /// # Arguments
    /// * `in_` - Little endian input
    /// * `n_values` - Number of values to parse
    ///
    /// # Returns
    /// * Vector of constant values
    pub fn parse(in_: &mut dyn LittleEndianInput, n_values: usize) -> Vec<AnyValue> {
        if n_values == 0 {
            return Vec::new();
        }

        let mut result = Vec::with_capacity(n_values);
        for _ in 0..n_values {
            result.push(Self::read_a_constant_value(in_));
        }
        result
    }

    fn read_a_constant_value(in_: &mut dyn LittleEndianInput) -> AnyValue {
        let grbit = in_.read_byte() as u8;
        match grbit {
            Self::TYPE_EMPTY => {
                in_.read_long(); // 8 byte 'not used' field
                Self::EMPTY_REPRESENTATION
            }
            Self::TYPE_NUMBER => AnyValue::Number(in_.read_double() as i64),
            Self::TYPE_STRING => AnyValue::String(StringUtil::read_unicode_string(in_).unwrap()),
            Self::TYPE_BOOLEAN => Self::read_boolean(in_),
            Self::TYPE_ERROR_CODE => {
                let err_code = in_.read_u_short();
                // next 6 bytes are unused
                in_.read_u_short();
                in_.read_int();
                AnyValue::Object(Box::new(ErrorConstant::new(err_code as i32)))
            }
            _ => panic!("Unknown grbit value ({})", grbit),
        }
    }

    fn read_boolean(in_: &mut dyn LittleEndianInput) -> AnyValue {
        let val = in_.read_long() as u8; // read byte from 7 bytes 'not used'

        match val {
            Self::FALSE_ENCODING => AnyValue::Boolean(false),
            Self::TRUE_ENCODING => AnyValue::Boolean(true),
            _ => panic!("unexpected boolean encoding ({})", val),
        }
    }

    /// Get the encoded size of constant values
    ///
    /// # Arguments
    /// * `values` - Array of constant values
    ///
    /// # Returns
    /// * Encoded size in bytes
    pub fn get_encoded_size(values: &Vec<AnyValue>) -> usize {
        // start with one byte 'type' code for each value
        let mut result = values.len();
        for value in values {
            result += Self::get_encoded_size_single(value);
        }
        result
    }

    /// Get encoded size without the 'type' code byte
    fn get_encoded_size_single(object: &AnyValue) -> usize {
        match object {
            AnyValue::Null | AnyValue::Boolean(_) | AnyValue::Float(_) => 8,
            AnyValue::String(str_val) => StringUtil::get_encoded_size(str_val),
            _ => 8,
        }
    }

    /// Encode constant values to output
    ///
    /// # Arguments
    /// * `out` - Little endian output
    /// * `values` - Array of constant values
    pub fn encode(out: &mut dyn LittleEndianOutput, values: &Vec<AnyValue>) {
        for value in values {
            Self::encode_single_value(out, value);
        }
    }

    fn encode_single_value(out: &mut dyn LittleEndianOutput, value: &AnyValue) {
        match value {
            AnyValue::Null => {
                out.write_byte(Self::TYPE_EMPTY);
                out.write_long(0);
            }
            AnyValue::Boolean(val) => {
                out.write_byte(Self::TYPE_BOOLEAN);
                let long_val = if *val { 1 } else { 0 };
                out.write_long(long_val);
            }
            AnyValue::Float(val) => {
                out.write_byte(Self::TYPE_NUMBER);
                out.write_double(*val);
            }
            AnyValue::String(val) => {
                out.write_byte(Self::TYPE_STRING);
                StringUtil::write_unicode_string(out, val);
            }
            AnyValue::Object(obj) => {
                out.write_byte(Self::TYPE_ERROR_CODE);
                if let Some(error_val) = (obj as &dyn std::any::Any).downcast_ref::<ErrorConstant>()
                {
                    out.write_long(error_val.get_error_code() as u64);
                } else {
                    warn!("Conversion 'ErrorConstant' failed ({:?})", obj);
                }
            }

            _ => panic!("Unexpected value type ({})", value.to_string()),
        }
    }
}
