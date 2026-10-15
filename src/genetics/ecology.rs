#![allow(dead_code)]

use bevy::prelude::*;
use crate::anatomy::damage::MetabolicCaloricPool;
use crate::procgen::flora::{PluckableFruit, ProceduralTree};

#[derive(Resource, Debug)]
pub struct SoilFertilityGrid {
    pub cell_size: f32,
    pub width: i32,
    pub height: i32,
    pub fertility: Vec<f32>,
}

impl SoilFertilityGrid {
    pub fn new(cell_size: f32, width: i32, height: i32) -> Self {
        let total = (width * height) as usize;
        Self {
            cell_size,
            width,
            height,
            fertility: vec![1.0; total],
        }
    }

    pub fn world_to_index(&self, pos: Vec3) -> Option<usize> {
        let gx = (pos.x / self.cell_size).floor() as i32 + self.width / 2;
        let gz = (pos.z / self.cell_size).floor() as i32 + self.height / 2;

        if gx >= 0 && gx < self.width && gz >= 0 && gz < self.height {
            Some((gz * self.width + gx) as usize)
        } else {
            None
        }
    }

    pub fn deposit_nutrients(&mut self, pos: Vec3, amount: f32) {
        if let Some(idx) = self.world_to_index(pos) {
            self.fertility[idx] += amount;
        }
    }

    pub fn absorb_nutrients(&mut self, pos: Vec3, amount: f32) -> f32 {
        if let Some(idx) = self.world_to_index(pos) {
            let available = self.fertility[idx].min(amount);
            self.fertility[idx] -= available;
            available
        } else {
            0.0
        }
    }
}

pub fn update_corpse_decomposition_system(
    mut commands: Commands,
    mut query: Query<(Entity, &Transform, &MetabolicCaloricPool)>,
    mut soil: ResMut<SoilFertilityGrid>,
) {
    for (entity, tf, metabolism) in &mut query {
        if metabolism.is_dead {
            // Deposit organic matter and nutrients into local soil
            soil.deposit_nutrients(tf.translation, 5.0);
            commands.entity(entity).despawn();
            info!("Corpse at {:?} decomposed into soil nutrients.", tf.translation);
        }
    }
}

pub fn update_plant_absorption_and_fruiting(
    mut soil: ResMut<SoilFertilityGrid>,
    tree_query: Query<(&Transform, &ProceduralTree)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    let fruit_mesh_handle = meshes.add(Sphere::new(0.18).mesh().ico(2).expect("valid sphere mesh"));
    let fruit_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.2, 0.3),
        emissive: LinearRgba::new(0.4, 0.05, 0.08, 1.0),
        perceptual_roughness: 0.3,
        ..default()
    });

    for (tf, tree) in &tree_query {
        // Trees draw nutrients from the soil beneath them
        let absorbed = soil.absorb_nutrients(tf.translation, 0.15 * dt);

        if absorbed > 0.1 && tree.fruit_socket_count > 0 {
            // Sprout new physical fruit
            let spawn_pos = tf.translation + Vec3::new(1.0, 5.0, 1.0);
            commands.spawn((
                Mesh3d(fruit_mesh_handle.clone()),
                MeshMaterial3d(fruit_material.clone()),
                Transform::from_translation(spawn_pos),
                PluckableFruit {
                    caloric_value: 250.0,
                    hydration_value: 60.0,
                    mass_kg: 0.3,
                    is_attached: true,
                },
            ));
        }
    }
}
