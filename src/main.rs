mod anatomy;
pub mod genetics;
mod procgen;
pub mod snn;

use bevy::prelude::*;
use anatomy::hull::BiologicalSex;

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
        .add_plugins((
            procgen::ProcGenPlugin,
            anatomy::AnatomyPlugin,
            snn::SnnPlugin,
            genetics::GeneticsPlugin,
        ))
        .add_systems(Startup, (setup_scene, spawn_starter_humans))
        .run();
}

fn setup_scene(mut commands: Commands) {
    // 3D Tactical Camera overlooking the terrain
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 18.0, 26.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
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

fn spawn_starter_humans(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn Starter Male Human
    anatomy::spawn_biomechanical_human(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(-2.0, 0.0, 0.0),
        BiologicalSex::Male,
        1.0,
        Color::srgb(0.85, 0.65, 0.52),
    );

    // Spawn Starter Female Human
    anatomy::spawn_biomechanical_human(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(2.0, 0.0, 0.0),
        BiologicalSex::Female,
        0.95,
        Color::srgb(0.78, 0.58, 0.45),
    );
}
