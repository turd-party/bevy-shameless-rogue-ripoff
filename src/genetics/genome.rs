#![allow(dead_code)]

use bevy::prelude::*;
use crate::snn::brain::EvolvedRewardMatrix;

#[derive(Clone, Debug, Reflect)]
pub struct MorphologyChromosome {
    pub skin_r: f32,
    pub skin_g: f32,
    pub skin_b: f32,
    pub adult_height_scale: f32,
    pub limb_thickness_factor: f32,
    pub procedural_seed: u64,
}

impl Default for MorphologyChromosome {
    fn default() -> Self {
        Self {
            skin_r: 0.82,
            skin_g: 0.62,
            skin_b: 0.48,
            adult_height_scale: 1.0,
            limb_thickness_factor: 1.0,
            procedural_seed: 12345,
        }
    }
}

#[derive(Clone, Debug, Reflect)]
pub struct NeuralChromosome {
    pub baseline_weights: [f32; 8],
    pub stdp_learning_rate: f32,
    pub refractory_period_ms: f32,
}

impl Default for NeuralChromosome {
    fn default() -> Self {
        Self {
            baseline_weights: [0.5; 8],
            stdp_learning_rate: 0.05,
            refractory_period_ms: 2.0,
        }
    }
}

#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct HumanGenome {
    pub morphology: MorphologyChromosome,
    pub neural: NeuralChromosome,
    pub reward_valence: EvolvedRewardMatrix,
    pub generation: u32,
}

impl HumanGenome {
    /// Meiosis crossover & stochastic mutation between two parents
    pub fn recombine_and_mutate(
        parent_a: &HumanGenome,
        parent_b: &HumanGenome,
        mutation_rate: f32,
        variance: f32,
        seed: u64,
    ) -> Self {
        let mut rng = SimpleLcg::new(seed);

        let blend = rng.next_f32();

        let skin_r = rng.mutate(parent_a.morphology.skin_r * blend + parent_b.morphology.skin_r * (1.0 - blend), mutation_rate, variance).clamp(0.1, 1.0);
        let skin_g = rng.mutate(parent_a.morphology.skin_g * blend + parent_b.morphology.skin_g * (1.0 - blend), mutation_rate, variance).clamp(0.1, 1.0);
        let skin_b = rng.mutate(parent_a.morphology.skin_b * blend + parent_b.morphology.skin_b * (1.0 - blend), mutation_rate, variance).clamp(0.1, 1.0);
        let adult_height = rng.mutate((parent_a.morphology.adult_height_scale + parent_b.morphology.adult_height_scale) * 0.5, mutation_rate, variance).clamp(0.7, 1.3);
        let limb_thick = rng.mutate((parent_a.morphology.limb_thickness_factor + parent_b.morphology.limb_thickness_factor) * 0.5, mutation_rate, variance).clamp(0.7, 1.3);

        let mut child_weights = [0.5; 8];
        for i in 0..8 {
            let p_val = if rng.next_f32() > 0.5 { parent_a.neural.baseline_weights[i] } else { parent_b.neural.baseline_weights[i] };
            child_weights[i] = rng.mutate(p_val, mutation_rate, variance).clamp(-1.0, 1.0);
        }

        // Evolved reward valences crossover & mutation
        let food_pleasure = rng.mutate((parent_a.reward_valence.food_pleasure_weight + parent_b.reward_valence.food_pleasure_weight) * 0.5, mutation_rate, variance).clamp(0.1, 3.0);
        let mating_reward = rng.mutate((parent_a.reward_valence.mating_reward_weight + parent_b.reward_valence.mating_reward_weight) * 0.5, mutation_rate, variance).clamp(0.1, 3.0);
        let pain_aversion = rng.mutate((parent_a.reward_valence.pain_aversion_weight + parent_b.reward_valence.pain_aversion_weight) * 0.5, mutation_rate, variance).clamp(0.1, 4.0);
        let starvation_penalty = rng.mutate((parent_a.reward_valence.starvation_penalty_weight + parent_b.reward_valence.starvation_penalty_weight) * 0.5, mutation_rate, variance).clamp(0.1, 3.0);
        let kin_affiliation = rng.mutate((parent_a.reward_valence.kin_affiliation_weight + parent_b.reward_valence.kin_affiliation_weight) * 0.5, mutation_rate, variance).clamp(0.1, 2.0);

        Self {
            morphology: MorphologyChromosome {
                skin_r,
                skin_g,
                skin_b,
                adult_height_scale: adult_height,
                limb_thickness_factor: limb_thick,
                procedural_seed: seed ^ 0xabcdef01,
            },
            neural: NeuralChromosome {
                baseline_weights: child_weights,
                stdp_learning_rate: rng.mutate(parent_a.neural.stdp_learning_rate, mutation_rate, variance).clamp(0.001, 0.2),
                refractory_period_ms: rng.mutate(parent_a.neural.refractory_period_ms, mutation_rate, variance).clamp(1.0, 5.0),
            },
            reward_valence: EvolvedRewardMatrix {
                food_pleasure_weight: food_pleasure,
                mating_reward_weight: mating_reward,
                pain_aversion_weight: pain_aversion,
                starvation_penalty_weight: starvation_penalty,
                kin_affiliation_weight: kin_affiliation,
            },
            generation: parent_a.generation.max(parent_b.generation) + 1,
        }
    }
}

struct SimpleLcg {
    state: u64,
}

impl SimpleLcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_f32(&mut self) -> f32 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.state >> 32) as u32 as f32) / (u32::MAX as f32)
    }

    fn mutate(&mut self, val: f32, mutation_rate: f32, variance: f32) -> f32 {
        if self.next_f32() < mutation_rate {
            val + (self.next_f32() * 2.0 - 1.0) * variance
        } else {
            val
        }
    }
}
