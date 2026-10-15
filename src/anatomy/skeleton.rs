#![allow(dead_code)]

use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BoneKind {
    Pelvis,
    Spine,
    Ribcage,
    Skull,
    LeftUpperArm,
    LeftForearm,
    LeftHand,
    RightUpperArm,
    RightForearm,
    RightHand,
    LeftThigh,
    LeftShin,
    LeftFoot,
    RightThigh,
    RightShin,
    RightFoot,
}

#[derive(Component, Debug)]
pub struct BoneSegment {
    pub kind: BoneKind,
    pub length: f32,
    pub radius: f32,
    pub mass_kg: f32,
    pub is_fractured: bool,
    pub max_stress_threshold: f32,
    pub current_stress: f32,
}

#[derive(Component, Debug)]
pub struct SkeletalJoint {
    pub parent_bone: Entity,
    pub child_bone: Entity,
    pub anchor_offset: Vec3,
    pub is_dislocated: bool,
    pub min_angles: Vec3,
    pub max_angles: Vec3,
    pub current_angles: Vec3,
}

#[derive(Component, Debug)]
pub struct HumanoidSkeleton {
    pub bones: Vec<(BoneKind, Entity)>,
    pub total_mass_kg: f32,
    pub is_ragdoll: bool,
}

impl HumanoidSkeleton {
    pub fn get_bone(&self, kind: BoneKind) -> Option<Entity> {
        self.bones.iter().find(|(k, _)| *k == kind).map(|(_, e)| *e)
    }
}

pub fn spawn_humanoid_skeleton(
    commands: &mut Commands,
    scale: f32,
    base_pos: Vec3,
) -> Entity {
    let mut bone_entities = Vec::with_capacity(16);

    // Root Pelvis
    let pelvis = commands
        .spawn((
            Transform::from_translation(base_pos + Vec3::new(0.0, 1.0 * scale, 0.0)),
            BoneSegment {
                kind: BoneKind::Pelvis,
                length: 0.25 * scale,
                radius: 0.16 * scale,
                mass_kg: 14.0 * scale.powi(3),
                is_fractured: false,
                max_stress_threshold: 4500.0,
                current_stress: 0.0,
            },
        ))
        .id();
    bone_entities.push((BoneKind::Pelvis, pelvis));

    // Spine
    let spine = commands
        .spawn((
            Transform::from_translation(base_pos + Vec3::new(0.0, 1.25 * scale, 0.0)),
            BoneSegment {
                kind: BoneKind::Spine,
                length: 0.28 * scale,
                radius: 0.14 * scale,
                mass_kg: 18.0 * scale.powi(3),
                is_fractured: false,
                max_stress_threshold: 5000.0,
                current_stress: 0.0,
            },
        ))
        .id();
    bone_entities.push((BoneKind::Spine, spine));

    // Skull
    let skull = commands
        .spawn((
            Transform::from_translation(base_pos + Vec3::new(0.0, 1.65 * scale, 0.0)),
            BoneSegment {
                kind: BoneKind::Skull,
                length: 0.22 * scale,
                radius: 0.12 * scale,
                mass_kg: 5.0 * scale.powi(3),
                is_fractured: false,
                max_stress_threshold: 3000.0,
                current_stress: 0.0,
            },
        ))
        .id();
    bone_entities.push((BoneKind::Skull, skull));

    // Arms & Hands
    let left_upper_arm = commands
        .spawn((
            Transform::from_translation(base_pos + Vec3::new(-0.28 * scale, 1.4 * scale, 0.0)),
            BoneSegment {
                kind: BoneKind::LeftUpperArm,
                length: 0.32 * scale,
                radius: 0.06 * scale,
                mass_kg: 2.5 * scale.powi(3),
                is_fractured: false,
                max_stress_threshold: 2500.0,
                current_stress: 0.0,
            },
        ))
        .id();
    bone_entities.push((BoneKind::LeftUpperArm, left_upper_arm));

    let left_forearm = commands
        .spawn((
            Transform::from_translation(base_pos + Vec3::new(-0.28 * scale, 1.05 * scale, 0.0)),
            BoneSegment {
                kind: BoneKind::LeftForearm,
                length: 0.30 * scale,
                radius: 0.05 * scale,
                mass_kg: 1.8 * scale.powi(3),
                is_fractured: false,
                max_stress_threshold: 2000.0,
                current_stress: 0.0,
            },
        ))
        .id();
    bone_entities.push((BoneKind::LeftForearm, left_forearm));

    let left_hand = commands
        .spawn((
            Transform::from_translation(base_pos + Vec3::new(-0.28 * scale, 0.72 * scale, 0.0)),
            BoneSegment {
                kind: BoneKind::LeftHand,
                length: 0.15 * scale,
                radius: 0.045 * scale,
                mass_kg: 0.6 * scale.powi(3),
                is_fractured: false,
                max_stress_threshold: 1200.0,
                current_stress: 0.0,
            },
        ))
        .id();
    bone_entities.push((BoneKind::LeftHand, left_hand));

    // Right Arm & Hand
    let right_upper_arm = commands
        .spawn((
            Transform::from_translation(base_pos + Vec3::new(0.28 * scale, 1.4 * scale, 0.0)),
            BoneSegment {
                kind: BoneKind::RightUpperArm,
                length: 0.32 * scale,
                radius: 0.06 * scale,
                mass_kg: 2.5 * scale.powi(3),
                is_fractured: false,
                max_stress_threshold: 2500.0,
                current_stress: 0.0,
            },
        ))
        .id();
    bone_entities.push((BoneKind::RightUpperArm, right_upper_arm));

    let right_forearm = commands
        .spawn((
            Transform::from_translation(base_pos + Vec3::new(0.28 * scale, 1.05 * scale, 0.0)),
            BoneSegment {
                kind: BoneKind::RightForearm,
                length: 0.30 * scale,
                radius: 0.05 * scale,
                mass_kg: 1.8 * scale.powi(3),
                is_fractured: false,
                max_stress_threshold: 2000.0,
                current_stress: 0.0,
            },
        ))
        .id();
    bone_entities.push((BoneKind::RightForearm, right_forearm));

    let right_hand = commands
        .spawn((
            Transform::from_translation(base_pos + Vec3::new(0.28 * scale, 0.72 * scale, 0.0)),
            BoneSegment {
                kind: BoneKind::RightHand,
                length: 0.15 * scale,
                radius: 0.045 * scale,
                mass_kg: 0.6 * scale.powi(3),
                is_fractured: false,
                max_stress_threshold: 1200.0,
                current_stress: 0.0,
            },
        ))
        .id();
    bone_entities.push((BoneKind::RightHand, right_hand));

    // Legs
    let left_thigh = commands
        .spawn((
            Transform::from_translation(base_pos + Vec3::new(-0.14 * scale, 0.7 * scale, 0.0)),
            BoneSegment {
                kind: BoneKind::LeftThigh,
                length: 0.45 * scale,
                radius: 0.09 * scale,
                mass_kg: 9.0 * scale.powi(3),
                is_fractured: false,
                max_stress_threshold: 6000.0,
                current_stress: 0.0,
            },
        ))
        .id();
    bone_entities.push((BoneKind::LeftThigh, left_thigh));

    let left_shin = commands
        .spawn((
            Transform::from_translation(base_pos + Vec3::new(-0.14 * scale, 0.28 * scale, 0.0)),
            BoneSegment {
                kind: BoneKind::LeftShin,
                length: 0.42 * scale,
                radius: 0.07 * scale,
                mass_kg: 4.5 * scale.powi(3),
                is_fractured: false,
                max_stress_threshold: 5000.0,
                current_stress: 0.0,
            },
        ))
        .id();
    bone_entities.push((BoneKind::LeftShin, left_shin));

    let right_thigh = commands
        .spawn((
            Transform::from_translation(base_pos + Vec3::new(0.14 * scale, 0.7 * scale, 0.0)),
            BoneSegment {
                kind: BoneKind::RightThigh,
                length: 0.45 * scale,
                radius: 0.09 * scale,
                mass_kg: 9.0 * scale.powi(3),
                is_fractured: false,
                max_stress_threshold: 6000.0,
                current_stress: 0.0,
            },
        ))
        .id();
    bone_entities.push((BoneKind::RightThigh, right_thigh));

    let right_shin = commands
        .spawn((
            Transform::from_translation(base_pos + Vec3::new(0.14 * scale, 0.28 * scale, 0.0)),
            BoneSegment {
                kind: BoneKind::RightShin,
                length: 0.42 * scale,
                radius: 0.07 * scale,
                mass_kg: 4.5 * scale.powi(3),
                is_fractured: false,
                max_stress_threshold: 5000.0,
                current_stress: 0.0,
            },
        ))
        .id();
    bone_entities.push((BoneKind::RightShin, right_shin));

    // Root Humanoid Skeleton Entity
    commands
        .spawn((
            Transform::from_translation(base_pos),
            HumanoidSkeleton {
                bones: bone_entities,
                total_mass_kg: 70.0 * scale.powi(3),
                is_ragdoll: false,
            },
        ))
        .id()
}
