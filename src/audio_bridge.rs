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
use fundsp::prelude::lerp;
use std::f32;
use std::time::{Duration, Instant};

use crate::generator::{MonoSampleIterator, PlayableChunk, CHUNK_SAMPLES};

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

pub struct ChunkCollection {
    chunks: Vec<PlayableChunk>,
    smoothing_type: SmoothingType,
}

type SampleIter = Box<dyn Iterator<Item = (f32, f32)> + Send>;

struct BlendingChunksIter<I, J> {
    chunk_iter: I,
    current_chunk: J,
    next_chunk: J,
    blend_type: BlendType,
}

impl ChunkCollection {
    // pub fn new(chunk: Chunk) -> Result<Self, anyhow::Error> {
    pub fn new(chunks: Vec<PlayableChunk>) -> Result<Self, anyhow::Error> {
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

    fn into_iter(self) -> Result<SampleIter, anyhow::Error> {
        match self.smoothing_type {
            SmoothingType::Mirror => {
                let first_chunk = self.chunks.into_iter().next().unwrap();

                let iter = first_chunk
                    .clone()
                    .into_iter()
                    .chain(first_chunk.into_iter().rev())
                    .map(|f| (f, f))
                    .cycle();

                Ok(Box::new(iter))
            }
            SmoothingType::Blend(blend_type) => Ok(Box::new(BlendingChunksIter::new(
                self.chunks.into_iter(),
                blend_type,
            )?)),
        }
    }
}

impl<I: Iterator<Item = J>, J: IntoIterator<IntoIter = K>, K> BlendingChunksIter<I, K> {
    fn new(mut chunk_iter: I, blend_type: BlendType) -> Result<Self, anyhow::Error> {
        let current_chunk = chunk_iter.next().unwrap();
        let next_chunk = chunk_iter.next().unwrap();

        Ok(Self {
            chunk_iter: chunk_iter,
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
    > Iterator for BlendingChunksIter<I, K>
{
    type Item = (f32, f32);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_chunk.len() <= BLEND_WINDOW {
            let weight = 1.0 - self.current_chunk.len() as f32 / BLEND_WINDOW as f32;
            let s1 = self.current_chunk.next().unwrap();
            let s2 = self.next_chunk.next().unwrap();

            let s = self.blend_type.blend(s1, s2, weight);

            if self.current_chunk.len() == 0 {
                let new_next = self.chunk_iter.next().unwrap().into_iter();
                let old_next = std::mem::replace(&mut self.next_chunk, new_next);
                self.current_chunk = old_next;
            }

            Some((s, s))
        } else {
            let s = self.current_chunk.next().unwrap();
            Some((s, s))
        }
    }
}

pub struct AudioStream {
    stream: cpal::Stream,
}

pub fn play(chunks: ChunkCollection) -> AudioStream {
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
    chunks: ChunkCollection,
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

fn write_data2<'a, T>(
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
