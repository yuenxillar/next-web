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

/// A Name, be that a Named Range or a Function / User Defined
/// Function, addressed in the HSSF External Sheet style.
///
/// # Note
/// This is HSSF only, as it matches the HSSF file format way of
/// referring to the sheet by an extern index. The XSSF equivalent
/// is `NameXPxg`
pub struct NameXPtg {
    /// index to REF entry in externsheet record
    sheet_ref_index: u16,
    /// index to defined name or externname table(1 based)
    name_number: u16,
    /// reserved must be 0
    reserved: u16,

    base: OperandPtg,
}

impl NameXPtg {
    pub const SID: i8 = 0x39;
    const SIZE: usize = 7;

    pub fn new(sheet_ref_index: u16, name_number: u16, reserved: u16) -> Self {
        Self {
            sheet_ref_index,
            name_number, // Convert from 0-based to 1-based
            reserved,
            base: Default::default(),
        }
    }

    /// Create a new NameXPtg
    ///
    /// # Arguments
    /// * `sheet_ref_index` - index to REF entry in externsheet record
    /// * `name_index` - index to defined name or externname table (0-based)
    pub fn new_with_index(sheet_ref_index: u16, name_index: u16) -> Self {
        Self {
            sheet_ref_index,
            name_number: name_index + 1, // Convert from 0-based to 1-based
            reserved: 0,
            base: Default::default(),
        }
    }

    /// Read NameXPtg from LittleEndian input
    pub fn from_input(inp: &mut dyn LittleEndianInput) -> Self {
        Self {
            sheet_ref_index: inp.read_u_short(),
            name_number: inp.read_u_short(),
            reserved: inp.read_u_short(),
            base: Default::default(),
        }
    }

    /// Convert to formula string using a workbook
    pub fn to_formula_string_with_book(&self, book: &dyn FormulaRenderingWorkbook) -> String {
        // -1 to convert definedNameIndex from 1-based to zero-based
        book.resolve_name_x_text(self)
    }

    /// Get sheet reference index
    pub fn get_sheet_ref_index(&self) -> u16 {
        self.sheet_ref_index
    }

    /// Get name index (0-based)
    pub fn get_name_index(&self) -> u16 {
        self.name_number - 1 // Convert from 1-based to 0-based
    }
}

impl PtgExt for NameXPtg {
    fn is_base_token(&self) -> bool {
        false
    }

    fn write(&self, out: &mut dyn LittleEndianOutput) -> std::io::Result<()> {
        out.write_byte((Self::SID as u8) + self.get_ptg_class())?;
        out.write_short(self.sheet_ref_index)?;
        out.write_short(self.name_number)?;
        out.write_short(self.reserved)?;
        Ok(())
    }

    fn get_size(&self) -> usize {
        Self::SIZE
    }

    fn get_sid(&self) -> i8 {
        Self::SID
    }
    fn to_formula_string(&self) -> Option<String> {
        panic!("3D references need a workbook to determine formula text")
    }

    fn get_default_operand_class(&self) -> u8 {
        Ptg::CLASS_VALUE
    }
}

impl GenericRecord for NameXPtg {
    fn get_generic_properties(&self) -> Option<IndexMap<String, AnyValue>> {
        let properties = GenericRecordUtil::get_generic_properties_2(
            "sheetRefIndex",
            AnyValue::Number(self.get_sheet_ref_index() as i64),
            "nameIndex",
            AnyValue::Number(self.get_name_index() as i64),
        );

        Some(properties)
    }
}

impl Deref for NameXPtg {
    type Target = OperandPtg;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for NameXPtg {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
