use anyhow::Result;
use axum::{response::IntoResponse, routing::get, Extension, Json, Router};
use bevy::{app::ScheduleRunnerPlugin, log::tracing_subscriber, prelude::*};
use bevy_quinnet::shared::ClientId;
use clap::{arg, Parser};
use engine::{models::api::servers::Server, resources::TokioRuntimeResource};
use plugins::network::{ConnectionResource, NetworkPlugin};
use std::{collections::HashMap, net::IpAddr, time::Duration};
use tokio::sync::{mpsc, oneshot};
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

enum AppMessage {
    GetConnections(oneshot::Sender<HashMap<ClientId, Uuid>>),
}

#[derive(Resource)]
struct AppState {
    connections: HashMap<ClientId, Uuid>,
    tx: mpsc::Sender<AppMessage>,
    rx: mpsc::Receiver<AppMessage>,
}

impl AppState {
    fn new(tx: mpsc::Sender<AppMessage>, rx: mpsc::Receiver<AppMessage>) -> Self {
        Self {
            connections: HashMap::new(),
            tx,
            rx,
        }
    }
}

enum TokioServerMessage {
    PingServer(Server),
    RegisterServer(Server),
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let (tx, rx) = mpsc::channel::<AppMessage>(32);

    let args = ServerArgs::parse();
    let port = args.port.clone();
    let web_port = args.web_port.clone();

    let bevy_tx = tx.clone();
    let app_handle = tokio::spawn(async move {
        App::new()
            .add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
                Duration::from_secs_f64(1.0 / 60.0),
            )))
            .insert_resource(AppState::new(bevy_tx, rx))
            .insert_resource(args)
            .insert_resource(TokioRuntimeResource::<TokioServerMessage>::new())
            .add_systems(Update, tokio_receiver_system)
            .add_plugins(NetworkPlugin::new(port))
            .add_systems(Update, app_message_system)
            .run();
    });

    let app = Router::new().route("/", get(get_root)).layer(Extension(tx));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", web_port))
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();

    app_handle.await.unwrap();

    Ok(())
}

fn app_message_system(mut state: ResMut<AppState>) {
    if let Ok(message) = state.rx.try_recv() {
        match message {
            AppMessage::GetConnections(tx) => {
                tx.send(state.connections.clone()).unwrap();
            }
        }
    }
}

#[axum::debug_handler]
async fn get_root(Extension(tx): Extension<mpsc::Sender<AppMessage>>) -> impl IntoResponse {
    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();

    tx.send(AppMessage::GetConnections(resp_tx)).await.unwrap();

    let connections = resp_rx.await.unwrap();

    Json(connections)
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
