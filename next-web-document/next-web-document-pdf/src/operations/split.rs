use std::collections::BTreeSet;

use lopdf::Document;
use tracing::debug;

use crate::{error::pdf_error::PdfError, operation::PdfOperation};

pub struct PdfSplitOperation {
    pub split_type: SplitType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SplitType {
    /// 单一拆分点：3,7（在第 3 与第 7 页后拆分）
    Single { start: usize, end: usize },

    /// 范围拆分点：3-8（在第 3 页前与第 8 页后拆分）
    Scope { start: usize, end: usize },

    /// 混合：2,5-10,15（在第 2 页后、第 5 页前、第 10 页后与第 15 页后拆分）
    Mixed(Vec<(usize, usize)>),
}

impl PdfOperation<Vec<Document>> for PdfSplitOperation {
    fn execute(self, doc: &mut Document) -> Result<Vec<Document>, PdfError> {
        // 1. 获取总页数
        // 注意：lopdf 的 get_pages() 返回 BTreeMap<u32, ObjectId>，可能会比较慢，
        // 如果只是想拿页数，可以用 doc.get_pages().len()
        let pages_map = doc.get_pages();
        let total_pages = pages_map.len();

        if total_pages == 0 {
            return Err(PdfError::AnalysisError(
                "The document is empty and cannot be split".into(),
            ));
        }

        // 2. 计算拆分区间
        let chunks = self.split_type.generate_chunks(total_pages);

        debug!(
            "Splitting PDF (of {} pages) into {} documents",
            total_pages,
            chunks.len()
        );

        let mut result_docs = Vec::new();

        // 3. 针对每个区间生成新 Document 对象
        for (start, end) in chunks {
            // 计算需要保留的页码集合 (start..=end)
            // 注意：PDF页码从1开始
            let keep_pages: BTreeSet<u32> = (start..=end).map(|p| p as u32).collect();

            // 克隆原文档（这是最安全的方式，避免破坏源 doc 的引用关系）
            // 如果性能是瓶颈，可以考虑更底层的对象转移，但比较复杂
            let mut new_doc = doc.clone();

            // 找出需要删除的页码
            // 逻辑：遍历新文档的所有页码，如果不在 keep_pages 里，就删掉
            let current_pages = new_doc.get_pages();
            let mut pages_to_delete = Vec::new();

            for page_num in current_pages.keys() {
                if !keep_pages.contains(page_num) {
                    pages_to_delete.push(*page_num);
                }
            }

            // 执行删除
            new_doc.delete_pages(&pages_to_delete);

            // 核心步骤：清理未引用的对象 (Pruning)
            // 删除页面后，如果不运行这个，文件体积不会变小，因为图片/字体对象还在
            new_doc.prune_objects();

            // 这里的 new_doc 已经是独立的、裁剪好的 PDF 对象了
            result_docs.push(new_doc);
        }

        Ok(result_docs)
    }
}

impl SplitType {
    /// 计算切分点（Cut Points）
    /// 切分点 N 表示：在第 N 页和第 N+1 页之间断开
    fn get_cut_points(&self) -> BTreeSet<usize> {
        let mut cuts = BTreeSet::new();

        match self {
            // 在 start 后切，在 end 后切
            SplitType::Single { start, end } => {
                cuts.insert(*start);
                cuts.insert(*end);
            }
            // 在 start 前切 (即 start-1 后)，在 end 后切
            SplitType::Scope { start, end } => {
                if *start > 1 {
                    cuts.insert(start - 1);
                }
                cuts.insert(*end);
            }
            // 混合模式处理
            SplitType::Mixed(ranges) => {
                for &(s, e) in ranges {
                    if s == e {
                        // 视为单点：在 s 后切
                        cuts.insert(s);
                    } else {
                        // 视为范围：在 s 前切，e 后切
                        if s > 1 {
                            cuts.insert(s - 1);
                        }
                        cuts.insert(e);
                    }
                }
            }
        }
        cuts
    }

    /// 根据总页数生成每个子文档的页码区间 (start, end)
    pub fn generate_chunks(&self, total_pages: usize) -> Vec<(usize, usize)> {
        let cuts = self.get_cut_points();
        let mut chunks = Vec::new();
        let mut current_start = 1;

        // 遍历切点，生成区间
        for cut in cuts {
            // 切点必须在文档范围内，且大于当前起始点
            if cut >= current_start && cut < total_pages {
                chunks.push((current_start, cut));
                current_start = cut + 1;
            }
        }

        // 添加最后一段（即最后一个切点到文档末尾）
        if current_start <= total_pages {
            chunks.push((current_start, total_pages));
        }

        chunks
    }
}
