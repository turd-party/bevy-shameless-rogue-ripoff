pub mod brain;
pub mod eyes;
pub mod pheromones;

use bevy::prelude::*;
use crate::rpg::SimulationSet;

pub struct SnnPlugin;

impl Plugin for SnnPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(pheromones::PheromoneGrid3D::new(4.0, IVec3::new(32, 8, 32)))
            .add_systems(
                FixedUpdate,
                (
                    eyes::update_stereoscopic_eyes,
                    pheromones::update_pheromones_system,
                )
                    .chain()
                    .in_set(SimulationSet::SensoryInput),
            )
            .add_systems(
                FixedUpdate,
                brain::update_snn_executive_system
                    .in_set(SimulationSet::NeuralPlasticityAndIntent),
            )
            .add_systems(
                FixedUpdate,
                (
                    brain::update_snn_locomotion_system,
                    brain::update_snn_manipulation_system,
                )
                    .chain()
                    .in_set(SimulationSet::BiomechanicalActuation),
            );
    }
}
