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
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::{Operation, SpiBus, SpiDevice};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::dma::{DmaRxBuf, DmaTxBuf};
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig};
use esp_hal::spi::slave::dma::SpiDma;
use esp_hal::spi::slave::Spi;
use esp_hal::spi::DataMode;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{dma_buffers, peripheral, peripherals, spi, Blocking};
use esp_println::{dbg, println};
use log::{info, warn};
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

    // DMA init
    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(4000, 4000);
    let rx_buffer = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
    let tx_buffer = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();

    // SPI Pins
    let sck = peripherals.GPIO14;
    let mosi = peripherals.GPIO15;
    let miso = peripherals.GPIO2;

    // Card Check
    let sd_cd = Input::new(peripherals.GPIO34, InputConfig::default());

    // Power the sd card
    let mut sd_pwr = Output::new(peripherals.GPIO13, Level::High, OutputConfig::default());
    Delay::new().delay_nanos(100);
    sd_pwr.set_level(sd_cd.level());

    // Create SPI Bus
    let spi = Spi::new(peripherals.SPI2, spi::Mode::_0)
        .with_mosi(mosi)
        .with_miso(miso)
        .with_sck(sck)
        .with_dma(peripherals.DMA_SPI2);

    // Initialize Drivers
    let driver = SdDevice::new(spi, rx_buffer, tx_buffer, Delay::new());
    let sd = embedded_sdmmc::SdCard::new(driver, Delay::new());

    // Test
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
    loop {
        // println!("SD_DET Level {:?}", sd_cd.level());
        Delay::new().delay_millis(100);
    }
}

type BusInner<'a> = (SpiDma<'a, Blocking>, DmaRxBuf, DmaTxBuf);

struct SdDevice<'a, DELAY: DelayNs> {
    inner: Option<BusInner<'a>>,
    delay: DELAY,
}

impl<'a, DELAY: DelayNs> SdDevice<'a, DELAY> {
    pub fn new(
        bus: SpiDma<'a, Blocking>,
        rx_buffer: DmaRxBuf,
        tx_buffer: DmaTxBuf,
        delay: DELAY,
    ) -> Self {
        Self {
            inner: Some((bus, rx_buffer, tx_buffer)),
            delay,
        }
    }

    fn use_inner<Result>(
        &mut self,
        func: &mut impl FnMut(BusInner<'a>) -> (BusInner<'a>, Result),
    ) -> Result {
        match self.inner.take() {
            Some(inner) => {
                let (inner, res) = func(inner);
                let _ = self.inner.replace(inner);
                res
            }
            None => {
                unreachable!()
            }
        }
    }
}

impl<'a, DELAY: DelayNs> embedded_hal::spi::ErrorType for SdDevice<'a, DELAY> {
    type Error = spi::Error;
}

impl<'a, DELAY: DelayNs> SpiDevice<u8> for SdDevice<'a, DELAY> {
    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<(), Self::Error> {
        'ops: {
            for op in operations {
                let res = match op {
                    Operation::Read(buf) => self.read(buf),
                    Operation::Write(buf) => self.write(buf),
                    Operation::Transfer(read, write) => self.transfer(read, write),
                    Operation::TransferInPlace(buf) => self.transfer_in_place(buf),
                    Operation::DelayNs(ns) => {
                        self.delay.delay_ns(*ns);
                        Ok(())
                    }
                };
                if let Err(e) = res {
                    break 'ops Err(e);
                }
            }
            Ok(())
        }
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        self.use_inner(
            &mut |(bus, rx_buffer, tx_buffer)| match bus.read(buf.len(), rx_buffer) {
                Ok(transfer) => {
                    let (bus, rx_buffer) = transfer.wait();

                    // Write data into buf
                    let _ = rx_buffer.read_received_data(buf);
                    ((bus, rx_buffer, tx_buffer), Ok(()))
                }
                Err(err) => {
                    let (err, bus, rx_buffer) = err;
                    ((bus, rx_buffer, tx_buffer), Err(err))
                }
            },
        )
    }

    fn write(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        self.use_inner(&mut |(bus, rx_buffer, mut tx_buffer)| {
            // Write to tx
            tx_buffer.fill(buf);
            match bus.write(buf.len(), tx_buffer) {
                Ok(transfer) => {
                    let (bus, tx_buffer) = transfer.wait();
                    ((bus, rx_buffer, tx_buffer), Ok(()))
                }
                Err(err) => {
                    let (err, bus, tx_buffer) = err;
                    ((bus, rx_buffer, tx_buffer), Err(err))
                }
            }
        })
    }

    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Self::Error> {
        self.use_inner(&mut |(bus, rx_buffer, mut tx_buffer)| {
            // Write to tx
            tx_buffer.fill(write);
            match bus.transfer(read.len(), rx_buffer, write.len(), tx_buffer) {
                Ok(transfer) => {
                    let (bus, (rx_buffer, tx_buffer)) = transfer.wait();

                    // Write data into buf
                    let _ = rx_buffer.read_received_data(read);
                    ((bus, rx_buffer, tx_buffer), Ok(()))
                }
                Err(err) => {
                    let (err, bus, rx_buffer, tx_buffer) = err;
                    ((bus, rx_buffer, tx_buffer), Err(err))
                }
            }
        })
    }

    fn transfer_in_place(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        self.use_inner(&mut |(bus, rx_buffer, mut tx_buffer)| {
            // Write to tx
            tx_buffer.fill(buf);
            match bus.transfer(buf.len(), rx_buffer, buf.len(), tx_buffer) {
                Ok(transfer) => {
                    let (bus, (rx_buffer, tx_buffer)) = transfer.wait();

                    // Write data into buf
                    let _ = rx_buffer.read_received_data(buf);
                    ((bus, rx_buffer, tx_buffer), Ok(()))
                }
                Err(err) => {
                    let (err, bus, rx_buffer, tx_buffer) = err;
                    ((bus, rx_buffer, tx_buffer), Err(err))
                }
            }
        })
    }
}
