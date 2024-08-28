use bevy::prelude::*;
use bevy_quinnet::shared::ClientId;
use uuid::Uuid;

#[derive(Component)]
pub struct Player {
    pub client_id: ClientId,
    pub user_id: Uuid,
}

#[derive(Default, Component)]
pub struct PlayerPosition(pub Vec3);
