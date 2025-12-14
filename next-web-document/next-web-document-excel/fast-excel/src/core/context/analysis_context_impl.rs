use std::marker::PhantomData;

use crate::{
    TodoEnum,
    core::{
        context::analysis_context::AnalysisContext,
        read::{
            metadata::{
                holder::{
                    read_holder::ReadHolder, read_row_holder::ReadRowHolder,
                    read_sheet_holder::ReadSheetHolder, read_workbook_holder::ReadWorkbookHolder,
                },
                read_sheet::ReadSheet,
                read_workbook::ReadWorkbook,
            },
            processor::default_analysis_event_processor::DefaultAnalysisEventProcessor,
        },
        support::excel_type::ExcelType,
    },
};

#[derive(Clone)]
pub struct AnalysisContextImpl<T, P = DefaultAnalysisEventProcessor<TodoEnum>> {
    read_workbook_holder: ReadWorkbookHolder,
    read_sheet_holder: ReadSheetHolder,
    read_row_holder: ReadRowHolder,
    analysis_event_processor: P,

    _marker: PhantomData<T>,
}

impl<T, P> AnalysisContextImpl<T, P> {
    pub fn new(read_workbook: ReadWorkbook, excel_type: ExcelType) -> Self {
        let read_workbook_holder = match excel_type {
            ExcelType::Csv => {}
            ExcelType::Xls => {}
            ExcelType::Xlsx => {}
        };

        todo!()
    }
}

impl<T, P> AnalysisContext<T> for AnalysisContextImpl<T, P> {
    /// Select the current table
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
