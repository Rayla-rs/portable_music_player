use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_sdmmc::{filesystem::ToShortFileName, TimeSource, Timestamp, VolumeIdx};
use esp_hal::{
    delay::Delay,
    gpio::{
        interconnect::{PeripheralInput, PeripheralOutput},
        AnyPin, Input, InputConfig, InputPin, Level, Output, OutputConfig, OutputPin,
    },
    spi::{
        self,
        master::{Config, Spi},
        AnySpi,
    },
    time::Rate,
    Blocking,
};
use esp_println::{dbg, println};
use pmp_config::Track;
use postcard::accumulator::{CobsAccumulator, FeedResult};
use serde::Deserialize;

use crate::player::TrackDecoder;

const MAX_DIRS: usize = 4;
const MAX_FILES: usize = 4;
const MAX_VOLUMES: usize = 1;

pub type SdCard<'a> = embedded_sdmmc::SdCard<
    embedded_hal_bus::spi::ExclusiveDevice<Spi<'a, Blocking>, Output<'a>, Delay>,
    Delay,
>;
pub type File<'a> =
    embedded_sdmmc::File<'a, SdCard<'a>, DummyTimesource, MAX_DIRS, MAX_FILES, MAX_VOLUMES>;
pub type Volume<'a> =
    embedded_sdmmc::Volume<'a, SdCard<'a>, DummyTimesource, MAX_DIRS, MAX_FILES, MAX_VOLUMES>;
pub type Directory<'a> =
    embedded_sdmmc::Directory<'a, SdCard<'a>, DummyTimesource, MAX_DIRS, MAX_FILES, MAX_VOLUMES>;
pub type VolumeManager<'a> =
    embedded_sdmmc::VolumeManager<SdCard<'a>, DummyTimesource, MAX_DIRS, MAX_FILES, MAX_VOLUMES>;
type Error = embedded_sdmmc::Error<embedded_sdmmc::SdCardError>;

/// A dummy timesource, which is mostly important for creating files.
#[derive(Default)]
pub struct DummyTimesource;

impl TimeSource for DummyTimesource {
    // In theory you could use the RTC of the rp2040 here, if you had
    // any external time synchronizing device.
    fn get_timestamp(&self) -> Timestamp {
        Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

/// Decoding Errors
#[derive(Debug)]
pub enum DecodeError {
    Read,
    Overfull,
    DeserError,
}

/// Decode a file using an internal accumulator
pub fn decode<T: for<'de> Deserialize<'de>>(file: File) -> Result<T, DecodeError> {
    let mut raw_buf = [0u8; 32];
    let mut cobs_buf = CobsAccumulator::<256>::new();

    loop {
        match file.read(&mut raw_buf) {
            Ok(read) => {
                match cobs_buf.feed::<T>(&raw_buf[..read]) {
                    FeedResult::Consumed => {
                        log::info!("continue");
                        continue;
                    }
                    FeedResult::OverFull(_) => break Err(DecodeError::Overfull),
                    FeedResult::DeserError(_) => break Err(DecodeError::DeserError),
                    FeedResult::Success { data, .. } => break Ok(data),
                };
            }
            Err(_) => break Err(DecodeError::Read),
        }
    }
}

/// File System wrapper for embedded_sdmmc
pub struct FileSystem<'a>(VolumeManager<'a>);

impl<'a> FileSystem<'a> {
    pub fn new(
        spi: impl Into<AnySpi<'a>>,
        cs: impl OutputPin + 'a,
        dummy: impl OutputPin + 'a,

        sclk: impl PeripheralOutput<'a>,
        mosi: impl PeripheralOutput<'a>,
        miso: impl PeripheralInput<'a> + InputPin,
    ) -> Result<Self, Error> {
        // let mut out = Output::new(
        //     cs,
        //     Level::Low,
        //     OutputConfig::default()
        //         .with_pull(esp_hal::gpio::Pull::None)
        //         .with_drive_mode(esp_hal::gpio::DriveMode::OpenDrain),
        // );
        let spi = dbg!(Spi::new(
            spi.into(),
            Config::default()
                // .with_frequency(Rate::from_khz(100u32))
                .with_frequency(Rate::from_mhz(20u32))
                .with_mode(spi::Mode::_0),
        )
        .unwrap()
        .with_sck(sclk)
        .with_sio0(mosi)
        .with_miso(miso));

        let driver = ExclusiveDevice::new(
            spi,
            Output::new(cs, Level::High, OutputConfig::default()),
            Delay::new(),
        )
        .unwrap();

        let sd = embedded_sdmmc::SdCard::new(driver, Delay::new());

        // let sd = embedded_sdmmc::SdCard::new(driver, Delay::new());
        println!("[LOOK_HERE] {:?}", sd.num_bytes());
        todo!()
        // Ok(FileSystem(embedded_sdmmc::VolumeManager::new(
        // sd,
        // DummyTimesource,
        // )))
    }

    pub fn open_file(&'a self, name: impl ToShortFileName) -> Result<File<'a>, Error> {
        Ok(self
            .0
            .open_volume(VolumeIdx(0))?
            .open_root_dir()?
            .open_file_in_dir(name, embedded_sdmmc::Mode::ReadOnly)?
            .to_raw_file()
            .to_file(&self.0))
    }

    pub fn open_track<'b>(&'a self, track: &'b Track) -> Result<TrackDecoder<'a, 'b>, Error> {
        let file = self.open_file(track.title.as_str())?;
        TrackDecoder::new(track, file)
    }
}
