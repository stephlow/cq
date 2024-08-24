use crate::models;
use josekit::{
    jws::JwsHeader,
    jwt::{self, JwtPayload},
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
}

impl AuthResponse {
    pub fn from_user(value: models::data::users::User) -> Self {
        let mut header = JwsHeader::new();
        header.set_token_type("JWT");

        let mut payload = JwtPayload::new();
        payload.set_subject(value.id.to_string().as_str());

        let token = jwt::encode_unsecured(&payload, &header).unwrap();

        Self { token }
    }
}
