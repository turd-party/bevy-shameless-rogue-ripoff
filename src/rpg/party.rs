#![allow(dead_code)]

use bevy::prelude::*;

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

pub fn handle_party_selection_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut PartyMember, Entity)>,
) {
    if keyboard.just_pressed(KeyCode::Digit1) {
        for (mut member, e) in &mut query {
            member.is_selected = member.index == 0;
            if member.is_selected {
                info!("Selected party member 1 ({:?})", e);
            }
        }
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        for (mut member, e) in &mut query {
            member.is_selected = member.index == 1;
            if member.is_selected {
                info!("Selected party member 2 ({:?})", e);
            }
        }
    }
}

pub fn handle_tactical_order_input(
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&PartyMember, &mut OrderQueue, &Transform)>,
) {
    // Right click issues MoveTo order to selected party members
    if mouse.just_pressed(MouseButton::Right) {
        let is_shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

        for (member, mut queue, tf) in &mut query {
            if member.is_selected {
                if !is_shift {
                    queue.clear();
                }

                // Forward target waypoint for demonstration
                let target_pos = tf.translation + tf.forward().as_vec3() * 6.0;
                let pushed = queue.push_order(TacticalOrder::MoveTo(target_pos));

                if pushed {
                    info!("Party member {} queued order: MoveTo({:?})", member.designation, target_pos);
                }
            }
        }
    }
}
