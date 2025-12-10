use lopdf::Document;
use std::path::Path;

use crate::PdfResult;

/// 以后所有功能都挂在这里的最终高层入口
pub struct PdfProcessor {
    pub(crate) doc: Document,
}

impl PdfProcessor {
    pub fn load<P: AsRef<Path>>(path: P) -> PdfResult<Self> {
        let doc = Document::load(path)?;
        Ok(Self { doc })
    }

    pub fn from_bytes(bytes: &[u8]) -> PdfResult<Self> {
        let doc = Document::load_from(bytes)?;
        Ok(Self { doc })
    }

    // ============ 操作入口 ============

    // /// 无损合并多个文件路径
    // pub fn merge_files<P: AsRef<Path>>(
    //     mut self,
    //     paths: impl IntoIterator<Item = P>,
    // ) -> PdfResult<Self> {
    //     MergeFiles {
    //         paths: paths.into_iter().collect(),
    //     }
    //     .apply(&mut self.doc)?;
    //     Ok(self)
    // }

    // /// 无损合并多个已加载的 Document（最高性能）
    // pub fn merge_docs(mut self, docs: Vec<Document>) -> PdfResult<Self> {
    //     MergeDocs { docs }.apply(&mut self.doc)?;
    //     Ok(self)
    // }

    // ============ 输出 ============

    // pub fn save<P: AsRef<Path>>(&self, path: P) -> PdfResult<()> {
    //     self.doc.save(path)?;
    //     Ok(())
    // }

    // pub fn into_bytes(self) -> PdfResult<Vec<u8>> {
    //     self.doc.save_to_bytes().map_err(Into::into)
    // }

    pub fn page_count(&self) -> usize {
        self.doc.get_pages().len()
    }
}
