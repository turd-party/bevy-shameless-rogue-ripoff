pub mod brain;
pub mod eyes;
pub mod pheromones;

use bevy::prelude::*;

pub struct SnnPlugin;

impl Plugin for SnnPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(pheromones::PheromoneGrid3D::new(4.0, IVec3::new(32, 8, 32)))
            .add_systems(
                Update,
                (
                    eyes::update_stereoscopic_eyes,
                    pheromones::update_pheromones_system,
                    brain::update_snn_executive_system,
                    brain::update_snn_locomotion_system,
                    brain::update_snn_manipulation_system,
                ),
            );
    }
}
