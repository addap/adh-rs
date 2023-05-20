use fundsp::prelude::{lerp, sqrt};
use rand::{self, distributions::Distribution, SeedableRng};
use rustdct::DctPlanner;
use std::time::{Duration, Instant};

use crate::{audio_bridge::play_samples, Weights, WEIGHTS_NUM};

pub type Chunk = Box<[f32; CHUNK_SAMPLES]>;

pub const CHUNK_SAMPLES: usize = 44_100 * 3;
const SAMPLE_FREQ: f32 = 44_100.0;
const MIN_FREQ: f32 = 20.0;
const MAX_FREQ: f32 = 20_000.0;

// For a frequency in 0..SAMPLE_FREQ/2, compute a weight.
// The weight is the linear interpolation between the two defined weights in `weights`.
pub fn get_freq_weight(weights: &Weights, freq: f32) -> f32 {
    // some frequency between 0 and 22050 Hz.
    // we want to compute a weight based on weights between MIN_FREQ and MAX_FREQ
    if freq <= MIN_FREQ {
        return weights.v[0];
    } else if freq >= MAX_FREQ {
        return weights.v[weights.v.len() - 1];
    }

    // the ratio between consecutive frequencies of our weights
    let freq_ratio = (MAX_FREQ / MIN_FREQ).powf(1.0 / (WEIGHTS_NUM - 1) as f32);
    // which weight bin `freq` belongs to
    let weight_bin = f32::log(freq / MIN_FREQ, freq_ratio).clamp(0.0, (WEIGHTS_NUM - 1) as f32);
    let (left, right) = (weight_bin.floor(), weight_bin.ceil());

    let t = weight_bin - left;
    let weight = lerp(weights.v[left as usize], weights.v[right as usize], t);
    return weight;
}

// For `i` in 0..N and sample frequency f_s, the formula for which frequency this sample stands is i/(2N)*f_s.
// So a DCT gives you N frequencies evenly spaced between 0Hz and half the sample frequency (Nyquist property).
pub fn freq_domain_bin2(i: usize) -> f32 {
    SAMPLE_FREQ * i as f32 / (2.0 * CHUNK_SAMPLES as f32)
}

// Generate noise with weighted frequency bands according to `weights`.
pub fn gen_weighted_noise(weights: &Weights) -> Chunk {
    let mut freqs: Chunk = gen_white_freqs();

    for i in 0..CHUNK_SAMPLES {
        // get the frequency bin of i in the frequency domain
        let freq = freq_domain_bin2(i);
        let weight = get_freq_weight(weights, freq);
        freqs[i] *= weight;
    }

    idct(freqs.as_mut_slice());

    freqs
}

// Inverse discrete cosine transform to transform frequencies back into audio waves.
// rustdct does not apply normalization, so we do it explicitly here.
pub fn idct(fs: &mut [f32]) {
    // let now = Instant::now();
    let idct = DctPlanner::new().plan_dct3(fs.len());
    idct.process_dct3(fs);

    let scale: f32 = sqrt(2.0 / CHUNK_SAMPLES as f32);
    for f in fs {
        *f = *f * scale;
    }

    // println!("{:#?}", fs);
    // println!("Took {}", now.elapsed().as_millis());
}

// White noise frequencies are sampled uniformly random from -1..=1.
pub fn gen_white_freqs() -> Chunk {
    let r = rand::distributions::Uniform::new_inclusive(-1.0, 1.0);
    let small_rng = rand::rngs::SmallRng::from_entropy();
    let my_freqs: Vec<f32> = r.sample_iter(small_rng).take(CHUNK_SAMPLES).collect();

    let freqs = Box::into_raw(my_freqs.into_boxed_slice()) as *mut [f32; CHUNK_SAMPLES];
    unsafe { Box::from_raw(freqs) }
}
