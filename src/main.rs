#![no_std]
#![no_main]

extern crate alloc;

use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    delay::Delay,
    peripherals::Peripherals,
    prelude::*,
    system::SystemControl,
};
use esp_println::println;

mod apdu;
mod crypto;
mod openpgp;
mod pin;
mod storage;
mod usb;

use openpgp::OpenPgpCard;

/// Heap size for the application (64KB)
const HEAP_SIZE: usize = 64 * 1024;
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

#[esp_hal::entry]
fn main() -> ! {
    // Initialize the heap allocator
    esp_alloc::heap_allocator!(unsafe { &mut HEAP });

    println!("ESP32-S3 OpenPGP Card Initializing...");

    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::max(system.clock_control).freeze();

    println!("Hardware initialized");

    // Initialize storage
    let storage = match storage::Storage::new(peripherals.RNG) {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to initialize storage: {:?}", e);
            let delay = Delay::new();
            loop {
                delay.delay_ms(1000u32);
            }
        }
    };

    println!("Storage initialized");

    // Initialize OpenPGP card state
    let _openpgp_card = OpenPgpCard::new(storage);

    println!("OpenPGP card state initialized");

    // Note: Full USB/CCID implementation would be added here
    // This requires USB peripheral initialization and CCID protocol handling
    println!("USB interface: Not implemented in this build");
    println!("To use the card, flash this firmware and connect via USB");

    println!("ESP32-S3 OpenPGP Card Ready!");
    println!("Waiting for USB CCID commands...");

    // Main event loop
    let delay = Delay::new();
    let mut counter = 0u32;
    loop {
        delay.delay_ms(5000u32);
        counter += 1;
        if counter % 12 == 0 {
            println!("Card alive - waiting for commands ({}m)", counter / 12);
        }
    }
}
