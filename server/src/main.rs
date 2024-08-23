use axum::{response::IntoResponse, routing::get, Router};
use bevy::{app::ScheduleRunnerPlugin, log::tracing_subscriber, prelude::*};
use clap::{arg, Parser};
use engine::{
    api_client::{ping_server, register_server},
    models::api::{GameServer, RegisterGameServer},
    resources::TokioRuntimeResource,
};
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::mpsc::channel;

#[derive(Parser, Debug, Resource)]
#[command(version, about, long_about = None)]
struct ServerArgs {
    /// The name of the server
    #[arg(short, long, default_value = "My Server")]
    name: String,

    #[arg(short, long, default_value = "http://localhost:3000")]
    api_base_url: String,

    /// The port to run the server on
    #[arg(short, long, default_value = "2525")]
    port: u16,

    /// The port to run the management web server on
    #[arg(short, long, default_value = "3001")]
    web_port: u16,
}

enum ServerMessage {
    RegisterServer(GameServer),
    PingServer(GameServer),
}

#[derive(Default, Resource)]
struct ConnectionResource {
    server: Option<GameServer>,
}

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let args = ServerArgs::parse();
    let (tx, rx) = channel::<ServerMessage>(10);

    App::new()
        .add_plugins(
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                1.0 / 60.0,
            ))),
        )
        .insert_resource(args)
        .insert_resource(ConnectionResource::default())
        .insert_resource(TokioRuntimeResource::new(tx, rx))
        .add_systems(Update, tokio_receiver_system)
        .add_systems(Startup, register_server_system)
        .add_systems(Update, ping_server_system)
        .add_systems(Startup, start_webserver)
        .run();
}

fn tokio_receiver_system(
    mut connection_resource: ResMut<ConnectionResource>,
    mut tokio_runtime_resource: ResMut<TokioRuntimeResource<ServerMessage>>,
) {
    if let Ok(message) = tokio_runtime_resource.receiver.try_recv() {
        match message {
            ServerMessage::RegisterServer(server) => connection_resource.server = Some(server),
            ServerMessage::PingServer(server) => connection_resource.server = Some(server),
        }
    }
}

fn register_server_system(
    server_args: Res<ServerArgs>,
    tokio_runtime_resource: Res<TokioRuntimeResource<ServerMessage>>,
) {
    let tx = tokio_runtime_resource.sender.clone();
    let api_base_url = server_args.api_base_url.clone();
    let name = server_args.name.clone();

    tokio_runtime_resource.runtime.spawn(async move {
        let result = register_server(&api_base_url, &RegisterGameServer { name }).await;

        match result {
            Ok(server) => tx
                .send(ServerMessage::RegisterServer(server))
                .await
                .unwrap(),
            Err(error) => error!(error = ?error, "Create"),
        }
    });
}

fn ping_server_system(
    server_args: Res<ServerArgs>,
    connection_resource: Res<ConnectionResource>,
    tokio_runtime_resource: Res<TokioRuntimeResource<ServerMessage>>,
) {
    if let Some(server) = &connection_resource.server {
        let now = OffsetDateTime::now_utc();

        if now - server.last_ping >= Duration::from_secs(10) {
            let tx = tokio_runtime_resource.sender.clone();

            let api_base_url = server_args.api_base_url.clone();
            let id = server.id;

            tokio_runtime_resource.runtime.spawn(async move {
                let result = ping_server(&api_base_url, &id).await;

                match result {
                    Ok(server) => tx.send(ServerMessage::PingServer(server)).await.unwrap(),
                    Err(error) => error!(error = ?error, "Ping"),
                }
            });
        }
    }
}

#[derive(Clone)]
struct ApiState {
    pub name: String,
}

fn start_webserver(
    server_args: Res<ServerArgs>,
    tokio_runtime_resource: Res<TokioRuntimeResource<ServerMessage>>,
) {
    let web_port = server_args.web_port.clone();
    let api_state = ApiState {
        name: server_args.name.clone(),
    };

    tokio_runtime_resource.runtime.spawn(async move {
        let app = Router::new()
            .route("/", get(get_root))
            .with_state(api_state);

        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", web_port))
            .await
            .unwrap();
        axum::serve(listener, app).await
    });
}

#[axum::debug_handler]
async fn get_root(
    axum::extract::State(api_state): axum::extract::State<ApiState>,
) -> impl IntoResponse {
    format!("Server name: {}", api_state.name)
}
