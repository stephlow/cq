use axum::{Extension, Json};
use bcrypt::{hash, DEFAULT_COST};
use engine::models::{self};
use josekit::{
    jws::JwsHeader,
    jwt::{self, JwtPayload},
};
use sqlx::{query_as, PgPool};

pub async fn register_user(
    Extension(pool): Extension<PgPool>,
    Json(new_user): Json<models::api::users::NewUser>,
) -> Json<models::api::auth::AuthResponse> {
    let password_hash = hash(new_user.password, DEFAULT_COST).unwrap();

    let user: models::data::users::User = query_as(
        "INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3) RETURNING *;",
    )
    .bind(new_user.username)
    .bind(new_user.email)
    .bind(password_hash)
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
