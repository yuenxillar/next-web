pub struct ExcelXmlConstants;

impl ExcelXmlConstants {
    pub const DIMENSION_TAG: &str = "dimension";

    pub const ROW_TAG: &str = "row";
    pub const CELL_FORMULA_TAG: &str = "f";
    pub const CELL_VALUE_TAG: &str = "v";
    pub const CELL_INLINE_STRING_VALUE_TAG: &str = "t";
    pub const CELL_TAG: &str = "c";
    pub const MERGE_CELL_TAG: &str = "mergeCell";
    pub const HYPERLINK_TAG: &str = "hyperlink";

    pub const X_DIMENSION_TAG: &str = "x:dimension";
    pub const NS2_DIMENSION_TAG: &str = "ns2:dimension";

    pub const X_ROW_TAG: &str = "x:row";
    pub const NS2_ROW_TAG: &str = "ns2:row";

    pub const X_CELL_FORMULA_TAG: &str = "x:f";
    pub const NS2_CELL_FORMULA_TAG: &str = "ns2:f";
    pub const X_CELL_VALUE_TAG: &str = "x:v";
    pub const NS2_CELL_VALUE_TAG: &str = "ns2:v";

    pub const X_CELL_INLINE_STRING_VALUE_TAG: &str = "x:t";
    pub const NS2_CELL_INLINE_STRING_VALUE_TAG: &str = "ns2:t";

    pub const X_CELL_TAG: &str = "x:c";
    pub const NS2_CELL_TAG: &str = "ns2:c";
    pub const X_MERGE_CELL_TAG: &str = "x:mergeCell";
    pub const NS2_MERGE_CELL_TAG: &str = "ns2:mergeCell";
    pub const X_HYPERLINK_TAG: &str = "x:hyperlink";
    pub const NS2_HYPERLINK_TAG: &str = "ns2:hyperlink";

    pub const ATTRIBUTE_S: &'static str = "s";
    pub const ATTRIBUTE_REF: &'static str = "ref";
    pub const ATTRIBUTE_R: &'static str = "r";
    pub const ATTRIBUTE_T: &'static str = "t";
    pub const ATTRIBUTE_LOCATION: &'static str = "location";
    pub const ATTRIBUTE_RID: &'static str = "r:id";
    pub const CELL_RANGE_SPLIT: &'static str = ":";

    // 文本标签
    pub const SHAREDSTRINGS_T_TAG: &'static str = "t";
    pub const SHAREDSTRINGS_X_T_TAG: &'static str = "x:t";
    pub const SHAREDSTRINGS_NS2_T_TAG: &'static str = "ns2:t";

    // SharedStringItem 标签
    pub const SHAREDSTRINGS_SI_TAG: &'static str = "si";
    pub const SHAREDSTRINGS_X_SI_TAG: &'static str = "x:si";
    pub const SHAREDSTRINGS_NS2_SI_TAG: &'static str = "ns2:si";

    // Mac 2016/2017 的额外字段（需要忽略）
    pub const SHAREDSTRINGS_RPH_TAG: &'static str = "rPh";
    pub const SHAREDSTRINGS_X_RPH_TAG: &'static str = "x:rPh";
    pub const SHAREDSTRINGS_NS2_RPH_TAG: &'static str = "ns2:rPh";
}
