use anyhow::Result;
use axum::{http::HeaderMap, Extension, Json};
use engine::models;
use josekit::jwt::{self};
use sqlx::{query_as, PgPool};
use std::str::FromStr;
use uuid::Uuid;

pub async fn authenticate(
    Extension(pool): Extension<PgPool>,
    Json(credentials): Json<models::api::auth::Credentials>,
) -> Result<Json<models::api::auth::AuthResponse>, ()> {
    let user: models::data::users::User = query_as("SELECT * FROM users WHERE username = $1;")
        .bind(credentials.username)
        .fetch_one(&pool)
        .await
        .unwrap();

    if user.verify_password(&credentials.password) {
        return Ok(Json(models::api::auth::AuthResponse::from_user(user)));
    }

    Err(())
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
