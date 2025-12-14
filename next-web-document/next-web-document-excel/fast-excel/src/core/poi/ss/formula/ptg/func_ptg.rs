use std::io;

use crate::core::poi::ss::formula::function::function_metadata::FunctionMetadata;
use crate::core::poi::ss::formula::function::function_metadata_registry::FunctionMetadataRegistry;
use crate::core::poi::ss::formula::ptg::PtgExt;
use crate::core::poi::ss::formula::ptg::base_function_ptg::BaseFunctionPtg;
use crate::core::poi::util::little_endian_input::LittleEndianInput;
use crate::core::poi::util::little_endian_output::LittleEndianOutput;

/// FuncPtg - Represents a built-in function call in a formula
///
/// This PTG is used for functions with a fixed number of arguments
#[derive(Debug, Clone)]
pub struct FuncPtg {
    base: BaseFunctionPtg,
}

impl FuncPtg {
    /// PTG structure ID for FuncPtg
    pub const SID: i8 = 0x21;

    /// Size of FuncPtg in bytes (including the SID byte)
    pub const SIZE: usize = 3;

    /// Creates a new FuncPtg from the input stream
    pub fn create(input: &mut dyn LittleEndianInput) -> io::Result<Self> {
        Self::create_with_index(input.read_u_short())
    }

    /// Creates a new FuncPtg from a function index
    pub fn create_with_index(function_index: u16) -> io::Result<Self> {
        let registry = FunctionMetadataRegistry::get_function_by_index(function_index);
        let metadata = registry.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid built-in function index ({})", function_index),
            )
        })?;

        Ok(Self::new(function_index, metadata))
    }

    /// Creates a new FuncPtg with the specified function metadata
    fn new(function_index: u16, fm: &FunctionMetadata) -> Self {
        Self {
            base: BaseFunctionPtg::new(
                function_index,
                fm.get_return_class_code(),
                fm.get_parameter_class_codes(),
                fm.get_min_params(),
            ),
        }
    }
}

impl PtgExt for FuncPtg {
    fn get_size(&self) -> usize {
        Self::SIZE
    }

    fn write(&self, out: &mut dyn LittleEndianOutput) -> io::Result<()> {
        out.write_byte((Self::SID as u8) + self.base.get_ptg_class())?;
        out.write_short(self.base.get_function_index())?;

        Ok(())
    }

    fn get_default_operand_class(&self) -> u8 {
        self.base.get_default_operand_class()
    }

    fn to_formula_string(&self) -> Option<String> {
        self.base.to_formula_string()
    }

    fn is_base_token(&self) -> bool {
        false // This is a classified token
    }

    fn get_sid(&self) -> i8 {
        Self::SID
    }
}
