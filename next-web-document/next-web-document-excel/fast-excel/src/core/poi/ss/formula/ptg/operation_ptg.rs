use std::ops::{Deref, DerefMut};

use crate::core::poi::ss::formula::ptg::Ptg;

#[derive(Debug, Clone)]
pub struct OperationPtg {
    base: Ptg,
}

impl Default for OperationPtg {
    fn default() -> Self {
        Self {
            base: Default::default(),
        }
    }
}

impl Deref for OperationPtg {
    type Target = Ptg;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for OperationPtg {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

pub trait OperationPtgExt {
    fn get_number_of_operands(&self) -> u16;
}
