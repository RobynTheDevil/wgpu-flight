#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unreachable_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_must_use)]

use glam::*;
use crate::{
    hasher::*,
    player::Player,
    render::IndexedMesh,
};

pub mod bobbins;

pub trait World {
    fn new() -> Self where Self: Sized;
    fn initialize(&mut self);
    fn update(&mut self, player: &Player);
    fn get_meshes(&self) -> (Vec<(SeaHashKey, &IndexedMesh)>, &SeaHashSet<SeaHashKey>) {panic!("Meshes Not Implemented")}
}

pub trait WorldObject {
    fn get_position(&self) -> DVec3;
    fn get_rotation(&self) -> DMat4;
}

