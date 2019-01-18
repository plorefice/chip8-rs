extern crate chip8;
extern crate sdl2;

use std::thread;
use std::time::Duration;

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;

use chip8::core::Chip8;

const CLOCK_FREQ_HZ: u64 = 500;
const VSYNC_FREQ_HZ: u64 = 60;

fn main() {
    let rom = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();

    let ctx = sdl2::init().unwrap();
    let video = ctx.video().unwrap();

    let win = video
        .window("CHIP-8", 640, 320)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = win.into_canvas().build().unwrap();

    let mut cpu = Chip8::new();
    cpu.load(&rom[..]);

    let instr_per_tick = CLOCK_FREQ_HZ / VSYNC_FREQ_HZ;

    loop {
        for _ in 0..instr_per_tick {
            cpu.step();
        }
        cpu.tick();

        render(&mut canvas, &mut cpu);

        thread::sleep(Duration::from_micros(1_000_000 / VSYNC_FREQ_HZ));
    }
}

fn render(canvas: &mut WindowCanvas, cpu: &mut Chip8) {
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();

    let vpu = cpu.vpu();

    for y in 0..vpu.size().1 {
        for x in 0..vpu.size().0 {
            if vpu.read((x as u8, y as u8)) {
                canvas.set_draw_color(Color::RGB(255, 255, 255));
            } else {
                canvas.set_draw_color(Color::RGB(0, 0, 0));
            }

            canvas
                .fill_rect(Rect::new(x as i32 * 10, y as i32 * 10, 10, 10))
                .unwrap();
        }
    }
    canvas.present();
}
