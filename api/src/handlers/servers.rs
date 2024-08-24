use axum::{
    extract::{ConnectInfo, Path},
    response::IntoResponse,
    Extension, Json,
};
use engine::models::api::servers::{RegisterServer, Server};
use std::net::SocketAddr;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::SharedState;

#[axum::debug_handler]
pub async fn list_servers(Extension(state): Extension<SharedState>) -> impl IntoResponse {
    let state = state.read().await;

    // TODO: Do a proper cleanup
    let servers: Vec<Server> = state
        .servers
        .iter()
        .filter(|server| {
            let now = OffsetDateTime::now_utc();

            now - server.last_ping <= Duration::seconds(30)
        })
        .cloned()
        .collect();

    Json(servers)
}

#[axum::debug_handler]
pub async fn register_server(
    Extension(state): Extension<SharedState>,
    // TODO: Verify addr
    ConnectInfo(_addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<RegisterServer>,
) -> impl IntoResponse {
    let mut state = state.write().await;

    let server = Server::new(payload.addr, payload.port, payload.name);

    state.servers.push(server.clone());

    Json(server)
}

#[axum::debug_handler]
pub async fn ping_server(
    Path(id): Path<Uuid>,
    Extension(state): Extension<SharedState>,
    // TODO: Verify addr
    ConnectInfo(_addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let mut state = state.write().await;

    let server = state
        .servers
        .iter_mut()
        .find(|server| server.id == id)
        .unwrap();

    server.ping();

    Json(server.clone())
}
