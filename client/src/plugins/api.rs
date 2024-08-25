use crate::ClientArgs;
use bevy::{
    prelude::*,
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
};
use engine::{api_client::list_servers, models::api::servers::Server};

pub struct ApiPlugin;

impl Plugin for ApiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ApiTasksResource>()
            .init_resource::<ApiResource>()
            .add_event::<ApiEvent>()
            .add_systems(Update, api_event_handler_system)
            .add_systems(Update, task_handler_system);
    }
}

#[derive(Event)]
pub enum ApiEvent {
    LoadServers,
}

#[derive(Default, Resource)]
struct ApiTasksResource {
    servers: Option<Task<Vec<Server>>>,
}

#[derive(Default, Resource)]
pub struct ApiResource {
    pub servers: LoadableData<Vec<Server>>,
}

#[derive(Clone)]
pub struct LoadableData<T> {
    pub data: Option<T>,
}

impl<T> Default for LoadableData<T> {
    fn default() -> Self {
        Self { data: None }
    }
}

impl<T> LoadableData<T> {
    fn set(&mut self, data: T) {
        self.data = Some(data);
    }
}

fn api_event_handler_system(
    client_args: Res<ClientArgs>,
    mut events: EventReader<ApiEvent>,
    mut task_resource: ResMut<ApiTasksResource>,
) {
    let task_pool = AsyncComputeTaskPool::get();

    for event in events.read() {
        match event {
            ApiEvent::LoadServers => {
                let api_base_url = client_args.api_base_url.clone();
                let task = task_pool.spawn(async move {
                    let results: Vec<Server> = vec![];
                    // println!("OK!");
                    // let x = list_servers(&api_base_url).await.unwrap();
                    // println!("DOKAY!");
                    //
                    // for server in x.iter() {
                    //     println!("FETCHED: {}", server.name);
                    // }
                    //
                    // x
                    results
                });

                task_resource.servers = Some(task);
            }
        }
    }
}

fn task_handler_system(
    mut api_resource: ResMut<ApiResource>,
    mut task_resource: ResMut<ApiTasksResource>,
) {
    if let Some(task) = &mut task_resource.servers {
        if let Some(data) = block_on(future::poll_once(task)) {
            api_resource.servers.set(data.clone());
        }
    }
}
