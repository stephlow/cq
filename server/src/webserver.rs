use crate::AppMessage;
use axum::{response::IntoResponse, routing::get, Extension, Json, Router};
use tokio::sync::mpsc;

pub fn create_router(tx: mpsc::Sender<AppMessage>) -> Router {
    Router::new()
        .route("/", get(get_server))
        .route("/players", get(get_players))
        .layer(Extension(tx))
}

#[axum::debug_handler]
async fn get_server(Extension(tx): Extension<mpsc::Sender<AppMessage>>) -> impl IntoResponse {
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();

    tx.send(AppMessage::GetServer(resp_tx)).await.unwrap();

    let connections = resp_rx.await.unwrap();

    Json(connections)
}

#[axum::debug_handler]
async fn get_players(Extension(tx): Extension<mpsc::Sender<AppMessage>>) -> impl IntoResponse {
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();

    tx.send(AppMessage::GetPlayers(resp_tx)).await.unwrap();

    let players = resp_rx.await.unwrap();

    Json(players)
}
