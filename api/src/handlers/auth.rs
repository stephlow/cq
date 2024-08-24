use anyhow::Result;
use axum::{http::HeaderMap, response::IntoResponse, Json};
use engine::models;
use josekit::{
    jws::JwsHeader,
    jwt::{self, JwtPayload},
};
use std::str::FromStr;
use uuid::Uuid;

pub async fn authenticate(
    Json(_credentials): Json<models::api::auth::Credentials>,
) -> impl IntoResponse {
    let id = Uuid::new_v4();

    let mut header = JwsHeader::new();
    header.set_token_type("JWT");

    let mut payload = JwtPayload::new();
    payload.set_subject(id.to_string().as_str());

    let token = jwt::encode_unsecured(&payload, &header).unwrap();

    Json(models::api::auth::AuthResponse { token })
}

pub async fn profile(headers: HeaderMap) -> impl IntoResponse {
    let id = get_user_id_from_auth(headers).unwrap();

    Json(models::api::users::User { id })
}

pub fn get_user_id_from_auth(headers: HeaderMap) -> Result<Uuid, ()> {
    if let Some(value) = headers.get("Authorization") {
        if let Ok(value) = value.to_str() {
            let (_key, token) = value.split_once("Bearer: ").unwrap();
            if let Ok((payload, _header)) = jwt::decode_unsecured(token) {
                if let Some(subject) = payload.subject() {
                    if let Ok(id) = FromStr::from_str(subject) {
                        return Ok(id);
                    }
                }
            }
        }
    }

    Err(())
}
