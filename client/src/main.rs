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
    shared::{channels::ChannelsConfiguration, ClientId},
};
use clap::Parser;
use engine::{
    api_client::{self, list_servers},
    models::api::{auth::Credentials, users::User, GameServer},
    network::{ClientMessage, ServerMessage},
    resources::TokioRuntimeResource,
};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
};
use tokio::sync::mpsc::channel;
use uuid::Uuid;

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

#[derive(Default, Resource)]
struct AuthInputState {
    username: String,
    password: String,
}

#[derive(Default, Resource)]
struct ChatInputState {
    text: String,
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
    LoadServers(Vec<GameServer>),
}

#[derive(Default, Resource)]
struct ServerBrowser {
    servers: Option<Vec<GameServer>>,
}

#[derive(Default, Resource)]
struct ServerInfo {
    id: Option<Uuid>,
    connected: HashMap<ClientId, Uuid>,
    messages: Vec<(ClientId, String)>,
}

fn main() {
    let args = ClientArgs::parse();
    let (tx, rx) = channel::<TokioClientMessage>(10);

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        .add_plugins(QuinnetClientPlugin::default())
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
        .insert_resource(AuthInputState::default())
        .insert_resource(ChatInputState::default())
        .insert_resource(TokioRuntimeResource::new(tx, rx))
        .insert_resource(ServerBrowser::default())
        .insert_resource(ServerInfo::default())
        .add_systems(Update, tokio_receiver_system)
        .add_systems(Startup, load_servers)
        .add_systems(
            Update,
            auth_ui_system.run_if(in_state(AuthState::Anonymous)),
        )
        .add_systems(
            Update,
            server_ui_system
                .run_if(in_state(AuthState::Authenticated))
                .run_if(in_state(ConnectionState::Connected)),
        )
        .add_systems(
            Update,
            server_browser_ui_system
                .run_if(in_state(AuthState::Authenticated))
                .run_if(in_state(ConnectionState::Disconnected)),
        )
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

fn auth_ui_system(
    client_args: Res<ClientArgs>,
    tokio: Res<TokioRuntimeResource<TokioClientMessage>>,
    mut contexts: EguiContexts,
    mut auth_input_state: ResMut<AuthInputState>,
) {
    egui::Window::new("Authenticate").show(contexts.ctx_mut(), |ui| {
        ui.label("Username:");
        ui.text_edit_singleline(&mut auth_input_state.username);
        ui.label("Password:");
        ui.text_edit_singleline(&mut auth_input_state.password);

        if ui.button("Login").clicked() {
            let username = auth_input_state.username.clone();
            let password = auth_input_state.password.clone();
            let tx = tokio.sender.clone();

            let api_base_url = client_args.api_base_url.clone();
            tokio.runtime.spawn(async move {
                let auth_response =
                    api_client::authenticate(&api_base_url, Credentials { username, password })
                        .await
                        .unwrap();

                let user = api_client::get_profile(&api_base_url, &auth_response.token)
                    .await
                    .unwrap();

                tx.send(TokioClientMessage::Authenticated {
                    token: auth_response.token,
                    user,
                })
                .await
                .unwrap();
            });
        }
    });
}

fn server_ui_system(
    mut contexts: EguiContexts,
    mut client_event_writer: EventWriter<ClientEvent>,
    server_info: Res<ServerInfo>,
    mut chat_input_state: ResMut<ChatInputState>,
    client: Res<QuinnetClient>,
) {
    egui::Window::new("Server").show(contexts.ctx_mut(), |ui| {
        ui.label("Connected");
        if ui.button("Disconnect").clicked() {
            client_event_writer.send(ClientEvent::Disconnect);
        }
        ui.label("Connected users:");
        for (_client_id, user_id) in server_info.connected.iter() {
            ui.label(format!("{}", user_id));
        }
    });

    egui::Window::new("Chat").show(contexts.ctx_mut(), |ui| {
        for (client_id, message) in server_info.messages.iter() {
            // TODO: Handle properly
            if let Some(user_id) = server_info.connected.get(client_id) {
                ui.label(format!("{}: {}", user_id, message));
            }
        }

        ui.text_edit_singleline(&mut chat_input_state.text);
        if ui.button("Send").clicked() {
            let message = chat_input_state.text.clone();
            client
                .connection()
                .send_message(ClientMessage::ChatMessage { message })
                .unwrap();
            chat_input_state.text = String::from("");
        }
    });
}

fn server_browser_ui_system(
    mut contexts: EguiContexts,
    server_browser_resource: Res<ServerBrowser>,
    mut client_event_writer: EventWriter<ClientEvent>,
    auth_info: Res<AuthInfo>,
) {
    egui::Window::new("Servers").show(contexts.ctx_mut(), |ui| {
        if let Some(user) = &auth_info.user {
            ui.label(format!("user_id: {}", user.id));
        }

        if let Some(servers) = &server_browser_resource.servers {
            for server in servers.iter() {
                ui.label(format!("Server name: {}:{}", server.name, server.port));
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
    mut server_info: ResMut<ServerInfo>,
    auth_info: Res<AuthInfo>,
) {
    for event in client_event_reader.read() {
        match event {
            ClientEvent::Connect(id) => {
                if let Some(servers) = &server_browser_resource.servers {
                    if let Some(server) = servers.iter().find(|server| &server.id == id) {
                        if let Some(user) = &auth_info.user {
                            let client_id = client
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
                            server_info.connected.insert(client_id, user.id);
                        }
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
            .send_message(engine::network::ClientMessage::Disconnect)
            .unwrap();
    }
}
