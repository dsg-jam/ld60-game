use std::f32::consts::TAU;

pub(super) const SAMPLE_RATE: u32 = 44_100;

/// Mono samples from -1 to 1.
#[derive(Clone, Debug)]
pub(super) struct Samples(Vec<f32>);

impl Samples {
    /// A key going down: the short tick of the switch, then the dull knock
    /// of the keycap hitting the bottom.
    pub(super) fn click() -> Self {
        const BOTTOM_OUT: f32 = 0.012;
        let mut noise = Noise::new(3);
        let mut hiss = Lowpass::new(0.5);
        let mut thud = Lowpass::new(0.08);
        Self::generate(0.08, |t| {
            let white = noise.next();
            let tick = (white - hiss.next(white)) * (-t * 1_800.0).exp();
            let knock = if t < BOTTOM_OUT {
                0.0
            } else {
                let t = t - BOTTOM_OUT;
                let attack = (t * 1_000.0).min(1.0);
                let body = (TAU * 180.0 * t).sin() * (-t * 70.0).exp() * 0.5
                    + (TAU * 410.0 * t).sin() * (-t * 110.0).exp() * 0.25;
                let rattle = thud.next(white) * (-t * 400.0).exp() * 2.0;
                (body + rattle) * attack
            };
            tick * 0.4 + knock
        })
        .normalized(0.8)
    }

    fn normalized(mut self, peak: f32) -> Self {
        let max = self
            .0
            .iter()
            .fold(0.0_f32, |max, sample| max.max(sample.abs()));
        if max > 0.0 {
            for sample in &mut self.0 {
                *sample *= peak / max;
            }
        }
        self
    }

    /// 16 bit mono WAV file.
    pub(super) fn wav(&self) -> Vec<u8> {
        let data_len = (self.0.len() * 2) as u32;
        let mut wav = Vec::with_capacity(44 + data_len as usize);
        wav.extend(b"RIFF");
        wav.extend((36 + data_len).to_le_bytes());
        wav.extend(b"WAVEfmt ");
        wav.extend(16_u32.to_le_bytes());
        wav.extend(1_u16.to_le_bytes());
        wav.extend(1_u16.to_le_bytes());
        wav.extend(SAMPLE_RATE.to_le_bytes());
        wav.extend((SAMPLE_RATE * 2).to_le_bytes());
        wav.extend(2_u16.to_le_bytes());
        wav.extend(16_u16.to_le_bytes());
        wav.extend(b"data");
        wav.extend(data_len.to_le_bytes());
        for sample in &self.0 {
            wav.extend(((sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16).to_le_bytes());
        }
        wav
    }

    fn generate(seconds: f32, mut sample: impl FnMut(f32) -> f32) -> Self {
        let count = (seconds * SAMPLE_RATE as f32) as usize;
        Self(
            (0..count)
                .map(|i| sample(i as f32 / SAMPLE_RATE as f32))
                .collect(),
        )
    }
}

/// Deterministic white noise from -1 to 1.
struct Noise(u32);

impl Noise {
    fn new(seed: u32) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9) | 1)
    }

    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0 as f32 / u32::MAX as f32 * 2.0 - 1.0
    }
}

/// One pole low-pass filter. `amount` from 0 (closed) to 1 (open).
struct Lowpass {
    amount: f32,
    value: f32,
}

impl Lowpass {
    fn new(amount: f32) -> Self {
        Self { amount, value: 0.0 }
    }

    fn next(&mut self, input: f32) -> f32 {
        self.value += (input - self.value) * self.amount;
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_matches_data() {
        let wav = Samples::click().wav();
        assert_eq!(&wav[..4], b"RIFF");
        let data_len = u32::from_le_bytes(wav[40..44].try_into().unwrap());
        assert_eq!(wav.len(), 44 + data_len as usize);
    }
}
