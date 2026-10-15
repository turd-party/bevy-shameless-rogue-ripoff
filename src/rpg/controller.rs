#![allow(dead_code)]

use bevy::prelude::*;
use super::party::{OrderQueue, PartyMember, TacticalOrder};
use crate::snn::brain::{LocomotionSnnModule, ManipulationSnnModule};

pub fn update_party_character_controller(
    mut query: Query<(
        &PartyMember,
        &mut OrderQueue,
        &mut Transform,
        &mut LocomotionSnnModule,
        &mut ManipulationSnnModule,
    )>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (member, mut queue, mut tf, mut loco, mut manip) in &mut query {
        if let Some(active_order) = queue.orders.first().cloned() {
            match active_order {
                TacticalOrder::MoveTo(target_pos) => {
                    let to_target = target_pos - tf.translation;
                    let dist_horizontal = Vec2::new(to_target.x, to_target.z).length();

                    if dist_horizontal < 0.6 {
                        // Reached waypoint!
                        queue.pop_order();
                        loco.left_leg_spike = 0.0;
                        loco.right_leg_spike = 0.0;
                        info!("Party member {} reached waypoint.", member.designation);
                    } else {
                        // Guide neural locomotion modular SNN towards target
                        let desired_dir = Vec3::new(to_target.x, 0.0, to_target.z).normalize();
                        
                        // Rotate towards heading
                        let target_rot = Quat::from_rotation_y(desired_dir.x.atan2(desired_dir.z));
                        tf.rotation = tf.rotation.slerp(target_rot, 8.0 * dt);

                        // Drive gait generator
                        loco.gait_phase = (loco.gait_phase + 5.0 * dt) % std::f32::consts::TAU;
                        loco.left_leg_spike = loco.gait_phase.sin().max(0.0) * 1.2;
                        loco.right_leg_spike = (loco.gait_phase + std::f32::consts::PI).sin().max(0.0) * 1.2;

                        // Physical step displacement
                        tf.translation += desired_dir * 3.5 * dt;
                    }
                }
                TacticalOrder::PickUpFood(_food_entity) => {
                    manip.arm_reach_progress = (manip.arm_reach_progress + 3.0 * dt).clamp(0.0, 1.0);
                    manip.bicep_spike = 0.9;

                    if manip.arm_reach_progress >= 1.0 {
                        manip.is_holding_food = true;
                        queue.pop_order();
                        info!("Party member {} picked up food.", member.designation);
                    }
                }
                TacticalOrder::HoldPosition => {
                    loco.left_leg_spike = 0.0;
                    loco.right_leg_spike = 0.0;
                }
                TacticalOrder::AttackTarget(_target) => {
                    manip.bicep_spike = 1.0;
                }
            }
        }
    }
}
