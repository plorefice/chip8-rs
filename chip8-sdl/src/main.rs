extern crate chip8;
extern crate sdl2;

use std::thread;
use std::time::Duration;

use sdl2::event::Event;
use sdl2::keyboard::Scancode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;

use chip8::core::Chip8;

const CLOCK_FREQ_HZ: u64 = 500;
const VSYNC_FREQ_HZ: u64 = 60;
const INSTR_PER_TICK: u64 = CLOCK_FREQ_HZ / VSYNC_FREQ_HZ;

const KEYPAD_MAP: [Scancode; 16] = [
    Scancode::X,    // 0
    Scancode::Num1, // 1
    Scancode::Num2, // 2
    Scancode::Num3, // 3
    Scancode::Q,    // 4
    Scancode::W,    // 5
    Scancode::E,    // 6
    Scancode::A,    // 7
    Scancode::S,    // 8
    Scancode::D,    // 9
    Scancode::Z,    // A
    Scancode::C,    // B
    Scancode::Num4, // C
    Scancode::R,    // D
    Scancode::F,    // E
    Scancode::V,    // F
];

fn main() {
    let fname = match std::env::args().nth(1) {
        Some(fname) => fname,
        None => {
            println!("USAGE: chip8-sdl ROM-FILE");
            std::process::exit(1);
        }
    };

    let rom = match std::fs::read(&fname) {
        Ok(b) => b,
        Err(e) => {
            println!("could not open {}: {}", &fname, e);
            std::process::exit(1);
        }
    };

    let ctx = sdl2::init().expect("could not init SDL2");
    let video = ctx.video().expect("could not retrieve video subsystem");
    let mut events = ctx.event_pump().expect("could not retrieve event pump");

    let mut canvas = video
        .window("CHIP-8", 640, 320)
        .position_centered()
        .build()
        .expect("could not create window")
        .into_canvas()
        .build()
        .expect("could not create canvas");

    let mut cpu = Chip8::with_rom(&rom[..]);

    'outer: loop {
        while let Some(e) = events.poll_event() {
            if let Event::Quit { .. } = e {
                break 'outer;
            }
        }

        for (i, sc) in KEYPAD_MAP.iter().enumerate() {
            cpu.keypad_mut()
                .set_state(i as u8, events.keyboard_state().is_scancode_pressed(*sc));
        }

        (0..INSTR_PER_TICK).for_each(|_| cpu.step());
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
            if vpu.read((x as u16, y as u16)) {
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
