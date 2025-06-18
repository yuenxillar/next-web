use axum::{http::StatusCode, response::IntoResponse};

use super::http_security::HttpSecurity;

pub trait WebSecurityConfigure: Send + Sync
{
    fn configure(&self) -> HttpSecurity;
}

struct Test;

impl WebSecurityConfigure for Test {

    fn configure(&self) -> HttpSecurity {
        HttpSecurity::new()
            .any_match("/user/**", |group| {
                group
                    .permissions(vec!["user.read", "user.write"])
                    .roles(vec!["user"])
            })
            .any_match("/admin/**", |group| group.roles(vec!["admin"]))
            .any_match("/goods/**", |group| group.permissions(vec!["goods"]))
            .any_match("/orders/**", |group| group.permissions(vec!["orders"]))
            .any_match("/notice/**", |group| group.permissions(vec!["notice"]))
            .any_match("/comment/**", |group| group.permissions(vec!["comment"]))
            .any_match("/test/**", |group| group.roles(vec!["test"]))
            .not_match("/**/*.js")
            .not_match("/**/*.css")
            .map_error(|error| {
                let error_msg = error.to_string();
                println!("error: {:?}", &error_msg);
                (StatusCode::UNAUTHORIZED, error_msg).into_response()
            })
            .disable()
    }
}
