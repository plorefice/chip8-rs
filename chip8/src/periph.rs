use alloc::vec::Vec;

#[derive(Default)]
pub struct Timer {
    counter: u8,
}

impl Timer {
    pub fn value(&self) -> u8 {
        self.counter
    }

    pub fn reload(&mut self, v: u8) {
        self.counter = v;
    }

    pub fn is_active(&self) -> bool {
        self.counter != 0
    }

    pub fn tick(&mut self) {
        if self.is_active() {
            self.counter -= 1;
        }
    }
}

#[derive(Default)]
pub struct Keypad {
    state: [bool; 16],
    changed: bool,
}

impl Keypad {
    pub fn set_state(&mut self, key: u8, pressed: bool) {
        if self.state[key as usize] != pressed {
            self.changed = true;
        }
        self.state[key as usize] = pressed;
    }

    pub fn get_state(&self, key: u8) -> bool {
        self.state[key as usize]
    }

    pub fn has_changed(&mut self) -> bool {
        let changed = self.changed;
        self.changed = false;
        changed
    }
}

pub struct VPU {
    data: Vec<bool>,
    w: usize,
    h: usize,
}

impl VPU {
    pub fn new(w: usize, h: usize) -> VPU {
        VPU {
            data: alloc::vec![false; w * h],
            w,
            h,
        }
    }

    pub fn get_data(&self) -> &[bool] {
        &self.data[..]
    }

    pub fn size(&self) -> (usize, usize) {
        (self.w, self.h)
    }

    pub fn clear(&mut self) {
        self.data = alloc::vec![false; self.w * self.h]
    }

    pub fn read(&self, c: (u16, u16)) -> bool {
        self.data[self.idx(c)]
    }

    pub fn write(&mut self, c: (u16, u16), v: bool) -> bool {
        if c.0 >= self.w as u16 || c.1 >= self.h as u16 {
            return false;
        }
        let i = self.idx(c);
        let r = self.data[i] && v;
        self.data[i] ^= v;
        r
    }

    fn idx(&self, c: (u16, u16)) -> usize {
        (c.1 as usize % self.h) * self.w + (c.0 as usize % self.w)
    }
}
