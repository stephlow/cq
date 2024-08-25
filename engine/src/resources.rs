use bevy::prelude::*;
use tokio::{
    runtime::Runtime,
    sync::mpsc::{channel, Receiver, Sender},
};

#[derive(Resource)]
pub struct TokioRuntimeResource<T> {
    pub runtime: Runtime,
    pub sender: Sender<T>,
    pub receiver: Receiver<T>,
}

impl<T> Default for TokioRuntimeResource<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> TokioRuntimeResource<T> {
    pub fn new() -> Self {
        let (tx, rx) = channel(100);

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        Self {
            runtime,
            sender: tx,
            receiver: rx,
        }
    }
}
