use std::ops::{Deref, DerefMut};

use crate::core::{
    context::{
        analysis_context::AnalysisContext, analysis_context_impl::AnalysisContextImpl,
        xlsx::xlsx_read_context::XlsxReadContext,
    },
    read::metadata::{
        holder::{
            read_holder::ReadHolder,
            read_row_holder::ReadRowHolder,
            read_sheet_holder::ReadSheetHolder,
            read_workbook_holder::ReadWorkbookHolder,
            xlsx::{
                xlsx_read_sheet_holder::XlsxReadSheetHolder,
                xlsx_read_workbook_holder::XlsxReadWorkbookHolder,
            },
        },
        read_sheet::ReadSheet,
        read_workbook::ReadWorkbook,
    },
    support::excel_type::ExcelType,
};

#[derive(Clone)]
pub struct DefaultXlsxReadContext<T> {
    analysis_context_impl: AnalysisContextImpl<T>,
}

impl<T> DefaultXlsxReadContext<T> {
    pub fn new(read_workbook: ReadWorkbook<T>, actual_excel_type: ExcelType) -> Self {
        DefaultXlsxReadContext {
            analysis_context_impl: AnalysisContextImpl::new(read_workbook, actual_excel_type),
        }
    }
}

impl<T> XlsxReadContext<T> for DefaultXlsxReadContext<T> {
    fn xlsx_read_workbook_holder(&mut self) -> &mut XlsxReadWorkbookHolder {
        // self.read_workbook_holder()
        todo!()
    }

    fn xlsx_read_sheet_holder(&mut self) -> &mut XlsxReadSheetHolder {
        // self.read_sheet_holder()
        todo!()
    }
}

impl<T> AnalysisContext<T> for DefaultXlsxReadContext<T> {
    fn current_sheet(&self, read_sheet: &ReadSheet<T>) {
        self.analysis_context_impl.current_sheet(read_sheet)
    }

    fn read_workbook_holder(&mut self) -> &mut ReadWorkbookHolder {
        self.analysis_context_impl.read_workbook_holder()
    }

    fn read_sheet_holder(&mut self) -> &mut ReadSheetHolder {
        self.analysis_context_impl.read_sheet_holder()
    }

    fn set_read_row_holder(&self, read_row_holder: &ReadRowHolder) {
        self.analysis_context_impl
            .set_read_row_holder(read_row_holder)
    }

    fn read_row_holder(&mut self) -> &mut ReadRowHolder {
        self.analysis_context_impl.read_row_holder()
    }

    fn current_read_holder(&self) -> &dyn ReadHolder<T> {
        self.analysis_context_impl.current_read_holder()
    }
}

impl<T> Deref for DefaultXlsxReadContext<T> {
    type Target = AnalysisContextImpl<T>;

    fn deref(&self) -> &Self::Target {
        &self.analysis_context_impl
    }
}

impl<T> DerefMut for DefaultXlsxReadContext<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.analysis_context_impl
    }
}
