use lopdf::Document;

use crate::error::pdf_error::PdfError;

pub trait PdfOperation<T> {
    /// 执行操作
    /// doc: 可变借用文档对象，允许直接修改
    fn execute(self, doc: &mut Document) -> Result<T, PdfError>;
}

pub trait PdfQuery<T> {
    /// Executes a read-only query against a document without mutating it.
    fn query(&self, doc: &Document) -> Result<T, PdfError>;
}
