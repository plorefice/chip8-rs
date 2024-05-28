use super::memory::Memory;
use super::periph::{Keypad, Timer, VPU};

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

pub struct Chip8 {
    // Memories
    sram: Memory,
    vpu: VPU,

    // CPU registers and stack
    regs: [u8; 16],
    stack: [u16; 16],
    ir: u16,
    pc: u16,
    sp: u8,

    // Timers
    dt: Timer,
    st: Timer,

    // Input
    keypad: Keypad,
}

impl Default for Chip8 {
    fn default() -> Chip8 {
        Chip8 {
            sram: Memory::new(0x1000),
            vpu: VPU::new(64, 32),

            regs: [0x0; 16],
            stack: [0x0; 16],
            pc: 0x200,
            ir: 0x0,
            sp: 0x0,

            dt: Timer::default(),
            st: Timer::default(),

            keypad: Keypad::default(),
        }
    }
}

impl Chip8 {
    pub fn new() -> Chip8 {
        Chip8::default()
    }

    pub fn with_rom(rom: &[u8]) -> Chip8 {
        let mut c = Chip8::new();
        c.load(rom);
        c
    }

    pub fn load(&mut self, rom: &[u8]) {
        // Digit sprites reside at 0x000
        self.sram.load(
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

        // Program/data start at 0x200
        self.sram.load(0x200, rom);
    }

    pub fn vpu(&self) -> &VPU {
        &self.vpu
    }

    pub fn keypad_mut(&mut self) -> &mut Keypad {
        &mut self.keypad
    }

    pub fn tick(&mut self) {
        self.dt.tick();
        self.st.tick();
    }

    pub fn step(&mut self) {
        let instr = self.fetch();

        let opcode = instr.opcode();
        let x = instr.reg_h();
        let y = instr.reg_l();
        let n = instr.imm4();
        let kk = instr.imm8();
        let nnn = instr.addr();

        match opcode {
            0x0 => match kk {
                0xE0 => self.vpu.clear(),
                0xEE => {
                    self.pc = self.stack[self.sp as usize];
                    self.sp -= 1;
                }
                _ => {}
            },
            0x1 => self.pc = nnn,
            0x2 => {
                self.sp += 1;
                self.stack[self.sp as usize] = self.pc;
                self.pc = nnn;
            }
            0x3 => {
                if self.regs[x] == kk {
                    self.pc += 2;
                }
            }
            0x4 => {
                if self.regs[x] != kk {
                    self.pc += 2;
                }
            }
            0x5 => {
                if self.regs[x] == self.regs[y] {
                    self.pc += 2;
                }
            }
            0x6 => self.regs[x] = kk,
            0x7 => self.regs[x] = self.regs[x].wrapping_add(kk),
            0x8 => match n {
                0x0 => self.regs[x] = self.regs[y],
                0x1 => {
                    self.regs[x] |= self.regs[y];
                    self.regs[0xF] = 0;
                }
                0x2 => {
                    self.regs[x] &= self.regs[y];
                    self.regs[0xF] = 0;
                }
                0x3 => {
                    self.regs[x] ^= self.regs[y];
                    self.regs[0xF] = 0;
                }
                0x4 => {
                    let a = u16::from(self.regs[x]) + u16::from(self.regs[y]);
                    self.regs[x] = (a & 0xFF) as u8;
                    self.regs[0xF] = (a >> 8) as u8;
                }
                0x5 => {
                    let vf = if self.regs[x] >= self.regs[y] { 1 } else { 0 };
                    self.regs[x] = self.regs[x].wrapping_sub(self.regs[y]);
                    self.regs[0xF] = vf;
                }
                0x6 => {
                    let vf = self.regs[y] & 0x1;
                    self.regs[x] = self.regs[y] >> 1;
                    self.regs[0xF] = vf;
                }
                0x7 => {
                    let vf = if self.regs[y] >= self.regs[x] { 1 } else { 0 };
                    self.regs[x] = self.regs[y].wrapping_sub(self.regs[x]);
                    self.regs[0xF] = vf;
                }
                0xE => {
                    let vf = (self.regs[y] >> 7) & 0x1;
                    self.regs[x] = self.regs[y] << 1;
                    self.regs[0xF] = vf;
                }
                _ => unreachable!(),
            },
            0x9 => match n {
                0x0 => {
                    if self.regs[x] != self.regs[y] {
                        self.pc += 2;
                    }
                }
                _ => unreachable!(),
            },
            0xA => self.ir = nnn,
            0xB => self.pc = nnn + u16::from(self.regs[0]),
            0xC => self.regs[x] = rand::random::<u8>() & kk,
            0xD => {
                let x = u16::from(self.regs[x]) % 0x40;
                let y = u16::from(self.regs[y]) % 0x20;

                self.regs[0xF] = 0;

                for i in 0..u16::from(n) {
                    let b = self.sram.read(self.ir + i);

                    for j in 0..8 {
                        let px = (b & (1 << j)) != 0;
                        if self.vpu.write((x + 7 - j, y + i), px) {
                            self.regs[0xF] = 1;
                        }
                    }
                }
            }
            0xE => match kk {
                0x9E => {
                    if self.keypad.get_state(self.regs[x]) {
                        self.pc += 2;
                    }
                }
                0xA1 => {
                    if !self.keypad.get_state(self.regs[x]) {
                        self.pc += 2;
                    }
                }
                _ => unreachable!(),
            },
            0xF => match kk {
                0x07 => self.regs[x] = self.dt.value(),
                0x0A => {
                    self.pc -= 2;

                    if self.keypad.has_changed() {
                        for i in 0u8..16 {
                            if self.keypad.get_state(i) {
                                self.regs[x] = i;
                                self.pc += 2;
                                break;
                            }
                        }
                    }
                }
                0x15 => self.dt.reload(self.regs[x]),
                0x18 => self.st.reload(self.regs[x]),
                0x1E => {
                    self.ir += u16::from(self.regs[x]);
                    self.regs[0xF] = if self.ir > 0xFFF { 1 } else { 0 };
                }
                0x29 => self.ir = u16::from(self.regs[x]) * 5,
                0x33 => {
                    self.sram.write(self.ir, self.regs[x] / 100);
                    self.sram.write(self.ir + 1, (self.regs[x] % 100) / 10);
                    self.sram.write(self.ir + 2, self.regs[x] % 10);
                }
                0x55 => {
                    for i in 0..=x {
                        self.sram.write(self.ir + i as u16, self.regs[i]);
                    }
                    self.ir += (x + 1) as u16;
                }
                0x65 => {
                    for i in 0..=x {
                        self.regs[i] = self.sram.read(self.ir + i as u16);
                    }
                    self.ir += (x + 1) as u16;
                }
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }

    fn fetch(&mut self) -> Instruction {
        let high = u16::from(self.sram.read(self.pc));
        let low = u16::from(self.sram.read(self.pc + 1));
        self.pc += 2;
        Instruction((high << 8) | low)
    }
}
