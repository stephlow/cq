use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use engine::{client::list_servers, models::api::GameServer, resources::TokioRuntimeResource};
use tokio::sync::mpsc::channel;
use uuid::Uuid;

#[derive(Event)]
enum ClientEvent {
    Connect(Uuid),
}

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
        .add_plugins(EguiPlugin)
        .add_event::<ClientEvent>()
        .insert_resource(TokioRuntimeResource::new(tx, rx))
        .insert_resource(ServerBrowser::default())
        .add_systems(Update, tokio_receiver_system)
        .add_systems(Startup, load_servers)
        .add_systems(Update, ui_system)
        .add_systems(Update, event_system)
        .run();
}

fn tokio_receiver_system(
    mut server_browser_resource: ResMut<ServerBrowser>,
    mut tokio_runtime_resource: ResMut<TokioRuntimeResource<ClientMessage>>,
) {
    if let Ok(message) = tokio_runtime_resource.receiver.try_recv() {
        match message {
            ClientMessage::LoadServers(server) => server_browser_resource.servers = Some(server),
        }
    }
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

fn ui_system(
    mut contexts: EguiContexts,
    server_browser_resource: Res<ServerBrowser>,
    mut client_event_writer: EventWriter<ClientEvent>,
) {
    egui::Window::new("Servers").show(contexts.ctx_mut(), |ui| {
        if let Some(servers) = &server_browser_resource.servers {
            for server in servers.iter() {
                ui.label(format!("Server name: {}", server.name));
                ui.label(server.addr.to_string());
                if ui.button("Connect").clicked() {
                    client_event_writer.send(ClientEvent::Connect(server.id));
                }
            }
        } else {
            ui.label("No servers");
        }
    });
}

fn event_system(mut client_event_reader: EventReader<ClientEvent>) {
    for event in client_event_reader.read() {
        match event {
            ClientEvent::Connect(_id) => todo!(),
        }
    }
}
