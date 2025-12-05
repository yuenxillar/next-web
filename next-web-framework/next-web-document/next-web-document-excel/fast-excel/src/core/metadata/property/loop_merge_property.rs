/// 循环合并属性配置
#[derive(Debug, Clone, Default)]
pub struct LoopMergeProperty {
    /// Each row
    each_row: u32,
    /// Extend column
    column_extend: u32,
}

impl LoopMergeProperty {
    /// 创建新的 LoopMergeProperty
    ///
    /// # 参数
    /// - `each_row`: 每行
    /// - `column_extend`: 扩展列数
    pub fn new(each_row: u32, column_extend: u32) -> Self {
        Self {
            each_row,
            column_extend,
        }
    }

    /// 获取每行值
    pub fn get_each_row(&self) -> u32 {
        self.each_row
    }

    /// 设置每行值
    pub fn set_each_row(&mut self, each_row: u32) {
        self.each_row = each_row;
    }

    /// 获取扩展列数值
    pub fn get_column_extend(&self) -> u32 {
        self.column_extend
    }

    /// 设置扩展列数值
    pub fn set_column_extend(&mut self, column_extend: u32) {
        self.column_extend = column_extend;
    }
}
