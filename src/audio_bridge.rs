//! Audio Bridge module
//!
//! This module interacts with the host's audio system to play the generated samples.
//!
//! Design:
//! We probably want some struct to hold the samples and the stream
//! with methods to start/stop the playback. Audio is playing as long as the stream
//! object exists, so we want to keep it around. But the current design with a loop
//! in the play method is not nice.
//! Also, we want to blend between our sample chunks.
//! We could have a SampleChunks struct that just holds the different chunks and
//! implements Iterator that does the blending in the next() methid.
//! Then we use the iterator analogously to how we use it now.
use anyhow::anyhow;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use fundsp::hacker32::soft_saw_hz;
use fundsp::prelude::lerp;
use std::f32;
use std::time::{Duration, Instant};

use crate::generator::{Chunk, CHUNK_SAMPLES};

const BLEND_WINDOW: usize = 1000;

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

        lerp(a, b, weight)
    }
}

#[derive(Debug, Clone, Copy)]
enum Direction {
    Forwards,
    Backwards,
}

impl Direction {
    fn change(self) -> Self {
        match self {
            Direction::Forwards => Direction::Backwards,
            Direction::Backwards => Direction::Forwards,
        }
    }
}

impl Default for Direction {
    fn default() -> Self {
        Self::Forwards
    }
}

pub struct SampleChunks {
    chunks: Vec<Chunk>,
    smoothing_type: SmoothingType,
}

struct MirrorChunkIter {
    chunk: Chunk,
    sample_idx: usize,
    direction: Direction,
}

trait SampleIter: Iterator<Item = (f32, f32)> {}

struct BlendingChunksIter {
    chunks: Vec<Chunk>,
    blend_type: BlendType,
    chunk_idx: usize,
    sample_idx: usize,
}

impl SampleChunks {
    // pub fn new(chunk: Chunk) -> Result<Self, anyhow::Error> {
    pub fn new(chunks: Vec<Chunk>) -> Result<Self, anyhow::Error> {
        if chunks.is_empty() {
            return Err(anyhow!("Empty chunks"));
        }
        Ok(Self {
            chunks,
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

    fn into_iter(self) -> Result<Box<dyn Iterator<Item = (f32, f32)> + Send>, anyhow::Error> {
        match self.smoothing_type {
            SmoothingType::Mirror => {
                let first_chunk = self.chunks.into_iter().next().unwrap();
                Ok(Box::new(MirrorChunkIter::new(first_chunk)))
                // let first_chunk = &self.chunks[0];

                // println!("before iter");
                // let iter = first_chunk
                //     .into_iter()
                //     .chain(first_chunk.into_iter().rev())
                //     .map(|f| (f, f))
                //     .cycle();
                // println!("after iter");

                // Ok(Box::new(iter))
                // unimplemented!()
            }
            SmoothingType::Blend(blend_type) => {
                Ok(Box::new(BlendingChunksIter::new(self.chunks, blend_type)?))
            }
        }
    }
}

impl MirrorChunkIter {
    fn new(chunk: Chunk) -> Self {
        Self {
            chunk,
            sample_idx: 0,
            direction: Direction::default(),
        }
    }
}

impl BlendingChunksIter {
    fn new(chunks: Vec<Chunk>, blend_type: BlendType) -> Result<Self, anyhow::Error> {
        Ok(Self {
            chunks,
            blend_type,
            chunk_idx: 0,
            sample_idx: 0,
        })
    }
}

impl Iterator for MirrorChunkIter {
    type Item = (f32, f32);

    fn next(&mut self) -> Option<Self::Item> {
        // we implement a symmetry around the first and last sample, so no duplication.
        const LAST_SAMPLE_IDX: usize = CHUNK_SAMPLES - 1;
        // self.direction = match (self.sample_idx, self.direction) {
        //     (0, Direction::Backwards) => Direction::Forwards,
        //     (LAST_SAMPLE_IDX, Direction::Forwards) => Direction::Backwards,
        //     _ => self.direction,
        // };

        // self.sample_idx = match self.direction {
        //     Direction::Forwards => self.sample_idx + 1,
        //     Direction::Backwards => self.sample_idx - 1,
        // };
        self.sample_idx = match (self.sample_idx, self.direction) {
            (0, Direction::Backwards) => {
                self.direction = self.direction.change();
                0
            }
            (LAST_SAMPLE_IDX, Direction::Forwards) => {
                self.direction = self.direction.change();
                LAST_SAMPLE_IDX
            }
            (sample_idx, Direction::Backwards) => sample_idx - 1,
            (sample_idx, Direction::Forwards) => sample_idx + 1,
        };
        let sample = self.chunk.get(self.sample_idx).unwrap();
        Some((*sample, *sample))
    }
}

impl Iterator for BlendingChunksIter {
    type Item = (f32, f32);

    fn next(&mut self) -> Option<Self::Item> {
        if self.sample_idx < CHUNK_SAMPLES - BLEND_WINDOW {
            // just return next sample from chunk
            let s = self.chunks[self.chunk_idx][self.sample_idx];
            self.sample_idx = (self.sample_idx + 1) % CHUNK_SAMPLES;

            Some((s, s))
        } else if self.sample_idx < CHUNK_SAMPLES {
            // weight = 0.0 if we are at left side of blending window.
            // weight = 1 - 1/BLEND_WINDOW if we are at right side of blending window.
            let weight = 1.0 - (CHUNK_SAMPLES - self.sample_idx) as f32 / BLEND_WINDOW as f32;

            let s1 = self.chunks[self.chunk_idx][self.sample_idx];
            let next_chunk_idx = (self.chunk_idx + 1) % self.chunks.len();
            let next_sample_idx = (self.sample_idx + BLEND_WINDOW) % CHUNK_SAMPLES;
            let s2 = self.chunks[next_chunk_idx][next_sample_idx];

            let s = self.blend_type.blend(s1, s2, weight);

            if self.sample_idx == CHUNK_SAMPLES - 1 {
                println!("Transferring to next chunk");
                self.chunk_idx = next_chunk_idx;
                self.sample_idx = next_sample_idx;
            }
            self.sample_idx += 1;

            Some((s, s))
        } else {
            unreachable!()
        }
    }
}

pub struct AudioStream {
    stream: cpal::Stream,
}

pub fn play(chunks: SampleChunks) -> AudioStream {
    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .expect("Failed to find a default output device");
    let config = device.default_output_config().unwrap();

    match config.sample_format() {
        cpal::SampleFormat::F32 => run2::<f32>(&device, &config.into(), chunks).unwrap(),
        cpal::SampleFormat::I16 => run2::<i16>(&device, &config.into(), chunks).unwrap(),
        cpal::SampleFormat::U16 => run2::<u16>(&device, &config.into(), chunks).unwrap(),
        _ => panic!("Unsupported format"),
    }
}

fn run2<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    chunks: SampleChunks,
) -> Result<AudioStream, anyhow::Error>
where
    T: SizedSample + FromSample<f32>,
{
    let sample_rate = config.sample_rate.0 as f32;
    println!("sample rate: {}", sample_rate);
    let channels = config.channels as usize;

    // let c =
    // let mut c = c;
    // c.reset(Some(sample_rate));
    // c.allocate();

    let mut samples = chunks.into_iter().unwrap();
    // let mut next_value = move || samples.next().unwrap();

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let now = Instant::now();

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            // println!("Now: {}", now.elapsed().as_nanos());
            write_data2(data, channels, &mut samples)
        },
        err_fn,
        None,
    )?;
    stream.play()?;

    Ok(AudioStream { stream })
}

fn write_data2<T>(
    output: &mut [T],
    channels: usize,
    next_sample: &mut impl Iterator<Item = (f32, f32)>,
) where
    T: SizedSample + FromSample<f32>,
{
    let now = Instant::now();
    for frame in output.chunks_mut(channels) {
        if let Some(sample) = next_sample.next() {
            // println!("sample ({}, {})", sample.0, sample.1);

            let left = T::from_sample(sample.0);
            let right: T = T::from_sample(sample.1);

            for (channel, sample) in frame.iter_mut().enumerate() {
                if channel & 1 == 0 {
                    *sample = left;
                } else {
                    *sample = right;
                }
            }
        } else {
            println!("Received no sample");
        }
    }
}
