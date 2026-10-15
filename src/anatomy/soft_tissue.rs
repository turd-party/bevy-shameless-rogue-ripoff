#![allow(dead_code)]

use bevy::prelude::*;
use super::skeleton::BoneKind;

#[derive(Debug, Clone)]
pub struct SoftTissueParticle {
    pub anchor_bone: BoneKind,
    pub local_offset: Vec3,
    pub current_displacement: Vec3,
    pub velocity: Vec3,
    pub mass_kg: f32,
    pub stiffness: f32,
    pub damping: f32,
}

#[derive(Component, Debug)]
pub struct SoftTissueViscera {
    pub particles: Vec<SoftTissueParticle>,
    pub blood_volume_liters: f32,
    pub max_blood_volume_liters: f32,
    pub bleeding_rate_lps: f32,
    pub coagulation_rate: f32,
}

impl SoftTissueViscera {
    pub fn new(scale: f32) -> Self {
        let max_blood = 5.0 * scale.powi(3);
        let mut particles = Vec::with_capacity(12);

        // Torso abdominal / visceral fat particles around Spine & Pelvis
        let offsets = [
            (BoneKind::Spine, Vec3::new(0.08 * scale, 0.0, 0.08 * scale)),
            (BoneKind::Spine, Vec3::new(-0.08 * scale, 0.0, 0.08 * scale)),
            (BoneKind::Spine, Vec3::new(0.0, 0.05 * scale, 0.10 * scale)),
            (BoneKind::Spine, Vec3::new(0.0, -0.05 * scale, -0.08 * scale)),
            (BoneKind::Pelvis, Vec3::new(0.06 * scale, 0.0, 0.06 * scale)),
            (BoneKind::Pelvis, Vec3::new(-0.06 * scale, 0.0, 0.06 * scale)),
            (BoneKind::LeftThigh, Vec3::new(0.0, 0.0, 0.04 * scale)),
            (BoneKind::RightThigh, Vec3::new(0.0, 0.0, 0.04 * scale)),
        ];

        for (bone, offset) in offsets {
            particles.push(SoftTissueParticle {
                anchor_bone: bone,
                local_offset: offset,
                current_displacement: Vec3::ZERO,
                velocity: Vec3::ZERO,
                mass_kg: 1.2 * scale.powi(3),
                stiffness: 120.0,
                damping: 8.0,
            });
        }

        Self {
            particles,
            blood_volume_liters: max_blood,
            max_blood_volume_liters: max_blood,
            bleeding_rate_lps: 0.0,
            coagulation_rate: 0.02,
        }
    }

    pub fn update_elastodynamics(&mut self, dt: f32) {
        for p in &mut self.particles {
            // Spring force towards anchor offset
            let spring_force = -p.stiffness * p.current_displacement;
            let damping_force = -p.damping * p.velocity;
            let net_force = spring_force + damping_force;

            let accel = net_force / p.mass_kg.max(0.01);
            p.velocity += accel * dt;
            p.current_displacement += p.velocity * dt;
        }

        // Blood loss and natural coagulation
        if self.bleeding_rate_lps > 0.0 {
            let lost = self.bleeding_rate_lps * dt;
            self.blood_volume_liters = (self.blood_volume_liters - lost).max(0.0);
            self.bleeding_rate_lps = (self.bleeding_rate_lps - self.coagulation_rate * dt).max(0.0);
        }
    }

    pub fn rupture_tissue(&mut self, trauma_severity: f32) {
        self.bleeding_rate_lps += (trauma_severity * 0.15).clamp(0.0, 1.5);
    }
}
