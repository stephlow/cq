use crate::AppState;
use bevy::prelude::*;
use bevy_quinnet::{
    server::{
        certificate::CertificateRetrievalMode, QuinnetServer, QuinnetServerPlugin,
        ServerEndpointConfiguration,
    },
    shared::channels::ChannelsConfiguration,
};
use engine::{
    components::player::Player,
    models::network::{ClientMessage, ServerMessage},
};
use std::net::{IpAddr, Ipv4Addr};

#[derive(Component)]
struct PlayerPosition(Vec3);

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

fn handle_client_messages(
    mut players: Query<(Entity, &Player, &mut PlayerPosition)>,
    mut commands: Commands,
    mut server: ResMut<QuinnetServer>,
    mut app_state: ResMut<AppState>,
) {
    let endpoint = server.endpoint_mut();
    for client_id in endpoint.clients() {
        while let Some((_channel_id, message)) =
            endpoint.try_receive_message_from::<ClientMessage>(client_id)
        {
            match message {
                ClientMessage::Join { user_id } => {
                    commands.spawn((
                        Player { client_id, user_id },
                        PlayerPosition(Vec3::new(0., 0., 0.)),
                    ));

                    endpoint
                        .broadcast_message(ServerMessage::ClientConnected { client_id, user_id })
                        .unwrap();

                    for (_, player, _) in players.into_iter() {
                        endpoint
                            .send_message(
                                client_id,
                                ServerMessage::ClientConnected {
                                    client_id: player.client_id,
                                    user_id: player.user_id,
                                },
                            )
                            .unwrap();
                    }
                }
                ClientMessage::Disconnect {} => {
                    if let Some((entity, _, _)) = players
                        .iter()
                        .find(|(_, player, _)| player.client_id == client_id)
                    {
                        commands.entity(entity).despawn();
                        endpoint
                            .broadcast_message(ServerMessage::ClientDisconnected { client_id })
                            .unwrap();

                        endpoint.disconnect_client(client_id).unwrap();
                    }
                }
                ClientMessage::ChatMessage { message } => {
                    endpoint
                        .broadcast_message(ServerMessage::ChatMessage { client_id, message })
                        .unwrap();
                }
                ClientMessage::UpdatePosition { position } => {
                    if let Some((_, _, mut player_position)) = players
                        .iter_mut()
                        .find(|(_, player, _)| player.client_id == client_id)
                    {
                        player_position.0 = position;

                        endpoint
                            .broadcast_message(ServerMessage::UpdatePosition {
                                client_id,
                                position,
                            })
                            .unwrap();
                    }
                }
            }
        }
    }
}
