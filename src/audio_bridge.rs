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
use fundsp::prelude::lerp;
use std::f32;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};

use crate::generator::{Chunk, CHUNK_SAMPLES};

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
pub struct SampleChunks {
    chunks: Vec<Chunk>,
    blend_type: BlendType,
}

struct SampleChunksIter {
    chunks: Vec<Chunk>,
    blend_type: BlendType,
    chunk_idx: usize,
    sample_idx: usize,
}

impl SampleChunks {
    // pub fn new(chunk: Chunk) -> Result<Self, anyhow::Error> {
    pub fn new(chunks: Vec<Chunk>, blend_type: BlendType) -> Result<Self, anyhow::Error> {
        if chunks.is_empty() {
            return Err(anyhow!("Empty chunks"));
        }
        Ok(Self { chunks, blend_type })
    }

    fn into_iter(self) -> Result<impl Iterator<Item = (f32, f32)>, anyhow::Error> {
        SampleChunksIter::new(self.chunks, self.blend_type)
        // return Ok(self.chunk.into_iter().map(|f| (f, f)).cycle());

        // let mut mirrored_chunk: Vec<f32> = self.chunk.iter().map(|f| *f).rev().collect();
        // let mut iter = self.chunk;
        // iter.extend_from_slice(mirrored_chunk.drain(..).as_slice());

        // Ok(iter.into_iter().map(|f| (f, f)).cycle())
    }
}

impl SampleChunksIter {
    fn new(chunks: Vec<Chunk>, blend_type: BlendType) -> Result<Self, anyhow::Error> {
        Ok(Self {
            chunks,
            blend_type,
            chunk_idx: 0,
            sample_idx: 0,
        })
    }
}

const BLEND_WINDOW: usize = 1000;

impl Iterator for SampleChunksIter {
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

pub fn play_samples(rx: Receiver<()>, s: Vec<f32>) {
    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .expect("Failed to find a default output device");
    let config = device.default_output_config().unwrap();

    match config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(rx, &device, &config.into(), s).unwrap(),
        cpal::SampleFormat::I16 => run::<i16>(rx, &device, &config.into(), s).unwrap(),
        cpal::SampleFormat::U16 => run::<u16>(rx, &device, &config.into(), s).unwrap(),
        _ => panic!("Unsupported format"),
    }
}

fn run<T>(
    rx: Receiver<()>,
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    s: Vec<f32>,
) -> Result<(), anyhow::Error>
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

    let mut samples = s.into_iter().cycle();
    let mut next_value = move || {
        let x = samples.next().unwrap();
        (x, x)
    };

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            write_data(data, channels, &mut next_value)
        },
        err_fn,
        None,
    )?;
    stream.play()?;

    loop {
        match rx.try_recv() {
            Ok(()) | Err(TryRecvError::Disconnected) => return Ok(()),
            _ => {}
        };
        thread::sleep(Duration::from_secs(1));
    }
}

fn write_data<T>(output: &mut [T], channels: usize, next_sample: &mut dyn FnMut() -> (f32, f32))
where
    T: SizedSample + FromSample<f32>,
{
    for frame in output.chunks_mut(channels) {
        let sample = next_sample();
        let left = T::from_sample(sample.0);
        let right: T = T::from_sample(sample.1);

        for (channel, sample) in frame.iter_mut().enumerate() {
            if channel & 1 == 0 {
                *sample = left;
            } else {
                *sample = right;
            }
        }
    }
}
