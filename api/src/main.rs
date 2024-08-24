use axum::{
    routing::{get, post},
    Extension, Router,
};
use clap::Parser;
use engine::models::api::GameServer;
use handlers::{
    auth::{authenticate, profile},
    servers::{list_servers, ping_server, register_server},
};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;

mod handlers;

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
        .route("/auth", get(profile).post(authenticate))
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
