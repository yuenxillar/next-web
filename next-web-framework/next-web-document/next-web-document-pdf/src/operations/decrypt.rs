use crate::{error::pdf_error::PdfError, operation::PdfOperation};
use lopdf::Document;

pub struct PdfDecryptOperation {
    password: String,
}

impl PdfDecryptOperation {
    pub fn new<T>(password: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            password: password.into(),
        }
    }
}

impl PdfOperation<()> for PdfDecryptOperation {
    fn execute(self, doc: &mut Document) -> Result<(), PdfError> {
        // 如果文档本身没有加密，任何密码都视为 '通过'
        // If the document itself is not encrypted, any password is considered 'passed'
        if !doc.is_encrypted() {
            return Ok(());
        }

        // 解密文档
        // Decrypt the document
        doc.decrypt(&self.password)?;
        Ok(())
    }
}
