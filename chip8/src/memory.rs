use alloc::vec::Vec;

pub struct Memory(Vec<u8>);

impl Memory {
    pub fn new(sz: u16) -> Memory {
        Memory(alloc::vec![0; sz as usize])
    }

    pub fn load(&mut self, from: u16, data: &[u8]) {
        for (i, d) in data.iter().enumerate() {
            self.write(from + i as u16, *d);
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.0[addr as usize]
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        self.0[addr as usize] = val;
    }
}
