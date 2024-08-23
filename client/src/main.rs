use bevy::prelude::*;
use engine::{client::list_servers, models::api::GameServer, resources::TokioRuntimeResource};
use tokio::sync::mpsc::channel;

enum ClientMessage {
    LoadServers(Vec<GameServer>),
}

#[derive(Default, Resource)]
struct ServerBrowser {
    servers: Option<Vec<GameServer>>,
}

fn main() {
    let (tx, rx) = channel::<ClientMessage>(10);

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(TokioRuntimeResource::new(tx, rx))
        .insert_resource(ServerBrowser::default())
        .add_systems(Update, tokio_receiver_system)
        .add_systems(Startup, load_servers)
        .add_systems(Update, servers)
        .run();
}

fn tokio_receiver_system(
    mut server_browser_resource: ResMut<ServerBrowser>,
    mut tokio_runtime_resource: ResMut<TokioRuntimeResource<ClientMessage>>,
) {
    if let Ok(message) = tokio_runtime_resource.receiver.try_recv() { match message {
        ClientMessage::LoadServers(server) => server_browser_resource.servers = Some(server),
    } }
}

fn load_servers(tokio: Res<TokioRuntimeResource<ClientMessage>>) {
    let tx = tokio.sender.clone();

    tokio.runtime.spawn(async move {
        match list_servers().await {
            Ok(servers) => tx.send(ClientMessage::LoadServers(servers)).await.unwrap(),
            Err(error) => error!(error = ?error, "Load servers"),
        }
    });
}

fn servers(server_browser_resource: Res<ServerBrowser>) {
    println!("{:?}", server_browser_resource.servers);
}
