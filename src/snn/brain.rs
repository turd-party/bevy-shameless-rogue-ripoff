#![allow(dead_code)]

use bevy::prelude::*;
use super::eyes::StereoscopicEyes;
use super::pheromones::PheromoneGrid3D;
use crate::anatomy::damage::MetabolicCaloricPool;
use crate::anatomy::muscles::{MuscleGroup, MusculoskeletalSystem};
use crate::anatomy::hull::{MouthContactSensor, ReproductiveContactSensor};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionIntent {
    Idle,
    ForageFood,
    EatInHand,
    SeekMate,
    FleeThreat,
}

#[derive(Component, Debug, Clone)]
pub struct EvolvedRewardMatrix {
    pub food_pleasure_weight: f32,
    pub mating_reward_weight: f32,
    pub pain_aversion_weight: f32,
    pub starvation_penalty_weight: f32,
    pub kin_affiliation_weight: f32,
}

impl Default for EvolvedRewardMatrix {
    fn default() -> Self {
        Self {
            food_pleasure_weight: 1.0,
            mating_reward_weight: 1.5,
            pain_aversion_weight: 2.0,
            starvation_penalty_weight: 1.2,
            kin_affiliation_weight: 0.8,
        }
    }
}

/// Module A: Executive Intent SNN
#[derive(Component, Debug)]
pub struct ExecutiveSnnModule {
    pub current_intent: ActionIntent,
    pub desired_heading: Vec2,
    pub desired_speed: f32,
    pub neuromodulator_dopamine: f32,
    pub neuromodulator_pain: f32,
}

impl Default for ExecutiveSnnModule {
    fn default() -> Self {
        Self {
            current_intent: ActionIntent::Idle,
            desired_heading: Vec2::Y,
            desired_speed: 0.0,
            neuromodulator_dopamine: 0.0,
            neuromodulator_pain: 0.0,
        }
    }
}

/// Module B: Locomotion & Gait SNN
#[derive(Component, Debug, Default)]
pub struct LocomotionSnnModule {
    pub gait_phase: f32, // Central Pattern Generator phase [0.0, 2*PI]
    pub left_leg_spike: f32,
    pub right_leg_spike: f32,
}

/// Module C: Manipulation SNN
#[derive(Component, Debug, Default)]
pub struct ManipulationSnnModule {
    pub arm_reach_progress: f32,
    pub is_holding_food: bool,
    pub bicep_spike: f32,
}

pub fn update_snn_executive_system(
    mut query: Query<(
        Entity,
        &Transform,
        &StereoscopicEyes,
        &MetabolicCaloricPool,
        &ReproductiveContactSensor,
        &EvolvedRewardMatrix,
        &mut ExecutiveSnnModule,
    )>,
    pheromones: Res<PheromoneGrid3D>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (_entity, tf, eyes, metabolism, repro, reward, mut exec) in &mut query {
        let chem = pheromones.sample(tf.translation);

        // Evaluate homeostatic urgency
        let hunger_level = (1.0 - metabolism.current_calories / metabolism.max_calories.max(1.0)).clamp(0.0, 1.0);
        let mating_urge = if !repro.is_pregnant && repro.mating_cooldown_secs <= 0.0 { 0.8 } else { 0.0 };

        // Sensory depth integration
        let mut min_left_depth = 1.0;
        let mut min_right_depth = 1.0;
        for i in 0..64 {
            if eyes.left_eye_depth[i] < min_left_depth { min_left_depth = eyes.left_eye_depth[i]; }
            if eyes.right_eye_depth[i] < min_right_depth { min_right_depth = eyes.right_eye_depth[i]; }
        }

        // Action Intent Selection modulated by evolved reward priors
        if hunger_level * reward.starvation_penalty_weight > 0.6 {
            exec.current_intent = ActionIntent::ForageFood;
            exec.desired_speed = 1.2;
            // Steer towards closer visual gradients or forward
            exec.desired_heading = Vec2::new(min_left_depth - min_right_depth, 1.0).normalize_or_zero();
        } else if mating_urge * reward.mating_reward_weight > 0.7 && chem.reproductive > 0.1 {
            exec.current_intent = ActionIntent::SeekMate;
            exec.desired_speed = 0.9;
            exec.desired_heading = Vec2::new(chem.reproductive, 1.0).normalize_or_zero();
        } else {
            exec.current_intent = ActionIntent::Idle;
            exec.desired_speed = 0.2;
        }

        // Decay neuromodulators
        exec.neuromodulator_dopamine = (exec.neuromodulator_dopamine - 0.5 * dt).max(0.0);
        exec.neuromodulator_pain = (exec.neuromodulator_pain - 1.0 * dt).max(0.0);
    }
}

pub fn update_snn_locomotion_system(
    mut query: Query<(
        Option<&ExecutiveSnnModule>,
        &mut LocomotionSnnModule,
        &mut MusculoskeletalSystem,
    )>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (maybe_exec, mut loco, mut muscles) in &mut query {
        let (speed, _heading) = if let Some(exec) = maybe_exec {
            (exec.desired_speed, exec.desired_heading)
        } else {
            // Player-controlled or externally guided
            (loco.left_leg_spike.max(loco.right_leg_spike), Vec2::Y)
        };

        if speed > 0.05 {
            let frequency = 4.0 * speed;
            loco.gait_phase = (loco.gait_phase + frequency * dt) % std::f32::consts::TAU;

            loco.left_leg_spike = (loco.gait_phase.sin().max(0.0)) * speed;
            loco.right_leg_spike = (((loco.gait_phase + std::f32::consts::PI).sin()).max(0.0)) * speed;
        } else {
            loco.left_leg_spike = 0.0;
            loco.right_leg_spike = 0.0;
        }

        // Apply motor spikes to skeletal leg muscles
        for m in &mut muscles.muscles {
            match m.group {
                MuscleGroup::LeftQuadricep | MuscleGroup::LeftCalf => {
                    m.apply_motor_spike(loco.left_leg_spike, dt);
                }
                MuscleGroup::RightQuadricep | MuscleGroup::RightCalf => {
                    m.apply_motor_spike(loco.right_leg_spike, dt);
                }
                _ => {}
            }
        }
    }
}

pub fn update_snn_manipulation_system(
    mut query: Query<(
        Option<&ExecutiveSnnModule>,
        &mut ManipulationSnnModule,
        &mut MouthContactSensor,
        &mut MusculoskeletalSystem,
    )>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (maybe_exec, mut manip, mut mouth, mut muscles) in &mut query {
        let is_eating = if let Some(exec) = maybe_exec {
            exec.current_intent == ActionIntent::EatInHand || mouth.is_chewing
        } else {
            mouth.is_chewing
        };

        if is_eating {
            manip.arm_reach_progress = (manip.arm_reach_progress + 2.0 * dt).clamp(0.0, 1.0);
            manip.bicep_spike = 0.85;

            // Chewing timer
            mouth.chewing_timer += dt;
            if mouth.chewing_timer > 1.2 {
                mouth.is_chewing = false;
                mouth.chewing_timer = 0.0;
                manip.arm_reach_progress = 0.0;
            }
        } else {
            manip.arm_reach_progress = (manip.arm_reach_progress - 1.5 * dt).max(0.0);
            manip.bicep_spike = 0.0;
        }

        for m in &mut muscles.muscles {
            if m.group == MuscleGroup::RightBicep || m.group == MuscleGroup::LeftBicep {
                m.apply_motor_spike(manip.bicep_spike, dt);
            }
        }
    }
}
