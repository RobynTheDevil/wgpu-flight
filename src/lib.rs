#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unreachable_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_must_use)]

use std::collections::HashSet;
use pollster::FutureExt as _;
use glam::*;
use sdl2::{
    Sdl, EventPump,
    event::{Event, WindowEvent},
    video::Window,
    keyboard::*,
    mouse::*,
};

pub mod gpu;
pub mod render;
pub mod world;
pub mod math;
pub mod game;
pub mod player;

use crate::gpu::Gpu;
use crate::game::Game;

pub struct App {
    pub sdl_context: Sdl,
    pub title: String,
    pub events: EventPump,
    pub window: Window,
    pub game: Game,
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
        let game = Game::new();
        sdl_context.mouse().set_relative_mouse_mode(true);
        Ok(Self {
            sdl_context,
            title,
            events,
            window,
            game,
        })
    }

    pub async fn run(&mut self) -> Result<(), String> {

        let mut gpu = Gpu::new(&self.window).await;

        self.game.initialize();
        let mut timer = std::time::Instant::now();
        let mut fps_avg = 0.0;
        let mut prev_keys = HashSet::new();
        let mut orig_pos = ivec2(400, 300);
        self.sdl_context.mouse().warp_mouse_in_window(&self.window, orig_pos.x, orig_pos.y);

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
                        orig_pos = ivec2(width / 2, height / 2);
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
            
            let mouse = self.events.mouse_state();
            let pos = ivec2(mouse.x(), mouse.y());
            let change_pos = pos - orig_pos;
            println!("pos: {:?}", change_pos);
            self.sdl_context.mouse().warp_mouse_in_window(&self.window, orig_pos.x, orig_pos.y);

            self.game.update(elapsed_seconds, &keys, change_pos)?;
            prev_keys = keys;

            // game render
            let gamedata = self.game.get_gamedata();

            gpu.render(&gamedata);
        }

    }

}

pub fn start() -> Result<(), String> {
    let title = String::from("SDFShader");
    let mut app = App::new(title, None, None)?;
    app.run().block_on()?;
    Ok(())
}

