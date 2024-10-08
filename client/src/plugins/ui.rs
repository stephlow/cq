use super::{
    api::{ApiEvent, ApiResource},
    network::ServerInfo,
};
use crate::{AuthState, ClientEvent, ConnectionState};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy_quinnet::client::QuinnetClient;
use engine::models::network::ClientMessage;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .init_resource::<LoginInputState>()
            .init_resource::<RegisterInputState>()
            .init_resource::<ChatInputState>()
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
            );
    }
}

#[derive(Default, Resource)]
struct LoginInputState {
    username: String,
    password: String,
}

#[derive(Default, Resource)]
struct RegisterInputState {
    username: String,
    email: String,
    password: String,
}

#[derive(Default, Resource)]
struct ChatInputState {
    text: String,
}

fn auth_ui_system(
    mut api_events: EventWriter<ApiEvent>,
    mut contexts: EguiContexts,
    mut login_input_state: ResMut<LoginInputState>,
    mut register_input_state: ResMut<RegisterInputState>,
) {
    egui::Window::new("Login").show(contexts.ctx_mut(), |ui| {
        ui.label("Username:");
        ui.text_edit_singleline(&mut login_input_state.username);
        ui.label("Password:");
        ui.text_edit_singleline(&mut login_input_state.password);

        if ui.button("Login").clicked() {
            let username = login_input_state.username.clone();
            let password = login_input_state.password.clone();

            api_events.send(ApiEvent::Authenticate { username, password });
        }
    });

    egui::Window::new("Register").show(contexts.ctx_mut(), |ui| {
        ui.label("Username:");
        ui.text_edit_singleline(&mut register_input_state.username);
        ui.label("Email:");
        ui.text_edit_singleline(&mut register_input_state.email);
        ui.label("Password:");
        ui.text_edit_singleline(&mut register_input_state.password);

        if ui.button("Register").clicked() {
            let username = register_input_state.username.clone();
            let email = register_input_state.email.clone();
            let password = register_input_state.password.clone();

            api_events.send(ApiEvent::Register {
                username,
                email,
                password,
            });
        }
    });
}

fn server_ui_system(
    api: Res<ApiResource>,
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
            let mut username = format!("{}", user_id);

            if let Some(loadable) = api.users.get(user_id) {
                if let Some(user) = &loadable.data {
                    username = user.username.clone();
                }
            }

            ui.label(username.to_string());
        }
    });

    egui::Window::new("Chat").show(contexts.ctx_mut(), |ui| {
        for (client_id, message) in server_info.messages.iter() {
            let mut username = format!("{}", client_id);

            if let Some(user_id) = server_info.connected.get(client_id) {
                username = format!("{}", user_id);

                if let Some(loadable) = api.users.get(user_id) {
                    if let Some(user) = &loadable.data {
                        username = user.username.clone();
                    }
                }
            }

            ui.label(format!("{}: {}", username, message));
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
    api: Res<ApiResource>,
    mut api_event_writer: EventWriter<ApiEvent>,
    mut contexts: EguiContexts,
    mut client_event_writer: EventWriter<ClientEvent>,
) {
    egui::Window::new("Servers").show(contexts.ctx_mut(), |ui| {
        if let Some(user) = &api.profile.data {
            ui.label(format!("user_id: {}", user.id));
        }

        if let Some(servers) = &api.servers.data {
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

        let bttn_text = match api.servers.loading {
            true => "Loading...",
            false => "Reload",
        };

        if ui.button(bttn_text).clicked && !api.servers.loading {
            api_event_writer.send(ApiEvent::LoadServers);
        }
    });
}
