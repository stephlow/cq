use bevy::prelude::*;
use bevy_quinnet::client::QuinnetClientPlugin;

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(QuinnetClientPlugin::default());
    }
}
