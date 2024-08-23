use bevy::{app::ScheduleRunnerPlugin, log::tracing_subscriber, prelude::*};
use engine::{
    client::{ping_server, register_server},
    models::api::{GameServer, RegisterGameServer},
    resources::TokioRuntimeResource,
};
use std::sync::mpsc::channel;
use std::time::Duration;

#[derive(Default, Resource)]
struct ConnectionResource {
    server: Option<GameServer>,
}

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    App::new()
        .add_plugins(
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                1.0 / 60.0,
            ))),
        )
        .insert_resource(ConnectionResource::default())
        .insert_resource(TokioRuntimeResource::new())
        .add_systems(Startup, register_server_system)
        .add_systems(Update, ping_server_system)
        .run();
}

fn register_server_system(
    mut connection_resource: ResMut<ConnectionResource>,
    tokio_runtime_resource: Res<TokioRuntimeResource>,
) {
    let (tx, rx) = channel();

    tokio_runtime_resource.runtime.spawn(async move {
        let result = register_server(RegisterGameServer {
            name: "Hello world".to_string(),
        })
        .await;

        match result {
            Ok(server) => tx.send(server).unwrap(),
            Err(error) => error!(error = ?error, "Create"),
        }
    });

    loop {
        match rx.try_recv() {
            Ok(server) => {
                connection_resource.server = Some(server);
                break;
            }
            _ => {}
        }
    }
}

fn ping_server_system(
    mut connection_resource: ResMut<ConnectionResource>,
    tokio_runtime_resource: Res<TokioRuntimeResource>,
) {
    if let Some(server) = &connection_resource.server {
        let (tx, rx) = channel();

        let id = server.id.clone();

        tokio_runtime_resource.runtime.spawn(async move {
            let result = ping_server(&id).await;

            match result {
                Ok(server) => tx.send(server).unwrap(),
                Err(error) => error!(error = ?error, "Ping"),
            }
        });

        loop {
            match rx.try_recv() {
                Ok(server) => {
                    connection_resource.server = Some(server);
                    break;
                }
                _ => {}
            }
        }
    }
}
