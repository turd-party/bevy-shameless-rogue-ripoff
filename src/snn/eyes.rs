#![allow(dead_code)]

use bevy::prelude::*;

#[derive(Component, Debug)]
pub struct StereoscopicEyes {
    pub interpupillary_distance: f32, // IPD (typically ~0.064m)
    pub fov_degrees: f32,
    pub max_depth_meters: f32,
    pub left_eye_depth: [f32; 64],  // 8x8 normalized depth map [0.0 = near, 1.0 = far/miss]
    pub right_eye_depth: [f32; 64], // 8x8 normalized depth map
    pub gaze_direction: Vec3,
}

impl StereoscopicEyes {
    pub fn new(scale: f32) -> Self {
        Self {
            interpupillary_distance: 0.065 * scale,
            fov_degrees: 85.0,
            max_depth_meters: 35.0 * scale,
            left_eye_depth: [1.0; 64],
            right_eye_depth: [1.0; 64],
            gaze_direction: Vec3::Z,
        }
    }

    /// Evaluates raycast grid for an eye given eye position, gaze direction, and terrain/colliders
    pub fn cast_eye_rays(
        eye_origin: Vec3,
        gaze: Vec3,
        fov: f32,
        max_dist: f32,
        obstacles: &[(Vec3, f32)], // Center & radius of nearby physical spheres/obstacles
    ) -> [f32; 64] {
        let mut depth_map = [1.0; 64];
        let forward = gaze.normalize();
        let up = Vec3::Y;
        let right = forward.cross(up).normalize();
        let true_up = right.cross(forward).normalize();

        let half_fov_rad = (fov * 0.5).to_radians();
        let tan_half_fov = half_fov_rad.tan();

        for row in 0..8 {
            for col in 0..8 {
                let idx = row * 8 + col;
                // Normalized Device Coordinates [-1.0, 1.0]
                let ndc_x = (col as f32 / 7.0) * 2.0 - 1.0;
                let ndc_y = (row as f32 / 7.0) * 2.0 - 1.0;

                let ray_dir = (forward + right * (ndc_x * tan_half_fov) + true_up * (ndc_y * tan_half_fov)).normalize();

                // Test against terrain ground plane (y = 0.0)
                let mut closest_hit = max_dist;
                if ray_dir.y < -0.001 {
                    let t_ground = -eye_origin.y / ray_dir.y;
                    if t_ground > 0.0 && t_ground < closest_hit {
                        closest_hit = t_ground;
                    }
                }

                // Test against obstacles
                for (center, radius) in obstacles {
                    let oc = eye_origin - *center;
                    let b = oc.dot(ray_dir);
                    let c = oc.dot(oc) - radius * radius;
                    let discriminant = b * b - c;

                    if discriminant > 0.0 {
                        let t = -b - discriminant.sqrt();
                        if t > 0.1 && t < closest_hit {
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
    mut query: Query<(&Transform, &mut StereoscopicEyes)>,
) {
    let dummy_obstacles = [
        (Vec3::new(0.0, 1.0, 5.0), 1.2),
        (Vec3::new(3.0, 1.5, 8.0), 2.0),
        (Vec3::new(-4.0, 0.5, 6.0), 0.8),
    ];

    for (tf, mut eyes) in &mut query {
        let head_pos = tf.translation + Vec3::new(0.0, 1.65, 0.0);
        let forward = tf.forward().as_vec3();
        let right = tf.right().as_vec3();

        let half_ipd = eyes.interpupillary_distance * 0.5;
        let left_eye_pos = head_pos - right * half_ipd;
        let right_eye_pos = head_pos + right * half_ipd;

        let fov = eyes.fov_degrees;
        let max_dist = eyes.max_depth_meters;

        eyes.left_eye_depth = StereoscopicEyes::cast_eye_rays(left_eye_pos, forward, fov, max_dist, &dummy_obstacles);
        eyes.right_eye_depth = StereoscopicEyes::cast_eye_rays(right_eye_pos, forward, fov, max_dist, &dummy_obstacles);
    }
}
