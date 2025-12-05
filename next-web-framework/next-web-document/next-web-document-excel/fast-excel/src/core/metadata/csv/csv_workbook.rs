use next_web_core::util::locale::Locale;

use crate::core::metadata::csv::{
    csv_cell_style::CsvCellStyle, csv_data_format::CsvDataFormat, csv_sheet::CsvSheet,
};

#[derive(Clone)]
pub struct CsvWorkbook {
    /// output
    out: Option<String>,
    /// true if date uses 1904 windowing, or false if using 1900 date windowing.
    use1904windowing: Option<bool>,
    /// locale
    locale: Option<Locale>,
    /// Whether to use scientific Format.
    use_scientific_format: Option<bool>,
    /// data format
    csv_data_format: Option<CsvDataFormat>,
    /// sheet
    csv_sheet: Option<CsvSheet>,
    /// cell style
    csv_cell_style_list: Option<Vec<CsvCellStyle>>,
    /// charset.
    charset: Option<String>,
    /// Set the encoding prefix in the csv file, otherwise the office may open garbled characters. Default true.
    with_bom: Option<bool>,
}

impl CsvWorkbook {
    pub fn get_out(&self) -> Option<&str> {
        self.out.as_deref()
    }

    pub fn get_use1904windowing(&self) -> Option<bool> {
        self.use1904windowing
    }

    pub fn get_locale(&self) -> Option<&Locale> {
        self.locale.as_ref()
    }

    pub fn get_use_scientific_format(&self) -> Option<bool> {
        self.use_scientific_format
    }

    pub fn get_csv_data_format(&self) -> Option<&CsvDataFormat> {
        self.csv_data_format.as_ref()
    }

    pub fn get_csv_sheet(&self) -> Option<&CsvSheet> {
        self.csv_sheet.as_ref()
    }

    pub fn get_csv_cell_style_list(&self) -> Option<&Vec<CsvCellStyle>> {
        self.csv_cell_style_list.as_ref()
    }

    pub fn get_charset(&self) -> Option<&str> {
        self.charset.as_deref()
    }

    pub fn get_with_bom(&self) -> Option<bool> {
        self.with_bom
    }

    pub fn set_out(&mut self, out: String) {
        self.out = Some(out);
    }

    pub fn set_use1904windowing(&mut self, use1904windowing: bool) {
        self.use1904windowing = Some(use1904windowing);
    }

    pub fn set_locale(&mut self, locale: Locale) {
        self.locale = Some(locale);
    }

    pub fn set_use_scientific_format(&mut self, use_scientific_format: bool) {
        self.use_scientific_format = Some(use_scientific_format);
    }

    pub fn set_csv_data_format(&mut self, csv_data_format: CsvDataFormat) {
        self.csv_data_format = Some(csv_data_format);
    }

    pub fn set_csv_sheet(&mut self, csv_sheet: CsvSheet) {
        self.csv_sheet = Some(csv_sheet);
    }

    pub fn set_csv_cell_style_list(&mut self, csv_cell_style_list: Vec<CsvCellStyle>) {
        self.csv_cell_style_list = Some(csv_cell_style_list);
    }

    pub fn set_charset(&mut self, charset: String) {
        self.charset = Some(charset);
    }

    pub fn set_with_bom(&mut self, with_bom: bool) {
        self.with_bom = Some(with_bom);
    }

    pub fn clear_out(&mut self) {
        self.out = None;
    }

    pub fn clear_use1904windowing(&mut self) {
        self.use1904windowing = None;
    }

    pub fn clear_locale(&mut self) {
        self.locale = None;
    }

    pub fn clear_use_scientific_format(&mut self) {
        self.use_scientific_format = None;
    }

    pub fn clear_csv_data_format(&mut self) {
        self.csv_data_format = None;
    }

    pub fn clear_csv_sheet(&mut self) {
        self.csv_sheet = None;
    }

    pub fn clear_csv_cell_style_list(&mut self) {
        self.csv_cell_style_list = None;
    }

    pub fn clear_charset(&mut self) {
        self.charset = None;
    }

    pub fn clear_with_bom(&mut self) {
        self.with_bom = None;
    }
}
