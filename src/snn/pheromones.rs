#![allow(dead_code)]

use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PheromoneType {
    KinScent,
    AlarmSignal,
    ReproductiveReadiness,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ChemicalConcentration {
    pub kin: f32,
    pub alarm: f32,
    pub reproductive: f32,
}

#[derive(Resource, Debug)]
pub struct PheromoneGrid3D {
    pub cell_size: f32,
    pub grid_extent: IVec3,
    pub cells: Vec<ChemicalConcentration>,
    pub diffusion_rate: f32,
    pub decay_rate: f32,
}

impl PheromoneGrid3D {
    pub fn new(cell_size: f32, extent: IVec3) -> Self {
        let total_cells = (extent.x * extent.y * extent.z) as usize;
        Self {
            cell_size,
            grid_extent: extent,
            cells: vec![ChemicalConcentration::default(); total_cells],
            diffusion_rate: 0.12,
            decay_rate: 0.05,
        }
    }

    pub fn world_to_grid(&self, pos: Vec3) -> Option<usize> {
        let gx = (pos.x / self.cell_size).floor() as i32 + self.grid_extent.x / 2;
        let gy = (pos.y / self.cell_size).floor() as i32 + self.grid_extent.y / 2;
        let gz = (pos.z / self.cell_size).floor() as i32 + self.grid_extent.z / 2;

        if gx >= 0 && gx < self.grid_extent.x &&
           gy >= 0 && gy < self.grid_extent.y &&
           gz >= 0 && gz < self.grid_extent.z {
            let idx = (gz * self.grid_extent.y * self.grid_extent.x + gy * self.grid_extent.x + gx) as usize;
            Some(idx)
        } else {
            None
        }
    }

    pub fn deposit(&mut self, pos: Vec3, ptype: PheromoneType, amount: f32) {
        if let Some(idx) = self.world_to_grid(pos) {
            match ptype {
                PheromoneType::KinScent => self.cells[idx].kin += amount,
                PheromoneType::AlarmSignal => self.cells[idx].alarm += amount,
                PheromoneType::ReproductiveReadiness => self.cells[idx].reproductive += amount,
            }
        }
    }

    pub fn sample(&self, pos: Vec3) -> ChemicalConcentration {
        if let Some(idx) = self.world_to_grid(pos) {
            self.cells[idx]
        } else {
            ChemicalConcentration::default()
        }
    }

    pub fn diffuse_and_decay(&mut self, dt: f32) {
        let decay_factor = (1.0 - self.decay_rate * dt).max(0.0);
        for cell in &mut self.cells {
            cell.kin *= decay_factor;
            cell.alarm *= decay_factor;
            cell.reproductive *= decay_factor;
        }
    }
}

pub fn update_pheromones_system(
    mut grid: ResMut<PheromoneGrid3D>,
    time: Res<Time>,
) {
    grid.diffuse_and_decay(time.delta_secs());
}
