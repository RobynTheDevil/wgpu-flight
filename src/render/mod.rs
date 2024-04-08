use wgpu::SurfaceError;
use crate::gpu::Gpu;

pub mod terrain;

pub trait Pass {
    fn draw(&mut self, gpu: &Gpu) -> Result<(), SurfaceError>;
}

//{{{ Vertex

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub position: [f32; 4],
    pub normal: [f32; 4],
    pub color: [f32; 4],
}
impl Vertex {
    pub const fn size_of() -> usize { std::mem::size_of::<Self>() }
}

//}}}

//{{{ CameraUniform

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct CameraUniform {
    position : [f32; 4],
    mat_view : [[f32; 4]; 4],
    mat_proj : [[f32; 4]; 4],
}

impl CameraUniform {

    const fn size_of() -> usize { std::mem::size_of::<Self>() }

    fn as_mem(&self) -> &[u8; Self::size_of()] {
        let arr = unsafe { std::mem::transmute::<&Self, &[u8; Self::size_of()]>(self) };
        arr
    }

}

//}}}

// LightUniform {{{

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct LightUniform {
    position: [f32; 4],
    color: [f32; 4],
    ambient_color_strength: [f32; 4],
    diffuse_color_strength: [f32; 4],
    specular_color_strength: [f32; 4],
    direction: [f32; 4],
}

impl LightUniform {
    const fn size_of() -> usize { std::mem::size_of::<Self>() }

    fn as_mem(&self) -> &[u8; Self::size_of()] {
        let arr = unsafe { std::mem::transmute::<&Self, &[u8; Self::size_of()]>(self) };
        arr
    }
}

//}}}

// Uniform pool?

