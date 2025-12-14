use std::ops::{Deref, DerefMut};

use indexmap::IndexMap;
use next_web_core::anys::any_value::AnyValue;

use crate::core::poi::common::usermodel::generic_record::GenericRecord;
use crate::core::poi::ss::formula::constant::constant_value_parser::ConstantValueParser;
use crate::core::poi::ss::formula::constant::error_constant::ErrorConstant;
use crate::core::poi::ss::formula::ptg::{Ptg, PtgExt};
use crate::core::poi::ss::util::generic_record_util::GenericRecordUtil;
use crate::core::poi::ss::util::number_to_text_converter::NumberToTextConverter;
use crate::core::poi::util::little_endian_output::LittleEndianOutput;

/// ArrayPtg - handles arrays
///
/// The ArrayPtg is a little weird, the size of the Ptg when parsing initially only
/// includes the Ptg sid and the reserved bytes. The next Ptg in the expression then follows.
/// It is only after the "size" of all the Ptgs is met, that the ArrayPtg data is actually
/// held after this. So Ptg.createParsedExpression keeps track of the number of
/// ArrayPtg elements and need to parse the data upto the FORMULA record size.
pub struct ArrayPtg {
    // 7 bytes of data (stored as an int, short and byte here)
    reserved0_int: i32,
    reserved1_short: i16,
    reserved2_byte: i8,

    // data from these fields comes after the Ptg data of all tokens in current formula
    n_columns: u16,
    n_rows: u16,
    array_values: Vec<AnyValue>,

    base: Ptg,
}

impl ArrayPtg {
    /// Constant SID value for ArrayPtg
    pub const SID: i8 = 0x20;

    const RESERVED_FIELD_LEN: usize = 7;
    /// The size of the plain tArray token written within the standard formula tokens
    /// (not including the data which comes after all formula tokens)
    pub const PLAIN_TOKEN_SIZE: usize = 1 + Self::RESERVED_FIELD_LEN;

    /// Create a new ArrayPtg with reserved fields
    pub fn new(
        reserved0: i32,
        reserved1: i16,
        reserved2: i8,
        n_columns: u16,
        n_rows: u16,
        array_values: Vec<AnyValue>,
    ) -> Self {
        ArrayPtg {
            reserved0_int: reserved0,
            reserved1_short: reserved1,
            reserved2_byte: reserved2,
            base: Default::default(),
            n_columns,
            n_rows,
            array_values,
        }
    }

    /// Create an ArrayPtg from a 2D array of values
    ///
    /// # Arguments
    /// * `values_2d` - Array values arranged in rows
    pub fn from_2d_array(values_2d: Vec<Vec<AnyValue>>) -> Self {
        let n_rows = values_2d.len() as u16;
        let n_columns = if n_rows > 0 {
            values_2d[0].len() as u16
        } else {
            0
        };

        // Convert 2-d to 1-d array (row by row according to get_value_index())
        let mut array_values = Vec::with_capacity((n_columns * n_rows) as usize);

        for (r, row_data) in values_2d.iter().enumerate() {
            for (c, value) in row_data.iter().enumerate() {
                let index = Self::get_value_index(c as u16, r as u16, n_columns);
                array_values.insert(index as usize, value.clone());
            }
        }

        ArrayPtg {
            reserved0_int: 0,
            reserved1_short: 0,
            reserved2_byte: 0,
            base: Default::default(),
            n_columns,
            n_rows,
            array_values,
        }
    }

    /// Get the 2-d array (inner index is row_ix, outer index is col_ix)
    ///
    /// # Returns
    /// * 2D array of values
    pub fn get_token_array_values(&self) -> Vec<Vec<AnyValue>> {
        let mut result = vec![
            vec![AnyValue::String("".to_string()); self.n_columns as usize];
            self.n_rows as usize
        ];

        for r in 0..self.n_rows {
            for c in 0..self.n_columns {
                let index = Self::get_value_index(c, r, self.n_columns);
                result[r as usize][c as usize] = self.array_values[index as usize].clone();
            }
        }
        result
    }

    /// Note - (2D) array elements are stored row by row
    ///
    /// # Arguments
    /// * `col_ix` - Column index
    /// * `row_ix` - Row index
    /// * `n_columns` - Total number of columns
    ///
    /// # Returns
    /// * The index into the internal 1D array for the specified column and row
    fn get_value_index(col_ix: u16, row_ix: u16, n_columns: u16) -> u16 {
        if col_ix >= n_columns {
            panic!(
                "Specified col_ix ({}) is outside the allowed range (0..{})",
                col_ix,
                n_columns - 1
            );
        }
        row_ix * n_columns + col_ix
    }

    /// Write the token value bytes (data that comes after all formula tokens)
    ///
    /// # Arguments
    /// * `out` - Little endian output
    ///
    /// # Returns
    /// * Number of bytes written
    pub fn write_token_value_bytes(&self, out: &mut dyn LittleEndianOutput) -> usize {
        out.write_byte((self.n_columns - 1) as u8);
        out.write_short(self.n_rows - 1);

        // Encode array values
        ConstantValueParser::encode(out, &self.array_values);

        3 + ConstantValueParser::get_encoded_size(&self.array_values)
    }

    /// Get the row count
    ///
    /// # Returns
    /// * Number of rows
    pub fn get_row_count(&self) -> u16 {
        self.n_rows
    }

    /// Get the column count
    ///
    /// # Returns
    /// * Number of columns
    pub fn get_column_count(&self) -> u16 {
        self.n_columns
    }

    /// Get the constant text representation of a value
    ///
    /// # Arguments
    /// * `value` - Array value
    ///
    /// # Returns
    /// * Text representation
    fn get_constant_text(value: &AnyValue) -> Result<String, String> {
        let result = match value {
            AnyValue::String(s) => format!("\"{}\"", s),
            AnyValue::Float(d) => NumberToTextConverter::to_text(*d as f64),
            AnyValue::Boolean(b) => {
                if *b {
                    "TRUE".to_string()
                } else {
                    "FALSE".to_string()
                }
            }
            AnyValue::Object(e) => {
                if let Some(e) = (e as &dyn std::any::Any).downcast_ref::<ErrorConstant>() {
                    return Ok(e.get_text());
                }
                return Err(format!("Unsupported type: {:?}", value));
            }
            _ => return Err(format!("Unsupported type: {:?}", value)),
        };

        Ok(result)
    }

    pub fn get_ptg_class(&self) -> u8 {
        Ptg::CLASS_ARRAY
    }
}

impl PtgExt for ArrayPtg {
    fn get_size(&self) -> usize {
        Self::PLAIN_TOKEN_SIZE
            // data written after the all tokens:
            + 1 + 2 // column, row
            + ConstantValueParser::get_encoded_size(&self.array_values)
    }

    fn write(&self, out: &mut dyn LittleEndianOutput) -> std::io::Result<()> {
        out.write_byte((Self::SID as u8) + self.get_ptg_class())?;
        out.write_int(self.reserved0_int as u32)?;
        out.write_short(self.reserved1_short as u16)?;
        out.write_byte(self.reserved2_byte as u8)?;

        Ok(())
    }

    fn to_formula_string(&self) -> Option<String> {
        let mut b = String::new();
        b.push('{');

        for y in 0..self.n_rows {
            if y > 0 {
                b.push(';');
            }
            for x in 0..self.n_columns {
                if x > 0 {
                    b.push(',');
                }
                let index = Self::get_value_index(x, y, self.n_columns);
                let value = &self.array_values[index as usize];
                b.push_str(&Self::get_constant_text(value).ok()?);
            }
        }
        b.push('}');

        Some(b)
    }

    fn get_default_operand_class(&self) -> u8 {
        Ptg::CLASS_ARRAY
    }

    fn is_base_token(&self) -> bool {
        false
    }

    fn get_sid(&self) -> i8 {
        Self::SID
    }
}

impl GenericRecord for ArrayPtg {
    fn get_generic_properties(&self) -> Option<IndexMap<String, AnyValue>> {
        let s = self.to_formula_string().unwrap_or_default();
        let properties = GenericRecordUtil::get_generic_properties6(
            "reserved0",
            AnyValue::Number(self.reserved0_int as i64),
            "reserved1",
            AnyValue::Number(self.reserved1_short as i64),
            "reserved2",
            AnyValue::Number(self.reserved2_byte as i64),
            "columnCount",
            AnyValue::Number(self.get_column_count() as i64),
            "rowCount",
            AnyValue::Number(self.get_row_count() as i64),
            "arrayValues",
            AnyValue::String(if s.is_empty() {
                "#values#uninitialised#".to_string()
            } else {
                s
            }),
        );

        Some(properties)
    }
}

impl Deref for ArrayPtg {
    type Target = Ptg;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for ArrayPtg {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
