use crate::{ServerArgs, ServerStateResource, TokioServerMessage};
use bevy::prelude::*;
use bevy_quinnet::{
    server::{
        certificate::CertificateRetrievalMode, QuinnetServer, QuinnetServerPlugin,
        ServerEndpointConfiguration,
    },
    shared::channels::ChannelsConfiguration,
};
use engine::{
    api_client::{ping_server, register_server},
    models::api::servers::{RegisterServer, Server},
    network::{ClientMessage, ServerMessage},
    resources::TokioRuntimeResource,
};
use std::net::{IpAddr, Ipv4Addr};
use time::{Duration, OffsetDateTime};

#[derive(Resource)]
pub struct ConnectionResource {
    pub server: Option<Server>,
    pub last_ping_attempt: OffsetDateTime,
}

impl Default for ConnectionResource {
    fn default() -> Self {
        Self {
            server: None,
            last_ping_attempt: OffsetDateTime::now_utc(),
        }
    }
}

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
            .insert_resource(ConnectionResource::default())
            .add_plugins(QuinnetServerPlugin::default())
            .add_systems(Startup, start_listening)
            .add_systems(Update, handle_client_messages)
            .add_systems(Startup, register_server_system)
            .add_systems(Update, ping_server_system);
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
    mut server: ResMut<QuinnetServer>,
    server_state_resource: Res<ServerStateResource>,
) {
    let endpoint = server.endpoint_mut();
    for client_id in endpoint.clients() {
        while let Some((_channel_id, message)) =
            endpoint.try_receive_message_from::<ClientMessage>(client_id)
        {
            match message {
                ClientMessage::Join { user_id } => {
                    let mut server_state = server_state_resource.0.lock().unwrap();
                    server_state.connections.insert(client_id, user_id);

                    endpoint
                        .broadcast_message(ServerMessage::ClientConnected { client_id, user_id })
                        .unwrap();

                    for (user_client_id, user_id) in server_state.connections.iter() {
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
                    let mut server_state = server_state_resource.0.lock().unwrap();
                    server_state.connections.remove(&client_id);

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

fn register_server_system(
    server_args: Res<ServerArgs>,
    tokio_runtime_resource: Res<TokioRuntimeResource<TokioServerMessage>>,
) {
    let tx = tokio_runtime_resource.sender.clone();
    let api_base_url = server_args.api_base_url.clone();
    let addr = server_args.addr;
    let port = server_args.port;
    let name = server_args.name.clone();

    tokio_runtime_resource.runtime.spawn(async move {
        let result = register_server(&api_base_url, &RegisterServer { addr, port, name }).await;

        match result {
            Ok(server) => tx
                .send(TokioServerMessage::RegisterServer(server))
                .await
                .unwrap(),
            Err(error) => error!(error = ?error, "Create"),
        }
    });
}

fn ping_server_system(
    server_args: Res<ServerArgs>,
    mut connection_resource: ResMut<ConnectionResource>,
    tokio_runtime_resource: Res<TokioRuntimeResource<TokioServerMessage>>,
) {
    if let Some(server) = &connection_resource.server {
        let now = OffsetDateTime::now_utc();
        let timeout = Duration::seconds(60);

        if now - server.last_ping >= timeout
            && now - connection_resource.last_ping_attempt >= timeout
        {
            let tx = tokio_runtime_resource.sender.clone();

            let api_base_url = server_args.api_base_url.clone();
            let id = server.id;

            tokio_runtime_resource.runtime.spawn(async move {
                let result = ping_server(&api_base_url, &id).await;

                match result {
                    Ok(server) => tx
                        .send(TokioServerMessage::PingServer(server))
                        .await
                        .unwrap(),
                    Err(error) => error!(error = ?error, "Ping"),
                }
            });

            connection_resource.last_ping_attempt = OffsetDateTime::now_utc();
        }
    }
}
