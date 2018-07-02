use sdl2;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;
use std::time::Instant;

use cpu::{Cpu, Display};

const SCREEN_WIDTH: u32 = 640;
const SCREEN_HEIGHT: u32 = 320;

const CELL_WIDTH: u32 = SCREEN_WIDTH / 64;
const CELL_HEIGHT: u32 = SCREEN_HEIGHT / 32;

fn draw_screen(display: &Display, canvas: &mut sdl2::render::WindowCanvas) {
    for (y, row) in display.pixels.iter().enumerate() {
        for i in (0..64).rev() {
            let mask = 1 << i;
            let filled = (row & mask) >> i != 0;
            if filled {
                canvas.set_draw_color(Color::RGB(255, 255, 255));
                canvas.fill_rect(Rect::new((63 - i) as i32 * CELL_WIDTH as i32, y as i32 * CELL_HEIGHT as i32,
                                           CELL_WIDTH, CELL_HEIGHT)).unwrap();
            }
        }
    }
}

pub fn run(filepath: Option<&str>, _debug_mode: bool) {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("rust-chip8", SCREEN_WIDTH, SCREEN_HEIGHT)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let (mut cpu, _) = Cpu::from_program_file(filepath);

    let mut cpu_timer = Instant::now();
    let mut sdl_timer = Instant::now();

    'running: loop {
        if sdl_timer.elapsed().subsec_nanos() > 1_000_000_000u32 / 60 {
            canvas.set_draw_color(Color::RGB(0, 0, 0));
            canvas.clear();

            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } |
                    Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                        break 'running
                    },
                    _ => {}
                }
            }

            draw_screen(&cpu.display, &mut canvas);
            canvas.present();

            sdl_timer = Instant::now();
        }

        if cpu_timer.elapsed().subsec_nanos() > 1_000_000_000u32 / 540 {
            cpu.tick();
            cpu_timer = Instant::now();
        }
    }
}
