use crate::{PdfResult, error::pdf_error::PdfError, operation::PdfQuery};
use lopdf::Document;

#[derive(Debug, Clone, Default)]
pub struct PdfExtractTextOperation {
    pub page_numbers: Option<Vec<u32>>,
}

impl PdfExtractTextOperation {
    pub fn all_pages() -> Self {
        Self { page_numbers: None }
    }

    pub fn from_pages<I>(page_numbers: I) -> Self
    where
        I: IntoIterator<Item = u32>,
    {
        Self {
            page_numbers: Some(page_numbers.into_iter().collect()),
        }
    }
}

impl PdfQuery<String> for PdfExtractTextOperation {
    fn query(&self, doc: &Document) -> PdfResult<String> {
        let page_numbers = match &self.page_numbers {
            Some(page_numbers) if !page_numbers.is_empty() => page_numbers.clone(),
            Some(_) => {
                return Err(PdfError::InvalidArgument(
                    "page_numbers cannot be empty".to_string(),
                ));
            }
            None => doc.get_pages().keys().copied().collect(),
        };

        if page_numbers.is_empty() {
            return Ok(String::new());
        }

        doc.extract_text(&page_numbers).map_err(Into::into)
    }
}
