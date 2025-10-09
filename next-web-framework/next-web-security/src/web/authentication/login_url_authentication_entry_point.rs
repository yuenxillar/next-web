pub struct LoginUrlAuthenticationEntryPoint {
    login_form_url: Box<str>,
}
impl LoginUrlAuthenticationEntryPoint {
    pub fn new(login_form_url: &str) -> Self {
        assert!(
            !login_form_url.trim().is_empty(),
            "login_form_url cannot be null"
        );
        let login_form_url = login_form_url.into();
        Self {
            login_form_url,
        }
    }
}
