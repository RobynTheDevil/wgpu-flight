use wgpu::*;
use crate::game::Game;
use super::Pass;

//{{{ CameraUniform

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CameraUniform {
    pub position : [f32; 4],
    pub mat_view : [[f32; 4]; 4],
    pub mat_proj : [[f32; 4]; 4],
}

impl CameraUniform {

    pub const fn size_of() -> usize { std::mem::size_of::<Self>() }

    pub fn as_mem(&self) -> &[u8; Self::size_of()] {
        let arr = unsafe { std::mem::transmute::<&Self, &[u8; Self::size_of()]>(self) };
        arr
    }

}

//}}}

// LightUniform {{{

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct LightUniform {
    pub position: [f32; 4],
    pub color: [f32; 4],
    pub ambient_color_strength: [f32; 4],
    pub diffuse_color_strength: [f32; 4],
    pub specular_color_strength: [f32; 4],
    pub direction: [f32; 4],
}

impl LightUniform {
    pub const fn size_of() -> usize { std::mem::size_of::<Self>() }

    pub fn as_mem(&self) -> &[u8; Self::size_of()] {
        let arr = unsafe { std::mem::transmute::<&Self, &[u8; Self::size_of()]>(self) };
        arr
    }
}

//}}}

pub struct Globals {
    pub uniform_buffer: Buffer,
    pub bind_group_layout: BindGroupLayout,
    pub bind_group: BindGroup,
}

impl Globals {
    pub fn new(
        device: &Device,
        surface_config: &SurfaceConfiguration,
    ) -> Self {
        let uniform_buffer = device.create_buffer(
            &BufferDescriptor {
                size: (CameraUniform::size_of()
                    + LightUniform::size_of()) as u64,
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                mapped_at_creation: false,
                label: Some("Camera and Light Uniform"),
        });
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                // camera and light uniform
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("Global Layout"),
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry{
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
            label: Some("Globals"),
        });

        Self {
            uniform_buffer,
            bind_group_layout,
            bind_group,
        }

    }
}

impl Pass for Globals {

    fn update(&mut self, queue: &Queue, game: &Game) {
        //uniforms
        let camera = game.get_camera_uniform();
        queue.write_buffer(&self.uniform_buffer, 0, camera.as_mem());
        let light = game.light.to_light_uniform();
        queue.write_buffer(&self.uniform_buffer, CameraUniform::size_of() as u64, light.as_mem());
    }

    fn draw(&mut self, view: &TextureView, encoder: &mut CommandEncoder) -> Result<(), SurfaceError> {
        panic!("dont draw globals");
    }
}

