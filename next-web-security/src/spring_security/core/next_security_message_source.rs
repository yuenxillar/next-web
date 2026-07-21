use next_web_context::support::MessageSourceAccessor;

#[derive(Clone)]
pub struct NextSecurityMessageSource;

impl NextSecurityMessageSource {
    pub fn get_accessor() -> MessageSourceAccessor {
        MessageSourceAccessor::new(Self {})
    }
}
