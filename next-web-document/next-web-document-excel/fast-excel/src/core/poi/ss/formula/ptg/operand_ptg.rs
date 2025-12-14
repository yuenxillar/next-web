use std::ops::{Deref, DerefMut};

use crate::core::poi::ss::formula::ptg::Ptg;

#[derive(Debug, Clone, Default)]
pub struct OperandPtg {
    pub base: Ptg,
}

impl Deref for OperandPtg {
    type Target = Ptg;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for OperandPtg {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
