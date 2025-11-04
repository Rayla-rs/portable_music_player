use byteorder::{BigEndian, ByteOrder};
use embassy_executor::task;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Channel};
use esp_hal::{
    dma::AnyGdmaChannel,
    gpio::{interconnect::PeripheralOutput, AnyPin},
    i2s::{
        master::{asynch::I2sWriteDmaTransferAsync, DataFormat, I2s, I2sTx, Standard},
        AnyI2s,
    },
    time::Rate,
    Async,
};
use nanomp3::{Decoder, FrameInfo};
use pmp_config::Track;

use crate::fs::File;

const TRANSFER_SIZE: usize = 256;
type Transfer<'a> = I2sWriteDmaTransferAsync<'a, &'a mut [u8; TRANSFER_SIZE]>;

pub struct Sink<'a> {
    volume: f32,
    driver: Transfer<'a>,
}

impl<'a> Sink<'a>
where
    'a: 'static,
{
    pub fn new(
        i2s: impl Into<AnyI2s<'a>>,
        dma: AnyGdmaChannel<'a>,
        mclk: impl PeripheralOutput<'static>,
        bclk: impl PeripheralOutput<'static>,
        ws: impl PeripheralOutput<'static>,
        dout: impl PeripheralOutput<'static>,
        words: &'a mut [u8; TRANSFER_SIZE],
    ) -> Result<Self, esp_hal::i2s::master::Error> {
        Ok(Self {
            volume: 0.5,
            driver: I2s::new(
                i2s.into(),
                Standard::Philips,
                DataFormat::Data16Channel16,
                Rate::from_hz(44100u32),
                dma,
            )
            .into_async()
            .with_mclk(mclk)
            .i2s_tx
            .with_bclk(bclk)
            .with_ws(ws)
            .with_dout(dout)
            .build(&mut [])
            .write_dma_circular_async(words)?,
        })
    }

    // fn get_volume(&self) -> f32 {
    //     self.volume
    // }

    async fn transfer_frame(&mut self, pcm_buf: &[f32]) -> Result<(), esp_hal::i2s::master::Error> {
        let n = pcm_buf.len();
        let bytes = &mut [0u8; nanomp3::MAX_SAMPLES_PER_FRAME * 4][..4 * n];
        for i in 0..n {
            // TODO Volume control here
            BigEndian::write_f32(&mut bytes[i * 4..i + 1 * 4], pcm_buf[i]);
        }

        self.write(bytes).await
    }

    async fn write(&mut self, mut bytes: &[u8]) -> Result<(), esp_hal::i2s::master::Error> {
        while bytes.len() > 0 {
            bytes = &bytes[self.driver.push(&bytes).await?..];
        }
        Ok(())
    }
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
    // create transfer

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
