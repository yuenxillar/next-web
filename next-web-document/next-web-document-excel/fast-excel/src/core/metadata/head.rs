use crate::core::metadata::property::{
    column_width_property::ColumnWidthProperty, font_property::FontProperty,
    loop_merge_property::LoopMergeProperty, style_property::StyleProperty,
};

#[derive(Debug, Clone)]
pub struct Head {
    /// Column index of head
    column_index: Option<u32>,
    field_name: Option<Box<str>>,
    /// Head name
    head_name_list: Option<Vec<String>>,
    /// Whether index is specified
    force_index: Option<bool>,
    /// Whether to specify a name
    force_name: Option<bool>,
    /// column with
    column_width_property: Option<ColumnWidthProperty>,
    /// Loop merge
    loop_merge_property: Option<LoopMergeProperty>,
    /// Head style
    head_style_property: Option<StyleProperty>,
    /// Head font
    head_font_property: Option<FontProperty>,
}

impl Head {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        column_index: u32,
        field_name: Option<Box<str>>,
        head_name_list: Option<Vec<String>>,
        force_index: bool,
        force_name: bool,
    ) -> Self {
        Self {
            field_name,
            head_name_list,
            column_index: Some(column_index),
            force_index: Some(force_index),
            force_name: Some(force_name),
            column_width_property: None,
            loop_merge_property: None,
            head_style_property: None,
            head_font_property: None,
        }
    }

    pub fn get_column_index(&self) -> Option<u32> {
        self.column_index
    }

    pub fn set_column_index(&mut self, column_index: u32) {
        self.column_index = Some(column_index);
    }

    pub fn get_field_name(&self) -> Option<&str> {
        self.field_name.as_deref()
    }

    pub fn set_field_name(&mut self, field_name: impl Into<Box<str>>) {
        self.field_name = Some(field_name.into());
    }

    pub fn get_head_name_list(&self) -> Option<&[String]> {
        self.head_name_list.as_deref()
    }

    pub fn get_mut_head_name_list(&mut self) -> Option<&mut Vec<String>> {
        self.head_name_list.as_mut()
    }

    pub fn set_head_name_list(&mut self, head_name_list: Vec<String>) {
        self.head_name_list = Some(head_name_list);
    }

    pub fn get_force_index(&self) -> Option<bool> {
        self.force_index
    }

    pub fn set_force_index(&mut self, force_index: bool) {
        self.force_index = Some(force_index);
    }

    pub fn get_force_name(&self) -> Option<bool> {
        self.force_name
    }

    pub fn set_force_name(&mut self, force_name: bool) {
        self.force_name = Some(force_name);
    }

    pub fn get_loop_merge_property(&self) -> Option<&LoopMergeProperty> {
        self.loop_merge_property.as_ref()
    }

    pub fn set_loop_merge_property(&mut self, loop_merge_property: LoopMergeProperty) {
        self.loop_merge_property = Some(loop_merge_property);
    }

    pub fn get_column_width_property(&self) -> Option<&ColumnWidthProperty> {
        self.column_width_property.as_ref()
    }

    pub fn set_column_width_property(&mut self, column_width_property: ColumnWidthProperty) {
        self.column_width_property = Some(column_width_property);
    }

    pub fn get_style_property(&self) -> Option<&StyleProperty> {
        self.head_style_property.as_ref()
    }

    pub fn set_style_property(&mut self, style_property: StyleProperty) {
        self.head_style_property = Some(style_property);
    }

    pub fn get_font_property(&self) -> Option<&FontProperty> {
        self.head_font_property.as_ref()
    }

    pub fn set_font_property(&mut self, font_property: FontProperty) {
        self.head_font_property = Some(font_property);
    }
}
