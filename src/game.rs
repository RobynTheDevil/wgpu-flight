use std::collections::HashSet;
use glam::*;
use sdl2::keyboard::Keycode;
use crate::{
    hasher::*,
    math::*,
    player::Player,
    direction::*,
    world::{*,
        //bobbins::BobbinsWorld,
        sdftest::SdfWorld,
    },
    render::{*,
        globals::CameraUniform,
    },
};

pub struct Game {
    pub world: Box<dyn World>,
    pub light: Light,
    pub player : Player,
    pub object_mesh: Mesh,
    pub mat_proj : DMat4,
    pub mat_view : DMat4,
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
            //world: Box::new(BobbinsWorld::new()), // 2^n
            world: Box::new(SdfWorld::new()),
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
        self.world.initialize();
        self.object_mesh = Mesh{..Default::default()};
        // self.object_mesh.load_from_object_file("./models/planejane.obj".to_string());
        // self.object_mesh.load_texture("./models/planejaneUV.png".to_string());

        Ok(())
    }

    pub fn update(&mut self, elapsed_time: f32, keys: &HashSet<Keycode>) -> Result<(), String>
    {
        self.player.get_input(elapsed_time as f64, keys);
        self.world.update(&self.player);
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

    pub fn get_gamedata(&self) -> GameData {
        GameData {
            terrain: self.world.get_data(),
            camera: self.get_camera_uniform(),
            light: self.light.to_light_uniform(),
        }
    }

}

