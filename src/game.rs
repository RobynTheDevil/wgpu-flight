use std::collections::HashSet;
use glam::*;
use sdl2::keyboard::*;
use crate::{
    hasher::*,
    math::*,
    world::*,
    render::{
        Light, Triangle, Mesh,
        globals::CameraUniform,
        terrain::ChunkMesh
    }
};

// Player {{{

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

//}}}

pub struct Game {
    pub world: World,
    pub light: Light,
    pub player : Player,
    pub object_mesh: Mesh,
    pub mat_proj : DMat4,
    pub mat_view : DMat4,
    pub cur_chunk: IVec3,
    pub last_chunk: IVec3,
    pub screen_width: i32,
    pub screen_height: i32,
}

impl Game {

//new{{{

    pub fn new() -> Self
    {
        let mut default_mesh: Mesh = Mesh{..Default::default()};
        // default_mesh.load_from_object_file("./models/planejane.obj".to_string());
        // default_mesh.load_texture("./models/planejaneUV.png".to_string());
        
        default_mesh.rotation = dvec3(0.0, 0.0, 0.0);
        // default_mesh.rotation = dvec3(0.0, std::f64::consts::PI * 3.0 / 2.0, 0.0);

        Self
        {
            world: World::new(), // 2^n
            light: Light::new(
                dvec3(1.0, 0.1, 0.1), 0.1, //ambient
                dvec3(1.0, 1.0, 1.0), 0.2, //diffuse
                dvec3(0.0, 0.1, 1.0), 1.0, //specular
            ),
            player : Player {
                mesh: default_mesh,
                position: DDirection::ZERO,
                rotation: mat_rotation(dvec3(0.0, 0.0, 0.0)),
                // camera_pos: dvec3(0.0, -5.0, 10.0),
                camera_pos: dvec3(0.0, 0.0, 0.0),
                camera_rot: mat_rotation(dvec3(0.0, 0.0, 0.0)),
                player_speed: 15.0,
                camera_speed: 3.0,
            },
            object_mesh: Mesh{..Default::default()},
            mat_view : dmat4(
                dvec4(0.0, 0.0, 0.0, 0.0),
                dvec4(0.0, 0.0, 0.0, 0.0),
                dvec4(0.0, 0.0, 0.0, 0.0),
                dvec4(0.0, 0.0, 0.0, 0.0),
            ),
            mat_proj : dmat4(
                dvec4(0.0, 0.0, 0.0, 0.0),
                dvec4(0.0, 0.0, 0.0, 0.0),
                dvec4(0.0, 0.0, 0.0, 0.0),
                dvec4(0.0, 0.0, 0.0, 0.0),
            ),
            cur_chunk: ivec3(0, 0, 0),
            last_chunk: ivec3(-1, 0, 0),
            screen_width: 800,
            screen_height: 400,
        }
    }

//}}}

    pub fn initialize(&mut self) -> Result<(), String>
    {
        let near = 0.1;
        let far = 10000.0;
        let fov = 90.0;
        // TODO gamestate resizing
        // self.screen_height = olc::screen_height();
        // self.screen_width = olc::screen_width();
        let ratio = self.screen_height as f64 / self.screen_width as f64;
        self.mat_proj = mat_projection(fov, ratio, near, far);

        self.world.generate_chunks(ivec3(0, 0, 0));

        self.object_mesh = Mesh{..Default::default()};
        // self.object_mesh.load_from_object_file("./models/planejane.obj".to_string());
        // self.object_mesh.load_texture("./models/planejaneUV.png".to_string());

        Ok(())
    }

    pub fn update(&mut self, elapsed_time: f32, keys: &HashSet<Keycode>) -> Result<(), String>
    {
        self.player.get_input(elapsed_time as f64, keys);
        self.cur_chunk = World::pos2chunk(self.player.get_position(), self.world.chunk_size);
        if self.cur_chunk != self.last_chunk {
            println!("chunk {} {} {}", self.cur_chunk.x, self.cur_chunk.y, self.cur_chunk.z);
        }
        self.world.generate_chunks(self.cur_chunk);
        self.last_chunk = self.cur_chunk;
        // inverse look at
        self.mat_view = self.player.mat_view();
        Ok(())
    }

    pub fn get_tris_to_raster(&self) -> Vec<Triangle> {
        let mut tris_to_raster = Vec::with_capacity(1000);
        self.object_mesh.preprocess_mesh(&mut tris_to_raster);
        self.player.mesh.preprocess_mesh(&mut tris_to_raster);
        tris_to_raster
    }

    pub fn get_chunks_to_write(&self) -> (Vec<(SeaHashKey, &ChunkMesh)>, &SeaHashSet<SeaHashKey>) {
        let visible = self.world.visible_meshes(self.cur_chunk);
        let updated = &self.world.chunk_updated;
        (visible, updated)
    }

    pub fn destroy(&mut self) -> Result<(), String>
    {
        Ok(())
    }

    pub fn get_camera_uniform(&self) -> CameraUniform {
        CameraUniform{
            position: self.player.get_camera_pos().as_vec3().extend(1.0).to_array(),
            mat_view: self.mat_view.as_mat4().to_cols_array_2d(),
            mat_proj: self.mat_proj.as_mat4().to_cols_array_2d(),
        }
    }

}

