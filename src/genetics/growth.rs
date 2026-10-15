#![allow(dead_code)]

use bevy::prelude::*;
use crate::anatomy::damage::MetabolicCaloricPool;

#[derive(Component, Debug, Reflect)]
pub struct ChildGrowth {
    pub current_scale: f32,
    pub target_adult_scale: f32,
    pub growth_rate_per_sec: f32,
}

pub fn update_child_growth_system(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut ChildGrowth, &MetabolicCaloricPool)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (entity, mut tf, mut growth, metabolism) in &mut query {
        // Growth occurs only when caloric intake is healthy (> 40% capacity)
        if metabolism.current_calories > metabolism.max_calories * 0.4 {
            growth.current_scale += growth.growth_rate_per_sec * dt;
            tf.scale = Vec3::splat(growth.current_scale);

            if growth.current_scale >= growth.target_adult_scale {
                tf.scale = Vec3::splat(growth.target_adult_scale);
                commands.entity(entity).remove::<ChildGrowth>();
                info!("Human {:?} has reached full adult maturity!", entity);
            }
        }
    }
}
