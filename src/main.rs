mod procgen;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Shameless Rogue - Emergent RTwP Sandbox".into(),
                resolution: (1280u32, 720u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(procgen::ProcGenPlugin)
        .add_systems(Startup, setup_scene)
        .run();
}

fn setup_scene(mut commands: Commands) {
    // 3D Tactical Camera overlooking the terrain
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 22.0, 32.0).looking_at(Vec3::new(0.0, 2.0, 0.0), Vec3::Y),
    ));

    // Directional Sunlight with shadows
    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.75, 0.6, 0.0)),
    ));
}
