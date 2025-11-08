use microfft::Complex32;

pub type VisualizerOutput = [Complex32; 64];

#[derive(Default)]
pub struct Visualizer {
    buf: heapless::HistoryBuf<f32, 128>,
}

impl Visualizer {
    /// Write slice to history buf
    pub fn extend_with_chan(&mut self, other: &[f32], channels: usize) {
        other
            .chunks(channels)
            .map(|chunk| {
                let sum: f32 = chunk.iter().sum();
                sum / channels as f32
            })
            .for_each(|value| self.buf.write(value));
    }

    fn read(&self) -> Option<VisualizerOutput> {
        if self.buf.is_full() {
            Some(
                microfft::real::rfft_128(&mut {
                    let mut input = [0f32; 128];
                    let mut iterator = self.buf.oldest_ordered().take(128);
                    input.fill_with(|| *iterator.next().expect("Iterator should be cyclic!"));
                    input
                })
                .clone(),
            )
        } else {
            None
        }
    }

    fn read_or_default(&self) -> VisualizerOutput {
        self.read().unwrap_or([Complex32::default(); 64])
    }

    /// Sample the FFT of the internal buffer
    pub fn sample(&self, sample_rate: f32) {
        // Nyquist frequency
        let freq = sample_rate / 2.;
        self.read_or_default()
            .map(|value| (value.re / freq).clamp(0.0, 1.0));
    }
}
