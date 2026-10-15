mod anatomy;
pub mod genetics;
mod procgen;
pub mod rpg;
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
            rpg::RpgPlugin,
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
    // 1. Spawn Player Party Member 1 (Vanguard Leader)
    // Executive SNN is removed: player tactical commands replace executive willpower,
    // directly driving the low-level Locomotion and Manipulation SNN modules.
    let party_member_1 = anatomy::spawn_biomechanical_human(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(-2.0, 0.0, 0.0),
        BiologicalSex::Male,
        1.0,
        Color::srgb(0.85, 0.65, 0.52),
    );
    commands
        .entity(party_member_1)
        .remove::<snn::brain::ExecutiveSnnModule>()
        .insert((
            rpg::party::PartyMember {
                index: 0,
                designation: "Vanguard Leader".into(),
                is_selected: true,
            },
            rpg::party::OrderQueue::default(),
        ));

    // 2. Spawn Player Party Member 2 (Apothecary Scout)
    let party_member_2 = anatomy::spawn_biomechanical_human(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(2.0, 0.0, 0.0),
        BiologicalSex::Female,
        0.95,
        Color::srgb(0.78, 0.58, 0.45),
    );
    commands
        .entity(party_member_2)
        .remove::<snn::brain::ExecutiveSnnModule>()
        .insert((
            rpg::party::PartyMember {
                index: 1,
                designation: "Apothecary Scout".into(),
                is_selected: false,
            },
            rpg::party::OrderQueue::default(),
        ));

    // 3. Spawn Autonomous Wild NPC Humans
    // Retain their full Executive SNN to forage, mate, and evolve naturally in the wild!
    anatomy::spawn_biomechanical_human(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(12.0, 0.0, -8.0),
        BiologicalSex::Male,
        1.05,
        Color::srgb(0.65, 0.48, 0.35),
    );
    anatomy::spawn_biomechanical_human(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(14.0, 0.0, -7.5),
        BiologicalSex::Female,
        0.92,
        Color::srgb(0.92, 0.72, 0.58),
    );
}
