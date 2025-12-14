use std::ops::{Deref, DerefMut};

use indexmap::IndexMap;
use next_web_core::anys::any_value::AnyValue;

use crate::core::poi::{
    common::usermodel::generic_record::GenericRecord,
    ss::{
        formula::{
            formula_rendering_workbook::FormulaRenderingWorkbook,
            ptg::{Ptg, PtgExt, operand_ptg::OperandPtg},
        },
        util::generic_record_util::GenericRecordUtil,
    },
    util::{little_endian_input::LittleEndianInput, little_endian_output::LittleEndianOutput},
};

#[derive(Debug, Clone)]
/// Represents a named range reference in a formula
pub struct NamePtg {
    /// one-based index to defined name record
    field_1_label_index: u16,
    /// reserved must be 0
    field_2_zero: i16,

    base: OperandPtg,
}

impl NamePtg {
    pub const SID: i8 = 0x23;
    const SIZE: usize = 5;

    /// Creates a new NamePtg with the specified name index
    ///
    /// # Arguments
    /// * `name_index` - zero-based index to name within workbook
    pub fn new(name_index: u16) -> Self {
        NamePtg {
            field_1_label_index: 1 + name_index, // convert to 1-based
            field_2_zero: 0,
            base: Default::default(),
        }
    }

    /// Creates a new NamePtg from binary input
    pub fn from_binary(in_stream: &mut dyn LittleEndianInput) -> std::io::Result<Self> {
        Ok(NamePtg {
            field_1_label_index: in_stream.read_u_short(),
            field_2_zero: in_stream.read_short(),
            base: Default::default(),
        })
    }

    /// Returns zero based index to a defined name record in the LinkTable
    pub fn get_index(&self) -> u16 {
        self.field_1_label_index - 1 // convert to zero based
    }

    /// Converts this PTG to a formula string using the provided workbook
    fn to_formula_string_with_book(&self, book: &dyn FormulaRenderingWorkbook) -> String {
        book.get_name_text(self)
            .map(ToString::to_string)
            .unwrap_or_default()
    }
}

impl PtgExt for NamePtg {
    fn is_base_token(&self) -> bool {
        false
    }

    /// Returns the size in bytes of this PTG when serialized
    fn get_size(&self) -> usize {
        Self::SIZE
    }

    /// Returns the SID of this PTG
    fn get_sid(&self) -> i8 {
        Self::SID
    }

    fn to_formula_string(&self) -> Option<String> {
        panic!("3D references need a workbook to determine formula text");
    }

    /// Writes the binary representation of this PTG to output
    fn write(&self, out: &mut dyn LittleEndianOutput) -> std::io::Result<()> {
        out.write_byte((Self::SID as u8) + self.base.get_ptg_class())?;
        out.write_short(self.field_1_label_index)?;
        out.write_short(self.field_2_zero as u16)?;
        Ok(())
    }

    /// Gets the default operand class for this PTG
    fn get_default_operand_class(&self) -> u8 {
        Ptg::CLASS_REF
    }
}

impl GenericRecord for NamePtg {
    fn get_generic_properties(&self) -> Option<IndexMap<String, AnyValue>> {
        Some(GenericRecordUtil::get_generic_properties_1(
            "index",
            AnyValue::Number(self.get_index() as i64),
        ))
    }
}

impl Deref for NamePtg {
    type Target = OperandPtg;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for NamePtg {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
