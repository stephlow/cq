use bevy::prelude::*;
use bevy_quinnet::shared::ClientId;
use engine::components::{
    movement::{MoveModifier, Movement},
    player::{Player, PlayerPosition},
};
use uuid::Uuid;

use crate::components::controllable::Controllable;

use super::api::ApiResource;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<RenderEvent>()
            .add_systems(Startup, setup)
            .add_systems(Update, handle_render_event)
            .add_systems(Update, update_position)
            .add_systems(Update, update_camera);
    }
}

#[derive(Component)]
struct CameraMarker;

#[derive(Event)]
pub enum RenderEvent {
    Spawn {
        client_id: ClientId,
        user_id: Uuid,
    },
    Despawn(ClientId),
    UpdatePosition {
        client_id: ClientId,
        position: Vec3,
    },
    UpdateMovement {
        client_id: ClientId,
        modifier: MoveModifier,
    },
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(20.0, 20.)),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3)),
        ..default()
    });

    // Light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    // Camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(10., 15., 10.).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        CameraMarker,
    ));
}

fn handle_render_event(
    api: Res<ApiResource>,
    mut players: Query<(Entity, &Player, &mut PlayerPosition, &mut Movement)>,
    mut events: EventReader<RenderEvent>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for event in events.read() {
        match event {
            RenderEvent::Spawn { client_id, user_id } => {
                let mut entity = commands.spawn((
                    Player {
                        client_id: *client_id,
                        user_id: *user_id,
                    },
                    PlayerPosition::default(),
                    Movement::default(),
                    PbrBundle {
                        mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
                        material: materials.add(Color::rgb_u8(124, 144, 255)),
                        transform: Transform::from_xyz(0., 0., 0.),
                        ..default()
                    },
                ));

                if let Some(user) = &api.profile.data {
                    if &user.id == user_id {
                        entity.insert(Controllable);
                    }
                }
            }
            RenderEvent::Despawn(client_id) => {
                if let Some((entity, _, _, _)) = players
                    .into_iter()
                    .find(|(_, player, _, _)| player.client_id == *client_id)
                {
                    commands.entity(entity).despawn();
                }
            }
            RenderEvent::UpdatePosition {
                client_id,
                position,
            } => {
                if let Some((_, _, mut player_position, _)) = players
                    .iter_mut()
                    .find(|(_, player, _, _)| player.client_id == *client_id)
                {
                    player_position.0.x = position.x;
                    player_position.0.y = position.y;
                    player_position.0.z = position.z;
                }
            }
            RenderEvent::UpdateMovement {
                client_id,
                modifier,
            } => {
                if let Some((_, _, _, mut movement)) = players
                    .iter_mut()
                    .find(|(_, player, _, _)| player.client_id == *client_id)
                {
                    movement.modify(modifier.clone());
                }
            }
        }
    }
}

fn update_position(mut players: Query<(&mut Transform, &PlayerPosition)>) {
    for (mut transform, player_position) in players.iter_mut() {
        transform.translation.x = player_position.0.x;
        transform.translation.y = player_position.0.y;
        transform.translation.z = player_position.0.z;
    }
}

fn update_camera(
    controllable: Query<(&Controllable, &PlayerPosition)>,
    mut camera: Query<(&mut Transform, &CameraMarker)>,
) {
    let camera_offset = Vec3::new(-10.0, 10.0, 0.0);

    for (_, position) in controllable.iter() {
        let player_pos = position.0;

        for (mut transform, _) in camera.iter_mut() {
            transform.translation = player_pos + camera_offset;

            transform.look_at(player_pos, Vec3::Y);
        }
    }
}
