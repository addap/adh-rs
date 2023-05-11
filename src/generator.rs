use fundsp::prelude::*;
// use plotters::prelude::*;
use rand::{self, distributions::Distribution, SeedableRng};
use rustdct::DctPlanner;
use std::f32::consts::PI;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};

// const FREQ_RATIO: f32 = 1.25;
pub const WEIGHTS_NUM: usize = 32;
const SAMPLE_SIZE: usize = 44100 * 10;
const SAMPLE_FREQ: f32 = 44100.0;
const MIN_FREQ: f32 = 20.0;
const MAX_FREQ: f32 = 20_000.0;

pub fn test_dct() {
    let mut samples = Vec::new();
    for i in 0..=512 {
        let r = 220.0 * i as f32 / 512 as f32 * 2.0 * PI;
        let s = sin(r);
        samples.push(s);
    }
    println!("{:?}", samples);

    dct(&mut samples);
    for x in &mut samples {
        if *x < 0.01 {
            *x = 0.0;
        }
    }
    println!("{:?}", samples)
}

pub fn test_dct2() {
    fn help(i: usize) {
        let mut samples = vec![0.0; 256];
        samples[i] = 10.0;

        idct(&mut samples);
        // let filename = format!("testdct/{:0>3}.png", i);
        // let root = BitMapBackend::new(&filename, (1280, 780)).into_drawing_area();
        // root.fill(&WHITE).unwrap();
        // let mut chart = ChartBuilder::on(&root)
        //     .margin(5)
        //     .x_label_area_size(30)
        //     .y_label_area_size(30)
        //     .build_cartesian_2d(0..256usize, -0.1f32..1f32)
        //     .unwrap();

        // chart.configure_mesh().draw().unwrap();

        // chart
        //     .draw_series(LineSeries::new(
        //         samples.into_iter().enumerate().map(|(x, y)| (x, y)),
        //         &RED,
        //     ))
        //     .unwrap();

        // chart
        //     .configure_series_labels()
        //     .background_style(&WHITE.mix(0.8))
        //     .border_style(&BLACK)
        //     .draw()
        //     .unwrap();

        // root.present().unwrap();
    }
    for i in 0..256 {
        help(i);
    }
}

pub fn save_white_noise() {
    let wave1 = Wave64::render(44100.0, 10.0, &mut (white()));
    wave1.save_wav16("test.wav").expect("Could not save wave.");
}

pub fn get_freq_weight(weights: &[f32; WEIGHTS_NUM], freq: f32) -> f32 {
    // some frequency between 0 and 22050 Hz.
    // we want to compute a weight based on weights between MIN_FREQ and MAX_FREQ
    if freq <= MIN_FREQ {
        return weights[0];
    } else if freq >= MAX_FREQ {
        return weights[weights.len() - 1];
    }

    // the ratio between consecutive frequencies of our weights
    let freq_ratio = (MAX_FREQ / MIN_FREQ).powf(1.0 / (WEIGHTS_NUM - 1) as f32);
    // which weight bin `freq` belongs to
    let weight_bin = f32::log(freq / MIN_FREQ, freq_ratio).clamp(0.0, (WEIGHTS_NUM - 1) as f32);
    let (left, right) = (weight_bin.floor(), weight_bin.ceil());

    let t = weight_bin - left;
    let weight = f32::lerp(weights[left as usize], weights[right as usize], t);
    return weight;
}

pub fn freq_domain_bin(i: usize) -> f32 {
    SAMPLE_FREQ * i as f32 / SAMPLE_SIZE as f32
}

pub fn freq_domain_bin2(i: usize) -> f32 {
    SAMPLE_FREQ * i as f32 / (2.0 * SAMPLE_SIZE as f32)
}

pub fn gen_weighted_noise(weights: &[f32; WEIGHTS_NUM]) {
    let mut freqs = gen_white_freqs();

    for i in 0..SAMPLE_SIZE / 2 {
        // get the frequency bin of i in the frequency domain
        let freq = freq_domain_bin(i);
        let weight = get_freq_weight(weights, freq);
        freqs[i] *= weight;
    }

    // mirror frequencies in second half
    for i in 0..SAMPLE_SIZE / 2 {
        freqs[SAMPLE_SIZE / 2 + i] = freqs[SAMPLE_SIZE / 2 - 1 - i];
        // freqs[SAMPLE_SIZE / 2 + i] = 0.0;
    }

    idct(&mut freqs);

    play_samples(freqs);
}

pub fn gen_weighted_noise_no_mirror(weights: &[f32; WEIGHTS_NUM]) {
    let mut freqs = gen_white_freqs();

    for i in 0..SAMPLE_SIZE {
        // get the frequency bin of i in the frequency domain
        let freq = freq_domain_bin2(i);
        let weight = get_freq_weight(weights, freq);
        freqs[i] *= weight;
    }

    idct(&mut freqs);

    play_samples(freqs);
}

pub fn gen_freqs_convert() {
    // give every frequency the same weight
    let mut samples = gen_white_freqs();

    idct(&mut samples);
    // println!("{:?}", samples);

    // draw_samples(&samples, &samples);
    play_samples(samples);
}

pub fn gen_freqs_convert_low() {
    // give every frequency the same weight
    let mut samples = gen_white_freqs();

    for i in SAMPLE_SIZE / 2 + 1..SAMPLE_SIZE {
        samples[i] = 0.0;
    }

    idct(&mut samples);
    // println!("{:?}", samples);

    // draw_samples(&samples, &samples);
    play_samples(samples);
}

pub fn gen_freqs_convert_high() {
    // give every frequency the same weight
    let mut samples = gen_white_freqs();

    for i in 0..SAMPLE_SIZE / 2 {
        samples[i] = 0.0;
    }

    idct(&mut samples);
    // println!("{:?}", samples);

    // draw_samples(&samples, &samples);
    play_samples(samples);
}

pub fn gen_white_noise_and_play() {
    let mut samples = gen_white_fundsp();

    let mut samples_copy = samples.clone();
    dct(&mut samples_copy);

    // println!("{:?}", samples_copy);
    println!("DC Component {:}", samples_copy[0]);

    idct(&mut samples_copy);
    // normalize output

    // draw_samples(&samples, &samples_copy);
    play_samples(samples_copy);
}

pub fn play_samples(s: Vec<f32>) {
    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .expect("Failed to find a default output device");
    let config = device.default_output_config().unwrap();

    match config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(&device, &config.into(), s).unwrap(),
        cpal::SampleFormat::I16 => run::<i16>(&device, &config.into(), s).unwrap(),
        cpal::SampleFormat::U16 => run::<u16>(&device, &config.into(), s).unwrap(),
        _ => panic!("Unsupported format"),
    }
}

pub fn run<T>(
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
        std::thread::sleep(std::time::Duration::from_millis(50000));
    }

    Ok(())
}

pub fn write_data<T>(output: &mut [T], channels: usize, next_sample: &mut dyn FnMut() -> (f32, f32))
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

// fn draw_samples(s1: &[f32], s2: &[f32]) {
//     let root = BitMapBackend::new("samples.png", (1280, 780)).into_drawing_area();
//     root.fill(&WHITE).unwrap();
//     let mut chart = ChartBuilder::on(&root)
//         .margin(5)
//         .x_label_area_size(30)
//         .y_label_area_size(30)
//         .build_cartesian_2d(0..SAMPLE_SIZE, -0.1f32..1f32)
//         .unwrap();

//     chart.configure_mesh().draw().unwrap();

//     chart
//         .draw_series(LineSeries::new(
//             s1.iter().enumerate().map(|(x, y)| (x, *y)),
//             &RED,
//         ))
//         .unwrap();
//     chart
//         .draw_series(LineSeries::new(
//             s2.iter().enumerate().map(|(x, y)| (x, *y)),
//             &BLUE,
//         ))
//         .unwrap();

//     chart
//         .configure_series_labels()
//         .background_style(&WHITE.mix(0.8))
//         .border_style(&BLACK)
//         .draw()
//         .unwrap();

//     root.present().unwrap();
// }

pub fn gen_white_fundsp() -> Vec<f32> {
    let mut c: An<Noise<f32>> = white();
    c.reset(Some(44100.0));
    c.allocate();

    let mut samples = Vec::new();
    for _ in 0..SAMPLE_SIZE {
        samples.push(c.get_mono());
    }
    samples
}

pub fn dct(fs: &mut [f32]) {
    let now = Instant::now();
    let dct = DctPlanner::new().plan_dct2(fs.len());
    dct.process_dct2(fs);

    let scale: f32 = sqrt(2.0 / SAMPLE_SIZE as f32);
    for f in fs {
        *f = *f * scale;
    }

    // println!("{:#?}", fs);
    println!("Took {}", now.elapsed().as_millis());
}

pub fn idct(fs: &mut [f32]) {
    let now = Instant::now();
    let idct = DctPlanner::new().plan_dct3(fs.len());
    idct.process_dct3(fs);

    let scale: f32 = sqrt(2.0 / SAMPLE_SIZE as f32);
    for f in fs {
        *f = *f * scale;
    }

    // println!("{:#?}", fs);
    println!("Took {}", now.elapsed().as_millis());
}

pub fn gen_white() -> Vec<f32> {
    let r = rand::distributions::Uniform::new_inclusive(-1.0, 1.0);
    let mut small_rng = rand::rngs::SmallRng::from_entropy();
    let mut my_signal: Vec<f32> = r.sample_iter(small_rng).take(SAMPLE_SIZE).collect();

    my_signal
}

pub fn gen_white_freqs() -> Vec<f32> {
    let r = rand::distributions::Uniform::new_inclusive(-1.0, 1.0);
    let mut small_rng = rand::rngs::SmallRng::from_entropy();
    let mut my_freqs: Vec<f32> = r.sample_iter(small_rng).take(SAMPLE_SIZE).collect();

    my_freqs
}
