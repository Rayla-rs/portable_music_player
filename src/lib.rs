#![no_std]
#![deny(unsafe_code)]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use byteorder::{BigEndian, ByteOrder};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Channel};
use esp_hal::{
    dma::AnyGdmaChannel,
    i2s::{
        master::{asynch::I2sWriteDmaTransferAsync, DataFormat, I2s, I2sTx, Standard},
        AnyI2s,
    },
    time::Rate,
    Async, Blocking,
};
use nanomp3::{Decoder, FrameInfo};
use pmp_config::Track;

use crate::fs::{File, Volume};

mod app;
pub mod fs;

const TRANSFER_SIZE: usize = 256;
type Transfer<'a> = I2sWriteDmaTransferAsync<'a, &'a mut [u8; TRANSFER_SIZE]>;

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

async fn player(
    driver: I2sTx<'static, Async>,
    track: Track,
    mut file: File<'static>,
    buffer: &'static mut [u8; TRANSFER_SIZE],
) -> Result<(), esp_hal::i2s::master::Error> {
    // TODO reconstruct i2s bus for rate control
    // let mut buffer = [0u8; TRANSFER_SIZE];
    let mut transfer = driver.write_dma_circular_async(buffer)?;
    let e = run(&mut transfer, file, &mut 1.).await;
    Ok(())
}

pub struct TrackPlayer {
    transfer: I2sWriteDmaTransferAsync<'static, [f32; 128]>,
    file: File<'static>,
    time: f64,
}

pub async fn run<'a>(
    transfer: &mut Transfer<'a>,
    mut file: File<'static>,
    time: &mut f64,
) -> Result<(), esp_hal::i2s::master::Error> {
    let mut decoder = Decoder::new();
    let mut mp3_buf = [0u8; 128];
    let mut pcm_buf = [0f32; nanomp3::MAX_SAMPLES_PER_FRAME];

    while let Ok(read) = embedded_io::Read::read(&mut file, &mut mp3_buf) {
        let mut raw_buf = &mp3_buf[0..read];
        while raw_buf.len() > 0 {
            let (consumed, info) = decoder.decode(&raw_buf, &mut pcm_buf);
            raw_buf = &raw_buf[consumed..];

            if let Some(info) = info {
                transfer_frame(transfer, &pcm_buf, info, time).await?
            }
        }
    }
    Ok(())
}

async fn transfer_frame<'a>(
    transfer: &mut Transfer<'a>,
    pcm_buf: &[f32],
    info: FrameInfo,
    time: &mut f64,
) -> Result<(), esp_hal::i2s::master::Error> {
    let n = info.samples_produced * usize::from(info.channels.num());
    let bytes = &mut [0u8; nanomp3::MAX_SAMPLES_PER_FRAME * 4][..4 * n];
    for i in 0..n {
        // TODO WARN NOTE WARNING volume control here :3
        BigEndian::write_f32(&mut bytes[i * 4..i + 1 * 4], pcm_buf[i]);
    }

    transfer_bytes(transfer, bytes).await?;

    // Update time
    *time += (info.samples_produced as f64) / (info.sample_rate as f64);

    // TODO check here for pausing
    // self.file.rewind();

    Ok(())
}

/// Transfers all bytes asyncronously
async fn transfer_bytes<'a>(
    transfer: &mut Transfer<'a>,
    mut bytes: &[u8],
) -> Result<(), esp_hal::i2s::master::Error> {
    while bytes.len() > 0 {
        bytes = &bytes[transfer.push(&bytes).await?..];
    }
    Ok(())
}

fn test() {
    let chan = Channel::<NoopRawMutex, u32, 3>::new();
}
