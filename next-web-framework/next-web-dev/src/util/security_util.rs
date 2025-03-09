use axum::{extract::Request, http::header};

use super::token_util::TokenUtil;

pub struct SecurityUtil;

impl SecurityUtil {
    pub fn get_token(req: &Request) -> Option<String> {
        req.headers().get(header::AUTHORIZATION).map(|auth| {
            auth.to_str()
                .map(|token| token.replace("Bearer ", ""))
                .unwrap_or_default()
        })
    }
}
