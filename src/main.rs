extern crate rand;
extern crate sdl2;

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;

use std::thread;
use std::time::Duration;

const CLOCK_FREQ_HZ: u64 = 500;
const VSYNC_FREQ_HZ: u64 = 60;

struct Chip8 {
    canvas: WindowCanvas,

    sram: Memory,
    vpu: VPU,

    regs: [u8; 16],
    ir: u16,
    pc: u16,

    stack: [u16; 16],
    sp: u8,

    dt: Timer,
    st: Timer,
}

struct Memory(Vec<u8>);

impl Memory {
    fn new(sz: u16) -> Memory {
        Memory(vec![0; sz as usize])
    }

    fn load(&mut self, from: u16, data: &[u8]) {
        for (i, d) in data.iter().enumerate() {
            self.write(from + i as u16, *d);
        }
    }

    fn read(&self, addr: u16) -> u8 {
        self.0[addr as usize]
    }

    fn write(&mut self, addr: u16, val: u8) {
        self.0[addr as usize] = val;
    }
}

struct VPU {
    data: Vec<bool>,
    w: usize,
    h: usize,
}

impl VPU {
    fn new(w: usize, h: usize) -> VPU {
        VPU {
            data: vec![false; w * h],
            w,
            h,
        }
    }

    fn clear(&mut self) {
        self.data = vec![false; self.w * self.h]
    }

    fn idx(&self, c: (u8, u8)) -> usize {
        (c.1 as usize % self.h) * self.w + (c.0 as usize % self.w)
    }

    fn read(&self, c: (u8, u8)) -> bool {
        self.data[self.idx(c)]
    }

    fn write(&mut self, c: (u8, u8), v: bool) -> bool {
        let i = self.idx(c);

        self.data[i] ^= v;
        !self.data[i]
    }
}

struct Instruction(u16);

impl Instruction {
    fn opcode(&self) -> u8 {
        (self.0 >> 12) as u8
    }
    fn addr(&self) -> u16 {
        self.0 & 0x0FFF
    }
    fn imm4(&self) -> u8 {
        (self.0 & 0x000F) as u8
    }
    fn imm8(&self) -> u8 {
        (self.0 & 0x00FF) as u8
    }
    fn reg_h(&self) -> usize {
        (self.0 >> 8) as usize & 0xF
    }
    fn reg_l(&self) -> usize {
        (self.0 >> 4) as usize & 0xF
    }
}

#[derive(Default)]
struct Timer {
    counter: u8,
}

impl Timer {
    fn value(&self) -> u8 {
        self.counter
    }

    fn reload(&mut self, v: u8) {
        self.counter = v;
    }

    fn tick(&mut self) {
        if self.is_active() {
            self.counter -= 1;
        }
    }

    fn is_active(&self) -> bool {
        self.counter != 0
    }
}

impl Chip8 {
    fn new(canvas: WindowCanvas, rom: &[u8]) -> Chip8 {
        let mut c = Chip8 {
            canvas: canvas,

            sram: Memory::new(0x1000),
            vpu: VPU::new(64, 32),

            regs: [0x0; 16],
            pc: 0x200,
            ir: 0x0,

            stack: [0x0; 16],
            sp: 0x0,

            dt: Timer::default(),
            st: Timer::default(),
        };

        // Digit sprites reside at 0x000
        c.sram.load(
            0x0,
            &[
                0xF0, 0x90, 0x90, 0x90, 0xF0, 0x20, 0x60, 0x20, 0x20, 0x70, 0xF0, 0x10, 0xF0, 0x80,
                0xF0, 0xF0, 0x10, 0xF0, 0x10, 0xF0, 0x90, 0x90, 0xF0, 0x10, 0x10, 0xF0, 0x80, 0xF0,
                0x10, 0xF0, 0xF0, 0x80, 0xF0, 0x90, 0xF0, 0xF0, 0x10, 0x20, 0x40, 0x40, 0xF0, 0x90,
                0xF0, 0x90, 0xF0, 0xF0, 0x90, 0xF0, 0x10, 0xF0, 0xF0, 0x90, 0xF0, 0x90, 0x90, 0xE0,
                0x90, 0xE0, 0x90, 0xE0, 0xF0, 0x80, 0x80, 0x80, 0xF0, 0xE0, 0x90, 0x90, 0x90, 0xE0,
                0xF0, 0x80, 0xF0, 0x80, 0xF0, 0xF0, 0x80, 0xF0, 0x80, 0x80,
            ],
        );

        // Program/data memory starts at 0x200
        c.sram.load(0x200, &rom[..]);

        c
    }

    fn fetch(&mut self) -> Instruction {
        let high = self.sram.read(self.pc) as u16;
        let low = self.sram.read(self.pc + 1) as u16;
        self.pc += 2;
        Instruction((high << 8) | low)
    }

    fn step(&mut self) {
        let instr = self.fetch();

        let x = instr.reg_h();
        let y = instr.reg_l();

        match instr.opcode() {
            0x0 => match instr.imm8() {
                0xE0 => self.vpu.clear(),
                0xEE => {
                    self.pc = self.stack[self.sp as usize];
                    self.sp -= 1;
                }
                _ => unreachable!(),
            },
            0x1 => self.pc = instr.addr(),
            0x2 => {
                self.sp += 1;
                self.stack[self.sp as usize] = self.pc;
                self.pc = instr.addr();
            }
            0x3 => {
                if self.regs[x] == instr.imm8() {
                    self.pc += 2;
                }
            }
            0x4 => {
                if self.regs[x] != instr.imm8() {
                    self.pc += 2;
                }
            }
            0x5 => {
                if self.regs[x] == self.regs[y] {
                    self.pc += 2;
                }
            }
            0x6 => self.regs[x] = instr.imm8(),
            0x7 => self.regs[x] = self.regs[x].wrapping_add(instr.imm8()),
            0x8 => match instr.imm4() {
                0x0 => self.regs[x] = self.regs[y],
                0x1 => self.regs[x] |= self.regs[y],
                0x2 => self.regs[x] &= self.regs[y],
                0x3 => self.regs[x] ^= self.regs[y],
                0x4 => {
                    let a = self.regs[x] as u16 + self.regs[y] as u16;
                    self.regs[x] = (a & 0xFF) as u8;
                    self.regs[0xF] = (a >> 8) as u8;
                }
                0x5 => {
                    self.regs[0xF] = if self.regs[y] > self.regs[x] { 0 } else { 1 };
                    self.regs[x] = self.regs[x].wrapping_sub(self.regs[y]);
                }
                0x6 => {
                    self.regs[0xF] = self.regs[x] & 0x1;
                    self.regs[x] >>= 1;
                }
                0x7 => {
                    self.regs[0xF] = if self.regs[x] > self.regs[y] { 0 } else { 1 };
                    self.regs[x] = self.regs[y].wrapping_sub(self.regs[x]);
                }
                0xE => {
                    self.regs[0xF] = self.regs[x] >> 7;
                    self.regs[x] <<= 1;
                }
                _ => unreachable!(),
            },
            0x9 => {
                if self.regs[x] != self.regs[y] {
                    self.pc += 2;
                }
            }
            0xA => self.ir = instr.addr(),
            0xB => self.pc = instr.addr() + self.regs[0] as u16,
            0xC => self.regs[x] = rand::random::<u8>() & instr.imm8(),
            0xD => {
                let x = self.regs[x];
                let y = self.regs[y];

                self.regs[0xF] = 0;

                for i in 0..instr.imm4() {
                    let b = self.sram.read(self.ir + i as u16);

                    for j in 0..8 {
                        let px = (b & (1 << j)) != 0;
                        if self.vpu.write((x + 7 - j, y + i), px) {
                            self.regs[0xF] = 1;
                        }
                    }
                }
            }
            0xE => (),
            0xF => match instr.imm8() {
                0x07 => self.regs[x] = self.dt.value(),
                0x0A => (),
                0x15 => self.dt.reload(self.regs[x]),
                0x18 => self.st.reload(self.regs[x]),
                0x1E => self.ir += self.regs[x] as u16,
                0x29 => self.ir = self.regs[x] as u16 * 5,
                0x33 => {
                    self.sram.write(self.ir + 0, self.regs[x] / 100);
                    self.sram.write(self.ir + 1, (self.regs[x] % 100) / 10);
                    self.sram.write(self.ir + 2, self.regs[x] % 10);
                }
                0x55 => {
                    for i in 0..x + 1 {
                        self.sram.write(self.ir + i as u16, self.regs[i]);
                    }
                }
                0x65 => {
                    for i in 0..x + 1 {
                        self.regs[i] = self.sram.read(self.ir + i as u16);
                    }
                }
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }

    fn run(&mut self) -> ! {
        let instr_per_tick = CLOCK_FREQ_HZ / VSYNC_FREQ_HZ;

        loop {
            for _ in 0..instr_per_tick {
                self.step();
            }

            self.dt.tick();
            self.st.tick();

            self.draw();

            thread::sleep(Duration::from_micros(1_000_000 / VSYNC_FREQ_HZ));
        }
    }

    fn draw(&mut self) {
        self.canvas.set_draw_color(Color::RGB(0, 0, 0));
        self.canvas.clear();

        for y in 0..self.vpu.h {
            for x in 0..self.vpu.w {
                if self.vpu.read((x as u8, y as u8)) {
                    self.canvas.set_draw_color(Color::RGB(255, 255, 255));
                } else {
                    self.canvas.set_draw_color(Color::RGB(0, 0, 0));
                }

                self.canvas
                    .fill_rect(Rect::new(x as i32 * 10, y as i32 * 10, 10, 10))
                    .unwrap();
            }
        }
        self.canvas.present();
    }
}

fn main() {
    let rom = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();

    let ctx = sdl2::init().unwrap();
    let video = ctx.video().unwrap();

    let win = video
        .window("CHIP-8", 640, 320)
        .position_centered()
        .build()
        .unwrap();

    let canvas = win.into_canvas().build().unwrap();

    Chip8::new(canvas, &rom[..]).run();
}
