use axum::{Extension, Json};
use engine::models::{self};
use sqlx::{query_as, PgPool};

pub async fn register_user(
    Extension(pool): Extension<PgPool>,
    Json(new_user): Json<models::api::users::NewUser>,
) -> Json<models::api::users::User> {
    // TODO Hash
    let user: models::data::users::User = query_as(
        "INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3) RETURNING *;",
    )
    .bind(new_user.username)
    .bind(new_user.email)
    .bind(new_user.password)
    .fetch_one(&pool)
    .await
    .unwrap();

    Json(user.into())
}
