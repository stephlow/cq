use bevy::{
    app::{App, PluginGroup, ScheduleRunnerPlugin},
    MinimalPlugins,
};
use std::time::Duration;

fn main() {
    App::new()
        .add_plugins(
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                1.0 / 60.0,
            ))),
        )
        .run();
}
