use crate::core::fast_excel_factory::FastExcelFactory;

pub struct FastExcel;

impl FastExcelFactory for FastExcel {}

#[cfg(test)]
mod fast_excel_tests {
    // use super::*;

    use crate::core::{
        error::excel_error::ExcelError, event::listener::Listener, fast_excel::FastExcel,
        fast_excel_factory::FastExcelFactory, read::listener::read_listener::ReadListener,
    };

    #[derive(Debug, Clone, Default)]
    struct TestHead {
        pub s: String,
    }

    #[derive(Clone, Default)]
    struct TestReadListener {}

    impl Listener for TestReadListener {}

    impl ReadListener<TestHead> for TestReadListener {
        fn invoke(
            &mut self,
            data: &TestHead,
            _context: &mut dyn crate::core::context::analysis_context::AnalysisContext<TestHead>,
        ) -> Result<(), next_web_core::error::BoxError> {
            println!("data: {:?}", data);

            Ok(())
        }

        fn do_after_all_analysed(
            &mut self,
            _context: &mut dyn crate::core::context::analysis_context::AnalysisContext<TestHead>,
        ) {
        }
    }

    #[tokio::test]
    async fn test_write_file() -> Result<(), ExcelError> {
        FastExcel::write_with_path("test.xlsx")
            .await
            .sheet_with_name("TestSheet1")?
            .do_write(&Vec::<()>::new())?;

        FastExcel::read_with_path_head_and_listener::<TestHead>(
            "test.xlsx",
            TestReadListener::default(),
        )
        .await?
        .sheet()?
        .do_read()?;

        Ok(())
    }
}
