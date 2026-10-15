pub mod ecology;
pub mod genome;
pub mod growth;
pub mod reproduction;

use bevy::prelude::*;

pub struct GeneticsPlugin;

impl Plugin for GeneticsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ecology::SoilFertilityGrid::new(4.0, 32, 32))
            .add_systems(
                Update,
                (
                    reproduction::update_reproductive_contacts,
                    reproduction::update_gestation_and_birth,
                    growth::update_child_growth_system,
                    ecology::update_corpse_decomposition_system,
                    ecology::update_plant_absorption_and_fruiting,
                ),
            );
    }
}
