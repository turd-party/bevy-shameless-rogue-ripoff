#![allow(dead_code)]

use bevy::prelude::*;
use super::party::{OrderQueue, PartyMember, TacticalOrder};
use crate::anatomy::damage::MetabolicCaloricPool;
use crate::procgen::flora::PluckableFruit;
use crate::snn::brain::{LocomotionSnnModule, ManipulationSnnModule};

pub fn update_party_character_controller(
    mut commands: Commands,
    mut query: Query<(
        &PartyMember,
        &mut OrderQueue,
        &mut Transform,
        &mut LocomotionSnnModule,
        &mut ManipulationSnnModule,
        &MetabolicCaloricPool,
    )>,
    fruit_query: Query<(&Transform, &PluckableFruit), Without<PartyMember>>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    for (member, mut queue, mut tf, mut loco, mut manip, metabolism) in &mut query {
        if metabolism.is_dead {
            loco.left_leg_spike = 0.0;
            loco.right_leg_spike = 0.0;
            continue;
        }

        if let Some(active_order) = queue.orders.first().cloned() {
            match active_order {
                TacticalOrder::MoveTo(target_pos) => {
                    let to_target = target_pos - tf.translation;
                    let dist_horizontal = Vec2::new(to_target.x, to_target.z).length();

                    if dist_horizontal < 0.55 {
                        queue.pop_order();
                        loco.left_leg_spike = 0.0;
                        loco.right_leg_spike = 0.0;
                        info!("Party member {} reached waypoint.", member.designation);
                    } else {
                        let desired_dir = Vec3::new(to_target.x, 0.0, to_target.z).normalize_or_zero();
                        let target_rot = Quat::from_rotation_y(desired_dir.x.atan2(desired_dir.z));
                        tf.rotation = tf.rotation.slerp(target_rot, 8.0 * dt);

                        loco.gait_phase = (loco.gait_phase + 5.5 * dt) % std::f32::consts::TAU;
                        loco.left_leg_spike = loco.gait_phase.sin().max(0.0) * 1.25;
                        loco.right_leg_spike =
                            (loco.gait_phase + std::f32::consts::PI).sin().max(0.0) * 1.25;

                        tf.translation += desired_dir * 3.8 * dt;
                    }
                }
                TacticalOrder::PickUpFood(food_entity) => {
                    if let Ok((_f_tf, fruit)) = fruit_query.get(food_entity) {
                        manip.arm_reach_progress =
                            (manip.arm_reach_progress + 2.5 * dt).clamp(0.0, 1.0);
                        manip.bicep_spike = 0.95;

                        if manip.arm_reach_progress >= 0.6 {
                            manip.is_holding_food = true;
                            manip.held_food_calories = fruit.caloric_value;
                            commands.entity(food_entity).despawn();
                            queue.pop_order();
                            info!(
                                "Party member {} physically grasped food ({:.0} kcal).",
                                member.designation, fruit.caloric_value
                            );
                        }
                    } else {
                        // Food was already picked up or consumed
                        queue.pop_order();
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
        } else {
            loco.left_leg_spike = 0.0;
            loco.right_leg_spike = 0.0;
        }
    }
}
