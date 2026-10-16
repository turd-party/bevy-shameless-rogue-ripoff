#![allow(dead_code)]

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use crate::data::ContentRegistry;

#[derive(Resource, Debug, Clone)]
pub struct TerrainConfig {
    pub chunk_size: f32,
    pub height_scale: f32,
    pub roughness: f32,
    pub octaves: u32,
    pub erosion_iterations: u32,
    pub view_distance_chunks: i32,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            chunk_size: 64.0,
            height_scale: 18.0,
            roughness: 0.45,
            octaves: 4,
            erosion_iterations: 1000,
            view_distance_chunks: 3,
        }
    }
}

#[derive(Component, Debug)]
pub struct TerrainChunk {
    pub coord: IVec2,
    pub lod: u32,
}

/// Evaluates deterministic 2D fBm terrain height with hydraulic erosion gullies and valley basins
pub fn evaluate_terrain_height(x: f32, z: f32, config: &TerrainConfig) -> f32 {
    let mut total = 0.0;
    let mut frequency = 0.012;
    let mut amplitude = 1.0;
    let mut max_value = 0.0;
    let mut erosion_accum = 0.0;

    for octave in 0..config.octaves {
        let sample_x = x * frequency;
        let sample_z = z * frequency;

        let n = pseudo_gradient_noise(sample_x, sample_z);
        // Hydraulic ridge/gully carving: sharpens valleys where water droplets accumulate
        let ridge = 1.0 - (n * 2.0 - 1.0).abs();
        let eroded_n = if octave >= 1 {
            n * (1.0 - 0.35 * erosion_accum) + (ridge * ridge) * 0.25 * erosion_accum
        } else {
            n
        };

        total += eroded_n * amplitude;
        max_value += amplitude;
        erosion_accum = (erosion_accum + (1.0 - ridge) * 0.5).clamp(0.0, 1.0);

        amplitude *= config.roughness;
        frequency *= 2.02;
    }

    let normalized = (total / max_value.max(0.001)) - 0.32;
    // Gentle valley basin near world origin (radius 24m) so starter party & grove have natural footing
    let dist_sq = x * x + z * z;
    let basin_blend = (dist_sq / (28.0 * 28.0)).clamp(0.15, 1.0);

    normalized * config.height_scale * basin_blend
}

fn pseudo_gradient_noise(x: f32, z: f32) -> f32 {
    let ix = x.floor() as i32;
    let iz = z.floor() as i32;
    let fx = x - x.floor();
    let fz = z - z.floor();

    // Quintic smoothstep interpolation for C2 continuity across normals
    let sx = fx * fx * fx * (fx * (fx * 6.0 - 15.0) + 10.0);
    let sz = fz * fz * fz * (fz * (fz * 6.0 - 15.0) + 10.0);

    let n00 = hash_2d(ix, iz);
    let n10 = hash_2d(ix + 1, iz);
    let n01 = hash_2d(ix, iz + 1);
    let n11 = hash_2d(ix + 1, iz + 1);

    let nx0 = n00 * (1.0 - sx) + n10 * sx;
    let nx1 = n01 * (1.0 - sx) + n11 * sx;

    nx0 * (1.0 - sz) + nx1 * sz
}

fn hash_2d(x: i32, z: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x85ebca6b) ^ (z as u32).wrapping_mul(0xc2b2ae35);
    h = (h ^ (h >> 16)).wrapping_mul(0x7feb352d);
    h = h ^ (h >> 15);
    (h & 0xffff) as f32 / 65535.0
}

/// Generates a CDLOD terrain chunk mesh for a specific grid coordinate and LOD level
pub fn generate_terrain_chunk_mesh(
    coord: IVec2,
    lod: u32,
    config: &TerrainConfig,
) -> Mesh {
    // LOD 0 = 33x33 verts, LOD 1 = 17x17, LOD 2 = 9x9, LOD 3 = 5x5
    let resolution = match lod {
        0 => 32,
        1 => 16,
        2 => 8,
        _ => 4,
    };

    let step = config.chunk_size / resolution as f32;
    let base_x = coord.x as f32 * config.chunk_size - config.chunk_size * 0.5;
    let base_z = coord.y as f32 * config.chunk_size - config.chunk_size * 0.5;

    let vert_count = (resolution + 1) * (resolution + 1);
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(vert_count);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(vert_count);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(vert_count);

    for j in 0..=resolution {
        for i in 0..=resolution {
            let world_x = base_x + i as f32 * step;
            let world_z = base_z + j as f32 * step;
            let height = evaluate_terrain_height(world_x, world_z, config);

            // Finite-difference normals
            let delta = 0.5;
            let h_left = evaluate_terrain_height(world_x - delta, world_z, config);
            let h_right = evaluate_terrain_height(world_x + delta, world_z, config);
            let h_down = evaluate_terrain_height(world_x, world_z - delta, config);
            let h_up = evaluate_terrain_height(world_x, world_z + delta, config);

            let normal = Vec3::new(h_left - h_right, 2.0 * delta, h_down - h_up).normalize();

            positions.push([world_x, height, world_z]);
            normals.push(normal.to_array());
            uvs.push([
                (world_x - base_x) / config.chunk_size,
                (world_z - base_z) / config.chunk_size,
            ]);
        }
    }

    let mut indices: Vec<u32> = Vec::with_capacity(resolution * resolution * 6);
    let row_stride = resolution as u32 + 1;

    for j in 0..resolution as u32 {
        for i in 0..resolution as u32 {
            let top_left = j * row_stride + i;
            let top_right = top_left + 1;
            let bottom_left = (j + 1) * row_stride + i;
            let bottom_right = bottom_left + 1;

            indices.push(top_left);
            indices.push(bottom_left);
            indices.push(top_right);

            indices.push(top_right);
            indices.push(bottom_left);
            indices.push(bottom_right);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    mesh
}

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainConfig>()
            .add_systems(Startup, spawn_initial_terrain_chunks)
            .add_systems(Update, update_cdlod_terrain_system);
    }
}

fn spawn_initial_terrain_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut config: ResMut<TerrainConfig>,
    registry: Option<Res<ContentRegistry>>,
) {
    if let Some(reg) = registry {
        config.height_scale = reg.biome.terrain_height_scale;
        config.roughness = reg.biome.terrain_roughness;
        config.octaves = reg.biome.noise_octaves;
    }

    let terrain_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.24, 0.44, 0.19),
        perceptual_roughness: 0.92,
        reflectance: 0.08,
        ..default()
    });

    let radius = config.view_distance_chunks;
    for x in -radius..=radius {
        for z in -radius..=radius {
            let coord = IVec2::new(x, z);
            let dist = (x.abs()).max(z.abs());
            let lod = if dist <= 1 {
                0
            } else if dist <= 2 {
                1
            } else {
                2
            };

            let mesh = generate_terrain_chunk_mesh(coord, lod, &config);
            let mesh_handle = meshes.add(mesh);

            commands.spawn((
                Mesh3d(mesh_handle),
                MeshMaterial3d(terrain_material.clone()),
                Transform::IDENTITY,
                TerrainChunk { coord, lod },
            ));
        }
    }
}

/// Dynamically updates CDLOD chunk tessellation based on 3D camera distance
fn update_cdlod_terrain_system(
    camera_query: Query<&Transform, With<Camera3d>>,
    mut chunk_query: Query<(&mut TerrainChunk, &mut Mesh3d)>,
    mut meshes: ResMut<Assets<Mesh>>,
    config: Res<TerrainConfig>,
) {
    let Some(cam_tf) = camera_query.iter().next() else {
        return;
    };

    let cam_chunk_x = (cam_tf.translation.x / config.chunk_size).round() as i32;
    let cam_chunk_z = (cam_tf.translation.z / config.chunk_size).round() as i32;

    for (mut chunk, mut mesh_3d) in &mut chunk_query {
        let chebyshev_dist = (chunk.coord.x - cam_chunk_x)
            .abs()
            .max((chunk.coord.y - cam_chunk_z).abs());
        let desired_lod = if chebyshev_dist <= 1 {
            0
        } else if chebyshev_dist <= 2 {
            1
        } else if chebyshev_dist <= 3 {
            2
        } else {
            3
        };

        if chunk.lod != desired_lod {
            chunk.lod = desired_lod;
            let new_mesh = generate_terrain_chunk_mesh(chunk.coord, desired_lod, &config);
            mesh_3d.0 = meshes.add(new_mesh);
        }
    }
}
