use byteorder::{BigEndian, ByteOrder};
use esp_hal::{
    dma::{DmaChannelFor, ReadBuffer},
    gpio::interconnect::PeripheralOutput,
    i2s::{
        master::{asynch::I2sWriteDmaTransferAsync, DataFormat, I2s, Standard},
        AnyI2s,
    },
    time::Rate,
};
use nanomp3::Decoder;
use pmp_config::Track;

use crate::{fs::File, visualizer::Visualizer};

pub struct Sink<'a, TXBUF: ReadBuffer> {
    volume: f32,
    driver: I2sWriteDmaTransferAsync<'a, TXBUF>,
}

impl<'a, TXBUF: ReadBuffer> Sink<'a, TXBUF> {
    pub fn new(
        i2s: impl Into<AnyI2s<'a>>,
        dma: impl DmaChannelFor<AnyI2s<'a>>,
        mclk: impl PeripheralOutput<'a>,
        bclk: impl PeripheralOutput<'a>,
        ws: impl PeripheralOutput<'a>,
        dout: impl PeripheralOutput<'a>,
        words: TXBUF,
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

    fn get_volume(&self) -> f32 {
        self.volume
    }

    async fn write_frame(&mut self, pcm_buf: &[f32]) -> Result<(), esp_hal::i2s::master::Error> {
        let n = pcm_buf.len();
        let bytes = &mut [0u8; nanomp3::MAX_SAMPLES_PER_FRAME * 4][..4 * n];
        for i in 0..n {
            BigEndian::write_f32(&mut bytes[i * 4..i + 1 * 4], pcm_buf[i] * self.get_volume());
        }

        self.write(bytes).await
    }

    async fn write(&mut self, mut bytes: &[u8]) -> Result<(), esp_hal::i2s::master::Error> {
        Ok(while bytes.len() > 0 {
            bytes = &bytes[self.driver.push(&bytes).await?..];
        })
    }
}

pub struct TrackDecoder<'a> {
    decoder: Decoder,
    visualizer: Visualizer,
    track: Track,
    file: File<'a>,
    time: f64,
}

impl<'a> TrackDecoder<'a> {
    pub fn new(
        track: Track,
        file: File<'a>,
    ) -> Result<Self, embedded_sdmmc::Error<embedded_sdmmc::SdCardError>> {
        Ok(Self {
            decoder: Decoder::new(),
            visualizer: Visualizer::default(),
            track,
            file,
            time: 0.,
        })
    }

    async fn next<TXBUF: ReadBuffer>(
        mut self,
        sink: &mut Sink<'a, TXBUF>,
    ) -> Result<Self, esp_hal::i2s::master::Error> {
        let mut mp3_buf = [0u8; 128];
        let mut pcm_buf = [0f32; nanomp3::MAX_SAMPLES_PER_FRAME];

        if let Ok(consumed) = self.file.read(&mut mp3_buf) {
            let mut raw_buf = &mp3_buf[0..consumed];
            while raw_buf.len() > 0 {
                let (consumed, info) = self.decoder.decode(&raw_buf, &mut pcm_buf);
                raw_buf = &raw_buf[consumed..];

                if let Some(info) = info {
                    let pcm_buf =
                        &pcm_buf[..info.samples_produced * usize::from(info.channels.num())];

                    sink.write_frame(pcm_buf).await?;

                    // FFT
                    self.visualizer
                        .extend_with_chan(pcm_buf, info.channels.num().into());

                    self.time += (info.samples_produced as f64) / (info.sample_rate as f64);
                }
            }
        }
        Ok(self)
    }
}

pub struct Player<'a, TXBUF: ReadBuffer> {
    track: Option<TrackDecoder<'a>>,
    sink: Sink<'a, TXBUF>,
}

impl<'a, TXBUF: ReadBuffer> Player<'a, TXBUF> {
    pub fn new(sink: Sink<'a, TXBUF>) -> Self {
        Self { track: None, sink }
    }

    pub fn play(&mut self, track: TrackDecoder<'a>) {
        self.track = Some(track)
    }

    pub async fn next(&mut self) -> Result<(), esp_hal::i2s::master::Error> {
        Ok(self.track = match self.track.take() {
            Some(track) => {
                if track.file.is_eof() {
                    None
                } else {
                    Some(track.next(&mut self.sink).await?)
                }
            }
            None => None,
        })
    }

    pub fn sample_visualizer(&self) {
        match self.track.as_ref() {
            Some(track) => {
                track.visualizer.sample(441000.0);
            }
            None => {
                Visualizer::default().sample(441000.0);
            }
        }
    }
}
