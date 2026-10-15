pub mod audio;
pub mod flora;
pub mod terrain;

use bevy::prelude::*;

pub struct ProcGenPlugin;

impl Plugin for ProcGenPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            terrain::TerrainPlugin,
            flora::FloraPlugin,
            audio::ProceduralAudioPlugin,
        ));
    }
}
