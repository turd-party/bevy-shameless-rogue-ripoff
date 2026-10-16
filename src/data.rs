#![allow(dead_code)]

use std::fs;
use std::path::Path;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use crate::anatomy::hull::BiologicalSex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanArchetype {
    pub species_id: String,
    pub metabolic_rate_per_sec: f32,
    pub base_caloric_capacity: f32,
    pub starvation_threshold: f32,
    pub gestation_duration_secs: f32,
    pub infant_spawn_scale: f32,
    pub adult_maturity_scale: f32,
    pub eye_vision_resolution: (u32, u32),
    pub eye_field_of_view_degrees: f32,
    pub eye_max_range_meters: f32,
    pub hearing_range_meters: f32,
    pub olfactory_decay_half_life_secs: f32,
    pub genetic_mutation_rate: f32,
    pub genetic_mutation_variance: f32,
}

impl Default for HumanArchetype {
    fn default() -> Self {
        Self {
            species_id: "human_sapiens".into(),
            metabolic_rate_per_sec: 1.2,
            base_caloric_capacity: 1000.0,
            starvation_threshold: 0.0,
            gestation_duration_secs: 180.0,
            infant_spawn_scale: 0.4,
            adult_maturity_scale: 1.0,
            eye_vision_resolution: (8, 8),
            eye_field_of_view_degrees: 90.0,
            eye_max_range_meters: 40.0,
            hearing_range_meters: 30.0,
            olfactory_decay_half_life_secs: 15.0,
            genetic_mutation_rate: 0.05,
            genetic_mutation_variance: 0.15,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ItemCategory {
    Food {
        caloric_value: f32,
        hydration_value: f32,
        chewing_duration_secs: f32,
    },
    Tool {
        sharpness: f32,
        durability: f32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDefinition {
    pub id: String,
    pub name: String,
    pub category: ItemCategory,
    pub physical_mass_kg: f32,
    pub bounding_radius: f32,
    pub procedural_mesh_seed: u64,
    pub procedural_hue: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ItemRegistry {
    pub items: Vec<ItemDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeDefinition {
    pub biome_id: String,
    pub ambient_temperature_celsius: f32,
    pub soil_base_fertility: f32,
    pub corpse_decomposition_rate_per_sec: f32,
    pub tree_density_per_hectare: f32,
    pub fruit_bloom_frequency_secs: f32,
    pub terrain_height_scale: f32,
    pub terrain_roughness: f32,
    pub noise_octaves: u32,
    pub gravity: (f32, f32, f32),
}

impl Default for BiomeDefinition {
    fn default() -> Self {
        Self {
            biome_id: "temperate_wilderness".into(),
            ambient_temperature_celsius: 21.0,
            soil_base_fertility: 1.0,
            corpse_decomposition_rate_per_sec: 0.05,
            tree_density_per_hectare: 45.0,
            fruit_bloom_frequency_secs: 90.0,
            terrain_height_scale: 18.0,
            terrain_roughness: 0.45,
            noise_octaves: 4,
            gravity: (0.0, -9.81, 0.0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyMemberTemplate {
    pub designation: String,
    pub sex: BiologicalSex,
    pub caloric_reserve: f32,
    pub morphology_seed: u64,
    pub neural_seed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyConfig {
    pub starting_party_size: usize,
    pub starting_party_members: Vec<PartyMemberTemplate>,
    pub tactical_order_queue_limit: usize,
    pub selection_ring_color_rgba: (f32, f32, f32, f32),
}

impl Default for PartyConfig {
    fn default() -> Self {
        Self {
            starting_party_size: 2,
            starting_party_members: vec![
                PartyMemberTemplate {
                    designation: "Vanguard Leader".into(),
                    sex: BiologicalSex::Male,
                    caloric_reserve: 800.0,
                    morphology_seed: 101,
                    neural_seed: 202,
                },
                PartyMemberTemplate {
                    designation: "Apothecary Scout".into(),
                    sex: BiologicalSex::Female,
                    caloric_reserve: 850.0,
                    morphology_seed: 303,
                    neural_seed: 404,
                },
            ],
            tactical_order_queue_limit: 8,
            selection_ring_color_rgba: (0.2, 0.9, 0.3, 0.8),
        }
    }
}

/// Central resource holding all parsed `.ron` data-driven definitions
#[derive(Resource, Debug, Clone, Default)]
pub struct ContentRegistry {
    pub human_archetype: HumanArchetype,
    pub items: ItemRegistry,
    pub biome: BiomeDefinition,
    pub party_config: PartyConfig,
    pub reload_timer: f32,
}

impl ContentRegistry {
    pub fn load_from_disk() -> Self {
        let human_archetype = load_ron_file::<HumanArchetype>("assets/data/archetypes/human_baseline.ron")
            .unwrap_or_default();
        let items = load_ron_file::<ItemRegistry>("assets/data/items/food_and_tools.ron")
            .unwrap_or_default();
        let biome = load_ron_file::<BiomeDefinition>("assets/data/biomes/temperate_wilderness.ron")
            .unwrap_or_default();
        let party_config = load_ron_file::<PartyConfig>("assets/data/rpg/party_starter.ron")
            .unwrap_or_default();

        info!(
            "[ContentRegistry] Loaded RON data: archetype='{}', items={}, biome='{}', party_members={}",
            human_archetype.species_id,
            items.items.len(),
            biome.biome_id,
            party_config.starting_party_members.len()
        );

        Self {
            human_archetype,
            items,
            biome,
            party_config,
            reload_timer: 0.0,
        }
    }

    pub fn primary_fruit_item(&self) -> Option<&ItemDefinition> {
        self.items
            .items
            .iter()
            .find(|item| matches!(item.category, ItemCategory::Food { .. }))
    }
}

fn load_ron_file<T: for<'de> Deserialize<'de>>(rel_path: &str) -> Option<T> {
    let path = Path::new(rel_path);
    match fs::read_to_string(path) {
        Ok(contents) => match ron::de::from_str::<T>(&contents) {
            Ok(parsed) => Some(parsed),
            Err(err) => {
                warn!("Failed to parse RON file '{}': {}", rel_path, err);
                None
            }
        },
        Err(err) => {
            warn!("Could not read RON file '{}': {}", rel_path, err);
            None
        }
    }
}

pub struct DataContentPlugin;

impl Plugin for DataContentPlugin {
    fn build(&self, app: &mut App) {
        let registry = ContentRegistry::load_from_disk();
        app.insert_resource(registry)
            .add_systems(Update, hot_reload_ron_system);
    }
}

fn hot_reload_ron_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut registry: ResMut<ContentRegistry>,
) {
    // Press F5 at any time to hot-reload all .ron files from disk
    if keyboard.just_pressed(KeyCode::F5) {
        *registry = ContentRegistry::load_from_disk();
        info!("[ContentRegistry] Hot-reloaded all .ron files via F5!");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genetics::genome::HumanGenome;
    use crate::physics::NexusPhysicsResource;
    use crate::procgen::flora::{generate_space_colonization_tree, SpaceColonizationConfig};
    use crate::procgen::terrain::{evaluate_terrain_height, generate_terrain_chunk_mesh, TerrainConfig};
    use crate::snn::brain::{ExecutiveSnnModule, LocomotionSnnModule, ManipulationSnnModule};

    #[test]
    fn all_four_ron_files_parse_cleanly() {
        let h: HumanArchetype =
            load_ron_file("assets/data/archetypes/human_baseline.ron").expect("human_baseline.ron");
        assert_eq!(h.species_id, "human_sapiens");
        assert_eq!(h.eye_vision_resolution, (8, 8));

        let items: ItemRegistry =
            load_ron_file("assets/data/items/food_and_tools.ron").expect("food_and_tools.ron");
        assert_eq!(items.items.len(), 3);

        let biome: BiomeDefinition =
            load_ron_file("assets/data/biomes/temperate_wilderness.ron")
                .expect("temperate_wilderness.ron");
        assert_eq!(biome.biome_id, "temperate_wilderness");

        let party: PartyConfig =
            load_ron_file("assets/data/rpg/party_starter.ron").expect("party_starter.ron");
        assert_eq!(party.starting_party_members.len(), 2);
    }

    #[test]
    fn nexus3d_multibody_and_mpm_registration_steps_cleanly() {
        let mut nexus = NexusPhysicsResource::default();
        let binding = nexus.register_humanoid_multibody(Vec3::new(0.0, 2.0, 0.0), 1.0);
        assert_eq!(binding.mpm_group_id, 1);
        assert!(!nexus.mpm_staging_particles.is_empty());

        let counts = nexus.state.counts();
        assert!(counts.rigid_bodies >= 6, "ground + humanoid multibody links");
        assert!(counts.multibodies >= 1, "humanoid multibody registered");

        // Step the underlying Rapier/Nexus world
        nexus.state.rbd_world_mut_untracked(0).step();
    }

    #[test]
    fn ferromorphic_modular_snn_spikes_and_plasticity_adapts() {
        let mut exec = ExecutiveSnnModule::new_with_seed(42);
        let initial_w0 = exec.sim.net.w[0];

        // Drive sensory afferents above LIF rheobase
        let ext = [5.0e-9, 4.5e-9, 4.0e-9, 4.0e-9, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let mut total_fired = 0;
        for _ in 0..25 {
            let fired = exec.sim.step(&ext);
            total_fired += fired.len();
            for &n_idx in &fired {
                let pre = n_idx as usize;
                for syn_idx in exec.sim.net.offset[pre]..exec.sim.net.offset[pre + 1] {
                    let w = exec.sim.net.w[syn_idx];
                    let _ = exec.stdp_synapses[syn_idx].on_pre(w);
                    let _ = exec.stdp_synapses[syn_idx].on_post(w);
                }
            }
        }
        assert!(total_fired > 0, "LIF network must emit spikes under suprathreshold sensory current");

        // Deliver dopamine reward and advance 3-factor RewardStdp
        exec.stdp_synapses[0].reward(1.0).unwrap();
        let updated_w0 = exec.stdp_synapses[0].advance(initial_w0, 0.02).unwrap();
        assert!(
            (updated_w0 - initial_w0).abs() > 1e-9,
            "RewardStdp must modify synaptic weight when eligibility trace and dopamine co-occur"
        );

        let mut loco = LocomotionSnnModule::default();
        let mut loco_spikes = 0;
        for _ in 0..25 {
            loco_spikes += loco.sim.step(&[5.0e-9, 5.0e-9, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]).len();
        }
        assert!(loco_spikes > 0);

        let mut manip = ManipulationSnnModule::default();
        let mut manip_spikes = 0;
        for _ in 0..25 {
            manip_spikes += manip.sim.step(&[5.0e-9, 5.0e-9, 0.0, 0.0, 0.0, 0.0]).len();
        }
        assert!(manip_spikes > 0);
    }

    #[test]
    fn procedural_terrain_flora_and_genetics_work() {
        let t_cfg = TerrainConfig::default();
        let h = evaluate_terrain_height(10.0, -5.0, &t_cfg);
        assert!(h.is_finite());

        let chunk_mesh = generate_terrain_chunk_mesh(IVec2::ZERO, 1, &t_cfg);
        assert!(chunk_mesh.count_vertices() > 0);

        let (tree_mesh, sockets) =
            generate_space_colonization_tree(12345, &SpaceColonizationConfig::default());
        assert!(tree_mesh.count_vertices() > 0);
        assert!(!sockets.is_empty());

        let parent_a = HumanGenome::default();
        let mut parent_b = HumanGenome::default();
        parent_b.reward_valence.food_pleasure_weight = 2.5;
        let child = HumanGenome::recombine_and_mutate(&parent_a, &parent_b, 0.5, 0.2, 999);
        assert_eq!(child.generation, 1);
    }
}

