#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unreachable_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_must_use)]

use std::collections::{HashMap, HashSet, BinaryHeap};
use std::hash::{Hasher, BuildHasher};
use seahash::SeaHasher;
use nohash_hasher::IntMap;
use noise::{Perlin, Worley, NoiseFn};
use pollster::FutureExt as _;
use glam::*;
use wgpu::*;
use sdl2::{EventPump, event::{Event, WindowEvent}, video::Window, keyboard::*};

pub mod gpu;

use crate::gpu::Gpu;
// use crate::game::Game;

pub struct Game {}

pub struct App {
    title: String,
    events: EventPump,
    window: Window,
    game: Game,
}

impl App {

    pub fn new(title: String, width: Option<u32>, height: Option<u32>) -> Result<Self, String> {
        env_logger::init();
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;
        let mut events = sdl_context.event_pump()?;
        let window = video_subsystem
            .window(&title, width.unwrap_or(800), height.unwrap_or(600))
            .position_centered()
            .resizable()
            .build()
            .map_err(|e| e.to_string())?;
        //let game = Game::new();
        let game = Game {};
        Ok(Self {
            title,
            events,
            window,
            game,
        })
    }

    pub async fn run(&mut self) -> Result<(), String> {

        let mut gpu = Gpu::new(&self.window).await;

        // self.game.initialize();
        let mut timer = std::time::Instant::now();
        let mut fps_avg = 0.0;
        let mut prev_keys = HashSet::new();

        'running: loop {
            let elapsed_seconds = timer.elapsed().as_secs_f32();
            timer = std::time::Instant::now();
            let fps = 1.0 / elapsed_seconds;
            fps_avg = fps_avg - fps_avg / 5.0 + fps;
            self.window.set_title(format!("{} // FPS {}", self.title, fps_avg as i32).as_str());

            for event in self.events.poll_iter() {
                match event {
                    Event::Window {
                        window_id,
                        win_event: WindowEvent::SizeChanged(width, height),
                        ..
                    } if window_id == self.window.id() => {
                        gpu.resize(width as u32, height as u32);
                    }
                    Event::Quit { .. } | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                    .. } => {
                        break 'running Ok(());
                    }
                    e => {
                        // dbg!(e);
                    }
                }
            }

            // Create a set of pressed Keys.
            let keys = self.events
                .keyboard_state()
                .pressed_scancodes()
                .filter_map(Keycode::from_scancode)
                .collect();
            // Get the difference between the new and old sets.
            let new_keys = &keys - &prev_keys;
            let old_keys = &prev_keys - &keys;
            // if !new_keys.is_empty() || !old_keys.is_empty() {
            //     println!("new_keys: {:?}\told_keys:{:?}", new_keys, old_keys);
            // }

            // self.game.update(elapsed_seconds, &keys)?;
            prev_keys = keys;

            // gpu.update(&self.game);
            // gpu.render();

        }

    }

}

pub fn start() -> Result<(), String> {
    let title = String::from("SDFShader");
    let mut app = App::new(title, None, None)?;
    app.run().block_on()?;
    Ok(())
}

