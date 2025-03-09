use super::user_authorization_options_properties::UserAuthorizationOptions;

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct SecurityProperties {
    authentication_options: Option<UserAuthorizationOptions>,
}

impl SecurityProperties {
    pub fn authentication_options(&self) -> Option<&UserAuthorizationOptions> {
        self.authentication_options.as_ref()
    }
}
