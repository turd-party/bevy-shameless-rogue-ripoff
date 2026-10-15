#![allow(dead_code)]

use bevy::prelude::*;
use super::skeleton::BoneKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MuscleGroup {
    NeckExtensor,
    NeckFlexor,
    SpineErector,
    Abdominals,
    LeftBicep,
    LeftTricep,
    LeftDeltoid,
    RightBicep,
    RightTricep,
    RightDeltoid,
    LeftQuadricep,
    LeftHamstring,
    LeftGlute,
    LeftCalf,
    RightQuadricep,
    RightHamstring,
    RightGlute,
    RightCalf,
}

#[derive(Component, Debug, Clone)]
pub struct MuscleFiber {
    pub group: MuscleGroup,
    pub origin_bone: BoneKind,
    pub insertion_bone: BoneKind,
    pub origin_anchor: Vec3,
    pub insertion_anchor: Vec3,
    pub max_force_newtons: f32,
    pub optimal_length: f32,
    pub current_activation: f32,
    pub fiber_tear_damage: f32,
}

impl MuscleFiber {
    pub fn new(
        group: MuscleGroup,
        origin_bone: BoneKind,
        insertion_bone: BoneKind,
        origin_anchor: Vec3,
        insertion_anchor: Vec3,
        max_force_newtons: f32,
        optimal_length: f32,
    ) -> Self {
        Self {
            group,
            origin_bone,
            insertion_bone,
            origin_anchor,
            insertion_anchor,
            max_force_newtons,
            optimal_length,
            current_activation: 0.0,
            fiber_tear_damage: 0.0,
        }
    }

    /// Evaluates current contractile force based on Hill muscle model
    pub fn evaluate_contractile_force(&self, current_length: f32) -> f32 {
        let length_ratio = current_length / self.optimal_length.max(0.01);
        let force_length_multiplier = (-((length_ratio - 1.0) / 0.45).powi(2)).exp();
        
        let health_multiplier = (1.0 - self.fiber_tear_damage).clamp(0.0, 1.0);
        self.max_force_newtons * self.current_activation * force_length_multiplier * health_multiplier
    }

    pub fn apply_motor_spike(&mut self, spike_intensity: f32, dt: f32) {
        // Fast rise, smooth decay
        let target = spike_intensity.clamp(0.0, 1.0);
        let rate = if target > self.current_activation { 25.0 } else { 12.0 };
        self.current_activation += (target - self.current_activation) * rate * dt;
    }
}

#[derive(Component, Debug, Default)]
pub struct MusculoskeletalSystem {
    pub muscles: Vec<MuscleFiber>,
    pub total_metabolic_energy_cost: f32,
}

impl MusculoskeletalSystem {
    pub fn build_humanoid_muscles(scale: f32) -> Self {
        let mut muscles = Vec::with_capacity(18);

        // Spine & Neck
        muscles.push(MuscleFiber::new(
            MuscleGroup::SpineErector,
            BoneKind::Pelvis,
            BoneKind::Spine,
            Vec3::new(0.0, 0.0, -0.05 * scale),
            Vec3::new(0.0, 0.0, -0.05 * scale),
            1200.0 * scale.powi(2),
            0.28 * scale,
        ));
        muscles.push(MuscleFiber::new(
            MuscleGroup::Abdominals,
            BoneKind::Pelvis,
            BoneKind::Spine,
            Vec3::new(0.0, 0.0, 0.08 * scale),
            Vec3::new(0.0, 0.0, 0.08 * scale),
            1000.0 * scale.powi(2),
            0.28 * scale,
        ));
        muscles.push(MuscleFiber::new(
            MuscleGroup::NeckFlexor,
            BoneKind::Spine,
            BoneKind::Skull,
            Vec3::new(0.0, 0.15 * scale, 0.04 * scale),
            Vec3::new(0.0, -0.05 * scale, 0.06 * scale),
            400.0 * scale.powi(2),
            0.18 * scale,
        ));

        // Left Arm
        muscles.push(MuscleFiber::new(
            MuscleGroup::LeftDeltoid,
            BoneKind::Spine,
            BoneKind::LeftUpperArm,
            Vec3::new(-0.15 * scale, 0.18 * scale, 0.0),
            Vec3::new(0.0, 0.1 * scale, 0.0),
            600.0 * scale.powi(2),
            0.20 * scale,
        ));
        muscles.push(MuscleFiber::new(
            MuscleGroup::LeftBicep,
            BoneKind::LeftUpperArm,
            BoneKind::LeftForearm,
            Vec3::new(0.0, 0.05 * scale, 0.03 * scale),
            Vec3::new(0.0, 0.05 * scale, 0.03 * scale),
            550.0 * scale.powi(2),
            0.25 * scale,
        ));
        muscles.push(MuscleFiber::new(
            MuscleGroup::LeftTricep,
            BoneKind::LeftUpperArm,
            BoneKind::LeftForearm,
            Vec3::new(0.0, 0.05 * scale, -0.03 * scale),
            Vec3::new(0.0, 0.05 * scale, -0.03 * scale),
            600.0 * scale.powi(2),
            0.25 * scale,
        ));

        // Right Arm
        muscles.push(MuscleFiber::new(
            MuscleGroup::RightDeltoid,
            BoneKind::Spine,
            BoneKind::RightUpperArm,
            Vec3::new(0.15 * scale, 0.18 * scale, 0.0),
            Vec3::new(0.0, 0.1 * scale, 0.0),
            600.0 * scale.powi(2),
            0.20 * scale,
        ));
        muscles.push(MuscleFiber::new(
            MuscleGroup::RightBicep,
            BoneKind::RightUpperArm,
            BoneKind::RightForearm,
            Vec3::new(0.0, 0.05 * scale, 0.03 * scale),
            Vec3::new(0.0, 0.05 * scale, 0.03 * scale),
            550.0 * scale.powi(2),
            0.25 * scale,
        ));
        muscles.push(MuscleFiber::new(
            MuscleGroup::RightTricep,
            BoneKind::RightUpperArm,
            BoneKind::RightForearm,
            Vec3::new(0.0, 0.05 * scale, -0.03 * scale),
            Vec3::new(0.0, 0.05 * scale, -0.03 * scale),
            600.0 * scale.powi(2),
            0.25 * scale,
        ));

        // Left Leg
        muscles.push(MuscleFiber::new(
            MuscleGroup::LeftQuadricep,
            BoneKind::Pelvis,
            BoneKind::LeftShin,
            Vec3::new(-0.12 * scale, 0.0, 0.05 * scale),
            Vec3::new(0.0, 0.15 * scale, 0.04 * scale),
            1800.0 * scale.powi(2),
            0.45 * scale,
        ));
        muscles.push(MuscleFiber::new(
            MuscleGroup::LeftHamstring,
            BoneKind::Pelvis,
            BoneKind::LeftShin,
            Vec3::new(-0.12 * scale, 0.0, -0.05 * scale),
            Vec3::new(0.0, 0.15 * scale, -0.04 * scale),
            1400.0 * scale.powi(2),
            0.45 * scale,
        ));
        muscles.push(MuscleFiber::new(
            MuscleGroup::LeftCalf,
            BoneKind::LeftThigh,
            BoneKind::LeftShin,
            Vec3::new(0.0, -0.15 * scale, -0.04 * scale),
            Vec3::new(0.0, -0.15 * scale, -0.03 * scale),
            1100.0 * scale.powi(2),
            0.35 * scale,
        ));

        // Right Leg
        muscles.push(MuscleFiber::new(
            MuscleGroup::RightQuadricep,
            BoneKind::Pelvis,
            BoneKind::RightShin,
            Vec3::new(0.12 * scale, 0.0, 0.05 * scale),
            Vec3::new(0.0, 0.15 * scale, 0.04 * scale),
            1800.0 * scale.powi(2),
            0.45 * scale,
        ));
        muscles.push(MuscleFiber::new(
            MuscleGroup::RightHamstring,
            BoneKind::Pelvis,
            BoneKind::RightShin,
            Vec3::new(0.12 * scale, 0.0, -0.05 * scale),
            Vec3::new(0.0, 0.15 * scale, -0.04 * scale),
            1400.0 * scale.powi(2),
            0.45 * scale,
        ));
        muscles.push(MuscleFiber::new(
            MuscleGroup::RightCalf,
            BoneKind::RightThigh,
            BoneKind::RightShin,
            Vec3::new(0.0, -0.15 * scale, -0.04 * scale),
            Vec3::new(0.0, -0.15 * scale, -0.03 * scale),
            1100.0 * scale.powi(2),
            0.35 * scale,
        ));

        Self {
            muscles,
            total_metabolic_energy_cost: 0.0,
        }
    }
}
