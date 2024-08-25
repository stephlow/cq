use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
};

use bevy::prelude::*;
use bevy_quinnet::{
    client::{
        certificate::CertificateVerificationMode, connection::ClientEndpointConfiguration,
        QuinnetClient, QuinnetClientPlugin,
    },
    shared::{channels::ChannelsConfiguration, ClientId},
};
use engine::models::{
    api::servers::Server,
    network::{ClientMessage, ServerMessage},
};
use uuid::Uuid;

use crate::{AuthState, ClientEvent, ConnectionState};

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(QuinnetClientPlugin::default())
            .insert_resource(ServerBrowser::default())
            .insert_resource(ServerInfo::default())
            .add_systems(Update, event_system)
            .add_systems(Last, handle_disconnect)
            .add_systems(
                Update,
                handle_server_messages
                    .run_if(in_state(AuthState::Authenticated))
                    .run_if(in_state(ConnectionState::Connected)),
            );
    }
}

#[derive(Default, Resource)]
pub struct ServerBrowser {
    pub servers: Option<Vec<Server>>,
}

#[derive(Default, Resource)]
pub struct ServerInfo {
    pub id: Option<Uuid>,
    pub connected: HashMap<ClientId, Uuid>,
    pub messages: Vec<(ClientId, String)>,
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
