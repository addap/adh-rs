use fundsp::prelude::*;
use plotters::prelude::*;
use rand::{self, distributions::Distribution, SeedableRng};
use rustdct::DctPlanner;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
// use fundsp::hacker::*;

const FREQ_RATIO: f64 = 1.25;
const WEIGHTS_NUM: usize = 32;
const SAMPLE_SIZE: usize = 44100;
const SAMPLE_FREQ: f64 = 44100.0;
const MIN_FREQ: f64 = 20.0;
const MAX_FREQ: f64 = 20_000.0;

const DEBUG_WEIGHTS: [f64; WEIGHTS_NUM] = [
    1.0,
    0.9811392012814704,
    0.9717706804749016,
    0.9620845228532783,
    0.9520584314786297,
    0.9416676760012369,
    0.9308847251489949,
    0.9196788071052001,
    0.9080153800994131,
    0.8958554902654713,
    0.8831549866738044,
    0.8698635536352637,
    0.8559235067297116,
    0.8412682797618402,
    0.8258205022560471,
    0.8094895268732046,
    0.7921682063542231,
    0.7737286288863457,
    0.754016379890074,
    0.7328426735604391,
    0.7099733285108573,
    0.6851129350852487,
    0.6578814551411558,
    0.6277784506674913,
    0.5941261547656673,
    0.5559743283015182,
    0.5119312769223014,
    0.45983940355260006,
    0.39608410317711157,
    0.31388922533374564,
    0.19804205158855578,
    0.0,
];
fn main() {
    // save_white_noise();
    // gen_freqs_convert();
    // gen_white_noise_and_play()
    gen_scaled_noise(&DEBUG_WEIGHTS);
}

fn save_white_noise() {
    let wave1 = Wave64::render(44100.0, 10.0, &mut (white()));
    wave1.save_wav16("test.wav").expect("Could not save wave.");
}

fn freq_index(i: usize) -> f64 {
    min(MIN_FREQ * math::pow(FREQ_RATIO, i as f64), MAX_FREQ)
}

fn get_freq_weight(weights: &[f64; WEIGHTS_NUM], i: usize) -> f64 {
    // some frequency between 0 and 22050 Hz.
    let freq = SAMPLE_FREQ * (i as f64 / SAMPLE_SIZE as f64);

    let mut left = (0.0, 0.0);
    for j in 0..WEIGHTS_NUM {
        let right_freq = freq_index(j);
        let right_weight = weights[j];

        if right_freq > freq {
            // interpolate between left and right
            let t = (freq - left.0) / (right_freq - left.0);
            let weight = math::lerp(left.1, right_weight, t);
            return weight;
        } else if j == WEIGHTS_NUM - 1 {
            // should interpolate
            return right_weight;
        } else {
            left = (right_freq, right_weight);
        }
    }
    unreachable!();
}

fn gen_scaled_noise(weights: &[f64; WEIGHTS_NUM]) {
    let mut freqs = gen_white_freqs();

    for i in 0..SAMPLE_SIZE / 2 {
        let weight = get_freq_weight(weights, i);
        freqs[i] *= weight;
    }

    // mirror frequencies in second half
    for i in 0..SAMPLE_SIZE / 2 {
        freqs[SAMPLE_SIZE / 2 + i] = freqs[SAMPLE_SIZE / 2 - 1 - i];
    }

    idct(&mut freqs);

    play_samples(freqs);
}

fn gen_freqs_convert() {
    // give every frequency the same weight
    let mut samples = gen_white_freqs();
    // for (i, s) in samples.iter_mut().enumerate() {
    //     if i == 0 {
    //         *s = 0.0;
    //     } else {
    //         *s = *s * log(i as f64);
    //     }
    // }
    // let mut samples = vec![0.5; SAMPLE_SIZE];
    // samples[0];

    idct(&mut samples);
    // normalize output
    // println!("{:?}", samples);

    draw_samples(&samples, &samples);
    play_samples(samples);
}

fn gen_white_noise_and_play() {
    let mut samples = gen_white_fundsp();

    let mut samples_copy = samples.clone();
    dct(&mut samples_copy);

    // println!("{:?}", samples_copy);
    println!("DC Component {:}", samples_copy[0]);

    idct(&mut samples_copy);
    // normalize output

    draw_samples(&samples, &samples_copy);
    play_samples(samples_copy);
}

fn play_samples(s: Vec<f64>) {
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

fn run<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    s: Vec<f64>,
) -> Result<(), anyhow::Error>
where
    T: SizedSample + FromSample<f64>,
{
    let sample_rate = config.sample_rate.0 as f64;
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

fn write_data<T>(output: &mut [T], channels: usize, next_sample: &mut dyn FnMut() -> (f64, f64))
where
    T: SizedSample + FromSample<f64>,
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

fn draw_samples(s1: &[f64], s2: &[f64]) {
    let root = BitMapBackend::new("samples.png", (1280, 780)).into_drawing_area();
    root.fill(&WHITE).unwrap();
    let mut chart = ChartBuilder::on(&root)
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0..SAMPLE_SIZE, -0.1f64..1f64)
        .unwrap();

    chart.configure_mesh().draw().unwrap();

    // chart
    //     .draw_series(LineSeries::new(
    //         s1.iter().enumerate().map(|(x, y)| (x, *y)),
    //         &RED,
    //     ))
    //     .unwrap();
    // chart
    //     .draw_series(LineSeries::new(
    //         s2.iter().enumerate().map(|(x, y)| (x, *y)),
    //         &BLUE,
    //     ))
    //     .unwrap();

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()
        .unwrap();

    root.present().unwrap();
}

fn gen_white_fundsp() -> Vec<f64> {
    let mut c: An<Noise<f64>> = white();
    c.reset(Some(44100.0));
    c.allocate();

    let mut samples = Vec::new();
    for _ in 0..SAMPLE_SIZE {
        samples.push(c.get_mono());
    }
    samples
}

fn dct(fs: &mut [f64]) {
    let now = Instant::now();
    let dct = DctPlanner::new().plan_dct2(fs.len());
    dct.process_dct2(fs);

    let scale: f64 = sqrt(2.0 / SAMPLE_SIZE as f64);
    for f in fs {
        *f = *f * scale;
    }

    // println!("{:#?}", fs);
    println!("Took {}", now.elapsed().as_millis());
}

fn idct(fs: &mut [f64]) {
    let now = Instant::now();
    let idct = DctPlanner::new().plan_dct3(fs.len());
    idct.process_dct3(fs);

    let scale: f64 = sqrt(2.0 / SAMPLE_SIZE as f64);
    for f in fs {
        *f = *f * scale;
    }

    // println!("{:#?}", fs);
    println!("Took {}", now.elapsed().as_millis());
}

fn gen_white() -> Vec<f64> {
    let r = rand::distributions::Uniform::new_inclusive(-1.0, 1.0);
    let mut small_rng = rand::rngs::SmallRng::from_entropy();
    let mut my_signal: Vec<f64> = r.sample_iter(small_rng).take(SAMPLE_SIZE).collect();

    my_signal
}

fn gen_white_freqs() -> Vec<f64> {
    let r = rand::distributions::Uniform::new_inclusive(-1.0, 1.0);
    let mut small_rng = rand::rngs::SmallRng::from_entropy();
    let mut my_freqs: Vec<f64> = r.sample_iter(small_rng).take(SAMPLE_SIZE).collect();

    my_freqs
}
