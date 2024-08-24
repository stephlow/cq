use anyhow::Result;
use axum::{routing::get, Extension, Json, Router};
use bevy::{app::ScheduleRunnerPlugin, log::tracing_subscriber, prelude::*};
use bevy_quinnet::shared::ClientId;
use clap::{arg, Parser};
use engine::{models::api::servers::Server, resources::TokioRuntimeResource};
use plugins::network::{ConnectionResource, NetworkPlugin};
use serde::Serialize;
use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, Mutex},
    time::Duration,
};
use uuid::Uuid;

mod plugins;

#[derive(Parser, Debug, Resource)]
#[command(version, about, long_about = None)]
struct ServerArgs {
    /// The name of the server
    #[arg(short, long, default_value = "My Server")]
    name: String,

    #[arg(long, default_value = "http://localhost:3000")]
    api_base_url: String,

    #[arg(long, default_value = "127.0.0.1")]
    addr: IpAddr,

    /// The port to run the server on
    #[arg(short, long, default_value = "2525")]
    port: u16,

    /// The port to run the management web server on
    #[arg(short, long, default_value = "3001")]
    web_port: u16,
}

enum TokioServerMessage {
    PingServer(Server),
    RegisterServer(Server),
}

#[derive(Default, Serialize, Clone)]
struct ServerState {
    connections: HashMap<ClientId, Uuid>,
}

// TODO: Should move to non-locking message passing
type ArcMutexServerState = Arc<Mutex<ServerState>>;

#[derive(Resource)]
struct ServerStateResource(ArcMutexServerState);

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let args = ServerArgs::parse();
    let port = args.port.clone();
    let web_port = args.web_port.clone();

    let server_state = ArcMutexServerState::default();
    let server_state_resource = ServerStateResource(server_state.clone());

    let app_handle = tokio::spawn(async move {
        App::new()
            .add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
                Duration::from_secs_f64(1.0 / 60.0),
            )))
            .insert_resource(server_state_resource)
            .insert_resource(args)
            .insert_resource(TokioRuntimeResource::<TokioServerMessage>::new())
            .add_systems(Update, tokio_receiver_system)
            .add_plugins(NetworkPlugin::new(port))
            .run();
    });

    let app = Router::new()
        .route("/", get(get_root))
        .layer(Extension(server_state));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", web_port))
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();

    app_handle.await.unwrap();

    Ok(())
}

#[axum::debug_handler]
async fn get_root(Extension(server_state): Extension<ArcMutexServerState>) -> Json<ServerState> {
    let value = server_state.lock().unwrap();
    let server_state: ServerState = value.clone();
    Json(server_state)
}

fn tokio_receiver_system(
    mut connection_resource: ResMut<ConnectionResource>,
    mut tokio_runtime_resource: ResMut<TokioRuntimeResource<TokioServerMessage>>,
) {
    if let Ok(message) = tokio_runtime_resource.receiver.try_recv() {
        match message {
            TokioServerMessage::RegisterServer(server) => connection_resource.server = Some(server),
            TokioServerMessage::PingServer(server) => connection_resource.server = Some(server),
        }
    }
}
