use crate::{DatabaseResource, TokioServerMessage};
use app::create_app;
use bevy::prelude::*;
use engine::resources::TokioRuntimeResource;

mod app;

#[derive(Resource)]
pub struct WebServerResource {
    port: u16,
    started: bool,
}

impl WebServerResource {
    fn new(port: u16) -> Self {
        Self {
            port,
            started: false,
        }
    }
}

pub struct WebServerPlugin {
    port: u16,
}

impl WebServerPlugin {
    pub fn new(port: u16) -> Self {
        Self { port }
    }
}

impl Plugin for WebServerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WebServerResource::new(self.port))
            .add_systems(Update, start_webserver);
    }
}

fn start_webserver(
    mut webserver_resource: ResMut<WebServerResource>,
    tokio_runtime_resource: Res<TokioRuntimeResource<TokioServerMessage>>,
    database_resource: Res<DatabaseResource>,
) {
    if database_resource.pool.is_some() && !webserver_resource.started {
        let web_port = webserver_resource.port;
        let pool = database_resource.pool.clone().unwrap();

        tokio_runtime_resource.runtime.spawn(async move {
            let app = create_app(pool).await;

            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", web_port))
                .await
                .unwrap();

            axum::serve(listener, app).await
        });

        webserver_resource.started = true;
    }
}
