use bevy::ecs::component::Component;
use bevy::math::Vec3;
use bevy_quinnet::shared::ClientId;
use uuid::Uuid;

#[derive(Component)]
pub struct Player {
    pub client_id: ClientId,
    pub user_id: Uuid,
}

#[derive(Default, Component)]
pub struct PlayerPosition(pub Vec3);
