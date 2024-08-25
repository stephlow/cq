use bevy::prelude::*;
use bevy_quinnet::client::{connection::ConnectionEvent, QuinnetClient};
use clap::Parser;
use engine::{
    models::{api::users::User, network::ClientMessage},
    resources::TokioRuntimeResource,
};
use plugins::{
    api::{ApiEvent, ApiPlugin},
    network::NetworkPlugin,
    ui::UiPlugin,
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
}

fn main() {
    let args = ClientArgs::parse();

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ApiPlugin)
        .add_plugins(UiPlugin)
        .add_plugins(NetworkPlugin)
        .init_state::<AuthState>()
        .init_state::<ConnectionState>()
        .add_systems(Update, connection_event_handler)
        .add_event::<ClientEvent>()
        .insert_resource(args)
        .insert_resource(AuthInfo::default())
        .insert_resource(TokioRuntimeResource::<TokioClientMessage>::new())
        .add_systems(Update, tokio_receiver_system)
        .add_systems(Startup, load_servers)
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

fn tokio_receiver_system(
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
        }
    }
}

fn load_servers(mut api_events: EventWriter<ApiEvent>) {
    api_events.send(ApiEvent::LoadServers);
}
