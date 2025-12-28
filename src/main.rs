#![no_std]
#![no_main]

extern crate alloc;

use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    embassy,
    peripherals::Peripherals,
    prelude::*,
    timer::timg::TimerGroup,
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

#[main]
async fn main(spawner: embassy_executor::Spawner) {
    // Initialize the heap allocator
    esp_alloc::heap_allocator!(unsafe { &mut HEAP });

    println!("ESP32-S3 OpenPGP Card Initializing...");

    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();

    // Initialize embassy timer
    let timg0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timg0.timer0);

    println!("Hardware initialized");

    // Initialize storage
    let storage = match storage::Storage::new(peripherals.RNG) {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to initialize storage: {:?}", e);
            loop {
                esp_hal::delay::Delay::new(&clocks).delay_ms(1000);
            }
        }
    };

    println!("Storage initialized");

    // Initialize OpenPGP card state
    let openpgp_card = OpenPgpCard::new(storage);

    println!("OpenPGP card state initialized");

    // Initialize and spawn USB task
    #[cfg(feature = "usb")]
    {
        let usb_task = usb::usb_task(peripherals.USB0, openpgp_card);
        spawner.spawn(usb_task).ok();
        println!("USB task spawned");
    }

    println!("ESP32-S3 OpenPGP Card Ready!");

    // Main event loop
    loop {
        embassy_time::Timer::after(embassy_time::Duration::from_secs(1)).await;
    }
}
