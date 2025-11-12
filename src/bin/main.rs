#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use embassy_executor::Spawner;
use esp_hal::clock::CpuClock;
use esp_hal::timer::timg::TimerGroup;
use log::info;
use portable_music_player::fs::FileSystem;
use portable_music_player::input::spawn_input_task;
use portable_music_player::player::{Player, Sink};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

static INPUT_CHANNEL: portable_music_player::input::Channel =
    portable_music_player::input::Channel::new();

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let timer0 = TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timer0.timer0);

    info!("Embassy initialized!");

    let mut words = [0u8; 256];

    portable_music_player::app::run(
        spawner,
        &FileSystem::new(
            peripherals.SPI2,
            peripherals.GPIO16, // DAT3 / CS
            peripherals.GPIO14, // CLK
            peripherals.GPIO15, // CMD
            peripherals.GPIO2,  // DAT0
        )
        .unwrap(),
        Player::new(
            // ES7243 DAC
            Sink::new(
                peripherals.I2S1,
                peripherals.DMA_I2S1,
                peripherals.GPIO0,  // MCLK
                peripherals.GPIO32, // BLCK
                peripherals.GPIO33, // WS
                &mut words,
            )
            .unwrap(),
        ),
        spawn_input_task(
            &spawner,
            &INPUT_CHANNEL,
            peripherals.GPIO36, // Up Button
            peripherals.GPIO35, // Down Button
            peripherals.GPIO37, // Enter Button
            peripherals.GPIO38, // Back Button
            peripherals.GPIO34, // Increment Volume Button
            peripherals.GPIO39, // Decrement Volume Button
        ),
    )
    .await
}
