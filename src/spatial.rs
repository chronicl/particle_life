use bevy::prelude::*;

pub struct SpatialPartitioning {
    partitions: Vec<Partition>,
    particle_infos: Vec<ParticleInfo>,

    bounds_x: Vec2,
    bounds_y: Vec2,
    // each dimension split the same number of times for now
    partitions_per_axis: usize,
}

impl SpatialPartitioning {
    pub fn new(bounds_x: Vec2, bounds_y: Vec2, partitions_per_axis: usize) -> Self {
        todo!()
    }
}

pub struct Partition {
    // pub particles: Vec<Particle>,
}

pub struct ParticleInfo {
    partition: PartitionId,
    location: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ParticleId(pub u32);

impl ParticleId {
    pub fn new(id: usize) -> Self {
        Self(id as u32)
    }

    pub fn id(&self) -> usize {
        self.0 as usize
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PartitionId(u32);

impl PartitionId {
    pub fn new(id: usize) -> Self {
        Self(id as u32)
    }

    pub fn id(&self) -> usize {
        self.0 as usize
    }
}
