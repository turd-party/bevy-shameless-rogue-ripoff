#![allow(dead_code)]

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::procgen::flora::PluckableFruit;
use crate::procgen::terrain::{evaluate_terrain_height, TerrainConfig};

#[derive(Clone, Debug, PartialEq)]
pub enum TacticalOrder {
    MoveTo(Vec3),
    PickUpFood(Entity),
    AttackTarget(Entity),
    HoldPosition,
}

#[derive(Component, Debug)]
pub struct OrderQueue {
    pub orders: Vec<TacticalOrder>,
    pub max_capacity: usize,
}

impl Default for OrderQueue {
    fn default() -> Self {
        Self {
            orders: Vec::with_capacity(8),
            max_capacity: 8,
        }
    }
}

impl OrderQueue {
    pub fn push_order(&mut self, order: TacticalOrder) -> bool {
        if self.orders.len() < self.max_capacity {
            self.orders.push(order);
            true
        } else {
            false
        }
    }

    pub fn pop_order(&mut self) -> Option<TacticalOrder> {
        if !self.orders.is_empty() {
            Some(self.orders.remove(0))
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.orders.clear();
    }
}

#[derive(Component, Debug)]
pub struct PartyMember {
    pub index: usize,
    pub designation: String,
    pub is_selected: bool,
}

/// Visual ring child entity rendered beneath a PartyMember when selected
#[derive(Component, Debug)]
pub struct SelectionRingVisual;

pub fn handle_party_selection_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut party_query: Query<(Entity, &mut PartyMember, &Transform)>,
) {
    if keyboard.just_pressed(KeyCode::Digit1) {
        for (_e, mut member, _) in &mut party_query {
            member.is_selected = member.index == 0;
        }
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        for (_e, mut member, _) in &mut party_query {
            member.is_selected = member.index == 1;
        }
    } else if keyboard.just_pressed(KeyCode::Tab) {
        // Select all party members
        for (_e, mut member, _) in &mut party_query {
            member.is_selected = true;
        }
    } else if mouse.just_pressed(MouseButton::Left) {
        // Raycast pick party member under cursor
        let Some(window) = window_query.iter().next() else { return };
        let Some(cursor_pos) = window.cursor_position() else { return };
        let Some((camera, cam_gtf)) = camera_query.iter().next() else { return };
        let Ok(ray) = camera.viewport_to_world(cam_gtf, cursor_pos) else { return };

        let mut clicked_member_idx = None;
        let mut best_dist = 2.0f32;
        for (_e, member, tf) in &party_query {
            let center = tf.translation + Vec3::new(0.0, 0.9, 0.0);
            let to_center = center - ray.origin;
            let proj = to_center.dot(*ray.direction);
            if proj > 0.0 {
                let closest_pt = ray.origin + *ray.direction * proj;
                let dist = closest_pt.distance(center);
                if dist < best_dist {
                    best_dist = dist;
                    clicked_member_idx = Some(member.index);
                }
            }
        }

        if let Some(target_idx) = clicked_member_idx {
            for (_e, mut member, _) in &mut party_query {
                member.is_selected = member.index == target_idx;
            }
        }
    }
}

pub fn handle_tactical_order_input(
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    fruit_query: Query<(Entity, &Transform), With<PluckableFruit>>,
    terrain_config: Res<TerrainConfig>,
    mut party_query: Query<(&PartyMember, &mut OrderQueue, &Transform)>,
) {
    // Press 'H' to issue HoldPosition to selected party members
    if keyboard.just_pressed(KeyCode::KeyH) {
        for (member, mut queue, _) in &mut party_query {
            if member.is_selected {
                queue.clear();
                queue.push_order(TacticalOrder::HoldPosition);
            }
        }
        return;
    }

    // Right click issues raycasted MoveTo or PickUpFood order to selected party members
    if mouse.just_pressed(MouseButton::Right) {
        let is_shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

        // Compute 3D ray from camera through mouse cursor onto CDLOD terrain
        let mut ray_hit_pos: Option<Vec3> = None;
        if let (Some(window), Some((camera, cam_gtf))) =
            (window_query.iter().next(), camera_query.iter().next())
        {
            if let Some(cursor_pos) = window.cursor_position() {
                if let Ok(ray) = camera.viewport_to_world(cam_gtf, cursor_pos) {
                    // Ray-march 48 steps to intersect terrain heightmap accurately
                    let max_t = 120.0f32;
                    for step in 1..=48 {
                        let t = (step as f32 / 48.0) * max_t;
                        let p = ray.origin + *ray.direction * t;
                        let h = evaluate_terrain_height(p.x, p.z, &terrain_config);
                        if p.y <= h {
                            ray_hit_pos = Some(Vec3::new(p.x, h, p.z));
                            break;
                        }
                    }
                }
            }
        }

        // Check if the ray hit near a physical PluckableFruit
        let mut target_fruit: Option<(Entity, Vec3)> = None;
        if let Some(hit) = ray_hit_pos {
            let mut best_d = 2.5f32;
            for (f_ent, f_tf) in &fruit_query {
                let d = Vec2::new(f_tf.translation.x - hit.x, f_tf.translation.z - hit.z).length();
                if d < best_d {
                    best_d = d;
                    target_fruit = Some((f_ent, f_tf.translation));
                }
            }
        }

        for (member, mut queue, tf) in &mut party_query {
            if member.is_selected {
                if !is_shift {
                    queue.clear();
                }

                if let Some((fruit_entity, fruit_pos)) = target_fruit {
                    let ground_y = evaluate_terrain_height(fruit_pos.x, fruit_pos.z, &terrain_config);
                    queue.push_order(TacticalOrder::MoveTo(Vec3::new(
                        fruit_pos.x,
                        ground_y,
                        fruit_pos.z,
                    )));
                    queue.push_order(TacticalOrder::PickUpFood(fruit_entity));
                    info!(
                        "[RTwP] {} queued MoveTo + PickUpFood({:?})",
                        member.designation, fruit_entity
                    );
                } else {
                    let target_pos = ray_hit_pos.unwrap_or_else(|| {
                        let p = tf.translation + tf.forward().as_vec3() * 6.0;
                        let h = evaluate_terrain_height(p.x, p.z, &terrain_config);
                        Vec3::new(p.x, h, p.z)
                    });
                    if queue.push_order(TacticalOrder::MoveTo(target_pos)) {
                        info!(
                            "[RTwP] {} queued order: MoveTo({:.1}, {:.1}, {:.1})",
                            member.designation, target_pos.x, target_pos.y, target_pos.z
                        );
                    }
                }
            }
        }
    }
}
