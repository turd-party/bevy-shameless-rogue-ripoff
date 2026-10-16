#![allow(dead_code)]

use bevy::prelude::*;
use crate::anatomy::skeleton::HumanoidSkeleton;
use crate::procgen::flora::{PluckableFruit, ProceduralTree};
use crate::procgen::terrain::{evaluate_terrain_height, TerrainConfig};

#[derive(Component, Debug)]
pub struct StereoscopicEyes {
    pub interpupillary_distance: f32, // IPD (typically ~0.065m)
    pub fov_degrees: f32,
    pub max_depth_meters: f32,
    pub left_eye_depth: [f32; 64],  // 8x8 normalized depth map [0.0 = near, 1.0 = far/miss]
    pub right_eye_depth: [f32; 64], // 8x8 normalized depth map
    pub gaze_direction: Vec3,
    pub nearest_visible_fruit_dir: Option<Vec3>,
    pub nearest_visible_fruit_dist: f32,
}

impl StereoscopicEyes {
    pub fn new(scale: f32) -> Self {
        Self {
            interpupillary_distance: 0.065 * scale,
            fov_degrees: 90.0,
            max_depth_meters: 40.0 * scale.max(0.5),
            left_eye_depth: [1.0; 64],
            right_eye_depth: [1.0; 64],
            gaze_direction: Vec3::Z,
            nearest_visible_fruit_dir: None,
            nearest_visible_fruit_dist: f32::MAX,
        }
    }

    /// Evaluates 8x8 raycast grid for an eye against live world spherical colliders and procedural terrain heightfield
    pub fn cast_eye_rays(
        eye_origin: Vec3,
        gaze: Vec3,
        fov: f32,
        max_dist: f32,
        obstacles: &[(Vec3, f32)],
        terrain_config: &TerrainConfig,
    ) -> [f32; 64] {
        let mut depth_map = [1.0; 64];
        let forward = gaze.normalize_or_zero();
        let forward = if forward.length_squared() < 0.01 { Vec3::Z } else { forward };
        let up = Vec3::Y;
        let right = forward.cross(up).normalize_or_zero();
        let true_up = right.cross(forward).normalize_or_zero();

        let half_fov_rad = (fov * 0.5).to_radians();
        let tan_half_fov = half_fov_rad.tan();

        for row in 0..8 {
            for col in 0..8 {
                let idx = row * 8 + col;
                let ndc_x = (col as f32 / 7.0) * 2.0 - 1.0;
                let ndc_y = (row as f32 / 7.0) * 2.0 - 1.0;

                let ray_dir = (forward
                    + right * (ndc_x * tan_half_fov)
                    + true_up * (ndc_y * tan_half_fov))
                    .normalize_or_zero();

                let mut closest_hit = max_dist;

                // 1. Coarse ray-march against procedural CDLOD terrain heightfield (4 steps for real-time speed)
                if ray_dir.y < 0.15 {
                    for step_idx in 1..=4 {
                        let t = (step_idx as f32 / 4.0) * max_dist * 0.6;
                        if t >= closest_hit {
                            break;
                        }
                        let sample_pt = eye_origin + ray_dir * t;
                        let terrain_h = evaluate_terrain_height(sample_pt.x, sample_pt.z, terrain_config);
                        if sample_pt.y <= terrain_h {
                            closest_hit = t;
                            break;
                        }
                    }
                }

                // 2. Analytic ray-sphere intersection against nearby trees, fruits, and humans
                for (center, radius) in obstacles {
                    let oc = eye_origin - *center;
                    let b = oc.dot(ray_dir);
                    let c = oc.dot(oc) - radius * radius;
                    let discriminant = b * b - c;

                    if discriminant > 0.0 {
                        let t = -b - discriminant.sqrt();
                        if t > 0.15 && t < closest_hit {
                            closest_hit = t;
                        }
                    }
                }

                depth_map[idx] = (closest_hit / max_dist).clamp(0.0, 1.0);
            }
        }

        depth_map
    }
}

pub fn update_stereoscopic_eyes(
    mut eye_query: Query<(Entity, &Transform, &mut StereoscopicEyes)>,
    tree_query: Query<&Transform, (With<ProceduralTree>, Without<StereoscopicEyes>)>,
    fruit_query: Query<&Transform, (With<PluckableFruit>, Without<StereoscopicEyes>)>,
    human_query: Query<(Entity, &Transform), With<HumanoidSkeleton>>,
    terrain_config: Res<TerrainConfig>,
) {
    // Gather live world obstacles once per frame
    let mut world_obstacles: Vec<(Vec3, f32)> = Vec::with_capacity(64);
    for tree_tf in &tree_query {
        world_obstacles.push((tree_tf.translation + Vec3::new(0.0, 2.0, 0.0), 0.85));
    }

    let mut fruit_positions: Vec<Vec3> = Vec::with_capacity(64);
    for fruit_tf in &fruit_query {
        fruit_positions.push(fruit_tf.translation);
        world_obstacles.push((fruit_tf.translation, 0.25));
    }

    let human_positions: Vec<(Entity, Vec3)> = human_query
        .iter()
        .map(|(e, tf)| (e, tf.translation + Vec3::new(0.0, 1.0, 0.0)))
        .collect();

    for (self_entity, tf, mut eyes) in &mut eye_query {
        let scale = tf.scale.y.max(0.4);
        let head_pos = tf.translation + Vec3::new(0.0, 1.65 * scale, 0.0);
        let forward = tf.forward().as_vec3();
        let right = tf.right().as_vec3();
        eyes.gaze_direction = forward;

        let half_ipd = eyes.interpupillary_distance * 0.5;
        let left_eye_pos = head_pos - right * half_ipd;
        let right_eye_pos = head_pos + right * half_ipd;

        // Filter nearby obstacles within max vision range (excluding self)
        let max_dist = eyes.max_depth_meters;
        let mut local_obstacles: Vec<(Vec3, f32)> = world_obstacles
            .iter()
            .copied()
            .filter(|(pos, _)| pos.distance_squared(head_pos) <= max_dist * max_dist)
            .collect();

        for (other_e, other_pos) in &human_positions {
            if *other_e != self_entity && other_pos.distance_squared(head_pos) <= max_dist * max_dist {
                local_obstacles.push((*other_pos, 0.45));
            }
        }

        // Find nearest visible fruit within field of view
        let mut best_fruit_dist = f32::MAX;
        let mut best_fruit_dir = None;
        for fpos in &fruit_positions {
            let to_fruit = *fpos - tf.translation;
            let horiz_dist = Vec2::new(to_fruit.x, to_fruit.z).length();
            if horiz_dist < best_fruit_dist && horiz_dist <= max_dist {
                best_fruit_dist = horiz_dist;
                best_fruit_dir = Some(Vec3::new(to_fruit.x, 0.0, to_fruit.z).normalize_or_zero());
            }
        }
        eyes.nearest_visible_fruit_dist = best_fruit_dist;
        eyes.nearest_visible_fruit_dir = best_fruit_dir;

        let fov = eyes.fov_degrees;
        eyes.left_eye_depth = StereoscopicEyes::cast_eye_rays(
            left_eye_pos,
            forward,
            fov,
            max_dist,
            &local_obstacles,
            &terrain_config,
        );
        eyes.right_eye_depth = StereoscopicEyes::cast_eye_rays(
            right_eye_pos,
            forward,
            fov,
            max_dist,
            &local_obstacles,
            &terrain_config,
        );
    }
}
