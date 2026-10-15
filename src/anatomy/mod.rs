pub mod damage;
pub mod hull;
pub mod muscles;
pub mod skeleton;
pub mod soft_tissue;

use bevy::prelude::*;
use hull::BiologicalSex;

pub struct AnatomyPlugin;

impl Plugin for AnatomyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                damage::update_metabolism_system,
                damage::check_skeletal_fractures,
            ),
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

/// Spawns a complete 4-layer biomechanical humanoid:
/// 1. Rigid skeleton hierarchy
/// 2. Hill-type contractile muscles
/// 3. MPM soft-tissue & viscera
/// 4. Assembled low-poly hull meshes with mouth & reproductive colliders
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
    let metabolism = damage::MetabolicCaloricPool::new(1000.0 * scale.powi(3), 1.2);
    let mouth = hull::MouthContactSensor {
        radius: 0.12 * scale,
        is_chewing: false,
        chewing_timer: 0.0,
        food_in_mouth_calories: 0.0,
    };
    let repro = hull::ReproductiveContactSensor::new(sex, scale);

    let skin_mat = materials.add(StandardMaterial {
        base_color: skin_tone,
        perceptual_roughness: 0.85,
        ..default()
    });

    // Attach procedural hull meshes to visual representation
    let skull_mesh = meshes.add(hull::generate_bone_hull_mesh(
        skeleton::BoneKind::Skull,
        0.22 * scale,
        0.12 * scale,
    ));
    let torso_mesh = meshes.add(hull::generate_bone_hull_mesh(
        skeleton::BoneKind::Spine,
        0.28 * scale,
        0.14 * scale,
    ));

    let eyes = crate::snn::eyes::StereoscopicEyes::new(scale);
    let reward = crate::snn::brain::EvolvedRewardMatrix::default();
    let exec = crate::snn::brain::ExecutiveSnnModule::default();
    let loco = crate::snn::brain::LocomotionSnnModule::default();
    let manip = crate::snn::brain::ManipulationSnnModule::default();

    let genome = crate::genetics::genome::HumanGenome {
        morphology: crate::genetics::genome::MorphologyChromosome {
            skin_r: skin_tone.to_srgba().red,
            skin_g: skin_tone.to_srgba().green,
            skin_b: skin_tone.to_srgba().blue,
            adult_height_scale: scale,
            limb_thickness_factor: 1.0,
            procedural_seed: skeleton_root.to_bits() ^ 0x5a5a5a5a,
        },
        ..default()
    };

    commands
        .entity(skeleton_root)
        .insert((
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
            // Visual Head Hull
            parent.spawn((
                Mesh3d(skull_mesh),
                MeshMaterial3d(skin_mat.clone()),
                Transform::from_xyz(0.0, 1.65 * scale, 0.0),
            ));
            // Visual Torso Hull
            parent.spawn((
                Mesh3d(torso_mesh),
                MeshMaterial3d(skin_mat.clone()),
                Transform::from_xyz(0.0, 1.25 * scale, 0.0),
            ));
        });

    skeleton_root
}
