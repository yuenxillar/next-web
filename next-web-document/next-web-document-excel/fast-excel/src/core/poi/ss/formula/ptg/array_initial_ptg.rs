use std::ops::{Deref, DerefMut};

use indexmap::IndexMap;
use next_web_core::anys::any_value::AnyValue;

use crate::core::poi::common::usermodel::generic_record::GenericRecord;
use crate::core::poi::ss::formula::constant::constant_value_parser::ConstantValueParser;
use crate::core::poi::ss::formula::ptg::array_ptg::ArrayPtg;
use crate::core::poi::ss::formula::ptg::{Ptg, PtgExt};
use crate::core::poi::ss::util::generic_record_util::GenericRecordUtil;
use crate::core::poi::util::little_endian_input::LittleEndianInput;
use crate::core::poi::util::little_endian_output::LittleEndianOutput;

/// Represents the initial plain tArray token (without the constant data that trails the whole
/// formula). Objects of this class are only temporary and cannot be used as `Ptg`s.
/// These temporary objects get converted to `ArrayPtg` by the `finish_reading` method.
pub struct ArrayInitialPtg {
    reserved0: i32,
    reserved1: u16,
    reserved2: u8,

    base: Ptg,
}

impl ArrayInitialPtg {
    /// Create a new ArrayInitialPtg from little endian input
    pub fn new(in_: &mut dyn LittleEndianInput) -> Self {
        ArrayInitialPtg {
            reserved0: in_.read_int(),
            reserved1: in_.read_u_short(),
            reserved2: in_.read_u_byte(),

            base: Default::default(),
        }
    }

    fn invalid<T>() -> T {
        panic!("This object is a partially initialised tArray, and cannot be used as a Ptg");
    }

    /// Read in the actual token (array) values. This occurs
    /// AFTER the last Ptg in the expression.
    /// See page 304-305 of Excel97-2007BinaryFileFormat(xls)Specification.pdf
    pub fn finish_reading(&self, in_: &mut dyn LittleEndianInput) -> ArrayPtg {
        let n_columns = in_.read_u_byte() as usize;
        let n_rows = in_.read_short() as usize;

        // The token_1_columns and token_2_rows do not follow the documentation.
        // The number of physical rows and columns is actually +1 of these values.
        // Which is not explicitly documented.
        let n_columns = n_columns + 1;
        let n_rows = n_rows + 1;

        let total_count = n_rows * n_columns;
        let array_values = ConstantValueParser::parse(in_, total_count);

        let mut result = ArrayPtg::new(
            self.reserved0,
            self.reserved1 as i16,
            self.reserved2 as i8,
            n_columns as u16,
            n_rows as u16,
            array_values,
        );

        result.set_class(self.get_ptg_class(), self).unwrap();
        result
    }
}

impl PtgExt for ArrayInitialPtg {
    fn get_size(&self) -> usize {
        ArrayPtg::PLAIN_TOKEN_SIZE
    }

    fn write(&self, _out: &mut dyn LittleEndianOutput) -> std::io::Result<()> {
        Self::invalid()
    }

    fn to_formula_string(&self) -> Option<String> {
        Self::invalid()
    }

    fn get_default_operand_class(&self) -> u8 {
        Self::invalid()
    }

    fn is_base_token(&self) -> bool {
        false
    }

    fn get_sid(&self) -> i8 {
        -1
    }
}

impl GenericRecord for ArrayInitialPtg {
    fn get_generic_properties(&self) -> Option<IndexMap<String, AnyValue>> {
        let properties = GenericRecordUtil::get_generic_properties3(
            "reserved0",
            AnyValue::Number(self.reserved0 as i64),
            "reserved1",
            AnyValue::Number(self.reserved1 as i64),
            "reserved2",
            AnyValue::Number(self.reserved2 as i64),
        );
        Some(properties)
    }
}

impl Deref for ArrayInitialPtg {
    type Target = Ptg;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for ArrayInitialPtg {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
