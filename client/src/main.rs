use bevy::prelude::*;
use bevy_quinnet::{
    client::{
        certificate::CertificateVerificationMode,
        connection::{ClientEndpointConfiguration, ConnectionEvent},
        QuinnetClient,
    },
    shared::{channels::ChannelsConfiguration, ClientId},
};
use clap::Parser;
use engine::{
    api_client::list_servers,
    models::{
        api::{servers::Server, users::User},
        network::{ClientMessage, ServerMessage},
    },
    resources::TokioRuntimeResource,
};
use plugins::{network::NetworkPlugin, ui::UiPlugin};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
};
use uuid::Uuid;

mod plugins;

#[derive(Parser, Debug, Resource)]
#[command(version, about, long_about = None)]
struct ClientArgs {
    #[arg(short, long, default_value = "http://localhost:3000")]
    api_base_url: String,
}

#[derive(Default, Resource)]
struct AuthInfo {
    token: Option<String>,
    user: Option<User>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
pub enum AuthState {
    Authenticated,
    #[default]
    Anonymous,
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
    Authenticated { token: String, user: User },
    LoadServers(Vec<Server>),
}

#[derive(Default, Resource)]
struct ServerBrowser {
    servers: Option<Vec<Server>>,
}

#[derive(Default, Resource)]
struct ServerInfo {
    id: Option<Uuid>,
    connected: HashMap<ClientId, Uuid>,
    messages: Vec<(ClientId, String)>,
}

fn main() {
    let args = ClientArgs::parse();

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(UiPlugin)
        .add_plugins(NetworkPlugin)
        .init_state::<AuthState>()
        .init_state::<ConnectionState>()
        .add_systems(Update, connection_event_handler)
        .add_systems(
            Update,
            handle_server_messages
                .run_if(in_state(AuthState::Authenticated))
                .run_if(in_state(ConnectionState::Connected)),
        )
        .add_event::<ClientEvent>()
        .insert_resource(args)
        .insert_resource(AuthInfo::default())
        .insert_resource(TokioRuntimeResource::<TokioClientMessage>::new())
        .insert_resource(ServerBrowser::default())
        .insert_resource(ServerInfo::default())
        .add_systems(Update, tokio_receiver_system)
        .add_systems(Startup, load_servers)
        .add_systems(Update, event_system)
        .add_systems(Last, handle_disconnect)
        .run();
}

fn connection_event_handler(
    mut connection_event_reader: EventReader<ConnectionEvent>,
    mut next_connection_state: ResMut<NextState<ConnectionState>>,
    client: Res<QuinnetClient>,
    auth_info: Res<AuthInfo>,
) {
    for _ in connection_event_reader.read() {
        if let Some(user) = &auth_info.user {
            client
                .connection()
                .send_message(ClientMessage::Join { user_id: user.id })
                .unwrap();
        }
        next_connection_state.set(ConnectionState::Connected);
    }
}

fn handle_server_messages(mut client: ResMut<QuinnetClient>, mut server_info: ResMut<ServerInfo>) {
    while let Ok(Some((_channel_id, message))) =
        client.connection_mut().receive_message::<ServerMessage>()
    {
        match message {
            ServerMessage::ClientConnected { client_id, user_id } => {
                server_info.connected.insert(client_id, user_id);
            }
            ServerMessage::ClientDisconnected { client_id } => {
                server_info.connected.remove(&client_id);
            }
            ServerMessage::ChatMessage { client_id, message } => {
                server_info.messages.push((client_id, message));
            }
        }
    }
}

fn tokio_receiver_system(
    mut server_browser_resource: ResMut<ServerBrowser>,
    mut tokio_runtime_resource: ResMut<TokioRuntimeResource<TokioClientMessage>>,
    mut next_auth_state: ResMut<NextState<AuthState>>,
    mut auth_info: ResMut<AuthInfo>,
) {
    if let Ok(message) = tokio_runtime_resource.receiver.try_recv() {
        match message {
            TokioClientMessage::Authenticated { token, user } => {
                auth_info.token = Some(token);
                auth_info.user = Some(user);
                next_auth_state.set(AuthState::Authenticated);
            }
            TokioClientMessage::LoadServers(server) => {
                server_browser_resource.servers = Some(server);
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

fn event_system(
    server_browser_resource: Res<ServerBrowser>,
    mut client_event_reader: EventReader<ClientEvent>,
    mut client: ResMut<QuinnetClient>,
    mut next_connection_state: ResMut<NextState<ConnectionState>>,
    mut server_info: ResMut<ServerInfo>,
) {
    for event in client_event_reader.read() {
        match event {
            ClientEvent::Connect(id) => {
                if let Some(servers) = &server_browser_resource.servers {
                    if let Some(server) = servers.iter().find(|server| &server.id == id) {
                        client
                            .open_connection(
                                ClientEndpointConfiguration::from_ips(
                                    server.addr,
                                    server.port,
                                    IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                                    0,
                                ),
                                CertificateVerificationMode::SkipVerification,
                                ChannelsConfiguration::default(),
                            )
                            .unwrap();
                        server_info.id = Some(*id);
                    }
                }
            }
            ClientEvent::Disconnect => {
                if let Some(client_id) = client.connection().client_id() {
                    server_info.connected.remove(&client_id);
                }

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
            .send_message(engine::models::network::ClientMessage::Disconnect)
            .unwrap();
    }
}
