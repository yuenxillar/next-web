use std::ops::{Deref, DerefMut};

use crate::core::{
    context::{
        analysis_context::AnalysisContext, analysis_context_impl::AnalysisContextImpl,
        csv::csv_read_context::CsvReadContext,
    },
    read::metadata::{
        holder::{
            csv::{
                csv_read_sheet_holder::CsvReadSheetHolder,
                csv_read_workbook_holder::CsvReadWorkbookHolder,
            },
            read_holder::ReadHolder,
            read_row_holder::ReadRowHolder,
            read_sheet_holder::ReadSheetHolder,
            read_workbook_holder::ReadWorkbookHolder,
        },
        read_sheet::ReadSheet,
        read_workbook::ReadWorkbook,
    },
    support::excel_type::ExcelType,
};

pub struct DefaultCsvReadContext<T> {
    analysis_context: AnalysisContextImpl<T>,
}

impl<T> DefaultCsvReadContext<T> {
    pub fn new(read_workbook: ReadWorkbook, excel_type: ExcelType) -> Self {
        Self {
            analysis_context: AnalysisContextImpl::new(read_workbook, excel_type),
        }
    }
}
impl<T> CsvReadContext<T> for DefaultCsvReadContext<T> {
    fn csv_read_workbook_holder(&mut self) -> &mut CsvReadWorkbookHolder {
        todo!()
    }

    /// All information about the sheet you are currently working on.
    fn csv_read_sheet_holder(&mut self) -> &mut CsvReadSheetHolder {
        todo!()
    }
}

impl<T> AnalysisContext<T> for DefaultCsvReadContext<T> {
    fn current_sheet(&self, read_sheet: &ReadSheet) {
        todo!()
    }

    /// All information about the workbook you are currently working on
    fn read_workbook_holder(&mut self) -> &mut ReadWorkbookHolder {
        todo!()
    }

    /// All information about the sheet you are currently working on
    fn read_sheet_holder(&mut self) -> &mut ReadSheetHolder {
        todo!()
    }

    /// Set row of currently operated cell
    fn set_read_row_holder(&self, read_row_holder: &ReadRowHolder) {
        todo!()
    }

    /// Row of currently operated cell
    fn read_row_holder(&mut self) -> &mut ReadRowHolder {
        todo!()
    }

    fn current_read_holder(&self) -> &dyn ReadHolder<T> {
        todo!()
    }
}

impl<T> Deref for DefaultCsvReadContext<T> {
    type Target = AnalysisContextImpl<T>;

    fn deref(&self) -> &Self::Target {
        &self.analysis_context
    }
}

impl<T> DerefMut for DefaultCsvReadContext<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.analysis_context
    }
}
