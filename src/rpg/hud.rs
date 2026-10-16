#![allow(dead_code)]

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use super::party::{OrderQueue, PartyMember, SelectionRingVisual};
use super::rtwp::RtwpController;
use crate::anatomy::damage::MetabolicCaloricPool;
use crate::anatomy::hull::ReproductiveContactSensor;
use crate::anatomy::skeleton::HumanoidSkeleton;
use crate::anatomy::soft_tissue::SoftTissueViscera;
use crate::genetics::genome::HumanGenome;
use crate::procgen::flora::PluckableFruit;
use crate::snn::brain::{ExecutiveSnnModule, LocomotionSnnModule, ManipulationSnnModule};

#[derive(Component, Debug)]
pub struct TacticalCamera {
    pub focus: Vec3,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for TacticalCamera {
    fn default() -> Self {
        Self {
            focus: Vec3::new(0.0, 1.0, 0.0),
            distance: 28.0,
            yaw: 0.0,
            pitch: 0.62,
        }
    }
}

#[derive(Component)]
pub struct HudStatusText;

pub fn spawn_hud_overlay(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                left: Val::Px(12.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.07, 0.10, 0.78)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Shameless Rogue - Emergent RTwP Sandbox"),
                TextFont {
                    font_size: bevy::text::FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.92, 0.95, 0.98)),
                HudStatusText,
            ));
        });
}

pub fn update_tactical_camera_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut scroll_events: MessageReader<MouseWheel>,
    mut camera_query: Query<(&mut Transform, &mut TacticalCamera), With<Camera3d>>,
    time: Res<Time<Real>>,
) {
    let dt = time.delta_secs().clamp(0.001, 0.1);

    let mut scroll_delta = 0.0f32;
    for ev in scroll_events.read() {
        scroll_delta += ev.y;
    }

    for (mut tf, mut cam) in &mut camera_query {
        // Yaw orbit with Q / E
        if keyboard.pressed(KeyCode::KeyQ) {
            cam.yaw += 1.6 * dt;
        }
        if keyboard.pressed(KeyCode::KeyE) {
            cam.yaw -= 1.6 * dt;
        }

        // Zoom with scroll wheel or Z / X
        if scroll_delta.abs() > 0.01 {
            cam.distance = (cam.distance - scroll_delta * 2.2).clamp(8.0, 65.0);
        }
        if keyboard.pressed(KeyCode::KeyZ) {
            cam.distance = (cam.distance - 18.0 * dt).clamp(8.0, 65.0);
        }
        if keyboard.pressed(KeyCode::KeyX) {
            cam.distance = (cam.distance + 18.0 * dt).clamp(8.0, 65.0);
        }

        // Pan focus in XZ plane with WASD / Arrow keys
        let forward_xz = Vec3::new(-cam.yaw.sin(), 0.0, -cam.yaw.cos()).normalize_or_zero();
        let right_xz = Vec3::new(cam.yaw.cos(), 0.0, -cam.yaw.sin()).normalize_or_zero();
        let pan_speed = 16.0 * (cam.distance / 25.0).max(0.5);

        if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
            cam.focus += forward_xz * pan_speed * dt;
        }
        if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
            cam.focus -= forward_xz * pan_speed * dt;
        }
        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
            cam.focus += right_xz * pan_speed * dt;
        }
        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
            cam.focus -= right_xz * pan_speed * dt;
        }

        let offset = Vec3::new(
            cam.yaw.sin() * cam.pitch.cos(),
            cam.pitch.sin(),
            cam.yaw.cos() * cam.pitch.cos(),
        ) * cam.distance;

        tf.translation = cam.focus + offset;
        tf.look_at(cam.focus, Vec3::Y);
    }
}

/// Attaches and updates selection rings beneath PartyMember entities
pub fn update_party_selection_rings_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    party_query: Query<(Entity, &PartyMember, Option<&Children>)>,
    mut ring_query: Query<&mut Visibility, With<SelectionRingVisual>>,
) {
    for (entity, member, maybe_children) in &party_query {
        let mut found_ring = false;
        if let Some(children) = maybe_children {
            for child in children.iter() {
                if let Ok(mut vis) = ring_query.get_mut(child) {
                    found_ring = true;
                    *vis = if member.is_selected {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    };
                }
            }
        }

        if !found_ring {
            let ring_mesh = meshes.add(Cylinder::new(0.48, 0.03));
            let ring_mat = materials.add(StandardMaterial {
                base_color: Color::srgba(0.2, 0.95, 0.35, 0.85),
                emissive: LinearRgba::new(0.1, 0.6, 0.2, 1.0),
                unlit: true,
                ..default()
            });
            let vis = if member.is_selected {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
            let ring_child = commands
                .spawn((
                    Mesh3d(ring_mesh),
                    MeshMaterial3d(ring_mat),
                    Transform::from_xyz(0.0, 0.03, 0.0),
                    vis,
                    SelectionRingVisual,
                ))
                .id();
            commands.entity(entity).add_child(ring_child);
        }
    }
}

pub fn update_hud_status_system(
    rtwp: Res<RtwpController>,
    party_query: Query<(
        &PartyMember,
        &OrderQueue,
        &MetabolicCaloricPool,
        &SoftTissueViscera,
        &LocomotionSnnModule,
        &ManipulationSnnModule,
    )>,
    wild_query: Query<
        (
            &ExecutiveSnnModule,
            &MetabolicCaloricPool,
            &ReproductiveContactSensor,
            &HumanGenome,
        ),
        (With<HumanoidSkeleton>, Without<PartyMember>),
    >,
    fruit_query: Query<&PluckableFruit>,
    mut text_query: Query<&mut Text, With<HudStatusText>>,
) {
    let Some(mut text) = text_query.iter_mut().next() else {
        return;
    };

    let state_label = if rtwp.is_paused {
        "PAUSED (Tactical Queue Active)"
    } else {
        "RUNNING (Real-Time Simulation)"
    };

    let mut party_lines = String::new();
    for (member, queue, meta, viscera, loco, manip) in &party_query {
        let sel = if member.is_selected { "[*]" } else { "[ ]" };
        let next_order = queue
            .orders
            .first()
            .map(|o| format!("{:?}", o))
            .unwrap_or_else(|| "Idle".into());
        party_lines.push_str(&format!(
            "  {} #{} {} | {:.0}/{:.0} kcal | Blood: {:.1}L | Orders({}): {} | Spikes(L/M): {}/{}\n",
            sel,
            member.index + 1,
            member.designation,
            meta.current_calories,
            meta.max_calories,
            viscera.blood_volume_liters,
            queue.orders.len(),
            next_order,
            loco.total_spikes,
            manip.total_spikes
        ));
    }

    let mut wild_count = 0usize;
    let mut pregnant_count = 0usize;
    let mut max_gen = 0u32;
    let mut total_exec_spikes = 0u64;
    for (exec, meta, repro, genome) in &wild_query {
        if !meta.is_dead {
            wild_count += 1;
            if repro.is_pregnant {
                pregnant_count += 1;
            }
            max_gen = max_gen.max(genome.generation);
            total_exec_spikes += exec.total_spikes;
        }
    }

    let fruit_count = fruit_query.iter().count();

    text.0 = format!(
        "[RTwP Status]: {}\n\
         [Controls]: Space=Pause | 1/2/Tab/LClick=Select | RClick=Move/Forage (Shift+RClick=Queue) | WASD/QE/ZX=Camera | F5=Reload .ron\n\
         [Player Party (Executive SNN Replaced by Player Orders)]:\n{}\
         [Wild Ecosystem]: {} Autonomous NPCs (Gestating: {}, Max Gen: {}) | Exec SNN Spikes: {} | World Fruits: {}",
        state_label, party_lines, wild_count, pregnant_count, max_gen, total_exec_spikes, fruit_count
    );
}
