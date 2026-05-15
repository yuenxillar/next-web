use std::sync::Arc;

use next_web_core::async_trait;

use crate::core::{
    authentication::Authentication,
    granted_authorities_container::GrantedAuthoritiesContainer,
    userdetails::{
        authentication_user_details_service::AuthenticationUserDetailsService, user::User,
        user_details::UserDetails, username_not_found_error::UsernameNotFoundError,
    },
};

use super::{
    pre_authenticated_authentication_token::PreAuthenticatedAuthenticationToken,
    pre_authenticated_granted_authorities_web_authentication_details::PreAuthenticatedGrantedAuthoritiesWebAuthenticationDetails,
};

#[derive(Clone, Default)]
pub struct PreAuthenticatedGrantedAuthoritiesUserDetailsService;

#[async_trait]
impl AuthenticationUserDetailsService<PreAuthenticatedAuthenticationToken>
    for PreAuthenticatedGrantedAuthoritiesUserDetailsService
{
    async fn load_user_details(
        &self,
        token: &PreAuthenticatedAuthenticationToken,
    ) -> Result<Arc<dyn UserDetails>, UsernameNotFoundError> {
        let details = token
            .get_details_ref()
            .and_then(|value| {
                value
                    .as_ref_object::<PreAuthenticatedGrantedAuthoritiesWebAuthenticationDetails>()
            })
            .ok_or_else(|| {
                UsernameNotFoundError(String::from(
                    "token.get_details() must contain GrantedAuthoritiesContainer details",
                ))
            })?;

        let username = token.get_name();
        Ok(Arc::new(User::with_flags(
            username,
            Some(String::from("N/A")),
            true,
            true,
            true,
            true,
            details.granted_authorities(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, extract::Request, http::Request as HttpRequest};
    use next_web_core::anys::any_value::AnyValue;

    use crate::core::{
        authority_utils::AuthorityUtils,
        userdetails::authentication_user_details_service::AuthenticationUserDetailsService,
    };

    use super::{
        PreAuthenticatedGrantedAuthoritiesUserDetailsService,
        PreAuthenticatedGrantedAuthoritiesWebAuthenticationDetails,
    };

    #[tokio::test]
    async fn service_creates_user_details_from_pre_authenticated_details() {
        let service = PreAuthenticatedGrantedAuthoritiesUserDetailsService;
        let request = Request::from(
            HttpRequest::builder()
                .header("x-forwarded-for", "127.0.0.1")
                .body(Body::empty())
                .unwrap(),
        );
        let mut token =
            crate::web::authentication::preauth::pre_authenticated_authentication_token::PreAuthenticatedAuthenticationToken::unauthenticated(
                Some(String::from("dummyUser")),
                Some(String::from("dummy")),
            );
        token.set_details_value(Some(AnyValue::Object(Box::new(
            PreAuthenticatedGrantedAuthoritiesWebAuthenticationDetails::new(
                &request,
                AuthorityUtils::create_authority_list(["Role1", "Role2"]),
            ),
        ))));

        let user = service.load_user_details(&token).await.unwrap();
        let authorities = user.get_authorities().await;
        let names = authorities
            .into_iter()
            .filter_map(|authority| futures::executor::block_on(authority.get_authority()))
            .collect::<Vec<_>>();

        assert_eq!(user.get_username().await, "dummyUser");
        assert!(user.is_account_non_expired().await);
        assert!(user.is_account_non_locked().await);
        assert!(user.is_credentials_non_expired().await);
        assert!(user.is_enabled().await);
        assert_eq!(names, vec![String::from("Role1"), String::from("Role2")]);
    }

    #[tokio::test]
    async fn service_rejects_missing_details() {
        let service = PreAuthenticatedGrantedAuthoritiesUserDetailsService;
        let token =
            crate::web::authentication::preauth::pre_authenticated_authentication_token::PreAuthenticatedAuthenticationToken::unauthenticated(
                Some(String::from("dummyUser")),
                Some(String::from("dummy")),
            );

        let error = match service.load_user_details(&token).await {
            Ok(_) => panic!("expected missing details to fail"),
            Err(error) => error,
        };
        assert!(error
            .0
            .contains("token.get_details() must contain GrantedAuthoritiesContainer details"));
    }
}
