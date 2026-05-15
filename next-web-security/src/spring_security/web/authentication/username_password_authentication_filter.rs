use next_web_core::traits::required::Required;

use crate::{
    core::{
        filter::Filter,
        username_password_authentication_token::UsernamePasswordAuthenticationToken,
    },
    web::authentication::abstract_authentication_processing_filter::AbstractAuthenticationProcessingFilter,
};

#[derive(Clone)]
pub struct UsernamePasswordAuthenticationFilter {
    username_parameter: Box<str>,
    password_parameter: Box<str>,
    post_only: bool,
    abstract_authentication_processing_filter: AbstractAuthenticationProcessingFilter,
}

impl Default for UsernamePasswordAuthenticationFilter {
    fn default() -> Self {
        Self {
            username_parameter: "username".into(),
            password_parameter: "password".into(),
            post_only: true,
            abstract_authentication_processing_filter:
                AbstractAuthenticationProcessingFilter::default(),
        }
    }
}

impl UsernamePasswordAuthenticationFilter {
    pub fn set_username_parameter(&mut self, username_parameter: &str) {
        assert!(
            !username_parameter.trim().is_empty(),
            "username_parameter cannot be empty"
        );
        self.username_parameter = username_parameter.into();
    }

    pub fn set_password_parameter(&mut self, password_parameter: &str) {
        assert!(
            !password_parameter.trim().is_empty(),
            "password_parameter cannot be empty"
        );
        self.password_parameter = password_parameter.into();
    }

    pub fn get_username_parameter(&self) -> &str {
        &self.username_parameter
    }

    pub fn get_password_parameter(&self) -> &str {
        &self.password_parameter
    }
}
impl Required<AbstractAuthenticationProcessingFilter> for UsernamePasswordAuthenticationFilter {
    fn get_object(&self) -> &AbstractAuthenticationProcessingFilter {
        &self.abstract_authentication_processing_filter
    }

    fn get_mut_object(&mut self) -> &mut AbstractAuthenticationProcessingFilter {
        &mut self.abstract_authentication_processing_filter
    }
}

impl Filter for UsernamePasswordAuthenticationFilter {
    fn do_filter(
        &self,
        req: &mut axum::extract::Request,
        res: &mut axum::response::Response,
    ) -> Result<(), next_web_core::error::BoxError> {
        if !self
            .abstract_authentication_processing_filter
            .requires_authentication(req)
        {
            return Ok(());
        }

        if self.post_only && req.method() != axum::http::Method::POST {
            *res.status_mut() = axum::http::StatusCode::METHOD_NOT_ALLOWED;
            return Ok(());
        }

        let username = request_parameter(req, &self.username_parameter).unwrap_or_default();
        let password = request_parameter(req, &self.password_parameter);
        let token = UsernamePasswordAuthenticationToken::unauthenticated(
            Some(username.trim().to_string()),
            password,
        );

        self.abstract_authentication_processing_filter
            .attempt_authentication(req, res, &token)?;
        Ok(())
    }
}

fn request_parameter(request: &axum::extract::Request, name: &str) -> Option<String> {
    request
        .uri()
        .query()
        .and_then(|query| parse_urlencoded_parameter(query, name))
        .or_else(|| {
            request
                .extensions()
                .get::<next_web_core::anys::any_map::AnyMap>()
                .and_then(|map| {
                crate::web::authentication::preauth::abstract_pre_authenticated_processing_filter::block_on(
                    map.get(name),
                )
                .and_then(|value| value.as_string())
            })
        })
}

fn parse_urlencoded_parameter(query: &str, name: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or_default();
        if key != name {
            return None;
        }
        let value = parts.next().unwrap_or_default();
        urlencoding::decode(value).ok().map(|value| value.into_owned())
    })
}
