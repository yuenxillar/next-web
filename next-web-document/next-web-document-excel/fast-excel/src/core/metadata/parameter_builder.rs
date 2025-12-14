use next_web_core::{traits::required::Required, util::locale::Locale};

use crate::core::{converters::converter::Converter, metadata::basic_parameter::BasicParameter};

pub trait ParameterBuilder<C>
where
    C: Required<BasicParameter>,
{
    fn head(&mut self, head: Vec<Vec<Box<str>>>) -> &mut Self {
        self.parameter().get_mut_object().set_head(head);
        self
    }

    fn register_converter<T: Converter>(&mut self, converter: T) -> &mut Self {
        self.parameter()
            .get_mut_object()
            .push_custom_converter(converter);
        self
    }

    fn use1904windowing(&mut self, use1904windowing: bool) -> &mut Self {
        self.parameter()
            .get_mut_object()
            .set_use1904windowing(use1904windowing);
        self
    }

    fn locale(&mut self, locale: Locale) -> &mut Self {
        self.parameter().get_mut_object().set_locale(locale);
        self
    }

    fn auto_trim(&mut self, auto_trim: bool) -> &mut Self {
        self.parameter().get_mut_object().set_auto_trim(auto_trim);
        self
    }

    fn parameter(&mut self) -> &mut C;
}
