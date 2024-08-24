use anyhow::Result;
use axum::{http::HeaderMap, response::IntoResponse, Extension, Json};
use engine::models;
use josekit::{
    jws::JwsHeader,
    jwt::{self, JwtPayload},
};
use sqlx::{query_as, PgPool};
use std::str::FromStr;
use uuid::Uuid;

pub async fn authenticate(
    Extension(pool): Extension<PgPool>,
    Json(credentials): Json<models::api::auth::Credentials>,
) -> impl IntoResponse {
    // TODO: Verify pass
    let user: models::data::users::User =
        query_as("SELECT * FROM users WHERE username = $1 AND password_hash = $2;")
            .bind(credentials.username)
            .bind(credentials.password)
            .fetch_one(&pool)
            .await
            .unwrap();

    let mut header = JwsHeader::new();
    header.set_token_type("JWT");

    let mut payload = JwtPayload::new();
    payload.set_subject(user.id.to_string().as_str());

    let token = jwt::encode_unsecured(&payload, &header).unwrap();

    Json(models::api::auth::AuthResponse { token })
}

pub async fn profile(
    Extension(pool): Extension<PgPool>,
    headers: HeaderMap,
) -> Json<models::api::users::User> {
    let id = get_user_id_from_auth(headers).unwrap();

    let user: models::data::users::User = query_as("SELECT * FROM users WHERE id = $1;")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();

    Json(user.into())
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
