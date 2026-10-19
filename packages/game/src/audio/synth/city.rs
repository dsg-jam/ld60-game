use super::{SAMPLE_RATE, Samples};

/// Loops fade from their end into their start over this many seconds.
pub(super) const SEAM: f32 = 0.5;

impl Samples {
    /// Plays the end of the samples over their start, `seam` seconds long,
    /// so they loop without a click.
    pub(super) fn looped(mut self, seam: f32) -> Self {
        let fade = (seam * SAMPLE_RATE as f32) as usize;
        let length = self.0.len() - fade;
        for i in 0..fade {
            let mix = i as f32 / fade as f32;
            self.0[i] = self.0[i] * mix.sqrt() + self.0[length + i] * (1.0 - mix).sqrt();
        }
        self.0.truncate(length);
        self
    }
}
