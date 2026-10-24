use std::fmt::Debug;

use next_web_core::{DynClone, clone_trait_object};

use crate::core::poi::ss::usermodel::font::Font;

pub trait RichTextString
where
    Self: DynClone,
    Self: Debug,
{
    fn apply_font_range_with_index(&mut self, start_index: u32, end_index: u32, font_index: u16);
    fn apply_font_range(&mut self, start_index: u32, end_index: u32, font: &dyn Font);
    fn apply_font(&mut self, font: &dyn Font);
    fn clear_formatting(&mut self);
    fn get_string(&self) -> &str;
    fn length(&self) -> usize;
    fn num_formatting_runs(&self) -> usize;
    fn get_index_of_formatting_run(&self, index: u32) -> usize;
    fn apply_font_with_index(&mut self, font_index: u16);
}

clone_trait_object!(RichTextString);
