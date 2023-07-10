//! Module to handle collections of audio samples, turning them into iterators and blending between different audio samples.
//! a.d. TODO does fundsp already provide something like this? Then I could just plug in the generated sample into the fundsp and don't need all this.
//! Yes, we may be able to use
//!   1. Wave32 to hold a sample slice
//!   2. WavePlayer32 to act as an AudioUnit for the Wave32
//!   3. Sequencer32 to play multiple WavePlayer32 and fade between them
//!   4. SequencerBackend32 to act as the final mixed AudioUnit that we give to cpal
//!   
//! Although I'm unsure if this is the better solution. The fade-in/fade-out of the Sequencer would have to be timed exactly right so that the final noise
//! does not change in volume during the fade between two samples because it fades to silence instead of fading between two different samples as we do here.
//! Also, the sequencer does not loop (only reset which would fuck with fading) so we would need to continuously push new WavePlayers to the Sequencer.

use anyhow::anyhow;
use lerp::Lerp;
use std::f32;

pub const CHUNK_SAMPLES: usize = 44_100 * 3;
const BLEND_WINDOW: usize = 1000;

pub type RawSample = [f32; CHUNK_SAMPLES];

#[derive(Debug, Clone)]
pub struct Sample {
    data: Box<RawSample>,
}

impl Sample {
    pub fn new(data: Vec<f32>) -> Result<Self, anyhow::Error> {
        if data.len() != CHUNK_SAMPLES {
            return Err(anyhow!("Length mismatch"));
        }

        // a.d. TODO how does into_boxed_slice restrict the capacity to the length.
        // For example, if data.len == 3 but data.capacity == 4 and we put the data into a box of type Box<[f32; 3]> without
        // altering the capcaity, then when we free the memory of the Box we trigger undefined behavior.
        // This is because we would only try to free the memory of 3 f32's while the allocation was for 4.
        // But apparently this method ensures that the undefined behavior does not happen.
        let boxed_data = data.into_boxed_slice();
        let ptr = Box::into_raw(boxed_data) as *mut [f32; CHUNK_SAMPLES];
        let data = unsafe { Box::from_raw(ptr) };
        Ok(Self { data })
    }

    pub fn get(&self, idx: usize) -> Option<&f32> {
        self.data.get(idx)
    }
}

impl IntoIterator for Sample {
    type Item = f32;

    type IntoIter = SampleIterator;

    fn into_iter(self) -> Self::IntoIter {
        SampleIterator {
            data: self.data,
            fwd_idx: 0,
            bwd_idx: CHUNK_SAMPLES,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SampleIterator {
    data: Box<RawSample>,
    fwd_idx: usize,
    bwd_idx: usize,
}

impl Iterator for SampleIterator {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.fwd_idx < self.bwd_idx {
            let res = self.data[self.fwd_idx];
            self.fwd_idx += 1;
            Some(res)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = CHUNK_SAMPLES - self.fwd_idx;
        (remaining, Some(remaining))
    }
}

impl DoubleEndedIterator for SampleIterator {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.fwd_idx < self.bwd_idx {
            self.bwd_idx -= 1;
            let res = self.data[self.bwd_idx];
            Some(res)
        } else {
            None
        }
    }
}

impl ExactSizeIterator for SampleIterator {}

#[derive(Debug, Clone, Copy)]
pub enum SmoothingType {
    Mirror,
    Blend(BlendType),
}

impl Default for SmoothingType {
    fn default() -> Self {
        Self::Mirror
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BlendType {
    Linear,
    Sigmoid,
}

impl BlendType {
    fn blend(self, a: f32, b: f32, t: f32) -> f32 {
        let weight = match self {
            BlendType::Linear => t,
            BlendType::Sigmoid => {
                // sigmoid (logistical) function, converges fast between -6 and 6
                let sig = |x: f32| 1.0 / (1.0 + f32::powf(f32::consts::E, -x));
                let scaled_t = 12.0 * t - 6.0;
                sig(scaled_t)
            }
        };

        Lerp::lerp(a, b, weight)
    }
}

pub struct BlendingSamples {
    samples: Vec<Sample>,
    smoothing_type: SmoothingType,
}

type StereoSampleIter = Box<dyn Iterator<Item = (f32, f32)> + Send>;

struct BlendingSamplesIterator<I, J> {
    chunk_iter: I,
    current_chunk: J,
    next_chunk: J,
    blend_type: BlendType,
}

impl BlendingSamples {
    pub fn new(chunks: Vec<Sample>) -> Result<Self, anyhow::Error> {
        if chunks.is_empty() {
            return Err(anyhow!("Empty chunks"));
        }
        Ok(Self {
            samples: chunks,
            smoothing_type: Default::default(),
        })
    }

    pub fn with_mirror(mut self) -> Self {
        self.smoothing_type = SmoothingType::Mirror;
        self
    }

    pub fn with_blend(mut self, blend_type: BlendType) -> Self {
        self.smoothing_type = SmoothingType::Blend(blend_type);
        self
    }

    pub fn into_iter(self) -> Result<StereoSampleIter, anyhow::Error> {
        match self.smoothing_type {
            SmoothingType::Mirror => {
                let first_chunk = self.samples.into_iter().next().unwrap();

                let iter = first_chunk
                    .clone()
                    .into_iter()
                    .chain(first_chunk.into_iter().rev())
                    .map(|f| (f, f))
                    .cycle();

                Ok(Box::new(iter))
            }
            SmoothingType::Blend(blend_type) => Ok(Box::new(BlendingSamplesIterator::new(
                self.samples.into_iter().cycle(),
                blend_type,
            )?)),
        }
    }
}

impl<I: Iterator<Item = J>, J: IntoIterator<IntoIter = K>, K> BlendingSamplesIterator<I, K> {
    fn new(mut chunk_iter: I, blend_type: BlendType) -> Result<Self, anyhow::Error> {
        let current_chunk = chunk_iter.next().unwrap();
        let next_chunk = chunk_iter.next().unwrap();

        Ok(Self {
            chunk_iter,
            blend_type,
            current_chunk: current_chunk.into_iter(),
            next_chunk: next_chunk.into_iter(),
        })
    }
}

impl<
        I: Iterator<Item = J>,
        J: IntoIterator<Item = f32, IntoIter = K>,
        K: Iterator<Item = f32> + ExactSizeIterator,
    > Iterator for BlendingSamplesIterator<I, K>
{
    type Item = (f32, f32);

    fn next(&mut self) -> Option<Self::Item> {
        let s = if self.current_chunk.len() <= BLEND_WINDOW {
            let weight = 1.0 - self.current_chunk.len() as f32 / BLEND_WINDOW as f32;
            let s1 = self.current_chunk.next().unwrap();
            let s2 = self.next_chunk.next().unwrap();

            let s = self.blend_type.blend(s1, s2, weight);

            if self.current_chunk.len() == 0 {
                let new_next = self.chunk_iter.next().unwrap().into_iter();
                let old_next = std::mem::replace(&mut self.next_chunk, new_next);
                self.current_chunk = old_next;
            }

            s
        } else {
            self.current_chunk.next().unwrap()
        };

        Some((s, s))
    }
}
