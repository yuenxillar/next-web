use std::ops::{Deref, DerefMut};

use bigdecimal::BigDecimal;

use crate::core::{
    enums::cell_data_type::CellDataType,
    metadata::data::{cell_data::CellData, data_format_data::DataFormatData},
};

#[derive(Debug, Clone, Default)]
pub struct ReadCellData<T> {
    /// originalNumberValue vs numberValue
    original_number_value: Option<BigDecimal>,
    /// data format.
    data_format_data: Option<DataFormatData>,

    cell_data: CellData<T>,
}

impl<T: Default> ReadCellData<T> {
    pub fn new_with_type(cell_data_type: CellDataType) -> Self {
        let mut read_cell_data = Self::default();
        read_cell_data.set_cell_data_type(cell_data_type);

        read_cell_data
    }

    pub fn new_with_data(data: T) -> Self {
        let mut read_cell_data = Self::default();
        read_cell_data.set_data(data);

        read_cell_data
    }

    pub fn from_string(value: impl Into<String>) -> Self {
        Self::from_string_with_type(CellDataType::String, value)
    }

    pub fn from_string_with_type(cell_data_type: CellDataType, value: impl Into<String>) -> Self {
        let mut read_cell_data = Self::default();

        if cell_data_type != CellDataType::String && cell_data_type != CellDataType::Error {
            panic!("Only CellDataType::String and CellDataType::Error are allowed");
        }

        let value = value.into();
        if value.is_empty() {
            panic!("stringValue cannot be empty");
        }
        read_cell_data.set_cell_data_type(cell_data_type);
        read_cell_data.set_string_value(value);

        read_cell_data
    }

    pub fn from_number(value: BigDecimal) -> Self {
        let mut read_cell_data = Self::default();
        read_cell_data.set_cell_data_type(CellDataType::Number);
        read_cell_data.set_number_value(value);

        read_cell_data
    }

    pub fn from_boolean(value: bool) -> Self {
        let mut read_cell_data = Self::default();
        read_cell_data.set_cell_data_type(CellDataType::Boolean);
        read_cell_data.set_boolean_value(value);

        read_cell_data
    }
}

impl<T: Default> ReadCellData<T> {
    pub fn new_empty_instance() -> Self {
        Self::new_with_type(CellDataType::Empty)
    }

    pub fn new_empty_instance_with_row_and_column(row: u32, column: u32) -> Self {
        let mut read_cell_data = Self::new_with_type(CellDataType::Empty);
        read_cell_data.set_cell_data_type(CellDataType::Empty);
        read_cell_data.set_row_index(row);
        read_cell_data.set_column_index(column);

        read_cell_data
    }

    pub fn new_instance_with_bool(value: bool) -> Self {
        Self::from_boolean(value)
    }

    pub fn new_instance_with_bool_and_index(value: bool, row: u32, column: u32) -> Self {
        let mut read_cell_data = Self::from_boolean(value);
        read_cell_data.set_row_index(row);
        read_cell_data.set_column_index(column);

        read_cell_data
    }

    pub fn new_instance_with_string_and_index(
        value: impl Into<String>,
        row: u32,
        column: u32,
    ) -> Self {
        let mut read_cell_data = Self::from_string(value);
        read_cell_data.set_row_index(row);
        read_cell_data.set_column_index(column);

        read_cell_data
    }

    pub fn new_instance_with_number_and_index(value: BigDecimal, row: u32, column: u32) -> Self {
        let mut read_cell_data = Self::from_number(value);
        read_cell_data.set_row_index(row);
        read_cell_data.set_column_index(column);

        read_cell_data
    }
}

// 为 ReadCellData 结构体生成的 Getter/Setter 方法

impl<T> ReadCellData<T> {
    pub fn get_original_number_value(&self) -> Option<&BigDecimal> {
        self.original_number_value.as_ref()
    }

    pub fn get_data_format_data(&self) -> Option<&DataFormatData> {
        self.data_format_data.as_ref()
    }

    pub fn set_original_number_value(&mut self, original_number_value: BigDecimal) {
        self.original_number_value = Some(original_number_value);
    }

    pub fn set_data_format_data(&mut self, data_format_data: DataFormatData) {
        self.data_format_data = Some(data_format_data);
    }

    pub fn clear_original_number_value(&mut self) {
        self.original_number_value = None;
    }

    pub fn clear_data_format_data(&mut self) {
        self.data_format_data = None;
    }
}

impl<T> Deref for ReadCellData<T> {
    type Target = CellData<T>;

    fn deref(&self) -> &Self::Target {
        &self.cell_data
    }
}

impl<T> DerefMut for ReadCellData<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cell_data
    }
}
