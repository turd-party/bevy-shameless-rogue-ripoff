pub mod damage;
pub mod hull;
pub mod muscles;
pub mod skeleton;
pub mod soft_tissue;

use bevy::prelude::*;
use hull::{BiologicalSex, MouthContactSensor};
use skeleton::{BoneKind, HumanoidSkeleton};
use crate::physics::{NexusBodyBinding, NexusPhysicsResource};
use crate::snn::brain::{LocomotionSnnModule, ManipulationSnnModule};

/// Component tagging a visual child hull mesh with its corresponding anatomical BoneKind and rest offset
#[derive(Component, Debug, Clone, Copy)]
pub struct VisualBoneHull {
    pub kind: BoneKind,
    pub rest_offset: Vec3,
}

pub struct AnatomyPlugin;

impl Plugin for AnatomyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                register_nexus_humanoids_system,
                animate_articulated_hull_system,
            )
                .chain(),
        )
        .add_systems(
            FixedUpdate,
            (
                damage::update_metabolism_system,
                damage::check_skeletal_fractures,
            )
                .chain()
                .in_set(crate::rpg::SimulationSet::MultiphysicsAndTerrain),
        )
        .add_observer(on_nociceptive_pain)
        .add_observer(on_bone_fracture);
    }
}

fn on_nociceptive_pain(trigger: On<damage::NociceptivePainEvent>) {
    trace!("Nociceptive pain received: {:?}", trigger.event());
}

fn on_bone_fracture(trigger: On<damage::BoneFractureEvent>) {
    info!("Bone fractured: {:?}", trigger.event());
}

/// Automatically binds newly spawned HumanoidSkeletons into the Nexus3D multiphysics engine
fn register_nexus_humanoids_system(
    mut commands: Commands,
    mut nexus: ResMut<NexusPhysicsResource>,
    query: Query<(Entity, &Transform), (With<HumanoidSkeleton>, Without<NexusBodyBinding>)>,
) {
    for (entity, tf) in &query {
        let scale = tf.scale.y.max(0.4);
        let binding = nexus.register_humanoid_multibody(tf.translation, scale);
        commands.entity(entity).insert(binding);
    }
}

/// Animates all 16 articulated bone hull meshes based on CPG gait phase, arm reach-to-mouth, and chewing
fn animate_articulated_hull_system(
    human_query: Query<(
        &HumanoidSkeleton,
        &LocomotionSnnModule,
        &ManipulationSnnModule,
        &MouthContactSensor,
        &Children,
    )>,
    mut hull_query: Query<(&VisualBoneHull, &mut Transform)>,
) {
    for (skeleton, loco, manip, mouth, children) in &human_query {
        let gait_sin = loco.gait_phase.sin();
        let gait_amp = (loco.left_leg_spike.max(loco.right_leg_spike)).clamp(0.0, 1.0);
        let reach = manip.arm_reach_progress.clamp(0.0, 1.0);
        let chew_nod = if mouth.is_chewing {
            (mouth.chewing_timer * 18.0).sin() * 0.12
        } else {
            0.0
        };

        for child in children.iter() {
            if let Ok((hull, mut tf)) = hull_query.get_mut(child) {
                if skeleton.is_ragdoll {
                    // Ragdoll collapse posture
                    tf.translation = Vec3::new(hull.rest_offset.x, 0.12, hull.rest_offset.y * 0.6);
                    tf.rotation = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
                    continue;
                }

                let mut offset = hull.rest_offset;
                let mut rot = Quat::IDENTITY;

                match hull.kind {
                    BoneKind::Skull => {
                        rot = Quat::from_rotation_x(chew_nod);
                    }
                    BoneKind::LeftThigh => {
                        let angle = gait_sin * 0.55 * gait_amp;
                        rot = Quat::from_rotation_x(angle);
                        offset.z += angle * 0.15;
                    }
                    BoneKind::LeftShin => {
                        let angle = gait_sin * 0.55 * gait_amp;
                        let knee_bend = (-gait_sin).max(0.0) * 0.45 * gait_amp;
                        rot = Quat::from_rotation_x(angle - knee_bend);
                        offset.z += (angle - knee_bend * 0.5) * 0.28;
                    }
                    BoneKind::LeftFoot => {
                        let angle = gait_sin * 0.55 * gait_amp;
                        offset.z += angle * 0.32;
                        offset.y += (-gait_sin).max(0.0) * 0.08 * gait_amp;
                    }
                    BoneKind::RightThigh => {
                        let angle = -gait_sin * 0.55 * gait_amp;
                        rot = Quat::from_rotation_x(angle);
                        offset.z += angle * 0.15;
                    }
                    BoneKind::RightShin => {
                        let angle = -gait_sin * 0.55 * gait_amp;
                        let knee_bend = gait_sin.max(0.0) * 0.45 * gait_amp;
                        rot = Quat::from_rotation_x(angle - knee_bend);
                        offset.z += (angle - knee_bend * 0.5) * 0.28;
                    }
                    BoneKind::RightFoot => {
                        let angle = -gait_sin * 0.55 * gait_amp;
                        offset.z += angle * 0.32;
                        offset.y += gait_sin.max(0.0) * 0.08 * gait_amp;
                    }
                    BoneKind::LeftUpperArm | BoneKind::LeftForearm | BoneKind::LeftHand => {
                        // Counter-swing left arm during walking gait
                        let swing = -gait_sin * 0.35 * gait_amp;
                        rot = Quat::from_rotation_x(swing);
                        offset.z += swing * (1.45 - hull.rest_offset.y).max(0.1) * 0.4;
                    }
                    BoneKind::RightUpperArm => {
                        // Lift right upper arm when reaching / bringing food to mouth
                        let swing = gait_sin * 0.35 * gait_amp * (1.0 - reach);
                        let pitch = reach * 1.15 + swing;
                        rot = Quat::from_rotation_x(pitch);
                    }
                    BoneKind::RightForearm => {
                        // Flex right elbow to bring forearm toward mouth
                        let swing = gait_sin * 0.35 * gait_amp * (1.0 - reach);
                        offset = offset.lerp(Vec3::new(0.14, 1.48, 0.18), reach);
                        rot = Quat::from_rotation_x(reach * 2.1 + swing);
                    }
                    BoneKind::RightHand => {
                        // Bring right hand directly to MouthContactSensor at (0.0, 1.60, 0.14) when eating!
                        let mouth_target = Vec3::new(0.02, 1.60, 0.14 + chew_nod * 0.1);
                        offset = offset.lerp(mouth_target, reach);
                    }
                    _ => {}
                }

                tf.translation = offset;
                tf.rotation = rot;
            }
        }
    }
}

/// Spawns a complete 4-layer biomechanical humanoid with all 16 articulated bone segment hulls:
/// 1. Rigid skeleton hierarchy (16 bones)
/// 2. Hill-type contractile muscles (18 muscle groups)
/// 3. MPM soft-tissue & viscera
/// 4. Assembled low-poly hull meshes for all 16 bone segments + mouth & reproductive colliders
pub fn spawn_biomechanical_human(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    sex: BiologicalSex,
    scale: f32,
    skin_tone: Color,
) -> Entity {
    let skeleton_root = skeleton::spawn_humanoid_skeleton(commands, scale, position);

    let muscles = muscles::MusculoskeletalSystem::build_humanoid_muscles(scale);
    let viscera = soft_tissue::SoftTissueViscera::new(scale);
    let metabolism = damage::MetabolicCaloricPool::new(1000.0 * scale.powi(3).max(0.25), 1.2);
    let mouth = hull::MouthContactSensor {
        radius: 0.12 * scale,
        is_chewing: false,
        chewing_timer: 0.0,
        food_in_mouth_calories: 0.0,
    };
    let repro = hull::ReproductiveContactSensor::new(sex, scale);

    let skin_mat = materials.add(StandardMaterial {
        base_color: skin_tone,
        perceptual_roughness: 0.82,
        ..default()
    });

    // All 16 anatomical bone segments with their rest offsets and dimensions (unscaled local space; root scale scales children)
    let bone_specs: [(BoneKind, Vec3, f32, f32); 16] = [
        (BoneKind::Pelvis, Vec3::new(0.0, 0.96, 0.0), 0.22, 0.14),
        (BoneKind::Spine, Vec3::new(0.0, 1.18, 0.0), 0.24, 0.12),
        (BoneKind::Ribcage, Vec3::new(0.0, 1.38, 0.0), 0.26, 0.15),
        (BoneKind::Skull, Vec3::new(0.0, 1.65, 0.0), 0.22, 0.11),
        (BoneKind::LeftUpperArm, Vec3::new(-0.24, 1.34, 0.0), 0.28, 0.055),
        (BoneKind::LeftForearm, Vec3::new(-0.24, 1.06, 0.0), 0.26, 0.045),
        (BoneKind::LeftHand, Vec3::new(-0.24, 0.86, 0.0), 0.12, 0.04),
        (BoneKind::RightUpperArm, Vec3::new(0.24, 1.34, 0.0), 0.28, 0.055),
        (BoneKind::RightForearm, Vec3::new(0.24, 1.06, 0.0), 0.26, 0.045),
        (BoneKind::RightHand, Vec3::new(0.24, 0.86, 0.0), 0.12, 0.04),
        (BoneKind::LeftThigh, Vec3::new(-0.12, 0.68, 0.0), 0.38, 0.075),
        (BoneKind::LeftShin, Vec3::new(-0.12, 0.32, 0.0), 0.36, 0.06),
        (BoneKind::LeftFoot, Vec3::new(-0.12, 0.06, 0.04), 0.16, 0.05),
        (BoneKind::RightThigh, Vec3::new(0.12, 0.68, 0.0), 0.38, 0.075),
        (BoneKind::RightShin, Vec3::new(0.12, 0.32, 0.0), 0.36, 0.06),
        (BoneKind::RightFoot, Vec3::new(0.12, 0.06, 0.04), 0.16, 0.05),
    ];

    let hull_meshes: Vec<(BoneKind, Vec3, Handle<Mesh>)> = bone_specs
        .iter()
        .map(|(kind, offset, len, rad)| {
            let mesh = meshes.add(hull::generate_bone_hull_mesh(*kind, *len, *rad));
            (*kind, *offset, mesh)
        })
        .collect();

    let eyes = crate::snn::eyes::StereoscopicEyes::new(scale);
    let reward = crate::snn::brain::EvolvedRewardMatrix::default();
    let exec = crate::snn::brain::ExecutiveSnnModule::new_with_seed(skeleton_root.to_bits() ^ 0x13579bdf);
    let loco = crate::snn::brain::LocomotionSnnModule::default();
    let manip = crate::snn::brain::ManipulationSnnModule::default();

    let genome = crate::genetics::genome::HumanGenome {
        morphology: crate::genetics::genome::MorphologyChromosome {
            skin_r: skin_tone.to_srgba().red,
            skin_g: skin_tone.to_srgba().green,
            skin_b: skin_tone.to_srgba().blue,
            adult_height_scale: scale.max(0.9),
            limb_thickness_factor: 1.0,
            procedural_seed: skeleton_root.to_bits() ^ 0x5a5a5a5a,
        },
        ..default()
    };

    commands
        .entity(skeleton_root)
        .insert((
            Transform::from_translation(position).with_scale(Vec3::splat(scale)),
            Visibility::default(),
            muscles,
            viscera,
            metabolism,
            mouth,
            repro,
            eyes,
            reward,
            exec,
            loco,
            manip,
            genome,
        ))
        .with_children(|parent| {
            for (kind, rest_offset, mesh_handle) in hull_meshes {
                parent.spawn((
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(skin_mat.clone()),
                    Transform::from_translation(rest_offset),
                    VisualBoneHull { kind, rest_offset },
                ));
            }
        });

    skeleton_root
}
