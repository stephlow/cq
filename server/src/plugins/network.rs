use bevy::{
    app::{App, Plugin, Startup, Update},
    time::{Time, Timer, TimerMode},
};
use bevy_ecs::prelude::*;
use bevy_quinnet::{
    server::{
        certificate::CertificateRetrievalMode, QuinnetServer, QuinnetServerPlugin,
        ServerEndpointConfiguration,
    },
    shared::channels::ChannelsConfiguration,
};
use engine::{
    components::{
        movement::Movement,
        player::{Player, PlayerPosition},
    },
    models::network::{ClientMessage, ServerMessage},
};
use std::{
    net::{IpAddr, Ipv4Addr},
    time::Duration,
};

#[derive(Resource)]
pub struct ServerConfig {
    port: u16,
    broadcast_timer: Timer,
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
        app.insert_resource(ServerConfig {
            port: self.port,
            broadcast_timer: Timer::new(Duration::from_millis(10), TimerMode::Repeating),
        })
        .add_plugins(QuinnetServerPlugin::default())
        .add_systems(Startup, start_listening)
        .add_systems(Update, handle_client_messages)
        .add_systems(Update, broadcast_positions);
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
    mut players: Query<(Entity, &Player, &mut PlayerPosition, &mut Movement)>,
    mut commands: Commands,
    mut server: ResMut<QuinnetServer>,
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
                        PlayerPosition::default(),
                        Movement::default(),
                    ));

                    endpoint
                        .broadcast_message(ServerMessage::ClientConnected { client_id, user_id })
                        .unwrap();

                    for (_, player, _, _) in players.into_iter() {
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
                    if let Some((entity, _, _, _)) = players
                        .iter()
                        .find(|(_, player, _, _)| player.client_id == client_id)
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
                    if let Some((_, _, mut player_position, _)) = players
                        .iter_mut()
                        .find(|(_, player, _, _)| player.client_id == client_id)
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
                ClientMessage::SendModifier(modifier) => {
                    if let Some((_, _, _, mut movement)) = players
                        .iter_mut()
                        .find(|(_, player, _, _)| player.client_id == client_id)
                    {
                        movement.modify(modifier.clone());

                        endpoint
                            .broadcast_message(ServerMessage::SendModifier {
                                client_id,
                                modifier,
                            })
                            .unwrap();
                    }
                }
            }
        }
    }
}

fn broadcast_positions(
    players: Query<(&Player, &PlayerPosition)>,
    mut server: ResMut<QuinnetServer>,
    time: Res<Time>,
    mut config: ResMut<ServerConfig>,
) {
    config.broadcast_timer.tick(time.delta());
    if config.broadcast_timer.finished() {
        let endpoint = server.endpoint_mut();
        for (player, position) in players.iter() {
            endpoint
                .broadcast_message(ServerMessage::UpdatePosition {
                    client_id: player.client_id,
                    position: position.0,
                })
                .unwrap()
        }
    }
}
