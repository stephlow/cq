use bevy::prelude::*;
use bevy_egui::{
    egui::{self},
    EguiContexts, EguiPlugin,
};
use bevy_quinnet::{
    client::{
        certificate::CertificateVerificationMode,
        connection::{ClientEndpointConfiguration, ConnectionEvent},
        QuinnetClient, QuinnetClientPlugin,
    },
    shared::channels::ChannelsConfiguration,
};
use clap::Parser;
use engine::{
    api_client::list_servers,
    models::api::GameServer,
    network::{ClientMessage, ServerMessage},
    resources::TokioRuntimeResource,
};
use std::net::{IpAddr, Ipv4Addr};
use tokio::sync::mpsc::channel;
use uuid::Uuid;

#[derive(Parser, Debug, Resource)]
#[command(version, about, long_about = None)]
struct ClientArgs {
    #[arg(short, long, default_value = "http://localhost:3000")]
    api_base_url: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
pub enum ConnectionState {
    Connected,
    #[default]
    Disconnected,
}

#[derive(Event)]
enum ClientEvent {
    Connect(Uuid),
    Disconnect,
}

enum TokioClientMessage {
    LoadServers(Vec<GameServer>),
}

#[derive(Default, Resource)]
struct ServerBrowser {
    servers: Option<Vec<GameServer>>,
}

fn main() {
    let args = ClientArgs::parse();
    let (tx, rx) = channel::<TokioClientMessage>(10);

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        .add_plugins(QuinnetClientPlugin::default())
        .init_state::<ConnectionState>()
        .add_systems(Update, connection_event_handler)
        // .add_systems(Startup, start_connection)
        .add_systems(
            Update,
            handle_server_messages.run_if(in_state(ConnectionState::Connected)),
        )
        .add_event::<ClientEvent>()
        .insert_resource(args)
        .insert_resource(TokioRuntimeResource::new(tx, rx))
        .insert_resource(ServerBrowser::default())
        .add_systems(Update, tokio_receiver_system)
        .add_systems(Startup, load_servers)
        .add_systems(
            Update,
            server_ui_system.run_if(in_state(ConnectionState::Connected)),
        )
        .add_systems(
            Update,
            server_browser_ui_system.run_if(in_state(ConnectionState::Disconnected)),
        )
        .add_systems(Update, event_system)
        .add_systems(Last, handle_disconnect)
        .run();
}

fn connection_event_handler(
    mut connection_event_reader: EventReader<ConnectionEvent>,
    mut next_connection_state: ResMut<NextState<ConnectionState>>,
    client: Res<QuinnetClient>,
) {
    for _ in connection_event_reader.read() {
        next_connection_state.set(ConnectionState::Connected);
        client
            .connection()
            .send_message(ClientMessage::Join {
                username: "Henk".to_string(),
            })
            .unwrap();
    }
}

// fn start_connection(mut client: ResMut<QuinnetClient>) {
//     client
//         .open_connection(
//             ClientEndpointConfiguration::from_ips(
//                 IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
//                 6000,
//                 IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
//                 0,
//             ),
//             CertificateVerificationMode::SkipVerification,
//             ChannelsConfiguration::default(),
//         )
//         .unwrap();
//
//     // When trully connected, you will receive a ConnectionEvent
// }

fn handle_server_messages(
    mut client: ResMut<QuinnetClient>,
    /*...*/
) {
    while let Ok(Some((client_id, message))) =
        client.connection_mut().receive_message::<ServerMessage>()
    {
        println!("ServerMessage: {:?}", message);
        match message {
            // Match on your own message types ...
            ServerMessage::ClientConnected {
                client_id,
                username,
            } => { /*...*/ }
            ServerMessage::ClientDisconnected { client_id } => { /*...*/ }
            ServerMessage::ChatMessage { client_id, message } => { /*...*/ }
        }
    }
}

fn tokio_receiver_system(
    mut server_browser_resource: ResMut<ServerBrowser>,
    mut tokio_runtime_resource: ResMut<TokioRuntimeResource<TokioClientMessage>>,
) {
    if let Ok(message) = tokio_runtime_resource.receiver.try_recv() {
        match message {
            TokioClientMessage::LoadServers(server) => {
                server_browser_resource.servers = Some(server)
            }
        }
    }
}

fn load_servers(
    client_args: Res<ClientArgs>,
    tokio: Res<TokioRuntimeResource<TokioClientMessage>>,
) {
    let api_base_url = client_args.api_base_url.clone();
    let tx = tokio.sender.clone();

    tokio.runtime.spawn(async move {
        match list_servers(&api_base_url).await {
            Ok(servers) => tx
                .send(TokioClientMessage::LoadServers(servers))
                .await
                .unwrap(),
            Err(error) => error!(error = ?error, "Load servers"),
        }
    });
}

fn server_ui_system(mut contexts: EguiContexts, mut client_event_writer: EventWriter<ClientEvent>) {
    egui::Window::new("Server").show(contexts.ctx_mut(), |ui| {
        ui.label("Connected");
        if ui.button("Disconnect").clicked() {
            client_event_writer.send(ClientEvent::Disconnect);
        }
    });
}

fn server_browser_ui_system(
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

fn event_system(
    server_browser_resource: Res<ServerBrowser>,
    mut client_event_reader: EventReader<ClientEvent>,
    mut client: ResMut<QuinnetClient>,
    mut next_connection_state: ResMut<NextState<ConnectionState>>,
) {
    for event in client_event_reader.read() {
        match event {
            ClientEvent::Connect(id) => {
                if let Some(servers) = &server_browser_resource.servers {
                    if let Some(server) = servers.iter().find(|server| &server.id == id) {
                        // TODO: Match server addr
                        client
                            .open_connection(
                                ClientEndpointConfiguration::from_ips(
                                    IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                                    6000,
                                    IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                                    0,
                                ),
                                CertificateVerificationMode::SkipVerification,
                                ChannelsConfiguration::default(),
                            )
                            .unwrap();
                    }
                }
            }
            ClientEvent::Disconnect => {
                client
                    .connection()
                    .send_message(ClientMessage::Disconnect)
                    .unwrap();

                next_connection_state.set(ConnectionState::Disconnected);
            }
        }
    }
}

fn handle_disconnect(client: Res<QuinnetClient>, mut app_exit_event_reader: EventReader<AppExit>) {
    for _ in app_exit_event_reader.read() {
        client
            .connection()
            .send_message(engine::network::ClientMessage::Disconnect)
            .unwrap();
    }
}
