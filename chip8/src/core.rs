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
    stall: bool,
}

impl Chip8 {
    pub fn new() -> Chip8 {
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

            keypad: Keypad::new(),
            stall: false,
        }
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
        self.sram.load(0x200, &rom[..]);
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
        if self.stall {
            return;
        }

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
            0xE => match instr.imm8() {
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
            0xF => match instr.imm8() {
                0x07 => self.regs[x] = self.dt.value(),
                0x0A => {
                    self.stall = true;

                    for i in 0u8..16 {
                        if self.keypad.get_state(i) {
                            self.regs[x] = i;
                            self.stall = false;
                            break;
                        }
                    }
                }
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

    fn fetch(&mut self) -> Instruction {
        let high = self.sram.read(self.pc) as u16;
        let low = self.sram.read(self.pc + 1) as u16;
        self.pc += 2;
        Instruction((high << 8) | low)
    }
}
