use bevy::prelude::*;

#[derive(Event, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParticleEvent {
    RandomizePositions,
    RandomizeColors,
}
