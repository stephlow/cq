use anyhow::Result;
use bevy::app::App;
use bevy::prelude::PluginGroup;
use bevy::MinimalPlugins;
use bevy::{
    app::{ScheduleRunnerPlugin, Update},
    log::tracing_subscriber,
    math::Vec3,
};
use bevy_ecs::prelude::*;
use bevy_quinnet::server::QuinnetServer;
use clap::{arg, Parser};
use engine::models::network::ServerMessage;
use engine::{
    api_client::{ping_server, register_server},
    components::player::{Player, PlayerPosition},
    plugins::movement::MovementPlugin,
};
use futures::future::join_all;
use models::api::servers::Server;
use plugins::network::NetworkPlugin;
use std::{net::IpAddr, time::Duration};
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
    GetPlayers(oneshot::Sender<Vec<(Uuid, Vec3)>>),
    GetServer(oneshot::Sender<Option<Server>>),
    KickPlayer(Uuid),
    SetServer(Server),
}

#[derive(Resource)]
struct AppState {
    server: Option<Server>,
    tx: mpsc::Sender<AppMessage>,
    rx: mpsc::Receiver<AppMessage>,
}

impl AppState {
    fn new(tx: mpsc::Sender<AppMessage>, rx: mpsc::Receiver<AppMessage>) -> Self {
        Self {
            server: None,
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
            .add_plugins(MovementPlugin)
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
            &models::api::servers::RegisterServer { addr, port, name },
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

fn app_message_system(
    mut commands: Commands,
    players: Query<(Entity, &Player, &PlayerPosition)>,
    mut state: ResMut<AppState>,
    mut server: ResMut<QuinnetServer>,
) {
    if let Ok(message) = state.rx.try_recv() {
        match message {
            AppMessage::SetServer(server) => {
                state.server = Some(server);
            }
            AppMessage::GetServer(tx) => {
                tx.send(state.server.clone()).unwrap();
            }
            AppMessage::GetPlayers(tx) => {
                let ids = players
                    .into_iter()
                    .map(|(_, player, position)| (player.user_id, position.0))
                    .collect();

                tx.send(ids).unwrap();
            }
            AppMessage::KickPlayer(id) => {
                if let Some((entity, player, _)) =
                    players.iter().find(|(_, player, _)| player.user_id == id)
                {
                    commands.entity(entity).despawn();
                    let endpoint = server.endpoint_mut();
                    endpoint
                        .broadcast_message(ServerMessage::ClientDisconnected {
                            client_id: player.client_id,
                        })
                        .unwrap();
                }
            }
        }
    }
}
