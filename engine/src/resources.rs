use bevy::prelude::*;
use tokio::{
    runtime::Runtime,
    sync::mpsc::{Receiver, Sender},
};

#[derive(Resource)]
pub struct TokioRuntimeResource<T> {
    pub runtime: Runtime,
    pub sender: Sender<T>,
    pub receiver: Receiver<T>,
}

impl<T> TokioRuntimeResource<T> {
    pub fn new(tx: Sender<T>, rx: Receiver<T>) -> Self {
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
