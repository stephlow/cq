use axum::{extract::Path, Extension, Json};
use engine::models::{self};
use sqlx::{query_as, PgPool};
use uuid::Uuid;

pub async fn register_user(
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<models::api::users::NewUser>,
) -> Json<models::api::auth::AuthResponse> {
    let new_user: models::data::users::NewUser = payload.into();

    let user: models::data::users::User = query_as(
        "INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3) RETURNING *;",
    )
    .bind(new_user.username)
    .bind(new_user.email)
    .bind(new_user.password_hash)
    .fetch_one(&pool)
    .await
    .unwrap();

    Json(models::api::auth::AuthResponse::from_user(user))
}

pub async fn get_user(
    Extension(pool): Extension<PgPool>,
    Path(id): Path<Uuid>,
) -> Json<models::api::users::User> {
    let user: models::data::users::User = query_as("SELECT * FROM users WHERE id = $1;")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();

    Json(user.into())
}
