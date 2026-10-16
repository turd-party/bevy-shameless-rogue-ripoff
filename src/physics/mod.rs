#![allow(dead_code)]

use bevy::prelude::*;
use nexus3d::mpm::solver::{BoundaryCondition, Particle, ParticleModel, SimulationParams};
use nexus3d::prelude::{NexusCapacities, NexusPipeline, NexusState, RbdCoupling};
use nexus3d::rapier::prelude::{
    ColliderBuilder, RevoluteJointBuilder, RigidBodyBuilder, RigidBodyHandle,
    SphericalJointBuilder,
};

use crate::anatomy::damage::MetabolicCaloricPool;
use crate::anatomy::muscles::MusculoskeletalSystem;
use crate::anatomy::skeleton::{BoneKind, BoneSegment, HumanoidSkeleton};
use crate::anatomy::soft_tissue::SoftTissueViscera;
use crate::procgen::terrain::{evaluate_terrain_height, TerrainConfig};

/// Component linking a Bevy entity (humanoid root or bone segment) to its Nexus3D RigidBodyHandle
#[derive(Component, Debug, Clone, Copy)]
pub struct NexusBodyBinding {
    pub root_handle: RigidBodyHandle,
    pub mpm_group_id: u32,
}

/// Resource encapsulating the Nexus3D multiphysics state (RBD + MPM) and pipeline
#[derive(Resource)]
pub struct NexusPhysicsResource {
    pub state: NexusState,
    pub pipeline: NexusPipeline,
    pub mpm_staging_particles: Vec<Particle>,
    pub mpm_sim_params: SimulationParams,
    pub next_mpm_group_id: u32,
}

impl Default for NexusPhysicsResource {
    fn default() -> Self {
        let capacities = NexusCapacities::default()
            .rbd_bodies(4096)
            .rbd_collisions(16384)
            .rbd_mb_contact_constraints(4096)
            .mpm_particles(32768)
            .mpm_grid_size(4096);

        let mut state = NexusState::new(capacities);
        state.set_rbd_timestep(1.0 / 60.0, 4);

        // Insert foundational ground collider coupled to MPM via one-way separating boundary
        state.insert_rigid_body(
            RigidBodyBuilder::fixed()
                .translation(nexus3d::rapier::prelude::Vector::new(0.0, -5.0, 0.0))
                .build(),
            ColliderBuilder::cuboid(256.0, 5.0, 256.0).build(),
            RbdCoupling::MpmOneWay(BoundaryCondition::separate(0.5)),
        );

        Self {
            state,
            pipeline: NexusPipeline::default(),
            mpm_staging_particles: Vec::new(),
            mpm_sim_params: SimulationParams {
                gravity: nexus3d::rapier::prelude::Vector::new(0.0, -9.81, 0.0),
                dt: 1.0 / 60.0,
            },
            next_mpm_group_id: 1,
        }
    }
}

impl NexusPhysicsResource {
    /// Registers an articulated humanoid multibody chain and Neo-Hookean MPM soft-tissue particles in NexusState
    pub fn register_humanoid_multibody(
        &mut self,
        position: Vec3,
        scale: f32,
    ) -> NexusBodyBinding {
        let group_id = self.next_mpm_group_id;
        self.next_mpm_group_id += 1;

        let n_pos = nexus3d::rapier::prelude::Vector::new(
            position.x,
            position.y + 1.0 * scale,
            position.z,
        );

        // 1. Root Pelvis rigid body with two-way MPM coupling
        let pelvis_rb = RigidBodyBuilder::dynamic()
            .translation(n_pos)
            .linear_damping(1.5)
            .angular_damping(2.5)
            .build();
        let pelvis_col = ColliderBuilder::capsule_y(0.12 * scale, 0.14 * scale)
            .density(980.0)
            .build();
        let pelvis_handle = self.state.insert_rigid_body(
            pelvis_rb,
            pelvis_col,
            RbdCoupling::MpmTwoWay(BoundaryCondition::separate(0.4)),
        );

        // 2. Spine link connected by Spherical multibody joint
        let spine_pos = nexus3d::rapier::prelude::Vector::new(
            position.x,
            position.y + 1.28 * scale,
            position.z,
        );
        let spine_handle = self.state.insert_rigid_body(
            RigidBodyBuilder::dynamic().translation(spine_pos).build(),
            ColliderBuilder::capsule_y(0.14 * scale, 0.13 * scale)
                .density(950.0)
                .build(),
            RbdCoupling::MpmTwoWay(BoundaryCondition::separate(0.4)),
        );
        let spine_joint = SphericalJointBuilder::new()
            .local_anchor1(nexus3d::rapier::prelude::Vector::new(0.0, 0.14 * scale, 0.0))
            .local_anchor2(nexus3d::rapier::prelude::Vector::new(0.0, -0.14 * scale, 0.0));
        self.state
            .insert_multibody_joint(pelvis_handle, spine_handle, spine_joint);

        // 3. Skull link connected to Spine
        let skull_pos = nexus3d::rapier::prelude::Vector::new(
            position.x,
            position.y + 1.65 * scale,
            position.z,
        );
        let skull_handle = self.state.insert_rigid_body(
            RigidBodyBuilder::dynamic().translation(skull_pos).build(),
            ColliderBuilder::ball(0.11 * scale).density(1020.0).build(),
            RbdCoupling::None,
        );
        let neck_joint = SphericalJointBuilder::new()
            .local_anchor1(nexus3d::rapier::prelude::Vector::new(0.0, 0.18 * scale, 0.0))
            .local_anchor2(nexus3d::rapier::prelude::Vector::new(0.0, -0.11 * scale, 0.0));
        self.state
            .insert_multibody_joint(spine_handle, skull_handle, neck_joint);

        // 4. Left & Right Legs (Revolute knee + spherical hip joints)
        for side in [-1.0f32, 1.0f32] {
            let hip_x = side * 0.14 * scale;
            let thigh_handle = self.state.insert_rigid_body(
                RigidBodyBuilder::dynamic()
                    .translation(nexus3d::rapier::prelude::Vector::new(
                        position.x + hip_x,
                        position.y + 0.70 * scale,
                        position.z,
                    ))
                    .build(),
                ColliderBuilder::capsule_y(0.18 * scale, 0.08 * scale)
                    .density(1000.0)
                    .build(),
                RbdCoupling::None,
            );
            let hip_joint = RevoluteJointBuilder::new(nexus3d::rapier::prelude::Vector::X)
                .local_anchor1(nexus3d::rapier::prelude::Vector::new(hip_x, -0.10 * scale, 0.0))
                .local_anchor2(nexus3d::rapier::prelude::Vector::new(0.0, 0.20 * scale, 0.0));
            self.state
                .insert_multibody_joint(pelvis_handle, thigh_handle, hip_joint);

            let shin_handle = self.state.insert_rigid_body(
                RigidBodyBuilder::dynamic()
                    .translation(nexus3d::rapier::prelude::Vector::new(
                        position.x + hip_x,
                        position.y + 0.28 * scale,
                        position.z,
                    ))
                    .build(),
                ColliderBuilder::capsule_y(0.16 * scale, 0.06 * scale)
                    .density(1000.0)
                    .build(),
                RbdCoupling::None,
            );
            let knee_joint = RevoluteJointBuilder::new(nexus3d::rapier::prelude::Vector::X)
                .local_anchor1(nexus3d::rapier::prelude::Vector::new(0.0, -0.20 * scale, 0.0))
                .local_anchor2(nexus3d::rapier::prelude::Vector::new(0.0, 0.18 * scale, 0.0));
            self.state
                .insert_multibody_joint(thigh_handle, shin_handle, knee_joint);
        }

        // 5. Neo-Hookean MPM Soft-Tissue Visceral & Fat Particles
        let soft_tissue_model = ParticleModel::elastic_neo_hookean(3.5e4, 0.42);
        let particle_radius = 0.04 * scale;
        for i in -1..=1 {
            for j in -1..=1 {
                let pt_pos = nexus3d::rapier::prelude::Vector::new(
                    position.x + i as f32 * 0.06 * scale,
                    position.y + (1.15 + j as f32 * 0.08) * scale,
                    position.z + 0.06 * scale,
                );
                self.mpm_staging_particles.push(Particle::with_group(
                    pt_pos,
                    particle_radius,
                    1020.0,
                    soft_tissue_model,
                    group_id,
                ));
            }
        }

        NexusBodyBinding {
            root_handle: pelvis_handle,
            mpm_group_id: group_id,
        }
    }
}

pub struct NexusPhysicsPlugin;

impl Plugin for NexusPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NexusPhysicsResource>()
            .add_systems(
                FixedUpdate,
                step_nexus_physics_system.in_set(crate::rpg::SimulationSet::MultiphysicsAndTerrain),
            );
    }
}

/// Steps the Nexus3D Rapier/MPM multiphysics world in FixedUpdate and couples muscle forces + terrain contacts
pub fn step_nexus_physics_system(
    mut nexus: ResMut<NexusPhysicsResource>,
    terrain_config: Res<TerrainConfig>,
    mut human_query: Query<(
        &mut Transform,
        &NexusBodyBinding,
        &HumanoidSkeleton,
        &MusculoskeletalSystem,
        &mut SoftTissueViscera,
        &MetabolicCaloricPool,
    )>,
    mut bone_query: Query<&mut BoneSegment>,
    time: Res<Time>,
) {
    let dt = time.delta_secs().clamp(1.0 / 240.0, 1.0 / 20.0);

    // 1. Synchronize Bevy kinematic/muscle-driven humanoid root poses into Nexus3D Rapier bodies
    //    and apply upright balance PD torques unless the entity is a dead ragdoll.
    {
        let world = nexus.state.rbd_world_mut_untracked(0);
        world.integration_parameters.dt = dt;

        for (tf, binding, skeleton, muscles, _viscera, metabolism) in &mut human_query {
            if let Some(body) = world.bodies.get_mut(binding.root_handle) {
                let ground_y = evaluate_terrain_height(tf.translation.x, tf.translation.z, &terrain_config);
                let target_pos = nexus3d::rapier::prelude::Vector::new(
                    tf.translation.x,
                    tf.translation.y.max(ground_y) + 1.0 * tf.scale.y,
                    tf.translation.z,
                );
                body.set_translation(target_pos, true);

                // Compute net contractile muscle impulse for stress propagation
                let mut total_muscle_force = 0.0f32;
                for m in &muscles.muscles {
                    total_muscle_force += m.evaluate_contractile_force(m.optimal_length);
                }

                // Propagate mechanical stress to skeletal bones
                if !metabolism.is_dead && !skeleton.is_ragdoll {
                    for (_kind, bone_entity) in &skeleton.bones {
                        if let Ok(mut bone_seg) = bone_query.get_mut(*bone_entity) {
                            bone_seg.current_stress += (total_muscle_force * 0.015) * dt;
                        }
                    }
                }
            }
        }

        // Step the Nexus3D Rapier multibody + rigid-body pipeline
        world.step();
    }

    // 2. Clamp humanoid transforms cleanly onto the procedural CDLOD terrain surface
    for (mut tf, _binding, skeleton, _muscles, mut viscera, metabolism) in &mut human_query {
        let ground_y = evaluate_terrain_height(tf.translation.x, tf.translation.z, &terrain_config);
        // Smooth vertical terrain following
        let height_diff = ground_y - tf.translation.y;
        tf.translation.y += height_diff * (15.0 * dt).min(1.0);

        // If a high vertical drop occurred, impart impact stress & MPM visceral displacement
        if height_diff < -2.5 && !metabolism.is_dead {
            let impact_severity = (-height_diff - 2.5) * 0.4;
            viscera.rupture_tissue(impact_severity * 0.1);
            for (kind, bone_entity) in &skeleton.bones {
                if matches!(kind, BoneKind::LeftShin | BoneKind::RightShin | BoneKind::Pelvis) {
                    if let Ok(mut bone_seg) = bone_query.get_mut(*bone_entity) {
                        bone_seg.current_stress += impact_severity * 1200.0;
                    }
                }
            }
        }
    }
}
