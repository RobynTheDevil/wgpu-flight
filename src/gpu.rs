#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unreachable_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_must_use)]

use wgpu::*;
use sdl2::video::Window;
use crate::{
    render::{*, sdf::*, terrain::*},
    game::Game
};

pub struct Gpu {
    pub surface: Surface,
    pub device: Device,
    pub queue: Queue,
    pub config: SurfaceConfiguration,
    pub pass: Box<dyn Pass>,
}

impl Gpu {

    // TODO max buffer size is only absolute max. could downscale if needed
    // expected 128 MB?
    pub fn max_verts() -> u64 { Limits::downlevel_defaults().max_buffer_size / Vertex::size_of() as u64 }
    pub fn max_inds() -> u64 { Limits::downlevel_defaults().max_buffer_size / 4 } // 4 bytes per u32

    // Creating some of the wgpu types requires async code
    pub async fn new(window: &Window) -> Gpu {
        let (width, height) = window.size();

        let instance = Instance::new( InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
            //dx12_shader_compiler: Default::default(),
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
                limits: Limits::downlevel_defaults(),
                features: Features::VERTEX_WRITABLE_STORAGE,
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

        let pass = Box::new( TerrainPass::new(&TerrainConfig {}, &device, &config) );
        //let pass = Box::new( SdfPass::new(&device, &config) );

        Self {
            surface,
            device,
            queue,
            config,
            pass,
        }
    }

    pub fn width(&self) -> u32 { self.config.width }
    pub fn height(&self) -> u32 { self.config.height }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width as u32;
        self.config.height = height as u32;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn get_current_texture(&self) -> SurfaceTexture {
        return match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(err) => {
                let reason = match err {
                    SurfaceError::Timeout => "Timeout",
                    SurfaceError::Outdated => "Outdated",
                    SurfaceError::Lost => "Lost",
                    SurfaceError::OutOfMemory => "OutOfMemory",
                };
                panic!("Failed to get current surface texture! Reason: {}", reason)
            }
        };
    }

    pub fn render(&mut self, gamedata: &GameData) -> Result<(), SurfaceError> {
        let frame = self.get_current_texture();
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("command_encoder"),
        });
        self.pass.update(&self.queue, gamedata);
        self.pass.draw(&view, &mut encoder);
        self.queue.submit([encoder.finish()]);
        frame.present();
        Ok(())
    }

}

