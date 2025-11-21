#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use core::cell::Cell;

// use alloc::vec::Vec;
use critical_section::Mutex;
use embassy_executor::Spawner;
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::dma::{DmaRxBuf, DmaTxBuf};
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig};
use esp_hal::spi::master::{ClockSource, Config, Spi};
use esp_hal::spi::DataMode;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{dma_buffers, peripherals, spi};
use esp_println::{dbg, println};
use log::info;
use portable_music_player::fs::FileSystem;
use portable_music_player::input::spawn_input_task;
use portable_music_player::player::{Player, Sink};

// extern crate alloc;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("[PANIC] {}", info);
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

    let (_, _, tx_buffer, tx_descriptors) = esp_hal::dma_circular_buffers!(3200, 3200);

    let timer0 = TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timer0.timer0);

    info!("Embassy initialized!");

    let pin_12 = Output::new(
        unsafe { peripherals.GPIO12.clone_unchecked() },
        Level::Low,
        OutputConfig::default(), // .with_drive_mode(esp_hal::gpio::DriveMode::OpenDrain)
                                 // .with_pull(esp_hal::gpio::Pull::None),
    );

    let sck = peripherals.GPIO14;
    let mosi = peripherals.GPIO15;
    let miso = peripherals.GPIO2;
    let cs = unsafe { peripherals.GPIO13.clone_unchecked() };

    let mut spi = dbg!(Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_khz(100))
            .with_mode(spi::Mode::_0),
    )
    .unwrap()
    .with_sck(sck)
    // .with_sio0(miso)
    .with_mosi(mosi)
    .with_miso(miso)
    .with_cs(cs));

    let delay = Delay::new();

    let cs_out = Output::new(
        unsafe { peripherals.GPIO13.clone_unchecked() },
        Level::Low,
        OutputConfig::default(), // .with_drive_mode(esp_hal::gpio::DriveMode::OpenDrain)
                                 // .with_pull(esp_hal::gpio::Pull::None),
    );

    for i in 0..u16::MAX {
        let mut data = [0xde, 0xca, 0xfb, 0xad];

        let _ = spi.half_duplex_read(
            spi::DataMode::SingleTwoDataLines,
            spi::master::Command::_1Bit(9, DataMode::SingleTwoDataLines),
            spi::master::Address::None,
            2,
            &mut data,
        );
        // spi.transfer(&mut data).unwrap();
        println!("{:x?}", data);
        delay.delay_millis(50);
    }

    let driver = ExclusiveDevice::new(spi, cs_out, Delay::new()).unwrap();

    let sd = embedded_sdmmc::SdCard::new(driver, Delay::new());

    // let sd = embedded_sdmmc::SdCard::new(driver, Delay::new());
    println!("[LOOK_HERE] {:?}", sd.num_bytes());

    // portable_music_player::app::run(
    //     spawner,
    //     &FileSystem::new(
    //         peripherals.SPI2,
    //         peripherals.GPIO13, // DAT3 / CS
    //         peripherals.GPIO22, // DUMMY
    //         peripherals.GPIO14, // CLK
    //         peripherals.GPIO15, // CMD / MOSI
    //         peripherals.GPIO2,  // DAT0 / MISO
    //     )
    //     .unwrap(),
    //     Player::new(
    //         // ES7243 DAC
    //         Sink::new(
    //             peripherals.I2S1,
    //             peripherals.DMA_I2S1,
    //             peripherals.GPIO0,  // MCLK
    //             peripherals.GPIO32, // BLCK
    //             peripherals.GPIO33, // WS
    //             tx_descriptors,
    //             tx_buffer,
    //         )
    //         .unwrap(),
    //     ),
    // spawn_input_task(
    //     &spawner,
    //     &INPUT_CHANNEL,
    //     peripherals.GPIO36, // Up Button
    //     peripherals.GPIO35, // Down Button
    //     peripherals.GPIO37, // Enter Button
    //     peripherals.GPIO38, // Back Button
    //     peripherals.GPIO34, // Increment Volume Button
    //     peripherals.GPIO39, // Decrement Volume Button
    // ),
    //  )
    // .await
    let mut a = Input::new(peripherals.GPIO34, InputConfig::default());
    loop {
        println!("SD_DET Level {:?}", a.level());
        Delay::new().delay_millis(100);
    }
}
