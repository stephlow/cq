use bevy::prelude::Resource;
use tokio::runtime::Runtime;

#[derive(Resource)]
pub struct TokioRuntimeResource {
    pub runtime: Runtime,
}

impl TokioRuntimeResource {
    pub fn new() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        Self { runtime }
    }
}
