use axum::{
    extract::{ConnectInfo, Path},
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use clap::Parser;
use engine::models::api::{GameServer, RegisterGameServer};
use std::{net::SocketAddr, sync::Arc};
use time::{Duration, OffsetDateTime};
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct ApiArgs {
    /// The port to run the server on
    #[arg(short, long, default_value = "3000")]
    port: u16,
}

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

    let args = ApiArgs::parse();

    let state = SharedState::default();

    let app = Router::new()
        .route("/servers", get(list_servers).post(register_server))
        .route("/servers/:id/ping", post(ping_server))
        .layer(Extension(state))
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", args.port))
        .await
        .unwrap();
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

    // TODO: Do a proper cleanup
    let servers: Vec<GameServer> = state
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
async fn register_server(
    Extension(state): Extension<SharedState>,
    // TODO: Verify addr
    ConnectInfo(_addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<RegisterGameServer>,
) -> impl IntoResponse {
    let mut state = state.write().await;

    let server = GameServer::new(payload.addr, payload.port, payload.name);

    state.servers.push(server.clone());

    Json(server)
}

#[axum::debug_handler]
async fn ping_server(
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
