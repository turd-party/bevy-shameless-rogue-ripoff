#![allow(dead_code)]

use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SimulationPauseState {
    #[default]
    Running,
    Paused,
}

#[derive(Resource, Debug, Default)]
pub struct RtwpController {
    pub is_paused: bool,
    pub pause_toggle_cooldown: f32,
}

pub fn handle_rtwp_pause_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut rtwp: ResMut<RtwpController>,
    mut time_virtual: ResMut<Time<Virtual>>,
    mut next_state: ResMut<NextState<SimulationPauseState>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        rtwp.is_paused = !rtwp.is_paused;

        if rtwp.is_paused {
            time_virtual.pause();
            next_state.set(SimulationPauseState::Paused);
            info!("[RTwP] Simulation PAUSED. Tactical orders can now be queued.");
        } else {
            time_virtual.unpause();
            next_state.set(SimulationPauseState::Running);
            info!("[RTwP] Simulation RESUMED.");
        }
    }
}
