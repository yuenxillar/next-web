use crate::core::write::metadata::style::write_font::WriteFont;

#[derive(Debug, Clone)]
pub struct RichTextStringData {
    text: String,
    write_font: Option<WriteFont>,
    interval_font_list: Option<Vec<IntervalFont>>,
}

impl RichTextStringData {
    pub fn new<T: ToString>(text: T) -> Self {
        Self {
            text: text.to_string(),
            write_font: None,
            interval_font_list: None,
        }
    }

    pub fn apply_font(&mut self, start_index: u32, end_index: u32, write_font: WriteFont) {
        if let Some(interval_font_list) = &mut self.interval_font_list {
            interval_font_list.push(IntervalFont::new(start_index, end_index, Some(write_font)));
        } else {
            self.interval_font_list = Some(vec![IntervalFont::new(
                start_index,
                end_index,
                Some(write_font),
            )]);
        }
    }

    pub fn get_text(&self) -> &str {
        &self.text
    }

    pub fn get_write_font(&self) -> Option<&WriteFont> {
        self.write_font.as_ref()
    }

    pub fn get_interval_font_list(&self) -> Option<&Vec<IntervalFont>> {
        self.interval_font_list.as_ref()
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
    }

    pub fn set_write_font(&mut self, write_font: WriteFont) {
        self.write_font = Some(write_font);
    }

    pub fn set_interval_font_list(&mut self, interval_font_list: Vec<IntervalFont>) {
        self.interval_font_list = Some(interval_font_list);
    }
}

#[derive(Debug, Clone, Default)]
pub struct IntervalFont {
    start_index: Option<u32>,
    end_index: Option<u32>,
    write_font: Option<WriteFont>,
}

impl IntervalFont {
    pub fn new(start_index: u32, end_index: u32, write_font: Option<WriteFont>) -> Self {
        Self {
            start_index: Some(start_index),
            end_index: Some(end_index),
            write_font,
        }
    }

    pub fn get_start_index(&self) -> Option<u32> {
        self.start_index
    }

    pub fn get_end_index(&self) -> Option<u32> {
        self.end_index
    }

    pub fn get_write_font(&self) -> Option<&WriteFont> {
        self.write_font.as_ref()
    }

    pub fn set_start_index(&mut self, start_index: u32) {
        self.start_index = Some(start_index);
    }

    pub fn set_end_index(&mut self, end_index: u32) {
        self.end_index = Some(end_index);
    }

    pub fn set_write_font(&mut self, write_font: WriteFont) {
        self.write_font = Some(write_font);
    }
}
