use axum::extract::Request;
use next_web_core::{anys::any_map::AnyMap, traits::required::Required};

use crate::{
    core::{authentication_error::AuthenticationError, filter::Filter},
    web::authentication::{
        rememberme::abstract_remember_me_services::AbstractRememberMeServices,
        username_password_authentication_filter::UsernamePasswordAuthenticationFilter,
    },
};

#[derive(Clone)]
pub struct DefaultLoginPageGeneratingFilter {
    login_page_url: Box<str>,
    logout_success_url: Box<str>,
    failure_url: Box<str>,
    form_login_enabled: bool,
    oauth2_login_enabled: bool,
    saml2_login_enabled: bool,
    authentication_url: Option<Box<str>>,
    username_parameter: Option<Box<str>>,
    password_parameter: Option<Box<str>>,
    remember_me_parameter: Option<Box<str>>,
}

impl DefaultLoginPageGeneratingFilter {
    const DEFAULT_LOGIN_PAGE_URL: &str = "/login";
    const ERROR_PARAMETER_NAME: &str = "error";

    pub fn new(auth_filter: Option<&UsernamePasswordAuthenticationFilter>) -> Self {
        let mut default = Self {
            login_page_url: "/login".into(),
            logout_success_url: "/login?logout".into(),
            failure_url: "/login?error".into(),
            form_login_enabled: Default::default(),
            oauth2_login_enabled: Default::default(),
            saml2_login_enabled: Default::default(),
            authentication_url: Default::default(),
            username_parameter: Default::default(),
            password_parameter: Default::default(),
            remember_me_parameter: Default::default(),
        };
        if let Some(auth_filter) = auth_filter {
            default.init_auth_filter(auth_filter);
        }

        default
    }

    fn init_auth_filter<'a>(&mut self, auth_filter: &'a UsernamePasswordAuthenticationFilter) {
        self.form_login_enabled = true;
        self.username_parameter = Some(auth_filter.get_username_parameter().into());
        self.password_parameter = Some(auth_filter.get_password_parameter().into());
        let obj: &dyn std::any::Any = auth_filter.get_object().get_remember_me_services();

        if let Some(remember_me_services) = obj.downcast_ref::<AbstractRememberMeServices>() {
            self.remember_me_parameter = Some(remember_me_services.get_parameter().into());
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.form_login_enabled || self.oauth2_login_enabled || self.saml2_login_enabled
    }

    pub fn set_logout_success_url(&mut self, url: impl Into<Box<str>>) {
        self.logout_success_url = url.into();
    }

    pub fn get_login_page_url(&self) -> &str {
        &self.login_page_url
    }

    pub fn set_form_login_enabled(&mut self, enabled: bool) {
        self.form_login_enabled = enabled;
    }

    pub fn set_username_parameter(&mut self, username_parameter: impl Into<Box<str>>) {
        self.username_parameter = Some(username_parameter.into());
    }

    pub fn set_password_parameter(&mut self, password_parameter: impl Into<Box<str>>) {
        self.password_parameter = Some(password_parameter.into());
    }

    pub fn set_login_page_url(&mut self, url: impl Into<Box<str>>) {
        self.login_page_url = url.into();
    }

    pub fn set_failure_url(&mut self, url: impl Into<Box<str>>) {
        self.failure_url = url.into();
    }

    pub fn set_authentication_url(&mut self, url: impl Into<Box<str>>) {
        self.authentication_url = Some(url.into());
    }

    async fn generate_login_page_html(
        &self,
        request: &mut Request,
        login_error: bool,
        logout_success: bool,
    ) -> String {
        let error_msg = if login_error {
            self.get_login_error_message(request).await
        } else {
            "Invalid credentials".to_string()
        };

        let mut html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1, shrink-to-fit=no">
    <meta name="description" content="">
    <meta name="author" content="">
    <title>Please sign in</title>
    <link href="https://maxcdn.bootstrapcdn.com/bootstrap/4.0.0-beta/css/bootstrap.min.css" rel="stylesheet" integrity="sha384-/Y6pD6FV/Vv2HJnA6t+vslU6fwYXjCFtcEpHbNJ0lyAFsXTsjBbfaDjzALeQsN6M" crossorigin="anonymous">
    <link href="https://getbootstrap.com/docs/4.0/examples/signin/signin.css" rel="stylesheet" integrity="sha384-oOE/3m0LUMPub4kaC09mrdEhIc+e3exm4xOGxAmuFXhBNF4hcg/6MiAXAf5p0P56" crossorigin="anonymous"/>
  </head>
  <body>
    <div class="container">
    "#
        );
      
        let s = format!(
            r#"<form class="form-signin" method="post" action="{context_path}{authentication_url}">
        <h2 class="form-signin-heading">Please sign in</h2>
{error_html}{logout_html}        <p>
          <label for="username" class="sr-only">Username</label>
          <input type="text" id="username" name="{username_parameter}" class="form-control" placeholder="Username" required autofocus>
        </p>
        <p>
          <label for="password" class="sr-only">Password</label>
          <input type="password" id="password" name="{password_parameter}" class="form-control" placeholder="Password" required>
        </p>
{remember_me_html}{hidden_inputs}        <button class="btn btn-lg btn-primary btn-block" type="submit">Sign in</button>
      </form>
    </div>
  </body>
</html>"#,
            context_path = "/ok",
            authentication_url = self.authentication_url.as_deref().unwrap_or_default(),
            username_parameter = self.username_parameter.as_deref().unwrap_or_default(),
            password_parameter = self.password_parameter.as_deref().unwrap_or_default(),
            error_html = self.create_error(login_error, & error_msg),
            logout_html = self.create_logout_success(logout_success),
            remember_me_html = self.create_remember_me(self.password_parameter.as_deref()),
            hidden_inputs = self.render_hidden_inputs(request),
        );

        if self.form_login_enabled {
            html = format!("{}\n{}", html, s)
        }

        if self.oauth2_login_enabled {
            html = format!( r#"
            <h2 class="form-signin-heading">Login with OAuth 2.0</h2>
            {create_error}
            {create_logout_success}
            <table class="table table-striped">

            </table>
            "#,
            create_error = self.create_error(login_error, & error_msg),
            create_logout_success = self.create_logout_success(logout_success),
        );
        }
        html += "</div>\n";
        html += "</body></html>";
        html
    }

    async fn get_login_error_message(&self, request: &mut Request) -> String {
        if let Some(map) = request.extensions().get::<AnyMap>() {
            if let Some(value) = map.get("NEXT_SECURITY_LAST_ERROR").await  {
                if let Some(value) = value.as_object::<AuthenticationError>() {
                    let msg = value.get_message().to_string();
                    if !msg.is_empty() {
                        return msg;
                    }
                }
            }
        }   
        
        "Invalid credentials".to_string()
    }

    fn create_remember_me(&self, param_name: Option<&str>) -> String {
        if let Some(name) =  param_name {
            format!("<p><input type='checkbox' name='{}'/> Remember me on this computer.</p>\n", name)
        } else { "".to_string() }
    }

    fn render_hidden_inputs(&self, request: &mut Request) -> String {
        // TODO 
        "".to_string()
    }

    fn create_error(&self, login_error: bool, error_msg: &str) -> String {
        if !login_error {
            "".to_string()
        } else {
            format!(
                "<div class=\"alert alert-danger\" role=\"alert\"> {} </div>",
                // TODO encodeing?
                // HtmlUtils.htmlEscape(error_msg)
                error_msg
            )
        }
    }

    fn create_logout_success(&self, is_logout_success: bool) -> String {
        if !is_logout_success {
            "".to_string()
        } else {
            "<div class=\"alert alert-success\" role=\"alert\">You have been signed out</div>"
                .to_string()
        }
    }
}

impl Filter for DefaultLoginPageGeneratingFilter {
    fn do_filter(&self, req: &mut axum::extract::Request, res: &mut axum::response::Response) -> Result<(), next_web_core::error::BoxError>{
        todo!()
    }
}
