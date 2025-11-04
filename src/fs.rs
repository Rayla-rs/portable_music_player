use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_sdmmc::{
    filesystem::ToShortFileName, SdCardError, ShortFileName, TimeSource, Timestamp, VolumeIdx,
};
use esp_hal::{
    delay::Delay,
    gpio::{AnyPin, Input, InputConfig, Level, Output, OutputConfig},
    spi::{
        master::{Config, Spi},
        AnySpi,
    },
    Blocking,
};
use postcard::accumulator::{CobsAccumulator, FeedResult};
use serde::Deserialize;

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
                    FeedResult::Consumed => continue,
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
        cs: impl Into<AnyPin<'a>>,
        sclk: impl Into<AnyPin<'a>>,
        mosi: impl Into<AnyPin<'a>>,
        miso: impl Into<AnyPin<'a>>,
    ) -> Result<Self, embedded_sdmmc::Error<SdCardError>> {
        Ok(FileSystem(embedded_sdmmc::VolumeManager::new(
            SdCard::new(
                ExclusiveDevice::new(
                    Spi::new(spi.into(), Config::default())
                        .unwrap()
                        .with_sck(Output::new(
                            sclk.into(),
                            Level::Low,
                            OutputConfig::default(),
                        ))
                        .with_mosi(Output::new(
                            mosi.into(),
                            Level::Low,
                            OutputConfig::default(),
                        ))
                        .with_miso(Input::new(miso.into(), InputConfig::default())),
                    Output::new(cs.into(), Level::High, OutputConfig::default()),
                    Delay::new(),
                )
                .unwrap(),
                Delay::new(),
            ),
            DummyTimesource,
        )))
    }

    pub fn open_file(
        &'a self,
        name: impl ToShortFileName,
    ) -> Result<File<'a>, embedded_sdmmc::Error<SdCardError>> {
        Ok(self
            .0
            .open_volume(VolumeIdx(0))?
            .open_root_dir()?
            .open_file_in_dir(name, embedded_sdmmc::Mode::ReadOnly)?
            .to_raw_file()
            .to_file(&self.0))
    }
}
