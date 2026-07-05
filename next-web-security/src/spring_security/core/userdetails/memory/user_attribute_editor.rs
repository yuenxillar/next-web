use super::user_attribute::UserAttribute;

#[derive(Clone, Default)]
pub struct UserAttributeEditor {
    value: Option<UserAttribute>,
}

impl UserAttributeEditor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_as_text(&mut self, text: Option<&str>) {
        let Some(text) = text else {
            self.value = None;
            return;
        };
        if text.trim().is_empty() {
            self.value = None;
            return;
        }

        let tokens = text.split(',').map(str::trim).collect::<Vec<_>>();
        let mut user_attribute = UserAttribute::new();
        let mut authorities = Vec::new();

        for (index, token) in tokens.into_iter().enumerate() {
            if index == 0 {
                user_attribute.set_password(token);
                continue;
            }

            if token.eq_ignore_ascii_case("enabled") {
                user_attribute.set_enabled(true);
            } else if token.eq_ignore_ascii_case("disabled") {
                user_attribute.set_enabled(false);
            } else if !token.is_empty() {
                authorities.push(token.to_string());
            }
        }

        user_attribute.set_authorities_as_string(authorities);
        self.value = user_attribute.is_valid().then_some(user_attribute);
    }

    pub fn value(&self) -> Option<&UserAttribute> {
        self.value.as_ref()
    }

    pub fn take_value(self) -> Option<UserAttribute> {
        self.value
    }
}
