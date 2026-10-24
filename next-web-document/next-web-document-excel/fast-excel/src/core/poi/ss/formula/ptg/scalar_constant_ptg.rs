use std::ops::{Deref, DerefMut};

use crate::core::poi::{
    ss::formula::ptg::{Ptg, PtgExt},
    util::little_endian_output::LittleEndianOutput,
};

#[derive(Debug, Clone, Default)]
pub struct ScalarConstantPtg {
    base: Ptg,
}

impl PtgExt for ScalarConstantPtg {
    fn is_base_token(&self) -> bool {
        true
    }

    fn get_size(&self) -> usize {
        unreachable!()
    }

    fn get_sid(&self) -> i8 {
        unreachable!()
    }

    fn to_formula_string(&self) -> Option<String> {
        unimplemented!()
    }

    fn write(&self, _out: &mut dyn LittleEndianOutput) -> std::io::Result<()> {
        unreachable!()
    }

    fn get_default_operand_class(&self) -> u8 {
        Ptg::CLASS_VALUE
    }
}

impl Deref for ScalarConstantPtg {
    type Target = Ptg;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for ScalarConstantPtg {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
