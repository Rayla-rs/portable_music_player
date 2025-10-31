#![no_std]
#![deny(unsafe_code)]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use embedded_sdmmc::{
    BlockDevice, Error, SdCardError, ShortFileName, TimeSource, Timestamp, Volume,
};
use esp_hal::{
    delay::Delay,
    dma::{self, AnyGdmaChannel},
    gpio::Output,
    i2s::{
        master::{DataFormat, I2s, Standard},
        AnyI2s,
    },
    peripherals::{DMA, DMA_CH0},
    spi::master::Spi,
    time::Rate,
    Blocking,
};
use nanomp3::Decoder;
use pmp_config::Library;
use postcard::accumulator::{CobsAccumulator, FeedResult};
use serde::Deserialize;

// display buffer
// play buffer
// history buffer (could be integrated with play buffer)

const MAX_DIRS: usize = 4;
const MAX_FILES: usize = 4;
const MAX_VOLUMES: usize = 4;
type SdCard<'a> = embedded_sdmmc::SdCard<
    embedded_hal_bus::spi::ExclusiveDevice<Spi<'a, Blocking>, Output<'a>, Delay>,
    Delay,
>;
type File<'a> =
    embedded_sdmmc::File<'a, SdCard<'a>, DummyTimesource, MAX_DIRS, MAX_FILES, MAX_VOLUMES>;

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

enum State {}

struct Player<'a> {
    volume: u8,
    driver: I2s<'a, Blocking>,
}

impl<'a> Player<'a> {
    fn new(i2s: AnyI2s<'a>, dma: AnyGdmaChannel<'a>) -> Self {
        Self {
            volume: 64,
            driver: I2s::new(
                i2s,
                Standard::Philips,
                DataFormat::Data16Channel16,
                Rate::from_hz(44100u32),
                dma,
            ),
        }
    }
}

struct Fs(SdCard<'static>);

struct Track {
    position: usize,
}

impl Track {
    fn consume(&mut self, consumed: usize) {
        self.position += consumed
        // assert not out of file
    }
}

fn test(player: Player) {
    let mut decoder = Decoder::new();
    let mut pcm = [0f32; 4];
    let (consumed, info) = decoder.decode(&[1, 2, 3], &mut pcm);
    // ring buffer moment
    if let Some(info) = info {}
    // decoder.decode(mp3, pcm)
    // I2s::new(i2s, standard, data_format, sample_rate, channel)
}

enum DecodeError {
    Read,
    Overfull,
    DeserError,
}

pub fn decode<T: for<'de> Deserialize<'de>>(file: File<'static>) -> Result<T, DecodeError> {
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

fn file_sys(
    card: Volume<'static, SdCard, DummyTimesource, MAX_DIRS, MAX_FILES, MAX_VOLUMES>,
) -> Result<(), Error<SdCardError>> {
    let root_dir = card.open_root_dir()?;
    root_dir.iterate_dir(|entry| {
        entry.attributes.is_directory();

        // entry.name;
        //
    })?;
    let file =
        root_dir.open_file_in_dir(ShortFileName::this_dir(), embedded_sdmmc::Mode::ReadOnly)?;

    Ok(())
}

// async_embedded_sdmmc or embedded_sdmmc
// display
// buttons
// audio codec
// audio out
//
// heapless for ring buffer

// no alloc, no std, hper performant blt and i2c audio library
// uwudio
// #[cfg(test)]
mod tests {
    use heapless::String;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct TestToml {
        other: String<20>,
    }

    // #[test]
    fn test_deserialize() {
        let res: TestToml = toml::from_str("other = \"testinging\" \n name = \"hell\"").unwrap();
    }
}
