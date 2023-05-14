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
use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};

use crate::generator::CHUNK_SAMPLES;

type Chunk = Vec<f32>;

struct SampleChunks {
    chunks: Vec<Chunk>,
}

struct SampleChunksIter {
    chunks: Vec<Chunk>,
    chunk_idx: usize,
    sample_idx: usize,
}

impl SampleChunks {
    fn new(chunks: Vec<Chunk>) -> Result<Self, anyhow::Error> {
        Ok(Self { chunks })
    }

    fn into_iter(self) -> Result<SampleChunksIter, anyhow::Error> {
        SampleChunksIter::new(self.chunks)
    }
}

impl SampleChunksIter {
    fn new(chunks: Vec<Chunk>) -> Result<Self, anyhow::Error> {
        Ok(Self {
            chunks,
            chunk_idx: 0,
            sample_idx: 0,
        })
    }
}

const BLEND_THRESHOLD: usize = 100;

impl Iterator for SampleChunksIter {
    type Item = (f32, f32);

    fn next(&mut self) -> Option<Self::Item> {
        if self.sample_idx < CHUNK_SAMPLES - BLEND_THRESHOLD {
            // just return next sample from chunk
            let s = self.chunks[self.chunk_idx][self.sample_idx];
            self.sample_idx += 1;

            Some((s, s))
        } else if self.sample_idx < CHUNK_SAMPLES {
            let weight = CHUNK_SAMPLES - self.sample_idx;

            let s1 = self.chunks[self.chunk_idx][self.sample_idx];
            let next_chunk_idx = (self.chunk_idx + 1) % self.chunks.len();
            let next_sample_idx = self.sample_idx + BLEND_THRESHOLD - CHUNK_SAMPLES;
            let s2 = self.chunks[next_chunk_idx][next_sample_idx];

            let s = lerp(s2, s1, weight as f32 / 100.0);

            if self.sample_idx == CHUNK_SAMPLES - 1 {
                self.chunk_idx = next_chunk_idx;
                self.sample_idx = next_sample_idx;
            }

            Some((s, s))
        } else {
            unreachable!()
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
