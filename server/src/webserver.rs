use crate::AppMessage;
use axum::{response::IntoResponse, routing::get, Extension, Json, Router};
use tokio::sync::mpsc;

pub fn create_router(tx: mpsc::Sender<AppMessage>) -> Router {
    Router::new()
        .route("/", get(get_server))
        .route("/connections", get(get_connections))
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
async fn get_connections(Extension(tx): Extension<mpsc::Sender<AppMessage>>) -> impl IntoResponse {
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();

    tx.send(AppMessage::GetConnections(resp_tx)).await.unwrap();

    let connections = resp_rx.await.unwrap();

    Json(connections)
}
