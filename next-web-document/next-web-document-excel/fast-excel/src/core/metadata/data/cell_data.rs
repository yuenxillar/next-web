use std::ops::{Deref, DerefMut};

use bigdecimal::BigDecimal;

use crate::core::{
    enums::cell_data_type::CellDataType, metadata::data::formula_data::FormulaData,
    poi::ss::usermodel::default_cell::DefaultCell,
};

/// Excel internal cell data.
#[derive(Debug, Clone)]
pub struct CellData<T> {
    cell_data_type: Option<CellDataType>,
    number_value: Option<BigDecimal>,
    string_value: Option<String>,
    boolean_value: Option<bool>,
    data: Option<T>,
    formula_data: Option<FormulaData>,

    default_cell: DefaultCell,
}

impl<T> CellData<T> {
    // 为 CellData 结构体生成的 Getter/Setter 方法

    pub fn get_cell_data_type(&self) -> Option<&CellDataType> {
        self.cell_data_type.as_ref()
    }

    pub fn get_number_value(&self) -> Option<&BigDecimal> {
        self.number_value.as_ref()
    }

    pub fn get_string_value(&self) -> Option<&str> {
        self.string_value.as_deref()
    }

    pub fn get_boolean_value(&self) -> Option<bool> {
        self.boolean_value
    }

    pub fn get_data(&self) -> Option<&T> {
        self.data.as_ref()
    }

    pub fn get_formula_data(&self) -> Option<&FormulaData> {
        self.formula_data.as_ref()
    }

    pub fn set_cell_data_type(&mut self, cell_data_type: CellDataType) {
        self.cell_data_type = Some(cell_data_type);
    }

    pub fn set_number_value(&mut self, number_value: BigDecimal) {
        self.number_value = Some(number_value);
    }

    pub fn set_string_value(&mut self, string_value: String) {
        self.string_value = Some(string_value);
    }

    pub fn set_boolean_value(&mut self, boolean_value: bool) {
        self.boolean_value = Some(boolean_value);
    }

    pub fn set_data(&mut self, data: T) {
        self.data = Some(data);
    }

    pub fn set_formula_data(&mut self, formula_data: FormulaData) {
        self.formula_data = Some(formula_data);
    }

    pub fn clear_cell_data_type(&mut self) {
        self.cell_data_type = None;
    }

    pub fn clear_number_value(&mut self) {
        self.number_value = None;
    }

    pub fn clear_string_value(&mut self) {
        self.string_value = None;
    }

    pub fn clear_boolean_value(&mut self) {
        self.boolean_value = None;
    }

    pub fn clear_data(&mut self) {
        self.data = None;
    }

    pub fn clear_formula_data(&mut self) {
        self.formula_data = None;
    }

    pub fn check_empty(&mut self) {
        if self.cell_data_type.is_none() {
            self.cell_data_type = Some(CellDataType::Empty);
        }

        let ty = self.cell_data_type.clone().unwrap_or(CellDataType::Empty);
        match ty {
            CellDataType::Error => {
                if self
                    .string_value
                    .as_ref()
                    .map(|s| !s.is_empty())
                    .unwrap_or_default()
                {
                    self.cell_data_type = Some(CellDataType::Empty);
                }
            }

            CellDataType::Number => {
                if self.number_value.is_none() {
                    self.cell_data_type = Some(CellDataType::Empty);
                }
            }

            CellDataType::Boolean => {
                if self.boolean_value.is_none() {
                    self.cell_data_type = Some(CellDataType::Empty);
                }
            }
            _ => {}
        }
    }
}

impl<T> Deref for CellData<T> {
    type Target = DefaultCell;

    fn deref(&self) -> &Self::Target {
        &self.default_cell
    }
}

impl<T> DerefMut for CellData<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.default_cell
    }
}

impl<T> Default for CellData<T> {
    fn default() -> Self {
        Self {
            cell_data_type: None,
            number_value: None,
            string_value: None,
            boolean_value: None,
            data: None,
            formula_data: None,
            default_cell: DefaultCell::default(),
        }
    }
}
