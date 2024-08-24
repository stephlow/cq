use bevy::{app::ScheduleRunnerPlugin, log::tracing_subscriber, prelude::*};
use clap::{arg, Parser};
use engine::{models::api::servers::Server, resources::TokioRuntimeResource};
use plugins::{
    database::{DatabasePlugin, SqliteServer},
    network::{ConnectionResource, NetworkPlugin},
    webserver::WebServerPlugin,
};
use sqlx::{Pool, Sqlite};
use std::{net::IpAddr, time::Duration};

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
    InitializePool(Pool<Sqlite>),
    PingServer(Server),
    RegisterServer(Server),
}

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let args = ServerArgs::parse();
    let port = args.port.clone();
    let web_port = args.web_port.clone();

    App::new()
        .add_plugins(
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                1.0 / 60.0,
            ))),
        )
        .insert_resource(args)
        .insert_resource(TokioRuntimeResource::<TokioServerMessage>::new())
        .add_systems(Update, tokio_receiver_system)
        .add_plugins(DatabasePlugin)
        .add_plugins(NetworkPlugin::new(port))
        .add_plugins(WebServerPlugin::new(web_port))
        .run();
}

fn tokio_receiver_system(
    mut connection_resource: ResMut<ConnectionResource>,
    mut tokio_runtime_resource: ResMut<TokioRuntimeResource<TokioServerMessage>>,
    mut sqlite_server: ResMut<SqliteServer>,
) {
    if let Ok(message) = tokio_runtime_resource.receiver.try_recv() {
        match message {
            TokioServerMessage::RegisterServer(server) => connection_resource.server = Some(server),
            TokioServerMessage::PingServer(server) => connection_resource.server = Some(server),
            TokioServerMessage::InitializePool(pool) => sqlite_server.pool = Some(pool),
        }
    }
}
