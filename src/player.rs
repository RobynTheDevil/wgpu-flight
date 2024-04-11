use std::collections::HashSet;
use glam::*;
use sdl2::keyboard::*;
use crate::{
    math::*,
    direction::*,
    render::Mesh,
};

pub struct Player
{
    pub mesh: Mesh,
    pub position: DVec3,
    pub rotation: DMat4,
    pub camera_pos: DVec3, // relative playerspace
    pub camera_rot: DMat4,
    pub player_speed: f64,
    pub camera_speed: f64,
}

impl Player
{

// get_input {{{

    pub fn get_input(&mut self, elapsed_time: f64, keys: &HashSet<Keycode>)
    {
        // camera space
        let mut trans = dvec3(0.0, 0.0, 0.0);
        let mut rot   = dvec3(0.0, 0.0, 0.0);
        if keys.contains(&Keycode::W) { trans += DDirection::FORWARD * self.player_speed * elapsed_time; }
        if keys.contains(&Keycode::S) { trans += DDirection::BACK * self.player_speed * elapsed_time; }
        if keys.contains(&Keycode::A) { trans += DDirection::LEFT * self.player_speed * elapsed_time; }
        if keys.contains(&Keycode::D) { trans += DDirection::RIGHT * self.player_speed * elapsed_time; }
        if keys.contains(&Keycode::LShift) { trans += DDirection::DOWN * self.player_speed * elapsed_time; }
        if keys.contains(&Keycode::Space) { trans  += DDirection::UP * self.player_speed * elapsed_time; }
        //rotations subtract for left handed rotation
        if keys.contains(&Keycode::Up) { rot -= DDirection::RIGHT * self.camera_speed * elapsed_time; }
        if keys.contains(&Keycode::Down) { rot -= DDirection::LEFT * self.camera_speed * elapsed_time; }
        if keys.contains(&Keycode::Left) { rot -= DDirection::UP * self.camera_speed * elapsed_time; }
        if keys.contains(&Keycode::Right) { rot -= DDirection::DOWN * self.camera_speed * elapsed_time; }
        if keys.contains(&Keycode::E) { rot -=  DDirection::BACK * self.camera_speed * elapsed_time; }
        if keys.contains(&Keycode::Q) { rot -= DDirection::FORWARD * self.camera_speed * elapsed_time; }
        self.position = self.position + (self.rotation * trans.extend(1.0)).truncate();
        self.rotation = self.rotation * mat_rotation(rot);
        let err = self.rotation.col(0).dot(self.rotation.col(1));
        if err * err > 0.0
        {
            // taylor series estimation for error
            let x_ort = self.rotation.col(0) - self.rotation.col(1) * (err / 2.0);
            let y_ort = self.rotation.col(1) - self.rotation.col(2) * (err / 2.0);
            let z_ort = x_ort.truncate().cross(y_ort.truncate()).extend(0.0);
            self.rotation.x_axis = x_ort.normalize();
            self.rotation.y_axis = y_ort.normalize();
            self.rotation.z_axis = z_ort.normalize();
        }
    }

    pub fn get_camera_pos(&self) -> DVec3 // to world space
    {
        (
             mat_translation(self.position) * self.get_camera_rot() * self.camera_pos.extend(1.0)
        ).truncate()
        // (mat_translation(self.position) * self.camera_pos.extend(1.0)).truncate()
    }

    pub fn get_camera_rot(&self) -> DMat4 // to world space
    {
         self.rotation * self.camera_rot
    }

    pub fn mat_view(&self) -> DMat4
    {
        mat_quick_inv(mat_look_at(self.get_camera_pos(), self.get_camera_rot()))
    }

//}}}

    pub fn get_position(&self) -> DVec3
    {
        self.mesh.position + self.position
    }

    pub fn get_rotation(&self) -> DMat4
    {
         self.rotation * mat_rotation(self.mesh.rotation)
    }

}

