use std::collections::HashMap;

use crate::{
    TodoEnum,
    core::{
        context::xlsx::default_xlsx_read_context::DefaultXlsxReadContext,
        poi::xssf::model::comments_table::CommentsTable, read::metadata::read_sheet::ReadSheet,
    },
};

pub struct XlsxSaxAnalyser<C = DefaultXlsxReadContext<TodoEnum>> {
    xlsx_read_context: C,
    sheet_list: Vec<ReadSheet>,

    /// excel comments key: sheetNo value: CommentsTable
    comments_table_map: HashMap<u32, CommentsTable>,
}
