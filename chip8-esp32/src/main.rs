#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use chip8::core::Chip8;
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    delay::Delay,
    gpio::IO,
    peripherals::Peripherals,
    prelude::*,
    spi::{master::Spi, SpiMode},
};
use sh1106::{mode::GraphicsMode, NoOutputPin};

#[global_allocator]
static ALLOC: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

#[entry]
fn main() -> ! {
    const HEAP_SIZE: usize = 32 * 1024;
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();
    unsafe {
        ALLOC.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
    }

    esp_println::logger::init_logger_from_env();

    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let sclk = io.pins.gpio0;
    let miso = io.pins.gpio2;
    let mosi = io.pins.gpio4;
    let cs = io.pins.gpio5;
    let dc = io.pins.gpio1;

    let spi = Spi::new(peripherals.SPI2, 100.kHz(), SpiMode::Mode0, &clocks).with_pins(
        Some(sclk),
        Some(mosi),
        Some(miso),
        Some(cs),
    );

    let mut display: GraphicsMode<_> = sh1106::Builder::new()
        .connect_spi(spi, dc.into_push_pull_output(), NoOutputPin::new())
        .into();

    display.init().unwrap();
    display.flush().unwrap();

    let delay = Delay::new(&clocks);

    let mut chip8 = Chip8::with_rom(include_bytes!("../../res/roms/TETRIS"));

    loop {
        // Read and update key state
        // chip8.keypad_mut().set_state(0, false);

        // Run the CPU @ 540 Hz
        for _ in 0..9 {
            chip8.step();
        }
        chip8.tick();

        // Render to screen
        display.clear();
        for y in 0..32 {
            for x in 0..64 {
                if chip8.vpu().read((x, y)) {
                    display.set_pixel(x.into(), y.into(), 1);
                }
            }
        }
        display.flush().unwrap();

        // Wait for 1/60th of a second
        // TODO: use a periodic timer instead
        delay.delay_micros(1_000_000 / 60);
    }
}
