use anyhow::Result;
use bevy::{app::ScheduleRunnerPlugin, log::tracing_subscriber, prelude::*};
use bevy_quinnet::shared::ClientId;
use clap::{arg, Parser};
use engine::{
    api_client::{ping_server, register_server},
    models::api::servers::Server,
};
use futures::future::join_all;
use plugins::network::NetworkPlugin;
use std::{collections::HashMap, net::IpAddr, time::Duration};
use time::OffsetDateTime;
use tokio::{
    sync::{mpsc, oneshot},
    time::sleep,
};
use uuid::Uuid;
use webserver::create_router;

mod plugins;
mod webserver;

#[derive(Parser, Debug)]
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
    SetServer(Server),
    GetServer(oneshot::Sender<Option<Server>>),
    GetConnections(oneshot::Sender<HashMap<ClientId, Uuid>>),
}

#[derive(Resource)]
struct AppState {
    server: Option<Server>,
    connections: HashMap<ClientId, Uuid>,
    tx: mpsc::Sender<AppMessage>,
    rx: mpsc::Receiver<AppMessage>,
}

impl AppState {
    fn new(tx: mpsc::Sender<AppMessage>, rx: mpsc::Receiver<AppMessage>) -> Self {
        Self {
            server: None,
            connections: HashMap::new(),
            tx,
            rx,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let (tx, rx) = mpsc::channel::<AppMessage>(32);

    let args = ServerArgs::parse();

    let port = args.port;
    let web_port = args.web_port;

    let bevy_tx = tx.clone();

    let bevy_handle = tokio::spawn(async move {
        App::new()
            .add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
                Duration::from_secs_f64(1.0 / 60.0),
            )))
            .insert_resource(AppState::new(bevy_tx, rx))
            .add_plugins(NetworkPlugin::new(port))
            .add_systems(Update, app_message_system)
            .run();
    });

    let api_base_url = args.api_base_url.clone();
    let addr = args.addr;
    let port = args.port;
    let name = args.name.clone();

    let api_tx = tx.clone();
    let api_handle = tokio::spawn(async move {
        let server = register_server(
            &api_base_url,
            &engine::models::api::servers::RegisterServer { addr, port, name },
        )
        .await
        .unwrap();

        let id = server.id;
        let mut last_ping = OffsetDateTime::now_utc();

        api_tx.send(AppMessage::SetServer(server)).await.unwrap();

        loop {
            let now = OffsetDateTime::now_utc();

            if now - last_ping >= Duration::from_secs(30) {
                let server = ping_server(&api_base_url, &id).await.unwrap();
                api_tx.send(AppMessage::SetServer(server)).await.unwrap();
            }

            last_ping = OffsetDateTime::now_utc();
            sleep(Duration::from_secs(30)).await;
        }
    });

    let webserver_tx = tx.clone();
    let webserver_handle = tokio::spawn(async move {
        let app = create_router(webserver_tx);

        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", web_port))
            .await
            .unwrap();

        axum::serve(listener, app).await.unwrap();
    });

    join_all([api_handle, bevy_handle, webserver_handle]).await;

    Ok(())
}

fn app_message_system(mut state: ResMut<AppState>) {
    if let Ok(message) = state.rx.try_recv() {
        match message {
            AppMessage::SetServer(server) => {
                state.server = Some(server);
            }
            AppMessage::GetConnections(tx) => {
                tx.send(state.connections.clone()).unwrap();
            }
            AppMessage::GetServer(tx) => {
                tx.send(state.server.clone()).unwrap();
            }
        }
    }
}
