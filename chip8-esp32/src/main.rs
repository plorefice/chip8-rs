#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use chip8::core::Chip8;
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{clock::ClockControl, delay::Delay, peripherals::Peripherals, prelude::*};

#[global_allocator]
static ALLOC: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

#[entry]
fn main() -> ! {
    const HEAP_SIZE: usize = 32 * 1024;
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();
    unsafe {
        ALLOC.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
    }

    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();

    let clocks = ClockControl::max(system.clock_control).freeze();
    let delay = Delay::new(&clocks);

    esp_println::logger::init_logger_from_env();

    let mut chip8 = Chip8::with_rom(include_bytes!("../../res/roms/TETRIS"));

    log::info!("CHIP-8 initialized!");

    loop {
        // Read and update key state
        // chip8.keypad_mut().set_state(0, false);

        // Run the CPU @ 540 Hz
        for _ in 0..9 {
            chip8.step();
        }
        chip8.tick();

        // Render to screen
        // chip8.vpu();

        // Wait for 1/60th of a second
        // TODO: use a periodic timer instead
        delay.delay_micros(1_000_000 / 60);
    }
}
