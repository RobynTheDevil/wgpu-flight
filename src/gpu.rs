#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unreachable_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_must_use)]

use wgpu::*;
use sdl2::video::Window;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct Vertex {
    position: [f32; 4],
    normal: [f32; 4],
    color: [f32; 4],
}
impl Vertex {
    const fn size_of() -> usize { std::mem::size_of::<Self>() }
}

pub struct Gpu {
    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
}

impl Gpu {

    // TODO max buffer size is only absolute max. could downscale if needed
    // expected 128 MB?
    fn max_verts() -> u64 { Limits::downlevel_defaults().max_buffer_size / Vertex::size_of() as u64 }
    fn max_inds() -> u64 { Limits::downlevel_defaults().max_buffer_size / 4 } // 4 bytes per u32

    // Creating some of the wgpu types requires async code
    pub async fn new(window: &Window) -> Gpu {
        let (width, height) = window.size();

        let instance = Instance::new( InstanceDescriptor {
            backends: Backends::PRIMARY,
            dx12_shader_compiler: Default::default(),
        });

        let surface = unsafe { instance.create_surface(window) }.unwrap();

        let adapter = instance.request_adapter(
            &RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            }
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &DeviceDescriptor {
                limits: Limits::default(),
                features: Features::empty(),
                label: Some("device"),
            },
            None,
        ).await.unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .copied()
            .filter(|f| f.describe().srgb)
            .next()
            .unwrap_or(surface_caps.formats[0]);
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            // present_mode: PresentMode::AutoVsync,
            present_mode: PresentMode::AutoNoVsync,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        Self {
            surface,
            device,
            queue,
            config,
        }
    }

    pub fn width(&self) -> u32 { self.config.width }
    pub fn height(&self) -> u32 { self.config.height }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width as u32;
        self.config.height = height as u32;
        self.surface.configure(&self.device, &self.config);
    }

}

