use std::ops::{Deref, DerefMut};

use bigdecimal::BigDecimal;
use chrono::{DateTime, Local};

use crate::core::{
    enums::cell_data_type::CellDataType,
    metadata::data::{
        cell_data::CellData, comment_data::CommentData, hyper_link_data::HyperlinkData,
        image_data::ImageData, rich_text_string_data::RichTextStringData,
    },
    poi::ss::usermodel::cell_style::CellStyle,
    write::metadata::style::write_cell_style::WriteCellStyle,
};

#[derive(Debug, Clone)]
pub struct WriteCellData<T, S: CellStyle> {
    /// Support only when writing.
    date_value: Option<DateTime<Local>>,
    /// rich text
    rich_text_string_data: Option<RichTextStringData>,
    /// image
    image_data_list: Option<Vec<ImageData>>,
    /// comment
    comment_data: Option<CommentData>,
    /// hyperlink
    hyperlink_data: Option<HyperlinkData>,
    /// style
    write_cell_style: Option<WriteCellStyle>,
    /// If originCellStyle is empty, one will be created. If both writeCellStyle and originCellStyle exist, copy from writeCellStyle to originCellStyle.
    origin_cell_style: Option<S>,

    cell_data: CellData<T>,
}

impl<T, S: CellStyle> From<CellDataType> for WriteCellData<T, S> {
    fn from(value: CellDataType) -> Self {
        let mut write_cell_data = Self::default();
        write_cell_data.set_cell_data_type(value);

        write_cell_data
    }
}

impl<T, S: CellStyle> TryFrom<(CellDataType, String)> for WriteCellData<T, S> {
    type Error = &'static str;

    fn try_from((cell_type, value): (CellDataType, String)) -> Result<Self, Self::Error> {
        if cell_type != CellDataType::String && cell_type != CellDataType::Error {
            return Err("Only support CellDataTypeEnum.STRING and  CellDataTypeEnum.ERROR");
        }
        let mut write_cell_data = Self::default();
        write_cell_data.set_cell_data_type(cell_type);
        write_cell_data.set_string_value(value);

        Ok(write_cell_data)
    }
}

impl<T, S: CellStyle> From<BigDecimal> for WriteCellData<T, S> {
    fn from(value: BigDecimal) -> Self {
        let mut write_cell_data = Self::default();
        write_cell_data.set_cell_data_type(CellDataType::Number);
        write_cell_data.set_number_value(value);

        write_cell_data
    }
}

impl<T, S: CellStyle> From<bool> for WriteCellData<T, S> {
    fn from(value: bool) -> Self {
        let mut write_cell_data = Self::default();
        write_cell_data.set_cell_data_type(CellDataType::Boolean);
        write_cell_data.set_boolean_value(value);

        write_cell_data
    }
}

impl<T, S: CellStyle> From<DateTime<Local>> for WriteCellData<T, S> {
    fn from(value: DateTime<Local>) -> Self {
        let mut write_cell_data = Self::default();
        write_cell_data.set_cell_data_type(CellDataType::Date);
        write_cell_data.date_value = Some(value);

        write_cell_data
    }
}

impl<T, S: CellStyle> From<Vec<u8>> for WriteCellData<T, S> {
    fn from(value: Vec<u8>) -> Self {
        let mut write_cell_data = Self::default();
        write_cell_data.set_cell_data_type(CellDataType::Empty);
        let mut image_data = ImageData::default();
        image_data.set_image(value);
        write_cell_data.image_data_list = Some(vec![image_data]);

        write_cell_data
    }
}

impl<T, S: CellStyle> Deref for WriteCellData<T, S> {
    type Target = CellData<T>;

    fn deref(&self) -> &Self::Target {
        &self.cell_data
    }
}

impl<T, S: CellStyle> DerefMut for WriteCellData<T, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cell_data
    }
}

impl<T, S: CellStyle> Default for WriteCellData<T, S> {
    fn default() -> Self {
        let cell_data = CellData::default();

        Self {
            date_value: None,
            rich_text_string_data: None,
            image_data_list: None,
            comment_data: None,
            hyperlink_data: None,
            write_cell_style: None,
            origin_cell_style: None,
            cell_data,
        }
    }
}
