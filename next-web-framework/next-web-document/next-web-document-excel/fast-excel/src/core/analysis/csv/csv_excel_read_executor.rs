use next_web_core::error::BoxError;

use crate::core::{
    analysis::excel_read_executor::ExcelReadExecutor,
    context::csv::{
        csv_read_context::CsvReadContext, default_csv_read_context::DefaultCsvReadContext,
    },
    read::metadata::read_sheet::ReadSheet,
    util::sheet_utils::SheetUtils,
};

pub struct CsvExcelReadExecutor<C = DefaultCsvReadContext> {
    /// List of sheets to be read
    sheet_list: Vec<ReadSheet>,
    /// Context for CSV reading operation
    csv_read_context: C,
}

impl<C> CsvExcelReadExecutor<C>
where
    C: CsvReadContext,
{
    pub fn new(csv_read_context: C) -> Self {
        let mut read_sheet = ReadSheet::default();
        read_sheet.set_sheet_no(0);
        Self {
            sheet_list: vec![read_sheet],
            csv_read_context,
        }
    }

    fn csv_parser(&self) {
        // Retrieve the CsvReadWorkbookHolder instance from the CsvReadContext.
        // self.csv_read_context.c
    }
}

impl<C> ExcelReadExecutor for CsvExcelReadExecutor<C>
where
    C: CsvReadContext,
{
    fn sheet_list(&self) -> Vec<&ReadSheet> {
        self.sheet_list.iter().collect()
    }

    /// Overrides the execute method to parse and process CSV files.
    /// This method first attempts to create a CSV parser, then iterates through each sheet,
    /// and processes each record in the CSV file.
    fn execute(&self) -> Result<(), BoxError> {
        // Create a CSV parser instance

        // Iterate through each sheet in the sheet list

        for read_sheet in self.sheet_list.iter() {
            // Match and update the readSheet object
            SheetUtils::matchs(read_sheet, self.csv_read_context);
        }
        Ok(())
    }
}
