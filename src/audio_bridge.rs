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
use std::f32;

use crate::samples::BlendingSamples;

pub struct AudioStream {
    pub stream: cpal::Stream,
}

pub fn play(samples: BlendingSamples) -> Result<AudioStream, anyhow::Error> {
    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .ok_or(anyhow!("Failed to find a default output device"))?;
    let config = device.default_output_config()?;

    match config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(&device, &config.into(), samples),
        cpal::SampleFormat::I16 => run::<i16>(&device, &config.into(), samples),
        cpal::SampleFormat::U16 => run::<u16>(&device, &config.into(), samples),
        _ => panic!("Unsupported format"),
    }
}

fn run<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    samples: BlendingSamples,
) -> Result<AudioStream, anyhow::Error>
where
    T: SizedSample + FromSample<f32>,
{
    let sample_rate = config.sample_rate.0;
    let channels = config.channels as usize;
    println!("Playing with sample rate {} on {} channels.", sample_rate, channels);

    let mut samples_iter = samples.into_iter()?;
    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    // At this point we give the samples_iter to another thread which actually plays the audio, so it needs to be Send.
    let stream = device.build_output_stream(
        config,
        move |output: &mut [T], _: &cpal::OutputCallbackInfo| write_data(output, channels, &mut samples_iter),
        err_fn,
        None,
    )?;
    stream.play()?;

    Ok(AudioStream { stream })
}

fn write_data<'a, T>(output: &mut [T], channels: usize, samples_iter: &mut impl Iterator<Item = (f32, f32)>)
where
    T: SizedSample + FromSample<f32>,
{
    // For each sample time we get a frame containing one element per channel.
    // a.d. TODO How many channels are there? Is it liek stereo -> 2 channels, dolby digital 5.1 -> 5 channels etc.?
    for frame in output.chunks_mut(channels) {
        if let Some(sample) = samples_iter.next() {
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
