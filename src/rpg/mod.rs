pub mod controller;
pub mod party;
pub mod rtwp;

use bevy::prelude::*;

pub struct RpgPlugin;

impl Plugin for RpgPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<rtwp::SimulationPauseState>()
            .init_resource::<rtwp::RtwpController>()
            .add_systems(
                Update,
                (
                    rtwp::handle_rtwp_pause_input,
                    party::handle_party_selection_input,
                    party::handle_tactical_order_input,
                    controller::update_party_character_controller,
                ),
            );
    }
}
