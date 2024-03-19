#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unreachable_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_must_use)]

extern crate glam;
extern crate tobj;
extern crate sdl2;
extern crate wgpu;
extern crate pollster;
extern crate nohash_hasher;

use std::collections::{HashMap, HashSet, BinaryHeap};
use std::hash::{Hasher, BuildHasher};
use seahash::SeaHasher;
use nohash_hasher::IntMap;
use noise::{Perlin, Worley, NoiseFn};
use pollster::FutureExt as _;
use glam::*;
use wgpu::*;
use sdl2::keyboard::*;

struct AppState {
    event_pump: sdl2::EventPump,
    window: sdl2::video::Window,
    gamestate: GameState,
}

impl AppState {

    fn new(title, width, height) -> Result<Self, String> {
        env_logger::init();
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;
        let mut event_pump = sdl_context.event_pump()?;
        let window = video_subsystem
            .window(title, width, height)
            .position_centered()
            .resizable()
            .build()
            .map_err(|e| e.to_string())?;
        let gamestate = GameState::new();
        Ok(Self {
            event_pump,
            window,
            gamestate,
        })
    }

    async fn run(&mut self) -> Result<(), String> {

        let mut gpustate = GPUState::new(&self.window).await;

        self.gamestate.initialize();
        let mut timer = std::time::Instant::now();
        let mut fps_avg = 0.0;
        let mut prev_keys = HashSet::new();

        'running: loop {
            let elapsed_seconds = timer.elapsed().as_secs_f32();
            timer = std::time::Instant::now();
            let fps = 1.0 / elapsed_seconds;
            fps_avg = fps_avg - fps_avg / 5.0 + fps;
            self.window.set_title(format!("Bobbins // FPS {}", fps_avg as i32).as_str());

            for event in self.event_pump.poll_iter() {
                match event {
                    sdl2::event::Event::Window {
                        window_id,
                        win_event: sdl2::event::WindowEvent::SizeChanged(width, height),
                        ..
                    } if window_id == self.window.id() => {
                        gpustate.resize(width as u32, height as u32);
                    }
                    sdl2::event::Event::Quit { .. }
                    | sdl2::event::Event::KeyDown {
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
            let keys = self.event_pump
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

            self.gamestate.update(elapsed_seconds, &keys)?;
            prev_keys = keys;

            gpustate.update(&self.gamestate);
            gpustate.render();

        }

    }

}

//}}}

fn main() -> Result<(), String> {
    let mut app = AppState::new()?;
    app.run().block_on()?;
    println!("{}", (0-1) as u32);
    Ok(())
}

