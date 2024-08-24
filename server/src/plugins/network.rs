use crate::AppState;
use bevy::prelude::*;
use bevy_quinnet::{
    server::{
        certificate::CertificateRetrievalMode, QuinnetServer, QuinnetServerPlugin,
        ServerEndpointConfiguration,
    },
    shared::channels::ChannelsConfiguration,
};
use engine::network::{ClientMessage, ServerMessage};
use std::net::{IpAddr, Ipv4Addr};

#[derive(Resource)]
pub struct ServerConfig {
    port: u16,
}

pub struct NetworkPlugin {
    port: u16,
}

impl NetworkPlugin {
    pub fn new(port: u16) -> Self {
        Self { port }
    }
}

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ServerConfig { port: self.port })
            .add_plugins(QuinnetServerPlugin::default())
            .add_systems(Startup, start_listening)
            .add_systems(Update, handle_client_messages);
    }
}

fn start_listening(server_config: Res<ServerConfig>, mut server: ResMut<QuinnetServer>) {
    server
        .start_endpoint(
            ServerEndpointConfiguration::from_ip(
                IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                server_config.port,
            ),
            CertificateRetrievalMode::GenerateSelfSigned {
                server_hostname: "127.0.0.1".to_string(),
            },
            ChannelsConfiguration::default(),
        )
        .unwrap();
}

fn handle_client_messages(mut server: ResMut<QuinnetServer>, mut app_state: ResMut<AppState>) {
    let endpoint = server.endpoint_mut();
    for client_id in endpoint.clients() {
        while let Some((_channel_id, message)) =
            endpoint.try_receive_message_from::<ClientMessage>(client_id)
        {
            match message {
                ClientMessage::Join { user_id } => {
                    app_state.connections.insert(client_id, user_id);

                    endpoint
                        .broadcast_message(ServerMessage::ClientConnected { client_id, user_id })
                        .unwrap();

                    for (user_client_id, user_id) in app_state.connections.iter() {
                        endpoint
                            .send_message(
                                client_id,
                                ServerMessage::ClientConnected {
                                    client_id: *user_client_id,
                                    user_id: *user_id,
                                },
                            )
                            .unwrap();
                    }
                }
                ClientMessage::Disconnect {} => {
                    app_state.connections.remove(&client_id);

                    endpoint
                        .broadcast_message(ServerMessage::ClientDisconnected { client_id })
                        .unwrap();

                    endpoint.disconnect_client(client_id).unwrap();
                }
                ClientMessage::ChatMessage { message } => {
                    endpoint
                        .broadcast_message(ServerMessage::ChatMessage { client_id, message })
                        .unwrap();
                }
            }
        }
    }
}
