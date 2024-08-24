use crate::{MessageRow, UserRow};
use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};
use sqlx::{query_as, Pool, Sqlite};

#[derive(Clone)]
struct ApiState {
    pub pool: Pool<Sqlite>,
}

pub async fn create_app(pool: Pool<Sqlite>) -> Router {
    let api_state = ApiState { pool };

    Router::new()
        .route("/api/messages", get(get_messages))
        .route("/api/users", get(get_users))
        .with_state(api_state)
}

#[axum::debug_handler]
async fn get_messages(State(api_state): State<ApiState>) -> impl IntoResponse {
    let messages: Vec<MessageRow> = query_as("SELECT * FROM messages;")
        .fetch_all(&api_state.pool)
        .await
        .unwrap();

    Json(messages)
}

#[axum::debug_handler]
async fn get_users(State(api_state): State<ApiState>) -> impl IntoResponse {
    let users: Vec<UserRow> = query_as("SELECT * FROM users;")
        .fetch_all(&api_state.pool)
        .await
        .unwrap();

    Json(users)
}
