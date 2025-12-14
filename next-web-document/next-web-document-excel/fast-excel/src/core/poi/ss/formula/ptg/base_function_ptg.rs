use std::ops::{Deref, DerefMut};

use indexmap::IndexMap;
use next_web_core::anys::any_value::AnyValue;

use crate::core::poi::{
    common::usermodel::generic_record::GenericRecord,
    ss::{
        formula::{
            function::function_metadata_registry::FunctionMetadataRegistry,
            ptg::{
                PtgExt,
                operation_ptg::{OperationPtg, OperationPtgExt},
            },
        },
        util::generic_record_util::GenericRecordUtil,
    },
};

#[derive(Debug, Clone)]
pub struct BaseFunctionPtg {
    return_class: u8,
    param_class: Vec<u8>,

    number_of_args: u16,
    function_index: u16,

    base: OperationPtg,
}

impl BaseFunctionPtg {
    /// The name of the IF function (i.e. "IF").  Extracted as a constant for clarity.
    pub const FUNCTION_NAME_IF: &'static str = "IF";
    /// All external functions have function index 255
    const FUNCTION_INDEX_EXTERNAL: u8 = 255;

    pub fn new(
        function_index: u16,
        p_return_class: u8,
        param_types: Vec<u8>,
        n_params: u16,
    ) -> Self {
        Self {
            function_index,
            return_class: p_return_class,
            param_class: param_types,
            number_of_args: n_params,
            base: Default::default(),
        }
    }

    pub fn get_function_index(&self) -> u16 {
        self.function_index
    }

    pub fn get_name(&self) -> String {
        self.lookup_name(self.get_function_index())
    }

    pub fn to_formula_string_with_operands(&self, operands: &[String]) -> String {
        let mut buf = String::new();

        if self.is_external_function() {
            buf.push_str(&operands[0]); // first operand is actually the function name
            self.append_args(&mut buf, 1, operands);
        } else {
            buf.push_str(&self.get_name());
            self.append_args(&mut buf, 0, operands);
        }
        buf
    }

    /// external functions get some special processing
    /// @return {@code true} if this is an external function
    pub fn is_external_function(&self) -> bool {
        self.get_function_index() == (Self::FUNCTION_INDEX_EXTERNAL as u16)
    }

    fn append_args(&self, buf: &mut String, first_arg_ix: usize, operands: &[String]) {
        buf.push('(');
        for i in first_arg_ix..operands.len() {
            if i > first_arg_ix {
                buf.push(',');
            }
            buf.push_str(&operands[i]);
        }
        buf.push(')');
    }

    /// Used to detect whether a function name found in a formula is one of the standard excel functions
    /// <p>
    /// The name matching is case insensitive.
    /// @return {@code true} if the name specifies a standard worksheet function,
    ///  {@code false} if the name should be assumed to be an external function.
    pub fn is_built_in_function_name(name: &str) -> bool {
        let ix = FunctionMetadataRegistry::lookup_index_by_name(&name.to_uppercase());
        ix >= 0
    }

    fn lookup_name(&self, index: u16) -> String {
        self.lookup_name_with_cetab(index, false)
    }

    fn lookup_name_with_cetab(&self, index: u16, is_cetab: bool) -> String {
        if index == FunctionMetadataRegistry::FUNCTION_INDEX_EXTERNAL {
            return "#external#".to_string();
        }

        let fm = if is_cetab {
            FunctionMetadataRegistry::get_cetab_function_by_index(index)
        } else {
            FunctionMetadataRegistry::get_function_by_index(index)
        };

        match fm {
            Some(fm) => fm.get_name().into(),
            None => panic!("bad function index ({}, {})", index, is_cetab),
        }
    }

    /// Resolves internal function names into function indexes.
    /// The name matching is case insensitive.
    ///
    /// # Returns
    /// the standard worksheet function index if found, otherwise {@code FUNCTION_INDEX_EXTERNAL}
    fn lookup_index(name: &str) -> u16 {
        let ix = FunctionMetadataRegistry::lookup_index_by_name(&name.to_uppercase());
        if ix < 0 {
            Self::FUNCTION_INDEX_EXTERNAL as u16
        } else {
            ix as u16
        }
    }

    pub fn get_parameter_class(&self, index: usize) -> u8 {
        let len = self.param_class.len();
        if index >= len {
            // For var-arg (and other?) functions, the metadata does not list all the parameter
            // operand classes.  In these cases, all extra parameters are assumed to have the
            // same operand class as the last one specified.
            self.param_class[len - 1]
        } else {
            self.param_class[index]
        }
    }
}

/// This class provides the base functionality for Excel sheet functions
/// There are two kinds of function Ptgs - tFunc and tFuncVar
/// Therefore, this class will have ONLY two subclasses
impl PtgExt for BaseFunctionPtg {
    fn is_base_token(&self) -> bool {
        false
    }

    fn write(
        &self,
        _out: &mut dyn crate::core::poi::util::little_endian_output::LittleEndianOutput,
    ) -> std::io::Result<()> {
        unimplemented!()
    }

    fn get_size(&self) -> usize {
        unimplemented!()
    }

    fn get_sid(&self) -> i8 {
        unimplemented!()
    }

    fn to_formula_string(&self) -> Option<String> {
        Some(self.get_name())
    }

    fn get_default_operand_class(&self) -> u8 {
        self.return_class
    }
}

impl GenericRecord for BaseFunctionPtg {
    fn get_generic_properties(&self) -> Option<IndexMap<String, AnyValue>> {
        let properties = GenericRecordUtil::get_generic_properties5(
            "functionIndex",
            AnyValue::Number(self.get_function_index() as i64),
            "functionName",
            AnyValue::String(self.get_name()),
            "numberOfOperands",
            AnyValue::Number(self.get_number_of_operands() as i64),
            "externalFunction",
            AnyValue::Boolean(self.is_external_function()),
            "defaultOperandClass",
            AnyValue::Number(self.get_default_operand_class() as i64),
        );

        Some(properties)
    }
}

impl OperationPtgExt for BaseFunctionPtg {
    fn get_number_of_operands(&self) -> u16 {
        self.number_of_args
    }
}

impl Deref for BaseFunctionPtg {
    type Target = OperationPtg;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for BaseFunctionPtg {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

pub trait BaseFunctionPtgExt {
    fn get_size(&self) -> usize;
}
