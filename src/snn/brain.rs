#![allow(dead_code)]

use bevy::prelude::*;
use ferromorphic::{
    plasticity::{Bounds, PairStdp, RewardStdp, WeightRule},
    reinforce::{Actor, Critic},
    Lif, Mode, NetBuilder, Rng, Sim,
};

use super::eyes::StereoscopicEyes;
use super::pheromones::{PheromoneGrid3D, PheromoneType};
use crate::anatomy::damage::MetabolicCaloricPool;
use crate::anatomy::hull::{BiologicalSex, MouthContactSensor, ReproductiveContactSensor};
use crate::anatomy::muscles::{MuscleGroup, MusculoskeletalSystem};
use crate::procgen::audio::ProceduralAudioEngine;
use crate::procgen::flora::PluckableFruit;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionIntent {
    Idle,
    ForageFood,
    EatInHand,
    SeekMate,
    FleeThreat,
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct EvolvedRewardMatrix {
    pub food_pleasure_weight: f32,
    pub mating_reward_weight: f32,
    pub pain_aversion_weight: f32,
    pub starvation_penalty_weight: f32,
    pub kin_affiliation_weight: f32,
}

impl Default for EvolvedRewardMatrix {
    fn default() -> Self {
        Self {
            food_pleasure_weight: 1.2,
            mating_reward_weight: 1.5,
            pain_aversion_weight: 2.0,
            starvation_penalty_weight: 1.4,
            kin_affiliation_weight: 0.8,
        }
    }
}

/// Module A: Executive Intent SNN powered by `ferromorphic::Sim<Lif>`, `Actor`/`Critic` TD(lambda), and `RewardStdp`
#[derive(Component, Debug)]
pub struct ExecutiveSnnModule {
    pub sim: Sim<Lif>,
    pub stdp_synapses: Vec<RewardStdp>,
    pub critic: Critic,
    pub actor: Actor,
    pub rng: Rng,
    pub last_state_idx: usize,
    pub current_intent: ActionIntent,
    pub desired_heading: Vec2,
    pub desired_speed: f32,
    pub neuromodulator_dopamine: f32,
    pub neuromodulator_pain: f32,
    pub total_spikes: u64,
    pub firing_rate_hz: f32,
}

impl Default for ExecutiveSnnModule {
    fn default() -> Self {
        Self::new_with_seed(2026)
    }
}

impl ExecutiveSnnModule {
    pub fn new_with_seed(seed: u64) -> Self {
        // 12 LIF neurons:
        // 0..4: Sensory Afferents (Hunger, MateScent, LeftVisualProximity, RightVisualProximity)
        // 4..8: Associative Interneurons
        // 8..12: Efferent Intent Neurons (ForageFood, EatInHand, SeekMate, FleeThreat)
        let n = 12;
        let mut b = NetBuilder::new(n);
        for s in 0..4u32 {
            for h in 4..8u32 {
                let _ = b.connect(s, h, 18e-3, 1);
            }
        }
        for h in 4..8u32 {
            for out in 8..12u32 {
                let _ = b.connect(h, out, 22e-3, 1);
            }
        }
        let net = b.build();
        let n_syn = net.n_syn;
        let sim = Sim::new(net, vec![Lif::default(); n], 1e-3, Mode::Clocked)
            .expect("valid executive Lif SNN");

        let bounds = Bounds::new(2e-3, 60e-3).expect("valid bounds");
        let pair = PairStdp::bi_poo_2001(1.5e-3, 1.6e-3, WeightRule::SoftBound { mu: 0.5 }, bounds)
            .expect("valid Bi-Poo STDP");
        let rstdp = RewardStdp::new(pair, 0.8, 0.25, bounds).expect("valid RewardStdp");

        // 8 discrete homeostatic states x 4 executive actions for TD(lambda) Actor-Critic
        let critic = Critic::new(8, 0.15, 0.92, 0.8).expect("valid TD critic");
        let actor = Actor::new(8, 4, 0.25, 0.92 * 0.8).expect("valid policy actor");

        Self {
            sim,
            stdp_synapses: vec![rstdp; n_syn],
            critic,
            actor,
            rng: Rng::new(seed.max(1)),
            last_state_idx: 0,
            current_intent: ActionIntent::Idle,
            desired_heading: Vec2::Y,
            desired_speed: 0.0,
            neuromodulator_dopamine: 0.0,
            neuromodulator_pain: 0.0,
            total_spikes: 0,
            firing_rate_hz: 0.0,
        }
    }
}

/// Module B: Locomotion & Gait CPG SNN powered by `ferromorphic::Sim<Lif>`
#[derive(Component, Debug)]
pub struct LocomotionSnnModule {
    pub sim: Sim<Lif>,
    pub gait_phase: f32,
    pub left_leg_spike: f32,
    pub right_leg_spike: f32,
    pub total_spikes: u64,
}

impl Default for LocomotionSnnModule {
    fn default() -> Self {
        // 8 LIF neurons forming a rhythmic half-center oscillator CPG:
        // 0, 1: Descending velocity command afferents
        // 2, 3: Mutually-inhibitory rhythm generator interneurons
        // 4, 5: Left leg flexor/extensor motor neurons
        // 6, 7: Right leg flexor/extensor motor neurons
        let n = 8;
        let mut b = NetBuilder::new(n);
        let _ = b.connect(0, 2, 24e-3, 1);
        let _ = b.connect(1, 3, 24e-3, 1);
        // Mutual inhibition between half-centers 2 and 3
        let _ = b.connect(2, 3, -18e-3, 2);
        let _ = b.connect(3, 2, -18e-3, 2);
        // Drive motor neurons
        let _ = b.connect(2, 4, 25e-3, 1);
        let _ = b.connect(2, 5, 20e-3, 2);
        let _ = b.connect(3, 6, 25e-3, 1);
        let _ = b.connect(3, 7, 20e-3, 2);

        let net = b.build();
        let sim = Sim::new(net, vec![Lif::default(); n], 1e-3, Mode::Clocked)
            .expect("valid locomotion Lif SNN");

        Self {
            sim,
            gait_phase: 0.0,
            left_leg_spike: 0.0,
            right_leg_spike: 0.0,
            total_spikes: 0,
        }
    }
}

/// Module C: Manipulation & Hand-to-Mouth SNN powered by `ferromorphic::Sim<Lif>`
#[derive(Component, Debug)]
pub struct ManipulationSnnModule {
    pub sim: Sim<Lif>,
    pub arm_reach_progress: f32,
    pub is_holding_food: bool,
    pub held_food_calories: f32,
    pub bicep_spike: f32,
    pub total_spikes: u64,
}

impl Default for ManipulationSnnModule {
    fn default() -> Self {
        let n = 6;
        let mut b = NetBuilder::new(n);
        let _ = b.connect(0, 2, 24e-3, 1);
        let _ = b.connect(1, 3, 24e-3, 1);
        let _ = b.connect(2, 4, 26e-3, 1);
        let _ = b.connect(3, 5, 26e-3, 1);
        let net = b.build();
        let sim = Sim::new(net, vec![Lif::default(); n], 1e-3, Mode::Clocked)
            .expect("valid manipulation Lif SNN");

        Self {
            sim,
            arm_reach_progress: 0.0,
            is_holding_food: false,
            held_food_calories: 0.0,
            bicep_spike: 0.0,
            total_spikes: 0,
        }
    }
}

pub fn update_snn_executive_system(
    mut human_query: Query<(
        Entity,
        &mut Transform,
        &StereoscopicEyes,
        &MetabolicCaloricPool,
        &ReproductiveContactSensor,
        &ManipulationSnnModule,
        &EvolvedRewardMatrix,
        &mut ExecutiveSnnModule,
    )>,
    mut pheromones: ResMut<PheromoneGrid3D>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    // Collect positions of all humans with ReproductiveContactSensor for mate-seeking navigation
    let all_humans: Vec<(Entity, Vec3, BiologicalSex, bool)> = human_query
        .iter()
        .map(|(e, tf, _, meta, repro, _, _, _)| (e, tf.translation, repro.sex, meta.is_dead))
        .collect();

    for (entity, mut tf, eyes, metabolism, repro, manip, reward, mut exec) in &mut human_query {
        if metabolism.is_dead {
            exec.desired_speed = 0.0;
            continue;
        }

        // Emit pheromones into 3D grid
        pheromones.deposit(tf.translation, PheromoneType::KinScent, 0.25 * dt);
        if !repro.is_pregnant && repro.mating_cooldown_secs <= 0.0 && metabolism.current_calories > 350.0 {
            pheromones.deposit(tf.translation, PheromoneType::ReproductiveReadiness, 0.45 * dt);
        }

        let chem = pheromones.sample(tf.translation);

        // Compute homeostatic drives
        let hunger_level = (1.0 - metabolism.current_calories / metabolism.max_calories.max(1.0)).clamp(0.0, 1.0);
        let mating_urge = if !repro.is_pregnant && repro.mating_cooldown_secs <= 0.0 && tf.scale.y >= 0.85 {
            0.85
        } else {
            0.0
        };

        // Visual depth summary from stereoscopic 8x8 grids
        let mut min_left_depth = 1.0f32;
        let mut min_right_depth = 1.0f32;
        for i in 0..64 {
            min_left_depth = min_left_depth.min(eyes.left_eye_depth[i]);
            min_right_depth = min_right_depth.min(eyes.right_eye_depth[i]);
        }

        // Inject sensory currents (in Amperes, ~2-5 nA above LIF rheobase) into neurons 0..4
        let mut ext_currents = [0.0f64; 12];
        ext_currents[0] = (hunger_level as f64 * 4.5e-9).max(0.5e-9);
        ext_currents[1] = ((mating_urge * (chem.reproductive + 0.3)) as f64 * 4.0e-9).max(0.2e-9);
        ext_currents[2] = ((1.0 - min_left_depth) as f64 * 3.5e-9).max(0.2e-9);
        ext_currents[3] = ((1.0 - min_right_depth) as f64 * 3.5e-9).max(0.2e-9);

        // Step ferromorphic LIF spiking network for 5 millisecond sub-ticks
        let mut step_spikes = 0u64;
        for _ in 0..5 {
            let fired = exec.sim.step(&ext_currents);
            step_spikes += fired.len() as u64;

            // Update 3-factor RewardStdp eligibility traces on active synapses
            for &neuron_idx in &fired {
                let pre = neuron_idx as usize;
                if pre + 1 < exec.sim.net.offset.len() {
                    let start = exec.sim.net.offset[pre];
                    let end = exec.sim.net.offset[pre + 1];
                    for syn_idx in start..end {
                        let w = exec.sim.net.w[syn_idx];
                        if let Some(rstdp) = exec.stdp_synapses.get_mut(syn_idx) {
                            let _ = rstdp.on_pre(w);
                            let _ = rstdp.on_post(w);
                        }
                    }
                }
            }
        }
        exec.total_spikes += step_spikes;
        exec.firing_rate_hz = exec.firing_rate_hz * 0.9 + (step_spikes as f32 / dt.max(1e-3)) * 0.1;

        // Broadcast 3rd-factor neuromodulator (dopamine - pain) weighted by EvolvedRewardMatrix
        let net_reward = (exec.neuromodulator_dopamine * reward.food_pleasure_weight)
            - (exec.neuromodulator_pain * reward.pain_aversion_weight)
            - (if metabolism.is_starving { 0.5 * reward.starvation_penalty_weight } else { 0.0 });

        if net_reward.abs() > 0.001 {
            let syn_count = exec.stdp_synapses.len();
            for syn_idx in 0..syn_count {
                let current_w = exec.sim.net.w[syn_idx];
                let _ = exec.stdp_synapses[syn_idx].reward(net_reward as f64 * 0.05);
                if let Ok(new_w) =
                    exec.stdp_synapses[syn_idx].advance(current_w, (dt as f64).min(0.05))
                {
                    exec.sim.net.w[syn_idx] = new_w;
                }
            }
        }

        // Encode discrete homeostatic state (0..8) for TD(lambda) Actor-Critic
        let state_idx = ((if hunger_level > 0.35 { 4 } else { 0 })
            | (if manip.is_holding_food { 2 } else { 0 })
            | (if mating_urge > 0.5 { 1 } else { 0 }))
        .min(7);

        let prev_state = exec.last_state_idx;
        let mut rng_clone = exec.rng.clone();
        let chosen_action = exec.actor.act(state_idx, &mut rng_clone).unwrap_or(0);
        exec.rng = rng_clone;

        if let Ok(td_delta) = exec.critic.update(prev_state, net_reward as f64, state_idx, false) {
            exec.actor.reinforce(td_delta);
        }
        exec.last_state_idx = state_idx;

        // Combine Actor-Critic policy with homeostatic survival imperatives
        if manip.is_holding_food {
            exec.current_intent = ActionIntent::EatInHand;
            exec.desired_speed = 0.0;
        } else if hunger_level * reward.starvation_penalty_weight > 0.25 || chosen_action == 0 {
            exec.current_intent = ActionIntent::ForageFood;
            exec.desired_speed = 1.15;

            if let Some(fruit_dir) = eyes.nearest_visible_fruit_dir {
                exec.desired_heading = Vec2::new(fruit_dir.x, fruit_dir.z).normalize_or_zero();
                let target_rot = Quat::from_rotation_y(fruit_dir.x.atan2(fruit_dir.z));
                tf.rotation = tf.rotation.slerp(target_rot, 6.0 * dt);
                tf.translation += fruit_dir * (3.2 * exec.desired_speed * dt);
            } else {
                // Exploratory foraging walk
                let forward = tf.forward().as_vec3();
                tf.rotation *= Quat::from_rotation_y(0.4 * dt);
                tf.translation += Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero() * (2.2 * dt);
            }
        } else if mating_urge * reward.mating_reward_weight > 0.5 || chosen_action == 2 {
            exec.current_intent = ActionIntent::SeekMate;
            exec.desired_speed = 1.0;

            // Find nearest opposite-sex live adult human
            let mut closest_mate_dir = None;
            let mut closest_mate_dist = f32::MAX;
            for (other_e, other_pos, other_sex, other_dead) in &all_humans {
                if *other_e != entity && *other_sex != repro.sex && !*other_dead {
                    let delta = *other_pos - tf.translation;
                    let d = Vec2::new(delta.x, delta.z).length();
                    if d < closest_mate_dist {
                        closest_mate_dist = d;
                        closest_mate_dir = Some(Vec3::new(delta.x, 0.0, delta.z).normalize_or_zero());
                    }
                }
            }

            if let Some(mate_dir) = closest_mate_dir {
                exec.desired_heading = Vec2::new(mate_dir.x, mate_dir.z);
                let target_rot = Quat::from_rotation_y(mate_dir.x.atan2(mate_dir.z));
                tf.rotation = tf.rotation.slerp(target_rot, 6.0 * dt);
                if closest_mate_dist > 0.15 {
                    tf.translation += mate_dir * (2.8 * dt);
                }
            }
        } else {
            exec.current_intent = ActionIntent::Idle;
            exec.desired_speed = 0.15;
        }

        // Decay neuromodulators
        exec.neuromodulator_dopamine = (exec.neuromodulator_dopamine - 0.5 * dt).max(0.0);
        exec.neuromodulator_pain = (exec.neuromodulator_pain - 1.0 * dt).max(0.0);
    }
}

pub fn update_snn_locomotion_system(
    mut query: Query<(
        Option<&ExecutiveSnnModule>,
        &mut LocomotionSnnModule,
        &mut MusculoskeletalSystem,
    )>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    for (maybe_exec, mut loco, mut muscles) in &mut query {
        let speed = if let Some(exec) = maybe_exec {
            exec.desired_speed
        } else {
            // Player-controlled party member: speed driven by controller gait commands
            loco.left_leg_spike.max(loco.right_leg_spike)
        };

        // Drive ferromorphic CPG LIF neurons
        let drive_current = (speed as f64 * 3.8e-9).clamp(0.0, 8.0e-9);
        let ext = [
            drive_current,
            drive_current * 0.9,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        ];
        for _ in 0..4 {
            let fired = loco.sim.step(&ext);
            loco.total_spikes += fired.len() as u64;
        }

        if speed > 0.05 {
            let frequency = 4.5 * speed;
            loco.gait_phase = (loco.gait_phase + frequency * dt) % std::f32::consts::TAU;

            loco.left_leg_spike = (loco.gait_phase.sin().max(0.0)) * speed;
            loco.right_leg_spike =
                (((loco.gait_phase + std::f32::consts::PI).sin()).max(0.0)) * speed;
        } else {
            loco.left_leg_spike = 0.0;
            loco.right_leg_spike = 0.0;
        }

        // Apply motor spikes to Hill-type skeletal leg & posture muscles
        for m in &mut muscles.muscles {
            match m.group {
                MuscleGroup::LeftQuadricep | MuscleGroup::LeftCalf => {
                    m.apply_motor_spike(loco.left_leg_spike, dt);
                }
                MuscleGroup::RightQuadricep | MuscleGroup::RightCalf => {
                    m.apply_motor_spike(loco.right_leg_spike, dt);
                }
                MuscleGroup::SpineErector | MuscleGroup::Abdominals => {
                    m.apply_motor_spike(0.35 + speed * 0.25, dt);
                }
                _ => {}
            }
        }
    }
}

pub fn update_snn_manipulation_system(
    mut commands: Commands,
    mut human_query: Query<(
        Entity,
        &Transform,
        Option<&mut ExecutiveSnnModule>,
        &mut ManipulationSnnModule,
        &mut MouthContactSensor,
        &mut MetabolicCaloricPool,
        &mut MusculoskeletalSystem,
    )>,
    fruit_query: Query<(Entity, &Transform, &PluckableFruit)>,
    audio_engine: Res<ProceduralAudioEngine>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    // Track claimed fruits this frame so two entities don't pluck the same fruit simultaneously
    let mut claimed_fruits: Vec<Entity> = Vec::new();

    for (_human_entity, tf, mut maybe_exec, mut manip, mut mouth, mut metabolism, mut muscles) in
        &mut human_query
    {
        if metabolism.is_dead {
            continue;
        }

        // 1. Check if the human is hungry or foraging and near a physical PluckableFruit to grasp
        let wants_food = if let Some(ref exec) = maybe_exec {
            matches!(exec.current_intent, ActionIntent::ForageFood | ActionIntent::EatInHand)
                || metabolism.current_calories < metabolism.max_calories * 0.85
        } else {
            // Player party member automatically plucks & eats when hungry or when ordered near fruit
            metabolism.current_calories < metabolism.max_calories * 0.90 || manip.is_holding_food
        };

        if wants_food && !manip.is_holding_food && !mouth.is_chewing {
            for (fruit_entity, fruit_tf, fruit) in &fruit_query {
                if claimed_fruits.contains(&fruit_entity) {
                    continue;
                }
                let horiz_dist = Vec2::new(
                    fruit_tf.translation.x - tf.translation.x,
                    fruit_tf.translation.z - tf.translation.z,
                )
                .length();

                // Within arm reach horizontally (including fallen or low-hanging fruit)
                if horiz_dist <= 2.4 {
                    claimed_fruits.push(fruit_entity);
                    manip.is_holding_food = true;
                    manip.held_food_calories = fruit.caloric_value;
                    manip.arm_reach_progress = 0.2;
                    commands.entity(fruit_entity).despawn();
                    info!(
                        "Human grasped physical fruit ({:.0} kcal) into hand!",
                        fruit.caloric_value
                    );
                    break;
                }
            }
        }

        // Step manipulation LIF network
        let ext = [
            if manip.is_holding_food { 4.2e-9 } else { 0.0 },
            if mouth.is_chewing { 3.8e-9 } else { 0.0 },
            0.0,
            0.0,
            0.0,
            0.0,
        ];
        for _ in 0..3 {
            let fired = manip.sim.step(&ext);
            manip.total_spikes += fired.len() as u64;
        }

        // 2. Hand-to-mouth physical trajectory: flex bicep to bring grasped fruit to MouthContactSensor
        if manip.is_holding_food {
            manip.arm_reach_progress = (manip.arm_reach_progress + 1.8 * dt).clamp(0.0, 1.0);
            manip.bicep_spike = 0.92;

            // When arm_reach_progress reaches 1.0, hand touches MouthContactSensor!
            if manip.arm_reach_progress >= 1.0 {
                manip.is_holding_food = false;
                mouth.is_chewing = true;
                mouth.chewing_timer = 0.0;
                mouth.food_in_mouth_calories = manip.held_food_calories.max(150.0);
                manip.held_food_calories = 0.0;

                // Synthesize real-time procedural chewing audio DSP waveform
                let _pcm_samples = audio_engine.synthesize_chewing_sound();
                info!(
                    "Hand delivered food to MouthContactSensor! Chewing {:.0} kcal...",
                    mouth.food_in_mouth_calories
                );
            }
        } else if mouth.is_chewing {
            // Chewing motion oscillates jaw/bicep and digests calories into MetabolicCaloricPool
            manip.bicep_spike = 0.45 + 0.25 * (mouth.chewing_timer * 12.0).sin();
            mouth.chewing_timer += dt;

            if mouth.chewing_timer >= 1.2 {
                let gained = mouth.food_in_mouth_calories;
                metabolism.current_calories =
                    (metabolism.current_calories + gained).min(metabolism.max_calories);
                mouth.is_chewing = false;
                mouth.chewing_timer = 0.0;
                mouth.food_in_mouth_calories = 0.0;
                manip.arm_reach_progress = 0.0;

                // Deliver positive dopamine reinforcement signal to Executive SNN
                if let Some(ref mut exec) = maybe_exec {
                    exec.neuromodulator_dopamine += 1.5;
                }
                info!(
                    "Finished chewing! Metabolic pool replenished (+{:.0} kcal -> {:.0}/{:.0} kcal).",
                    gained, metabolism.current_calories, metabolism.max_calories
                );
            }
        } else {
            manip.arm_reach_progress = (manip.arm_reach_progress - 1.5 * dt).max(0.0);
            manip.bicep_spike = 0.0;
        }

        // Drive upper-limb muscles from manipulation SNN
        for m in &mut muscles.muscles {
            if matches!(
                m.group,
                MuscleGroup::RightBicep
                    | MuscleGroup::LeftBicep
                    | MuscleGroup::RightDeltoid
                    | MuscleGroup::LeftDeltoid
            ) {
                m.apply_motor_spike(manip.bicep_spike, dt);
            }
        }
    }
}
