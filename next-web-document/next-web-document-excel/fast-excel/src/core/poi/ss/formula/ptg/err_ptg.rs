use indexmap::IndexMap;
use next_web_core::{anys::any_value::AnyValue, error::BoxError};

use crate::core::poi::{
    common::usermodel::generic_record::GenericRecord,
    ss::{
        formula::ptg::{PtgExt, scalar_constant_ptg::ScalarConstantPtg},
        usermodel::formula_error::FormulaError,
        util::generic_record_util::GenericRecordUtil,
    },
    util::{little_endian_input::LittleEndianInput, little_endian_output::LittleEndianOutput},
};

/// ErrPtg - Represents an error constant in a formula
///
/// Error constants include: #NULL!, #DIV/0!, #VALUE!, #REF!, #NAME?, #NUM!, #N/A
#[derive(Debug, Clone)]
pub struct ErrPtg {
    field_1_error_code: i32,
    base: ScalarConstantPtg,
}

// Static instances for common error types
impl ErrPtg {
    /// PTG structure ID for ErrPtg
    pub const SID: i8 = 0x1C;

    /// Size of ErrPtg in bytes (including the SID byte)
    const SIZE: usize = 2;

    /// Creates a new ErrPtg with the specified error code
    ///
    /// # Arguments
    /// * `error_code` - The error code (must be valid)
    ///
    /// # Panics
    /// Panics if the error code is invalid
    pub fn from_code(error_code: i32) -> Result<Self, BoxError> {
        if !FormulaError::is_valid_code(error_code) {
            return Err(format!("Invalid error code (0x{:02X})", error_code).into());
        }
        Ok(Self {
            field_1_error_code: error_code,
            base: Default::default(),
        })
    }

    /// Creates an ErrPtg from a FormulaError enum
    pub fn from_error(error: FormulaError) -> Self {
        Self::from_code(error.get_long_code()).unwrap()
    }

    /// Reads an ErrPtg from the input stream
    pub fn read(input: &mut dyn LittleEndianInput) -> Result<Self, BoxError> {
        Self::value_of(input.read_byte() as i32)
    }

    pub fn get_ptg_class(&self) -> u8 {
        return self.base.get_ptg_class();
    }

    pub fn value_of(code: i32) -> Result<Self, BoxError> {
        match FormulaError::from_int(code) {
            // <b>#DIV/0!</b> - Division by zero
            FormulaError::Div0 => Ok(ErrPtg::from_error(FormulaError::Div0)),
            // <b>#N/A</b> - Argument or function not available
            FormulaError::Na => Ok(ErrPtg::from_error(FormulaError::Na)),
            // <b>#NAME?</b> - Wrong function or range name
            FormulaError::Name => Ok(ErrPtg::from_error(FormulaError::Name)),
            // <b>#NULL!</b> - Intersection of two cell ranges is empty
            FormulaError::Null => Ok(ErrPtg::from_error(FormulaError::Null)),
            // <b>#NUM!</b> - Value range overflow
            FormulaError::Num => Ok(ErrPtg::from_error(FormulaError::Num)),
            // <b>#REF!</b> - Illegal or deleted cell reference
            FormulaError::Ref => Ok(ErrPtg::from_error(FormulaError::Ref)),
            // <b>#VALUE!</b> - Wrong type of operand
            FormulaError::Value => Ok(ErrPtg::from_error(FormulaError::Value)),
            _ => Err(format!("Unexpected error code ({})", code).into()),
        }
    }

    /// Returns the error code
    pub fn get_error_code(&self) -> i32 {
        self.field_1_error_code
    }

    /// Returns the FormulaError enum for this error
    pub fn get_error(&self) -> FormulaError {
        FormulaError::from_code(self.field_1_error_code)
    }
}

impl PtgExt for ErrPtg {
    fn is_base_token(&self) -> bool {
        self.base.is_base_token()
    }

    fn get_size(&self) -> usize {
        Self::SIZE
    }

    fn write(&self, out: &mut dyn LittleEndianOutput) -> std::io::Result<()> {
        out.write_byte(0x1c + self.get_ptg_class())?;
        out.write_byte(self.field_1_error_code as u8)?;
        Ok(())
    }

    fn to_formula_string(&self) -> Option<String> {
        Some(FormulaError::from_int(self.field_1_error_code as i32).get_string())
    }

    fn get_default_operand_class(&self) -> u8 {
        self.base.get_default_operand_class()
    }

    fn get_sid(&self) -> i8 {
        Self::SID
    }
}

impl GenericRecord for ErrPtg {
    fn get_generic_properties(&self) -> Option<IndexMap<String, AnyValue>> {
        Some(GenericRecordUtil::get_generic_properties_1(
            "errorCode",
            AnyValue::Number(self.get_error_code() as i64),
        ))
    }
}
