use crate::{
    TodoEnum,
    core::{context::analysis_context::AnalysisContext, read::metadata::read_sheet::ReadSheet},
};

pub struct SheetUtils;

impl SheetUtils {
    pub fn matchs<T: AnalysisContext<TodoEnum>>(read_sheet: &ReadSheet, analysis_context: &T) {}
}
