use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};

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
