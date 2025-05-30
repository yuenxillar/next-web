// use jsonwebtoken::{encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};


// pub struct TokenUtil;

// const SECRET_KEY: &'static [u8] = b"next-web-jwt-secret";

// impl TokenUtil {
//     pub fn generate_token(user_info: &UserInfo) -> String {
//         // generate a new JWT token
//         let mut header = Header::new(Algorithm::HS512);
//         header.kid = Some("daydream-key".into());
//         let token = encode(
//             &header,
//             user_info,
//             &EncodingKey::from_secret(SECRET_KEY.as_ref()),
//         )
//         .unwrap();
//         return token;
//     }

//     pub fn decode(token: &str) -> Result<UserInfo, jsonwebtoken::errors::Error> {
//         // decode a JWT token
//         let mut validation = Validation::new(Algorithm::HS512);
//         validation.set_required_spec_claims(&["exp"]);
//         let token_data = jsonwebtoken::decode::<UserInfo>(
//             token,
//             &DecodingKey::from_secret(SECRET_KEY.as_ref()),
//             &validation,
//         )?;
//         Ok(token_data.claims)
//     }
// }
