use next_web_core::util::locale::Locale;

use crate::core::converters::converter::Converter;

#[derive(Default)]
pub struct BasicParameter {
    head: Option<Vec<Vec<Box<str>>>>,
    // _type: Option<String>,
    custom_converter_list: Option<Vec<Box<dyn Converter>>>,
    auto_trim: Option<bool>,
    use1904windowing: Option<bool>,
    locale: Option<Locale>,
    use_scientific_format: Option<bool>,
}

impl BasicParameter {
    pub fn get_head(&self) -> Option<&Vec<Vec<Box<str>>>> {
        self.head.as_ref()
    }

    pub fn set_head(&mut self, head: Vec<Vec<Box<str>>>) {
        self.head = Some(head);
    }

    pub fn get_custom_converter_list(&self) -> Option<&Vec<Box<dyn Converter>>> {
        self.custom_converter_list.as_ref()
    }

    pub fn set_custom_converter_list(&mut self, custom_converter_list: Vec<Box<dyn Converter>>) {
        self.custom_converter_list = Some(custom_converter_list);
    }

    pub fn push_custom_converter<T: Converter>(&mut self, custom_converter: T) {
        let custom_converter = Box::new(custom_converter);
        if let Some(list) = &mut self.custom_converter_list {
            list.push(custom_converter);
        } else {
            self.custom_converter_list = Some(vec![custom_converter]);
        }
    }

    pub fn get_auto_trim(&self) -> Option<bool> {
        self.auto_trim
    }

    pub fn set_auto_trim(&mut self, auto_trim: bool) {
        self.auto_trim = Some(auto_trim);
    }

    pub fn get_use1904windowing(&self) -> Option<bool> {
        self.use1904windowing
    }

    pub fn set_use1904windowing(&mut self, use1904windowing: bool) {
        self.use1904windowing = Some(use1904windowing);
    }

    pub fn get_locale(&self) -> Option<&Locale> {
        self.locale.as_ref()
    }

    pub fn set_locale(&mut self, locale: Locale) {
        self.locale = Some(locale);
    }

    pub fn get_use_scientific_format(&self) -> Option<bool> {
        self.use_scientific_format
    }

    pub fn set_use_scientific_format(&mut self, use_scientific_format: bool) {
        self.use_scientific_format = Some(use_scientific_format);
    }
}
