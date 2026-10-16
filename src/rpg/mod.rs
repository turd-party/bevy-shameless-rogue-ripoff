pub mod controller;
pub mod hud;
pub mod party;
pub mod rtwp;

use bevy::prelude::*;
use rtwp::SimulationPauseState;

/// Deterministic system sets for FixedUpdate simulation execution
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum SimulationSet {
    SensoryInput,
    NeuralPlasticityAndIntent,
    BiomechanicalActuation,
    MultiphysicsAndTerrain,
    GeneticsAndEcology,
}

pub struct RpgPlugin;

impl Plugin for RpgPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<SimulationPauseState>()
            .init_resource::<rtwp::RtwpController>()
            .configure_sets(
                FixedUpdate,
                (
                    SimulationSet::SensoryInput,
                    SimulationSet::NeuralPlasticityAndIntent,
                    SimulationSet::BiomechanicalActuation,
                    SimulationSet::MultiphysicsAndTerrain,
                    SimulationSet::GeneticsAndEcology,
                )
                    .chain()
                    .run_if(in_state(SimulationPauseState::Running)),
            )
            .add_systems(Startup, hud::spawn_hud_overlay)
            .add_systems(
                Update,
                (
                    rtwp::handle_rtwp_pause_input,
                    party::handle_party_selection_input,
                    party::handle_tactical_order_input,
                    hud::update_tactical_camera_system,
                    hud::update_party_selection_rings_system,
                    hud::update_hud_status_system,
                )
                    .chain(),
            )
            .add_systems(
                FixedUpdate,
                controller::update_party_character_controller
                    .in_set(SimulationSet::NeuralPlasticityAndIntent),
            );
    }
}
