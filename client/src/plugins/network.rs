use bevy::prelude::*;
use bevy_quinnet::{
    client::{
        certificate::CertificateVerificationMode, connection::ClientEndpointConfiguration,
        QuinnetClient, QuinnetClientPlugin,
    },
    shared::{channels::ChannelsConfiguration, ClientId},
};
use engine::models::network::{ClientMessage, ServerMessage};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
};
use uuid::Uuid;

use crate::{AuthState, ClientEvent, ConnectionState};

use super::{
    api::{ApiEvent, ApiResource},
    render::RenderEvent,
};

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(QuinnetClientPlugin::default())
            .init_resource::<ServerInfo>()
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
pub struct ServerInfo {
    pub id: Option<Uuid>,
    pub connected: HashMap<ClientId, Uuid>,
    pub messages: Vec<(ClientId, String)>,
}

fn handle_server_messages(
    mut api_events: EventWriter<ApiEvent>,
    mut client: ResMut<QuinnetClient>,
    mut server_info: ResMut<ServerInfo>,
    mut render_events: EventWriter<RenderEvent>,
) {
    while let Ok(Some((_channel_id, message))) =
        client.connection_mut().receive_message::<ServerMessage>()
    {
        match message {
            ServerMessage::ClientConnected { client_id, user_id } => {
                server_info.connected.insert(client_id, user_id);
                api_events.send(ApiEvent::LoadUser(user_id));
                render_events.send(RenderEvent::Spawn { client_id, user_id });
            }
            ServerMessage::ClientDisconnected { client_id } => {
                server_info.connected.remove(&client_id);
                render_events.send(RenderEvent::Despawn(client_id));
            }
            ServerMessage::ChatMessage { client_id, message } => {
                server_info.messages.push((client_id, message));
            }
            ServerMessage::UpdatePosition {
                client_id,
                position,
            } => {
                render_events.send(RenderEvent::UpdatePosition {
                    client_id,
                    position,
                });
            }
            ServerMessage::SendModifier {
                client_id,
                key_code,
            } => {
                render_events.send(RenderEvent::UpdateMovement {
                    client_id,
                    modifier: key_code,
                });
            }
        }
    }
}

fn event_system(
    api: Res<ApiResource>,
    mut client_event_reader: EventReader<ClientEvent>,
    mut client: ResMut<QuinnetClient>,
    mut next_connection_state: ResMut<NextState<ConnectionState>>,
    mut server_info: ResMut<ServerInfo>,
) {
    for event in client_event_reader.read() {
        match event {
            ClientEvent::Connect(id) => {
                if let Some(servers) = &api.servers.data {
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
