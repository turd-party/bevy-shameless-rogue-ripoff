mod anatomy;
pub mod data;
pub mod genetics;
pub mod physics;
mod procgen;
pub mod rpg;
pub mod snn;

use bevy::prelude::*;
use anatomy::hull::BiologicalSex;
use data::ContentRegistry;
use procgen::terrain::{evaluate_terrain_height, TerrainConfig};

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
            data::DataContentPlugin,
            physics::NexusPhysicsPlugin,
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
    // 3D Tactical Camera with WASD pan, Q/E yaw orbit, and scroll/Z/X zoom
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 18.0, 26.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
        rpg::hud::TacticalCamera::default(),
    ));

    // Directional Sunlight with shadow maps
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
    registry: Res<ContentRegistry>,
    terrain_config: Res<TerrainConfig>,
) {
    // 1. Spawn Player Party Members driven by `assets/data/rpg/party_starter.ron`
    // Executive SNN is removed: player tactical commands replace executive willpower,
    // directly driving the low-level Locomotion and Manipulation SNN modules.
    let party_offsets = [Vec2::new(-2.5, 0.0), Vec2::new(2.5, 0.0)];
    let party_colors = [
        Color::srgb(0.85, 0.65, 0.52),
        Color::srgb(0.78, 0.58, 0.45),
    ];

    for (idx, member_template) in registry
        .party_config
        .starting_party_members
        .iter()
        .enumerate()
    {
        let offset = party_offsets.get(idx).copied().unwrap_or(Vec2::new(idx as f32 * 2.0, 0.0));
        let ground_y = evaluate_terrain_height(offset.x, offset.y, &terrain_config);
        let spawn_pos = Vec3::new(offset.x, ground_y, offset.y);
        let skin = party_colors.get(idx).copied().unwrap_or(Color::srgb(0.8, 0.6, 0.5));

        let entity = anatomy::spawn_biomechanical_human(
            &mut commands,
            &mut meshes,
            &mut materials,
            spawn_pos,
            member_template.sex,
            registry.human_archetype.adult_maturity_scale,
            skin,
        );

        commands
            .entity(entity)
            .remove::<snn::brain::ExecutiveSnnModule>()
            .insert((
                rpg::party::PartyMember {
                    index: idx,
                    designation: member_template.designation.clone(),
                    is_selected: idx == 0,
                },
                rpg::party::OrderQueue {
                    orders: Vec::with_capacity(registry.party_config.tactical_order_queue_limit),
                    max_capacity: registry.party_config.tactical_order_queue_limit,
                },
                anatomy::damage::MetabolicCaloricPool {
                    current_calories: member_template.caloric_reserve,
                    max_calories: registry.human_archetype.base_caloric_capacity,
                    baseline_burn_rate_per_sec: registry.human_archetype.metabolic_rate_per_sec,
                    starvation_damage_rate: 1.5,
                    is_starving: false,
                    is_dead: false,
                },
            ));
    }

    // 2. Spawn Autonomous Wild NPC Humans
    // Retain their full Executive SNN (ferromorphic LIF + 3-factor RewardStdp + Actor-Critic)
    // to forage, pluck fruit, bring food to mouth, mate, and pass on genes naturally in the wild!
    let wild_spawns = [
        (Vec2::new(6.0, 4.5), BiologicalSex::Male, 1.04, Color::srgb(0.68, 0.50, 0.36)),
        (Vec2::new(7.2, 5.0), BiologicalSex::Female, 0.94, Color::srgb(0.90, 0.70, 0.56)),
        (Vec2::new(-8.0, 4.0), BiologicalSex::Male, 1.00, Color::srgb(0.75, 0.55, 0.42)),
        (Vec2::new(-6.8, 4.5), BiologicalSex::Female, 0.96, Color::srgb(0.82, 0.64, 0.50)),
    ];

    for (xz, sex, scale, skin) in wild_spawns {
        let ground_y = evaluate_terrain_height(xz.x, xz.y, &terrain_config);
        anatomy::spawn_biomechanical_human(
            &mut commands,
            &mut meshes,
            &mut materials,
            Vec3::new(xz.x, ground_y, xz.y),
            sex,
            scale,
            skin,
        );
    }
}
