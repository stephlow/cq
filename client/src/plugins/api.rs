use std::collections::HashMap;

use crate::AuthState;
use bevy::prelude::*;
use engine::{
    api_client::{authenticate, get_profile, get_user, list_servers, register_user},
    models::api::{
        auth::{AuthResponse, Credentials},
        servers::Server,
        users::{NewUser, User},
    },
};
use tokio::{runtime::Runtime, sync::mpsc};
use uuid::Uuid;

pub struct ApiPlugin {
    base_url: String,
}

impl ApiPlugin {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }
}

impl Plugin for ApiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ApiResource::new(self.base_url.clone()))
            .add_event::<ApiEvent>()
            .add_systems(Update, api_event_handler_system)
            .add_systems(Update, api_message_handler_system);
    }
}

#[derive(Event)]
pub enum ApiEvent {
    Authenticate {
        username: String,
        password: String,
    },
    Register {
        username: String,
        email: String,
        password: String,
    },
    LoadProfile,
    LoadServers,
    LoadUser(Uuid),
}

enum ApiMessage {
    AuthenticateFulfilled(String),
    RegisterFulfilled(String),
    LoadProfileFulfilled(User),
    LoadServersFulfilled(Vec<Server>),
    LoadUserFulfilled(User),
}

#[derive(Resource)]
pub struct ApiResource {
    base_url: String,
    runtime: Runtime,
    tx: mpsc::Sender<ApiMessage>,
    rx: mpsc::Receiver<ApiMessage>,
    pub token: LoadableData<String>,
    pub profile: LoadableData<User>,
    pub servers: LoadableData<Vec<Server>>,
    pub users: HashMap<Uuid, LoadableData<User>>,
}

impl ApiResource {
    fn new(base_url: String) -> Self {
        let (tx, rx) = mpsc::channel(100);

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        Self {
            base_url,
            runtime,
            tx,
            rx,
            token: LoadableData::default(),
            profile: LoadableData::default(),
            servers: LoadableData::default(),
            users: HashMap::new(),
        }
    }
}

#[derive(Clone)]
pub struct LoadableData<T> {
    pub loading: bool,
    pub data: Option<T>,
}

impl<T> Default for LoadableData<T> {
    fn default() -> Self {
        Self {
            loading: false,
            data: None,
        }
    }
}

impl<T> LoadableData<T> {
    fn start(&mut self) {
        self.loading = true;
    }

    fn finish(&mut self, data: T) {
        self.data = Some(data);
        self.loading = true;
    }
}

fn api_event_handler_system(mut api: ResMut<ApiResource>, mut events: EventReader<ApiEvent>) {
    for event in events.read() {
        match event {
            ApiEvent::Authenticate { username, password } => {
                if !api.token.loading {
                    api.token.start();

                    let tx = api.tx.clone();
                    let api_base_url = api.base_url.clone();
                    let username = username.clone();
                    let password = password.clone();

                    api.runtime.spawn(async move {
                        match authenticate(&api_base_url, &Credentials { username, password }).await
                        {
                            Ok(AuthResponse { token }) => tx
                                .send(ApiMessage::AuthenticateFulfilled(token))
                                .await
                                .unwrap(),
                            Err(_) => {}
                        }
                    });
                }
            }
            ApiEvent::Register {
                username,
                email,
                password,
            } => {
                if !api.token.loading {
                    api.token.start();

                    let tx = api.tx.clone();
                    let api_base_url = api.base_url.clone();
                    let username = username.clone();
                    let email = email.clone();
                    let password = password.clone();

                    api.runtime.spawn(async move {
                        match register_user(
                            &api_base_url,
                            NewUser {
                                username,
                                email,
                                password,
                            },
                        )
                        .await
                        {
                            Ok(AuthResponse { token }) => {
                                tx.send(ApiMessage::RegisterFulfilled(token)).await.unwrap()
                            }
                            Err(_) => {}
                        }
                    });
                }
            }
            ApiEvent::LoadProfile => {
                if !api.profile.loading {
                    api.profile.start();

                    let tx = api.tx.clone();
                    let api_base_url = api.base_url.clone();
                    if let Some(token) = &api.token.data {
                        let token = token.clone();

                        api.runtime.spawn(async move {
                            match get_profile(&api_base_url, &token).await {
                                Ok(user) => tx
                                    .send(ApiMessage::LoadProfileFulfilled(user))
                                    .await
                                    .unwrap(),
                                Err(_) => {}
                            }
                        });
                    }
                }
            }
            ApiEvent::LoadServers => {
                if !api.servers.loading {
                    api.servers.start();

                    let tx = api.tx.clone();
                    let api_base_url = api.base_url.clone();

                    api.runtime.spawn(async move {
                        match list_servers(&api_base_url).await {
                            Ok(servers) => {
                                tx.send(ApiMessage::LoadServersFulfilled(servers))
                                    .await
                                    .unwrap();
                            }
                            Err(_) => {}
                        }
                    });
                }
            }
            ApiEvent::LoadUser(id) => {
                let loadable = match api.users.get_mut(&id) {
                    Some(loadable) => loadable,
                    None => {
                        api.users.insert(id.clone(), LoadableData::default());

                        // Should be safe it's just inserted
                        api.users.get_mut(&id).unwrap()
                    }
                };

                if !loadable.loading {
                    loadable.start();

                    let tx = api.tx.clone();
                    let api_base_url = api.base_url.clone();
                    let id = id.clone();

                    api.runtime.spawn(async move {
                        match get_user(&api_base_url, &id).await {
                            Ok(user) => tx.send(ApiMessage::LoadUserFulfilled(user)).await.unwrap(),
                            Err(_) => {}
                        }
                    });
                }
            }
        }
    }
}

fn api_message_handler_system(
    mut api: ResMut<ApiResource>,
    mut events: EventWriter<ApiEvent>,
    mut auth_state: ResMut<NextState<AuthState>>,
) {
    match api.rx.try_recv() {
        Ok(message) => match message {
            ApiMessage::AuthenticateFulfilled(token) => {
                api.token.finish(token);
                auth_state.set(AuthState::Authenticated);
                events.send(ApiEvent::LoadProfile);
                events.send(ApiEvent::LoadServers);
            }
            ApiMessage::RegisterFulfilled(token) => {
                api.token.finish(token);
                auth_state.set(AuthState::Authenticated);
                events.send(ApiEvent::LoadProfile);
                events.send(ApiEvent::LoadServers);
            }
            ApiMessage::LoadProfileFulfilled(user) => {
                api.profile.finish(user);
            }
            ApiMessage::LoadServersFulfilled(servers) => {
                api.servers.finish(servers);
            }
            ApiMessage::LoadUserFulfilled(user) => {
                if let Some(loadable) = api.users.get_mut(&user.id) {
                    loadable.finish(user);
                }
            }
        },
        Err(_) => {}
    }
}
