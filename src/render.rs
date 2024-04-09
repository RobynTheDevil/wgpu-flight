use wgpu::*;

pub mod terrain;
//pub mod sdf;

pub trait Pass {
    fn render(&mut self, view: &TextureView, encoder: &mut CommandEncoder) -> Result<(), SurfaceError>;
}

#[derive(Clone, Copy)]
pub struct Pixel
{
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

//{{{ Vertex

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub position: [f32; 4],
    pub normal: [f32; 4],
    pub color: [f32; 4],
}

impl Default for Vertex{
    fn default() -> Self {
        Self {
            position: [0.0; 4],
            normal: [0.0; 4],
            color: [1.0; 4],
        }
    }
}

impl Vertex {
    pub const fn size_of() -> usize { std::mem::size_of::<Self>() }
}

//}}}

//{{{ SimpleTexture

pub struct SimpleTexture {
    pub texture: Texture,
    pub view: TextureView,
    pub sampler: Sampler,
}

impl SimpleTexture {
    pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

    pub fn create_depth_texture(device: &Device, config: &SurfaceConfiguration, label: &str) -> Self {
        let size = Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
        let desc = TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = device.create_sampler(
            &SamplerDescriptor {
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                address_mode_w: AddressMode::ClampToEdge,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                mipmap_filter: FilterMode::Nearest,
                compare: Some(CompareFunction::LessEqual),
                lod_min_clamp: 0.0,
                lod_max_clamp: 100.0,
                ..Default::default()
            }
        );

        Self {
            texture,
            view,
            sampler,
        }
    }
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

