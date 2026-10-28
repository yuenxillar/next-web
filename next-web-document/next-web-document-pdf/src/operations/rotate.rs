use crate::{error::pdf_error::PdfError, operation::PdfOperation};
use lopdf::Document;
use std::collections::HashSet;

pub struct PdfRotateOperation {
    /// 旋转角度，必须是 90 的倍数
    /// The rotation angle must be a multiple of 90
    pub angle: i64,
    /// 指定需要旋转的页码 (1-based)。如果为 None，则旋转所有页面
    /// Specify the page number to be rotated (1-based). If None, rotate all pages
    pub target_pages: Option<HashSet<u32>>,
}

impl PdfRotateOperation {
    /// pages: 传入 None 表示旋转全部，传入 Some(HashSet[...]) 表示旋转指定页
    /// Pages: Passing None means rotating all pages, passing Some (HashSet [...]) means rotating a specified page
    pub fn new<T>(angle: i64, target_pages: T) -> Self
    where
        T: Into<HashSet<u32>>,
    {
        PdfRotateOperation {
            angle,
            target_pages: Some(target_pages.into()),
        }
    }

    /// 创建一个新的旋转操作，指定旋转角度, 默认旋转所有页面
    /// Create a new rotation operation, specify the rotation angle, and default to rotating all pages
    pub fn with_angle(angle: i64) -> Self {
        Self {
            angle,
            target_pages: None,
        }
    }
}

impl PdfOperation<()> for PdfRotateOperation {
    fn execute(self, doc: &mut Document) -> Result<(), PdfError> {
        // 1. 校验参数：角度必须是 90 的倍数
        // 1. Verification parameter: The angle must be a multiple of 90
        if self.angle % 90 != 0 {
            return Err(PdfError::AnalysisError(format!(
                "The rotation angle must be a multiple of 90, current input: {}",
                self.angle
            )));
        }

        // 2. 准备页码筛选器
        // 2. Prepare page number filter
        let target_pages: Option<HashSet<u32>> = self.target_pages;

        // 3. 获取所有页面的 ID
        // 3. Obtain the IDs of all pages
        let pages_map = doc.get_pages();

        // 4. 遍历文档中的每一页
        // 4. Iterate through each page in the document
        for (page_num, page_id) in pages_map {
            // 如果 target_pages 是 Some，且当前页码不在集合中，则跳过
            // If target_pages is Some, and the current page number is not in the set, skip it
            if let Some(ref targets) = target_pages {
                if !targets.contains(&page_num) {
                    continue;
                }
            }

            // 获取页面的字典对象
            // Get the page dictionary object
            let page_dict = doc
                .get_object_mut(page_id)
                .and_then(|obj| obj.as_dict_mut())
                .map_err(|e| {
                    PdfError::AnalysisError(format!(
                        "Page {:?} is not a dictionary: {:?}",
                        page_id, e
                    ))
                })?;

            // 获取当前的旋转角度 (默认为 0)
            // Get the current rotation angle (default is 0)
            let current_rotation = page_dict
                .get(b"Rotate")
                .and_then(|obj| obj.as_i64())
                .unwrap_or(0);

            // 计算新的旋转角度
            // Calculate the new rotation angle
            // ((curr + angle) % 360 + 360) % 360，确保结果为正数 [0, 90, 180, 270]
            let new_rotation = ((current_rotation + self.angle) % 360 + 360) % 360;

            // 更新字典中的 Rotate 字段
            // Update the Rotate field in the dictionary
            page_dict.set("Rotate", new_rotation);
        }

        Ok(())
    }
}
