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

pub struct Keypad {
    state: [bool; 16],
}

impl Keypad {
    pub fn new() -> Keypad {
        Keypad { state: [false; 16] }
    }

    pub fn set_state(&mut self, key: u8, pressed: bool) {
        assert!(key < 16);
        self.state[key as usize] = pressed;
    }

    pub fn get_state(&self, key: u8) -> bool {
        self.state[key as usize]
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
            data: vec![false; w * h],
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
        self.data = vec![false; self.w * self.h]
    }

    pub fn read(&self, c: (u8, u8)) -> bool {
        self.data[self.idx(c)]
    }

    pub fn write(&mut self, c: (u8, u8), v: bool) -> bool {
        let i = self.idx(c);

        self.data[i] ^= v;
        !self.data[i]
    }

    fn idx(&self, c: (u8, u8)) -> usize {
        (c.1 as usize % self.h) * self.w + (c.0 as usize % self.w)
    }
}
