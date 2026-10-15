#![allow(dead_code)]

use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct ProceduralAudioEngine {
    pub sample_rate: u32,
}

impl ProceduralAudioEngine {
    pub fn new(sample_rate: u32) -> Self {
        Self { sample_rate }
    }

    /// Generates raw PCM mono samples for a physical collision impact
    pub fn synthesize_impact_sound(&self, hardness: f32, energy: f32) -> Vec<f32> {
        let duration_secs = 0.25;
        let total_samples = (self.sample_rate as f32 * duration_secs) as usize;
        let mut samples = Vec::with_capacity(total_samples);

        let base_freq = 80.0 + hardness * 220.0;
        let decay = 15.0 + (1.0 - hardness) * 10.0;

        for i in 0..total_samples {
            let t = i as f32 / self.sample_rate as f32;
            let envelope = (-decay * t).exp() * energy.clamp(0.0, 1.0);
            
            // Fundamental tone + dissonant harmonic
            let wave = (2.0 * std::f32::consts::PI * base_freq * t).sin()
                + 0.3 * (2.0 * std::f32::consts::PI * base_freq * 2.37 * t).sin();

            // Transient click noise
            let noise = if t < 0.008 {
                (hash_1d(i as u32) * 2.0 - 1.0) * 0.5
            } else {
                0.0
            };

            samples.push((wave * 0.7 + noise) * envelope);
        }

        samples
    }

    /// Generates raw PCM mono samples for chewing / eating
    pub fn synthesize_chewing_sound(&self) -> Vec<f32> {
        let duration_secs = 0.35;
        let total_samples = (self.sample_rate as f32 * duration_secs) as usize;
        let mut samples = Vec::with_capacity(total_samples);

        for i in 0..total_samples {
            let t = i as f32 / self.sample_rate as f32;
            let envelope = (-(t - 0.15).powi(2) / 0.01).exp();
            let noise = hash_1d(i as u32) * 2.0 - 1.0;
            let squelch = (2.0 * std::f32::consts::PI * 340.0 * t).sin() * 0.4;

            samples.push((noise * 0.6 + squelch) * envelope * 0.4);
        }

        samples
    }
}

fn hash_1d(n: u32) -> f32 {
    let mut h = n.wrapping_mul(0x7feb352d);
    h = (h ^ (h >> 15)).wrapping_mul(0x846ca68b);
    (h & 0xffff) as f32 / 65535.0
}

pub struct ProceduralAudioPlugin;

impl Plugin for ProceduralAudioPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ProceduralAudioEngine::new(44100));
    }
}
