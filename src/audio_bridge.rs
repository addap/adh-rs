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

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use std::f32;
use std::time::Instant;

use crate::chunk::ChunkCollection;

pub struct AudioStream {
    pub stream: cpal::Stream,
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
