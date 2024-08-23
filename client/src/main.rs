use std::sync::mpsc::channel;

use bevy::prelude::*;
use engine::{client::list_servers, models::api::GameServer, resources::TokioRuntimeResource};

#[derive(Default, Resource)]
struct ServerBrowser {
    servers: Vec<GameServer>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(TokioRuntimeResource::new())
        .insert_resource(ServerBrowser::default())
        .add_systems(Startup, load_servers)
        .add_systems(Update, servers)
        .run();
}

fn load_servers(
    mut server_browser_resource: ResMut<ServerBrowser>,
    tokio: Res<TokioRuntimeResource>,
) {
    let (tx, rx) = channel();

    tokio.runtime.spawn(async move {
        match list_servers().await {
            Ok(servers) => tx.send(servers).unwrap(),
            Err(error) => error!(error = ?error, "Load servers"),
        }
    });

    loop {
        match rx.try_recv() {
            Ok(servers) => {
                server_browser_resource.servers = servers;
                break;
            }
            Err(_) => {}
        }
    }
}

fn servers(server_browser_resource: Res<ServerBrowser>) {
    println!("{:?}", server_browser_resource.servers);
}
