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

#[cfg(test)]
mod tests {
    use super::UserAttributeEditor;

    fn authority_names(editor: &UserAttributeEditor) -> Vec<String> {
        editor
            .value()
            .unwrap()
            .authorities()
            .into_iter()
            .filter_map(|authority| futures::executor::block_on(authority.get_authority()))
            .collect()
    }

    #[test]
    fn parses_password_and_roles_with_trailing_spaces() {
        let mut editor = UserAttributeEditor::new();
        editor.set_as_text(Some("password ,ROLE_ONE,ROLE_TWO "));

        let user = editor.value().unwrap();
        assert_eq!(user.password(), Some("password"));
        assert_eq!(
            authority_names(&editor),
            vec![String::from("ROLE_ONE"), String::from("ROLE_TWO")]
        );
    }

    #[test]
    fn disabled_keyword_sets_enabled_false() {
        let mut editor = UserAttributeEditor::new();
        editor.set_as_text(Some("password,disabled,ROLE_ONE,ROLE_TWO"));

        let user = editor.value().unwrap();
        assert!(user.is_valid());
        assert!(!user.is_enabled());
        assert_eq!(user.password(), Some("password"));
    }

    #[test]
    fn enabled_keyword_is_not_treated_as_authority() {
        let mut editor = UserAttributeEditor::new();
        editor.set_as_text(Some("password,ROLE_ONE,enabled,ROLE_TWO"));

        assert!(editor.value().unwrap().is_enabled());
        assert_eq!(
            authority_names(&editor),
            vec![String::from("ROLE_ONE"), String::from("ROLE_TWO")]
        );
    }

    #[test]
    fn malformed_or_empty_input_returns_none() {
        let mut editor = UserAttributeEditor::new();

        editor.set_as_text(None);
        assert!(editor.value().is_none());

        editor.set_as_text(Some(""));
        assert!(editor.value().is_none());

        editor.set_as_text(Some("MALFORMED_STRING"));
        assert!(editor.value().is_none());

        editor.set_as_text(Some("password,enabled"));
        assert!(editor.value().is_none());
    }
}
