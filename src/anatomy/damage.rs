#![allow(dead_code)]

use bevy::prelude::*;
use super::skeleton::{BoneKind, BoneSegment, HumanoidSkeleton};
use super::soft_tissue::SoftTissueViscera;

#[derive(Component, Debug)]
pub struct MetabolicCaloricPool {
    pub current_calories: f32,
    pub max_calories: f32,
    pub baseline_burn_rate_per_sec: f32,
    pub starvation_damage_rate: f32,
    pub is_starving: bool,
    pub is_dead: bool,
}

impl MetabolicCaloricPool {
    pub fn new(max_calories: f32, baseline_burn: f32) -> Self {
        Self {
            current_calories: max_calories * 0.85,
            max_calories,
            baseline_burn_rate_per_sec: baseline_burn,
            starvation_damage_rate: 1.5,
            is_starving: false,
            is_dead: false,
        }
    }
}

#[derive(Event, Debug, Clone)]
pub struct NociceptivePainEvent {
    pub entity: Entity,
    pub bone: BoneKind,
    pub intensity: f32,
}

#[derive(Event, Debug, Clone)]
pub struct BoneFractureEvent {
    pub entity: Entity,
    pub bone: BoneKind,
    pub impact_force: f32,
}

pub fn update_metabolism_system(
    mut commands: Commands,
    mut query: Query<(Entity, &mut MetabolicCaloricPool, &mut SoftTissueViscera, &mut HumanoidSkeleton)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (entity, mut metabolism, mut viscera, mut skeleton) in &mut query {
        if metabolism.is_dead {
            continue;
        }

        // Caloric decay from base metabolism + bleeding drain
        let bleeding_drain = viscera.bleeding_rate_lps * 45.0; // severe blood loss accelerates metabolic crash
        let total_burn = (metabolism.baseline_burn_rate_per_sec + bleeding_drain) * dt;
        
        metabolism.current_calories = (metabolism.current_calories - total_burn).max(0.0);

        if metabolism.current_calories <= 0.0 {
            metabolism.is_starving = true;
            
            // Starvation emits nociceptive visceral pain into SNN
            commands.trigger(NociceptivePainEvent {
                entity,
                bone: BoneKind::Spine,
                intensity: 0.75,
            });

            // Lethal threshold
            if viscera.blood_volume_liters < 2.0 {
                metabolism.is_dead = true;
                skeleton.is_ragdoll = true;
            }
        } else {
            metabolism.is_starving = false;
        }

        viscera.update_elastodynamics(dt);
    }
}

pub fn check_skeletal_fractures(
    mut commands: Commands,
    mut bone_query: Query<(Entity, &mut BoneSegment)>,
) {
    for (entity, mut bone) in &mut bone_query {
        if !bone.is_fractured && bone.current_stress > bone.max_stress_threshold {
            bone.is_fractured = true;
            
            commands.trigger(BoneFractureEvent {
                entity,
                bone: bone.kind,
                impact_force: bone.current_stress,
            });

            commands.trigger(NociceptivePainEvent {
                entity,
                bone: bone.kind,
                intensity: 1.0, // Maximum pain spike
            });
        }
        
        // Decay instantaneous stress
        bone.current_stress *= 0.85;
    }
}
