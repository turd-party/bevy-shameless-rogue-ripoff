#![allow(dead_code)]

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use super::terrain::{evaluate_terrain_height, TerrainConfig};
use crate::data::{ContentRegistry, ItemCategory};

#[derive(Component, Debug)]
pub struct ProceduralTree {
    pub seed: u64,
    pub branch_count: usize,
    pub fruit_socket_count: usize,
}

#[derive(Component, Debug)]
pub struct PluckableFruit {
    pub caloric_value: f32,
    pub hydration_value: f32,
    pub mass_kg: f32,
    pub is_attached: bool,
    pub ripeness_timer: f32,
    pub vertical_velocity: f32,
}

#[derive(Clone, Debug)]
struct BranchNode {
    position: Vec3,
    parent_index: Option<usize>,
    growth_direction: Vec3,
    attractor_count: u32,
    thickness: f32,
    is_terminal: bool,
}

pub struct SpaceColonizationConfig {
    pub crown_center: Vec3,
    pub crown_radius: Vec3,
    pub attractor_count: usize,
    pub segment_length: f32,
    pub influence_radius: f32,
    pub kill_distance: f32,
    pub max_iterations: usize,
}

impl Default for SpaceColonizationConfig {
    fn default() -> Self {
        Self {
            crown_center: Vec3::new(0.0, 6.2, 0.0),
            crown_radius: Vec3::new(3.5, 2.8, 3.5),
            attractor_count: 180,
            segment_length: 0.6,
            influence_radius: 5.0,
            kill_distance: 0.9,
            max_iterations: 30,
        }
    }
}

pub fn generate_space_colonization_tree(
    seed: u64,
    config: &SpaceColonizationConfig,
) -> (Mesh, Vec<Vec3>) {
    let mut attractors = Vec::with_capacity(config.attractor_count);
    let mut lcg = seed;
    let mut next_f32 = || -> f32 {
        lcg = lcg
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((lcg >> 32) as u32 as f32) / (u32::MAX as f32)
    };

    while attractors.len() < config.attractor_count {
        let u = next_f32() * 2.0 - 1.0;
        let v = next_f32() * 2.0 - 1.0;
        let w = next_f32() * 2.0 - 1.0;
        if u * u + v * v + w * w <= 1.0 {
            let pos = config.crown_center
                + Vec3::new(
                    u * config.crown_radius.x,
                    v * config.crown_radius.y,
                    w * config.crown_radius.z,
                );
            attractors.push(pos);
        }
    }

    let mut nodes: Vec<BranchNode> = vec![
        BranchNode {
            position: Vec3::ZERO,
            parent_index: None,
            growth_direction: Vec3::Y,
            attractor_count: 0,
            thickness: 0.45,
            is_terminal: false,
        },
        BranchNode {
            position: Vec3::new(0.0, 1.4, 0.0),
            parent_index: Some(0),
            growth_direction: Vec3::Y,
            attractor_count: 0,
            thickness: 0.38,
            is_terminal: false,
        },
        BranchNode {
            position: Vec3::new(0.0, 2.8, 0.0),
            parent_index: Some(1),
            growth_direction: Vec3::Y,
            attractor_count: 0,
            thickness: 0.32,
            is_terminal: true,
        },
    ];

    for _ in 0..config.max_iterations {
        if attractors.is_empty() {
            break;
        }

        for node in &mut nodes {
            node.growth_direction = Vec3::ZERO;
            node.attractor_count = 0;
        }

        for attractor in &attractors {
            let mut closest_node_idx = None;
            let mut min_dist = config.influence_radius;

            for (idx, node) in nodes.iter().enumerate() {
                let dist = node.position.distance(*attractor);
                if dist < min_dist {
                    min_dist = dist;
                    closest_node_idx = Some(idx);
                }
            }

            if let Some(idx) = closest_node_idx {
                let dir = (*attractor - nodes[idx].position).normalize_or_zero();
                nodes[idx].growth_direction += dir;
                nodes[idx].attractor_count += 1;
            }
        }

        let mut new_nodes = Vec::new();
        let current_node_count = nodes.len();

        for (idx, node) in nodes.iter_mut().enumerate() {
            if node.attractor_count > 0 {
                let avg_dir = (node.growth_direction / node.attractor_count as f32).normalize();
                let new_pos = node.position + avg_dir * config.segment_length;
                node.is_terminal = false;

                let height_factor = (new_pos.y / 10.0).clamp(0.0, 1.0);
                let thickness = (0.35 * (1.0 - height_factor * 0.75)).max(0.06);

                new_nodes.push(BranchNode {
                    position: new_pos,
                    parent_index: Some(idx),
                    growth_direction: avg_dir,
                    attractor_count: 0,
                    thickness,
                    is_terminal: true,
                });
            }
        }

        if new_nodes.is_empty() {
            break;
        }

        nodes.extend(new_nodes);

        attractors.retain(|attr| {
            !nodes[current_node_count..]
                .iter()
                .any(|n| n.position.distance(*attr) < config.kill_distance)
        });
    }

    let fruit_sockets: Vec<Vec3> = nodes
        .iter()
        .filter(|n| n.is_terminal && n.position.y > 3.5)
        .map(|n| n.position)
        .collect();

    let radial_segments = 6;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    for node in &nodes {
        if let Some(parent_idx) = node.parent_index {
            let parent = &nodes[parent_idx];
            let p0 = parent.position;
            let p1 = node.position;
            let forward = (p1 - p0).normalize_or_zero();

            let up = if forward.y.abs() > 0.95 { Vec3::X } else { Vec3::Y };
            let right = forward.cross(up).normalize();
            let true_up = right.cross(forward).normalize();

            let base_idx = positions.len() as u32;

            for seg in 0..radial_segments {
                let angle = (seg as f32 / radial_segments as f32) * std::f32::consts::TAU;
                let sin = angle.sin();
                let cos = angle.cos();

                let offset_p = (right * cos + true_up * sin) * parent.thickness;
                let offset_c = (right * cos + true_up * sin) * node.thickness;

                positions.push((p0 + offset_p).to_array());
                normals.push((right * cos + true_up * sin).to_array());
                uvs.push([seg as f32 / radial_segments as f32, 0.0]);

                positions.push((p1 + offset_c).to_array());
                normals.push((right * cos + true_up * sin).to_array());
                uvs.push([seg as f32 / radial_segments as f32, 1.0]);
            }

            for seg in 0..radial_segments as u32 {
                let next_seg = (seg + 1) % radial_segments as u32;
                let b0 = base_idx + seg * 2;
                let t0 = base_idx + seg * 2 + 1;
                let b1 = base_idx + next_seg * 2;
                let t1 = base_idx + next_seg * 2 + 1;

                indices.push(b0);
                indices.push(t0);
                indices.push(b1);

                indices.push(b1);
                indices.push(t0);
                indices.push(t1);
            }
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    (mesh, fruit_sockets)
}

pub struct FloraPlugin;

impl Plugin for FloraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_procedural_forest)
            .add_systems(Update, update_falling_fruit_system);
    }
}

fn spawn_procedural_forest(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    terrain_config: Res<TerrainConfig>,
    registry: Option<Res<ContentRegistry>>,
) {
    let (default_calories, default_hydration, default_mass) = registry
        .as_ref()
        .and_then(|r| r.primary_fruit_item())
        .map(|item| match item.category {
            ItemCategory::Food {
                caloric_value,
                hydration_value,
                ..
            } => (caloric_value, hydration_value, item.physical_mass_kg),
            _ => (250.0, 60.0, 0.25),
        })
        .unwrap_or((250.0, 60.0, 0.25));

    let bark_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.22, 0.12),
        perceptual_roughness: 0.95,
        ..default()
    });

    let fruit_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.92, 0.18, 0.26),
        emissive: LinearRgba::new(0.35, 0.06, 0.09, 1.0),
        perceptual_roughness: 0.3,
        ..default()
    });

    let fruit_mesh_handle = meshes.add(
        Sphere::new(0.20)
            .mesh()
            .ico(2)
            .expect("valid sphere mesh"),
    );

    let tree_coords = [
        (7.5, 6.5),
        (-10.0, 5.5),
        (11.0, -7.5),
        (-7.5, -11.0),
        (3.5, -9.0),
        (-4.5, 9.5),
    ];

    for (i, &(tx, tz)) in tree_coords.iter().enumerate() {
        let ground_y = evaluate_terrain_height(tx, tz, &terrain_config);
        let origin = Vec3::new(tx, ground_y, tz);

        let seed = 42 + i as u64 * 7919;
        let config = SpaceColonizationConfig::default();
        let (tree_mesh, fruit_sockets) = generate_space_colonization_tree(seed, &config);
        let tree_handle = meshes.add(tree_mesh);

        commands.spawn((
            Mesh3d(tree_handle),
            MeshMaterial3d(bark_material.clone()),
            Transform::from_translation(origin),
            ProceduralTree {
                seed,
                branch_count: 50,
                fruit_socket_count: fruit_sockets.len(),
            },
        ));

        // Spawn canopy fruits (which ripen and drop under gravity) + already-dropped ground fruits near trunk
        for (s_idx, socket) in fruit_sockets.iter().take(6).enumerate() {
            let is_already_fallen = s_idx < 3;
            let spawn_pos = if is_already_fallen {
                let fx = origin.x + socket.x * 0.65;
                let fz = origin.z + socket.z * 0.65;
                let fy = evaluate_terrain_height(fx, fz, &terrain_config) + 0.20;
                Vec3::new(fx, fy, fz)
            } else {
                origin + *socket
            };

            commands.spawn((
                Mesh3d(fruit_mesh_handle.clone()),
                MeshMaterial3d(fruit_material.clone()),
                Transform::from_translation(spawn_pos),
                PluckableFruit {
                    caloric_value: default_calories,
                    hydration_value: default_hydration,
                    mass_kg: default_mass,
                    is_attached: !is_already_fallen,
                    ripeness_timer: (s_idx as f32) * 4.0,
                    vertical_velocity: 0.0,
                },
            ));
        }
    }
}

/// Ripens canopy fruit so it detaches and falls under gravity onto the CDLOD terrain surface
pub fn update_falling_fruit_system(
    mut fruit_query: Query<(&mut Transform, &mut PluckableFruit)>,
    terrain_config: Res<TerrainConfig>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    for (mut tf, mut fruit) in &mut fruit_query {
        if fruit.is_attached {
            fruit.ripeness_timer += dt;
            if fruit.ripeness_timer > 18.0 {
                fruit.is_attached = false;
            }
        } else {
            let ground_y = evaluate_terrain_height(tf.translation.x, tf.translation.z, &terrain_config) + 0.20;
            if tf.translation.y > ground_y {
                fruit.vertical_velocity -= 9.81 * dt;
                tf.translation.y = (tf.translation.y + fruit.vertical_velocity * dt).max(ground_y);
                if tf.translation.y <= ground_y {
                    fruit.vertical_velocity = 0.0;
                }
            } else {
                tf.translation.y = ground_y;
            }
        }
    }
}
