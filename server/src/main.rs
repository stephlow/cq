use bevy::{app::ScheduleRunnerPlugin, log::tracing_subscriber, prelude::*};
use engine::{
    client::{ping_server, register_server},
    models::api::{GameServer, RegisterGameServer},
    resources::TokioRuntimeResource,
};
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::mpsc::channel;

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

    let (tx, rx) = channel::<ServerMessage>(10);

    App::new()
        .add_plugins(
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                1.0 / 60.0,
            ))),
        )
        .insert_resource(ConnectionResource::default())
        .insert_resource(TokioRuntimeResource::new(tx, rx))
        .add_systems(Update, tokio_receiver_system)
        .add_systems(Startup, register_server_system)
        .add_systems(Update, ping_server_system)
        .run();
}

fn tokio_receiver_system(
    mut connection_resource: ResMut<ConnectionResource>,
    mut tokio_runtime_resource: ResMut<TokioRuntimeResource<ServerMessage>>,
) {
    match tokio_runtime_resource.receiver.try_recv() {
        Ok(message) => match message {
            ServerMessage::RegisterServer(server) => connection_resource.server = Some(server),
            ServerMessage::PingServer(server) => connection_resource.server = Some(server),
        },
        Err(_) => {}
    }
}

fn register_server_system(tokio_runtime_resource: Res<TokioRuntimeResource<ServerMessage>>) {
    let tx = tokio_runtime_resource.sender.clone();

    tokio_runtime_resource.runtime.spawn(async move {
        let result = register_server(RegisterGameServer {
            name: "Hello world".to_string(),
        })
        .await;

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
    connection_resource: Res<ConnectionResource>,
    tokio_runtime_resource: Res<TokioRuntimeResource<ServerMessage>>,
) {
    if let Some(server) = &connection_resource.server {
        let now = OffsetDateTime::now_utc();

        if now - server.last_ping >= Duration::from_secs(10) {
            let tx = tokio_runtime_resource.sender.clone();

            let id = server.id.clone();

            tokio_runtime_resource.runtime.spawn(async move {
                let result = ping_server(&id).await;

                match result {
                    Ok(server) => tx.send(ServerMessage::PingServer(server)).await.unwrap(),
                    Err(error) => error!(error = ?error, "Ping"),
                }
            });
        }
    }
}
