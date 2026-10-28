use std::ops::{Deref, DerefMut};

use crate::core::metadata::data::{
    client_anchor_data::ClientAnchorData, rich_text_string_data::RichTextStringData,
};

#[derive(Debug, Clone)]
pub struct CommentData {
    /// Name of the original comment author
    author: Option<Box<str>>,
    /// rich text string
    rich_text_string_data: Option<RichTextStringData>,

    client_anchor_data: ClientAnchorData,
}

// 为 CommentData 结构体生成的 Getter/Setter 方法

impl CommentData {
    pub fn get_author(&self) -> Option<&str> {
        self.author.as_deref()
    }

    pub fn get_rich_text_string_data(&self) -> Option<&RichTextStringData> {
        self.rich_text_string_data.as_ref()
    }

    pub fn get_client_anchor_data(&self) -> &ClientAnchorData {
        &self.client_anchor_data
    }

    pub fn set_author(&mut self, author: impl Into<Box<str>>) {
        self.author = Some(author.into());
    }

    pub fn set_rich_text_string_data(&mut self, rich_text_string_data: RichTextStringData) {
        self.rich_text_string_data = Some(rich_text_string_data);
    }

    pub fn set_client_anchor_data(&mut self, client_anchor_data: ClientAnchorData) {
        self.client_anchor_data = client_anchor_data;
    }

    pub fn clear_author(&mut self) {
        self.author = None;
    }

    pub fn clear_rich_text_string_data(&mut self) {
        self.rich_text_string_data = None;
    }
}

impl Deref for CommentData {
    type Target = ClientAnchorData;

    fn deref(&self) -> &Self::Target {
        &self.client_anchor_data
    }
}

impl DerefMut for CommentData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client_anchor_data
    }
}
