use crate::{PdfResult, operation::PdfOperation};
use lopdf::Document;

#[derive(Debug, Clone, Copy, Default)]
pub struct PdfCompressOperation {
    pub prune_objects: bool,
}

impl PdfCompressOperation {
    pub fn new(prune_objects: bool) -> Self {
        Self { prune_objects }
    }
}

impl PdfOperation<()> for PdfCompressOperation {
    fn execute(self, doc: &mut Document) -> PdfResult<()> {
        if self.prune_objects {
            doc.prune_objects();
        }
        doc.compress();
        Ok(())
    }
}
