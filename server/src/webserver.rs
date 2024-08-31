use crate::AppMessage;
use axum::{
    extract::Path,
    response::IntoResponse,
    routing::{delete, get},
    Extension, Json, Router,
};
use models::server::api::{PlayerResponse, ServerInfoResponse};
use tokio::sync::mpsc;
use uuid::Uuid;

pub fn create_router(tx: mpsc::Sender<AppMessage>) -> Router {
    Router::new()
        .route("/", get(get_server))
        .route("/players", get(get_players))
        .route("/players/:id", delete(kick_player))
        .layer(Extension(tx))
}

#[axum::debug_handler]
async fn get_server(
    Extension(tx): Extension<mpsc::Sender<AppMessage>>,
) -> Json<ServerInfoResponse> {
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();

    tx.send(AppMessage::GetServer(resp_tx)).await.unwrap();

    let server = resp_rx.await.unwrap();

    Json(ServerInfoResponse { server })
}

#[axum::debug_handler]
async fn get_players(Extension(tx): Extension<mpsc::Sender<AppMessage>>) -> Json<PlayerResponse> {
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();

    tx.send(AppMessage::GetPlayers(resp_tx)).await.unwrap();

    let response = resp_rx.await.unwrap();

    let players = PlayerResponse {
        players: response.into_iter().map(|(id, _)| id).collect(),
    };

    Json(players)
}

#[axum::debug_handler]
async fn kick_player(
    Path(id): Path<Uuid>,
    Extension(tx): Extension<mpsc::Sender<AppMessage>>,
) -> impl IntoResponse {
    tx.send(AppMessage::KickPlayer(id)).await.unwrap();

    "Ok"
}
