#![allow(dead_code)]

use bevy::prelude::*;
use super::genome::HumanGenome;
use crate::anatomy::hull::{BiologicalSex, ReproductiveContactSensor};
use crate::anatomy::damage::MetabolicCaloricPool;
use crate::anatomy::spawn_biomechanical_human;

#[derive(Component, Debug)]
pub struct GestatingMother {
    pub embryo_genome: HumanGenome,
}

pub fn update_reproductive_contacts(
    mut query: Query<(
        Entity,
        &Transform,
        &mut ReproductiveContactSensor,
        &HumanGenome,
        &MetabolicCaloricPool,
    )>,
    mut commands: Commands,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    // Collect mature fertile individuals
    let candidates: Vec<(Entity, Vec3, BiologicalSex, HumanGenome, bool, f32)> = query
        .iter()
        .map(|(e, tf, repro, genome, _metabolism)| {
            (
                e,
                tf.translation + repro.local_anchor,
                repro.sex,
                genome.clone(),
                repro.is_pregnant,
                repro.mating_cooldown_secs,
            )
        })
        .collect();

    for (e, tf, mut repro, _genome, metabolism) in &mut query {
        if repro.mating_cooldown_secs > 0.0 {
            repro.mating_cooldown_secs = (repro.mating_cooldown_secs - dt).max(0.0);
        }

        // Only females can become pregnant if not already and well-nourished
        if repro.sex == BiologicalSex::Female && !repro.is_pregnant && repro.mating_cooldown_secs <= 0.0 && metabolism.current_calories > 300.0 {
            let my_repro_pos = tf.translation + repro.local_anchor;

            // Search for nearby male whose reproductive organ collides
            for (other_e, other_pos, other_sex, other_genome, _, other_cooldown) in &candidates {
                if *other_e != e && *other_sex == BiologicalSex::Male && *other_cooldown <= 0.0 {
                    let dist = my_repro_pos.distance(*other_pos);
                    let contact_threshold = repro.sensor_radius + 0.12;

                    if dist <= contact_threshold {
                        // Physical reproductive contact achieved!
                        repro.is_pregnant = true;
                        repro.gestation_progress_secs = 0.0;
                        repro.mating_cooldown_secs = 240.0;

                        let child_genome = HumanGenome::recombine_and_mutate(
                            _genome,
                            other_genome,
                            0.05,
                            0.15,
                            e.to_bits() ^ other_e.to_bits() ^ 0x9e3779b97f4a7c15,
                        );

                        commands.entity(e).insert(GestatingMother {
                            embryo_genome: child_genome,
                        });

                        info!("Physical reproductive contact! Female {:?} fertilized by Male {:?}", e, other_e);
                        break;
                    }
                }
            }
        }
    }
}

pub fn update_gestation_and_birth(
    mut query: Query<(
        Entity,
        &Transform,
        &mut ReproductiveContactSensor,
        &mut MetabolicCaloricPool,
        Option<&GestatingMother>,
    )>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (mother_entity, tf, mut repro, mut metabolism, maybe_gestating) in &mut query {
        if repro.is_pregnant {
            repro.gestation_progress_secs += dt;

            // Pregnancy consumes extra metabolic energy
            metabolism.current_calories = (metabolism.current_calories - 0.4 * dt).max(0.0);

            if repro.gestation_progress_secs >= repro.gestation_duration_secs {
                repro.is_pregnant = false;
                repro.gestation_progress_secs = 0.0;

                if let Some(gestating) = maybe_gestating {
                    let child_genome = gestating.embryo_genome.clone();
                    let child_sex = if (child_genome.morphology.procedural_seed & 1) == 0 {
                        BiologicalSex::Male
                    } else {
                        BiologicalSex::Female
                    };

                    let child_pos = tf.translation + Vec3::new(0.4, 0.0, 0.4);
                    let skin_tone = Color::srgb(
                        child_genome.morphology.skin_r,
                        child_genome.morphology.skin_g,
                        child_genome.morphology.skin_b,
                    );

                    let child_entity = spawn_biomechanical_human(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        child_pos,
                        child_sex,
                        0.4, // Spawn at 0.4x infant scale
                        skin_tone,
                    );

                    commands.entity(child_entity).insert((
                        child_genome,
                        super::growth::ChildGrowth {
                            current_scale: 0.4,
                            target_adult_scale: 1.0,
                            growth_rate_per_sec: 0.015, // reaches maturity in ~40 seconds of sustained feeding
                        },
                    ));

                    commands.entity(mother_entity).remove::<GestatingMother>();
                    info!("Child born to mother {:?} at {:?}!", mother_entity, child_pos);
                }
            }
        }
    }
}
