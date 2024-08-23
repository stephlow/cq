use axum::{
    extract::{ConnectInfo, Path},
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use engine::models::api::{GameServer, RegisterGameServer};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

type SharedState = Arc<RwLock<ApiState>>;

#[derive(Default, Clone)]
struct ApiState {
    servers: Vec<GameServer>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let state = SharedState::default();

    let app = Router::new()
        .route("/servers", get(list_servers).post(register_server))
        .route("/servers/:id/ping", post(ping_server))
        .layer(Extension(state))
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

#[axum::debug_handler]
async fn list_servers(Extension(state): Extension<SharedState>) -> impl IntoResponse {
    let state = state.read().await;

    let servers = state.servers.clone();

    Json(servers)
}

#[axum::debug_handler]
async fn register_server(
    Extension(state): Extension<SharedState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<RegisterGameServer>,
) -> impl IntoResponse {
    let mut state = state.write().await;

    let server = GameServer::new(addr, payload.name);

    state.servers.push(server.clone());

    Json(server)
}

#[axum::debug_handler]
async fn ping_server(
    Path(id): Path<Uuid>,
    Extension(state): Extension<SharedState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let mut state = state.write().await;

    let server = state
        .servers
        .iter_mut()
        .find(|server| server.id == id && server.addr == addr)
        .unwrap();

    server.ping();

    Json(server.clone())
}
