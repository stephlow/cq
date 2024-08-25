use bevy::prelude::*;
use bevy_quinnet::client::{connection::ConnectionEvent, QuinnetClient};
use clap::Parser;
use engine::models::network::ClientMessage;
use plugins::{
    api::{ApiEvent, ApiPlugin, ApiResource},
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
        .add_systems(Startup, load_servers)
        .run();
}

fn connection_event_handler(
    api: Res<ApiResource>,
    mut connection_event_reader: EventReader<ConnectionEvent>,
    mut next_connection_state: ResMut<NextState<ConnectionState>>,
    client: Res<QuinnetClient>,
) {
    for _ in connection_event_reader.read() {
        if let Some(user) = &api.profile.data {
            client
                .connection()
                .send_message(ClientMessage::Join { user_id: user.id })
                .unwrap();
        }
        next_connection_state.set(ConnectionState::Connected);
    }
}

fn load_servers(mut api_events: EventWriter<ApiEvent>) {
    api_events.send(ApiEvent::LoadServers);
}
