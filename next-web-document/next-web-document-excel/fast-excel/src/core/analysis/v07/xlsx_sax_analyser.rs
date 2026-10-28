use std::collections::HashMap;

use next_web_core::error::BoxError;

use crate::core::{
    context::xlsx::default_xlsx_read_context::DefaultXlsxReadContext,
    poi::xssf::model::comments_table::CommentsTable, read::metadata::read_sheet::ReadSheet,
};

pub struct XlsxSaxAnalyser<T, C = DefaultXlsxReadContext<T>> {
    xlsx_read_context: C,
    sheet_list: Vec<ReadSheet<T>>,

    /// excel comments key: sheetNo value: CommentsTable
    comments_table_map: HashMap<u32, CommentsTable>,
}

impl<T, C> XlsxSaxAnalyser<T, C> {
    pub fn new(xlsx_read_context: C, sheet_list: Vec<ReadSheet<T>>) -> Result<Self, BoxError> {
        Self {
            xlsx_read_context,
            sheet_list,
            comments_table_map: HashMap::new(),
        }
    }
}
