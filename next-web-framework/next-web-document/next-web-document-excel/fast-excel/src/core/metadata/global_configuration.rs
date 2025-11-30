use next_web_core::util::locale::Locale;

#[derive(Debug, Clone)]
pub struct GlobalConfiguration {
    /// Automatic trim includes sheet name and content
    auto_trim: bool,
    /// true if date uses 1904 windowing, or false if using 1900 date windowing.
    /// default is false
    use1904windowing: bool,

    locale: Option<Locale>,

    /// Whether to use scientific Format.
    /// default is false
    use_scientific_format: bool,
}

impl GlobalConfiguration {
    pub fn get_locale(&self) -> Option<&Locale> {
        self.locale.as_ref()
    }

    pub fn set_locale(&mut self, locale: Locale) {
        self.locale = Some(locale);
    }

    pub fn get_use_scientific_format(&self) -> bool {
        self.use_scientific_format
    }

    pub fn set_use_scientific_format(&mut self, use_scientific_format: bool) {
        self.use_scientific_format = use_scientific_format;
    }

    pub fn get_auto_trim(&self) -> bool {
        self.auto_trim
    }

    pub fn set_auto_trim(&mut self, auto_trim: bool) {
        self.auto_trim = auto_trim;
    }

    pub fn get_use1904windowing(&self) -> bool {
        self.use1904windowing
    }

    pub fn set_use1904windowing(&mut self, use1904windowing: bool) {
        self.use1904windowing = use1904windowing;
    }
}

impl Default for GlobalConfiguration {
    fn default() -> Self {
        Self {
            auto_trim: true,
            use1904windowing: false,
            // TODO()
            locale: Some(Locale::default()),
            use_scientific_format: false,
        }
    }
}
