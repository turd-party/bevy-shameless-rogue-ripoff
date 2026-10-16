#![allow(dead_code)]

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use super::skeleton::BoneKind;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
pub enum BiologicalSex {
    Male,
    Female,
}

#[derive(Component, Debug)]
pub struct MouthContactSensor {
    pub radius: f32,
    pub is_chewing: bool,
    pub chewing_timer: f32,
    pub food_in_mouth_calories: f32,
}

#[derive(Component, Debug)]
pub struct ReproductiveContactSensor {
    pub sex: BiologicalSex,
    pub sensor_radius: f32,
    pub local_anchor: Vec3,
    pub is_pregnant: bool,
    pub gestation_progress_secs: f32,
    pub gestation_duration_secs: f32,
    pub mating_cooldown_secs: f32,
}

impl ReproductiveContactSensor {
    pub fn new(sex: BiologicalSex, scale: f32) -> Self {
        let anchor = match sex {
            BiologicalSex::Male => Vec3::new(0.0, -0.05 * scale, 0.12 * scale),
            BiologicalSex::Female => Vec3::new(0.0, -0.05 * scale, 0.08 * scale),
        };

        Self {
            sex,
            sensor_radius: 0.10 * scale,
            local_anchor: anchor,
            is_pregnant: false,
            gestation_progress_secs: 0.0,
            gestation_duration_secs: 180.0,
            mating_cooldown_secs: 0.0,
        }
    }
}

/// Generates a low-poly beveled convex hull mesh for a specific bone segment
pub fn generate_bone_hull_mesh(kind: BoneKind, length: f32, radius: f32) -> Mesh {
    let half_l = length * 0.5;
    let r = radius;

    let (sx, sy, sz) = match kind {
        BoneKind::Skull => (r * 1.1, r * 1.2, r * 1.2),
        BoneKind::Spine | BoneKind::Ribcage => (r * 1.4, half_l, r * 1.1),
        BoneKind::Pelvis => (r * 1.5, half_l * 0.8, r * 1.2),
        BoneKind::LeftUpperArm | BoneKind::RightUpperArm => (r * 0.9, half_l, r * 0.9),
        BoneKind::LeftForearm | BoneKind::RightForearm => (r * 0.8, half_l, r * 0.8),
        BoneKind::LeftHand | BoneKind::RightHand => (r * 0.7, half_l * 0.6, r * 0.4),
        BoneKind::LeftThigh | BoneKind::RightThigh => (r * 1.1, half_l, r * 1.0),
        BoneKind::LeftShin | BoneKind::RightShin => (r * 0.9, half_l, r * 0.8),
        BoneKind::LeftFoot | BoneKind::RightFoot => (r * 0.8, half_l * 0.4, r * 1.4),
    };

    let positions = vec![
        // Top 4 vertices
        [-sx, sy, -sz],
        [sx, sy, -sz],
        [sx, sy, sz],
        [-sx, sy, sz],
        // Bottom 4 vertices
        [-sx, -sy, -sz],
        [sx, -sy, -sz],
        [sx, -sy, sz],
        [-sx, -sy, sz],
    ];

    let normals = vec![
        [-0.577, 0.577, -0.577],
        [0.577, 0.577, -0.577],
        [0.577, 0.577, 0.577],
        [-0.577, 0.577, 0.577],
        [-0.577, -0.577, -0.577],
        [0.577, -0.577, -0.577],
        [0.577, -0.577, 0.577],
        [-0.577, -0.577, 0.577],
    ];

    let uvs = vec![
        [0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0],
        [0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0],
    ];

    let indices = vec![
        // Top
        0, 2, 1, 0, 3, 2,
        // Bottom
        4, 5, 6, 4, 6, 7,
        // Front
        3, 6, 2, 3, 7, 6,
        // Back
        0, 1, 5, 0, 5, 4,
        // Left
        0, 4, 7, 0, 7, 3,
        // Right
        1, 2, 6, 1, 6, 5,
    ];

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    mesh
}
