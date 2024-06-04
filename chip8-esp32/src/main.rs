#![no_std]
#![no_main]

use core::mem::MaybeUninit;

use chip8::core::Chip8;
use embassy_executor::Spawner;
use embassy_time::{Duration, Ticker};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    embassy,
    gpio::IO,
    peripherals::Peripherals,
    prelude::*,
    spi::{master::Spi, SpiMode},
    timer::TimerGroup,
};
use sh1106::{mode::GraphicsMode, NoOutputPin};

#[global_allocator]
static ALLOC: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

#[main]
async fn main(_spawner: Spawner) {
    const HEAP_SIZE: usize = 32 * 1024;
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();
    unsafe {
        ALLOC.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
    }

    esp_println::logger::init_logger_from_env();

    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();

    let timg0 = TimerGroup::new_async(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timg0);

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let sclk = io.pins.gpio0;
    let miso = io.pins.gpio2;
    let mosi = io.pins.gpio4;
    let cs = io.pins.gpio5;
    let dc = io.pins.gpio1;

    let spi = Spi::new(peripherals.SPI2, 1.MHz(), SpiMode::Mode0, &clocks).with_pins(
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

    let mut chip8 = Chip8::with_rom(include_bytes!("../../res/roms/TETRIS"));

    log::info!("*** CHIP-8 for ESP32 ***");

    let mut ticker = Ticker::every(Duration::from_micros(16667)); // 60 Hz

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
                    let (x, y) = (x as u32, y as u32);

                    display.set_pixel(x * 2, y * 2, 1);
                    display.set_pixel(x * 2 + 1, y * 2, 1);
                    display.set_pixel(x * 2, y * 2 + 1, 1);
                    display.set_pixel(x * 2 + 1, y * 2 + 1, 1);
                }
            }
        }
        display.flush().unwrap();

        ticker.next().await;
    }
}
