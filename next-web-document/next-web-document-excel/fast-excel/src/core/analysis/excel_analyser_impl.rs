use next_web_core::error::BoxError;

use crate::{
    TodoEnum,
    core::{
        analysis::{excel_analyser::ExcelAnalyser, excel_read_executor::ExcelReadExecutor},
        context::analysis_context::AnalysisContext,
        read::metadata::{read_sheet::ReadSheet, read_workbook::ReadWorkbook},
        support::excel_type::ExcelType,
    },
};

pub struct ExcelAnalyserImpl<C, E> {
    analysis_context: C,
    excel_read_executor: E,
    finished: bool,
}

impl<C, E> ExcelAnalyserImpl<C, E> {
    pub fn new(read_workbook: ReadWorkbook) -> Self {
        // Self {
        //     analysis_context: AnalysisContext::new(),
        //     excel_read_executor: ExcelReadExecutor::new(todo!()),
        //     finished: false,
        // }
        todo!()
    }

    fn choose_excel_executor(read_workbook: &ReadWorkbook) -> Result<(), BoxError> {
        // Determine the type of Excel file based on the provided readWorkbook
        let excel_type = ExcelType::from_read_workbook(read_workbook)?;
        match excel_type {
            ExcelType::Xls => {}
            ExcelType::Xlsx => {}
            ExcelType::Csv => {
                // Create a context and executor for processing CSV files
            }
        };

        Ok(())
    }
}

impl<C, E> ExcelAnalyser for ExcelAnalyserImpl<C, E> {
    fn analysis(
        &mut self,
        read_sheet_list: Vec<ReadSheet>,
        read_all: bool,
    ) -> Result<(), BoxError> {
        todo!()
    }

    fn finish(&mut self) -> Result<(), BoxError> {
        todo!()
    }

    fn excel_executor<T: ExcelReadExecutor>(&self) -> T {
        todo!()
    }

    fn analysis_context<T: AnalysisContext<TodoEnum>>(&self) -> T {
        todo!()
    }
}
