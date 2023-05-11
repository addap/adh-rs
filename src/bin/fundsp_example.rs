//! Make some noise via cpal.
#![allow(clippy::precedence)]
#![recursion_limit = "520"]

use std::fs::OpenOptions;
use std::io::Read;

use assert_no_alloc::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use fundsp::hacker::*;

#[cfg(debug_assertions)] // required when disable_release is set (default)
#[global_allocator]
static A: AllocDisabler = AllocDisabler;

macro_rules! sumfreqs {
    // The pattern for a single `eval`
    ($e:literal) => {
        sine_hz($e)
    };

    // Decompose multiple `eval`s recursively
    ($e:literal, $($es:literal),+) => {
        sumfreqs!($e) + sumfreqs!($($es),+)
    };
}

fn main() {
    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .expect("Failed to find a default output device");
    let config = device.default_output_config().unwrap();

    match config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(&device, &config.into()).unwrap(),
        cpal::SampleFormat::I16 => run::<i16>(&device, &config.into()).unwrap(),
        cpal::SampleFormat::U16 => run::<u16>(&device, &config.into()).unwrap(),
        _ => panic!("Unsupported format"),
    }
}

fn run<T>(device: &cpal::Device, config: &cpal::StreamConfig) -> Result<(), anyhow::Error>
where
    T: SizedSample + FromSample<f64>,
{
    // let mut f = OpenOptions::new().read(true).open("./export.txt").unwrap();
    // let mut s = String::new();
    // f.read_to_string(&mut s).unwrap();
    // let values: Vec<Config> = serde_json::from_str(&s).unwrap();

    let sample_rate = config.sample_rate.0 as f64;
    println!("sample rate: {}", sample_rate);
    let channels = config.channels as usize;

    // let mut c = zero();
    let mut c = brown();
    // let mut net = Net64::new(0, 1);

    // let mut i = 0;
    // for config in values {
    //     println!("{}", config.hz);
    //     let net2 = Net64::wrap(Box::new(sine_hz(config.hz)));
    //     net = net + net2;
    //     i += 1;
    //     // if i == 50 {
    //     // break;
    //     // }
    // }

    // freqs.iter().map(|f| sine_hz(*f)).reduce(|a, b| a | b);

    //let c = mls();
    //let c = (mls() | dc(400.0) | dc(50.0)) >> resonator();
    // let c = brown();
    // let c = sine_hz(freqs[0]) + sine_hz(freqs[1]) + sine_hz(freqs[2]) + sine_hz(freqs[3]);
    // let c = sine_hz(f);

    // FM synthesis.
    //let f = 110.0;
    //let m = 5.0;
    //let c = oversample(sine_hz(f) * f * m + f >> sine());

    // Pulse wave.
    //let c = lfo(|t| {
    //    let pitch = 220.0;
    //    let duty = lerp11(0.01, 0.99, sin_hz(0.05, t));
    //    (pitch, duty)
    //}) >> pulse();

    //let c = zero() >> pluck(220.0, 0.8, 0.8);
    //let c = dc(110.0) >> dsf_saw_r(0.99);
    //let c = dc(110.0) >> triangle();
    //let c = dc(110.0) >> soft_saw();
    //let c = lfo(|t| xerp11(20.0, 2000.0, sin_hz(0.1, t))) >> dsf_square_r(0.99) >> lowpole_hz(1000.0);
    //let c = dc(110.0) >> square();
    // let c = 0.2 * (organ_hz(midi_hz(57.0)) + organ_hz(midi_hz(61.0)) + organ_hz(midi_hz(64.0)));
    //let c = dc(440.0) >> rossler();
    //let c = dc(110.0) >> lorenz();
    //let c = organ_hz(110.1) + organ_hz(54.9);
    //let c = pink() >> hold_hz(440.0, 0.0);

    // Filtered noise tone.
    //let c = (noise() | dc((440.0, 50.0))) >> !resonator() >> resonator();

    // Test ease_noise.
    //let c = lfo(|t| xerp11(50.0, 5000.0, ease_noise(smooth9, 0, t))) >> triangle();

    // Bandpass filtering.
    //let c = c >> (pass() | envelope(|t| xerp11(500.0, 5000.0, sin_hz(0.05, t)))) >> bandpass_q(5.0);
    //let c = c >> (pass() | envelope(|t| (xerp11(500.0, 5000.0, sin_hz(0.05, t)), 0.9))) >> !bandrez() >> bandrez();

    // Waveshaper.
    //let c = c >> shape(Shape::Crush(20.0));

    // Add feedback delay.
    //let c = c >> (pass() & feedback(butterpass_hz(1000.0) >> delay(1.0) * 0.5));

    // Apply Moog filter.
    //let c = (c | lfo(|t| (xerp11(110.0, 11000.0, sin_hz(0.1, t)), 0.6))) >> moog();

    // let c = c >> pan(0.0);

    //let c = fundsp::sound::risset_glissando(false);

    // Add chorus.
    // let c = c >> (chorus(0, 0.0, 0.01, 0.2) | chorus(1, 0.0, 0.01, 0.2));

    // Add flanger.
    //let c = c
    //    >> (flanger(0.6, 0.005, 0.01, |t| lerp11(0.005, 0.01, sin_hz(0.1, t)))
    //        | flanger(0.6, 0.005, 0.01, |t| lerp11(0.005, 0.01, cos_hz(0.1, t))));

    // Add phaser.
    //let c = c
    //    >> (phaser(0.5, |t| sin_hz(0.1, t) * 0.5 + 0.5)
    //        | phaser(0.5, |t| cos_hz(0.1, t) * 0.5 + 0.5));

    // let mut c = c
    //     >> (declick() | declick())
    //     >> (dcblock() | dcblock())
    //     //>> (multipass() & 0.2 * reverb_stereo(10.0, 3.0))
    //     >> limiter_stereo((1.0, 5.0));
    //let mut c = c * 0.1;

    let mut c = c;
    c.reset(Some(sample_rate));
    c.allocate();

    let mut next_value = move || assert_no_alloc(|| c.get_stereo());

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

    while true {
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

const freqs: [f64; 511] = [
    46.875, 93.75, 140.625, 187.5, 234.375, 281.25, 328.125, 375.0, 421.875, 468.75, 515.625,
    562.5, 609.375, 656.25, 703.125, 750.0, 796.875, 843.75, 890.625, 937.5, 984.375, 1031.25,
    1078.125, 1125.0, 1171.875, 1218.75, 1265.625, 1312.5, 1359.375, 1406.25, 1453.125, 1500.0,
    1546.875, 1593.75, 1640.625, 1687.5, 1734.375, 1781.25, 1828.125, 1875.0, 1921.875, 1968.75,
    2015.625, 2062.5, 2109.375, 2156.25, 2203.125, 2250.0, 2296.875, 2343.75, 2390.625, 2437.5,
    2484.375, 2531.25, 2578.125, 2625.0, 2671.875, 2718.75, 2765.625, 2812.5, 2859.375, 2906.25,
    2953.125, 3000.0, 3046.875, 3093.75, 3140.625, 3187.5, 3234.375, 3281.25, 3328.125, 3375.0,
    3421.875, 3468.75, 3515.625, 3562.5, 3609.375, 3656.25, 3703.125, 3750.0, 3796.875, 3843.75,
    3890.625, 3937.5, 3984.375, 4031.25, 4078.125, 4125.0, 4171.875, 4218.75, 4265.625, 4312.5,
    4359.375, 4406.25, 4453.125, 4500.0, 4546.875, 4593.75, 4640.625, 4687.5, 4734.375, 4781.25,
    4828.125, 4875.0, 4921.875, 4968.75, 5015.625, 5062.5, 5109.375, 5156.25, 5203.125, 5250.0,
    5296.875, 5343.75, 5390.625, 5437.5, 5484.375, 5531.25, 5578.125, 5625.0, 5671.875, 5718.75,
    5765.625, 5812.5, 5859.375, 5906.25, 5953.125, 6000.0, 6046.875, 6093.75, 6140.625, 6187.5,
    6234.375, 6281.25, 6328.125, 6375.0, 6421.875, 6468.75, 6515.625, 6562.5, 6609.375, 6656.25,
    6703.125, 6750.0, 6796.875, 6843.75, 6890.625, 6937.5, 6984.375, 7031.25, 7078.125, 7125.0,
    7171.875, 7218.75, 7265.625, 7312.5, 7359.375, 7406.25, 7453.125, 7500.0, 7546.875, 7593.75,
    7640.625, 7687.5, 7734.375, 7781.25, 7828.125, 7875.0, 7921.875, 7968.75, 8015.625, 8062.5,
    8109.375, 8156.25, 8203.125, 8250.0, 8296.875, 8343.75, 8390.625, 8437.5, 8484.375, 8531.25,
    8578.125, 8625.0, 8671.875, 8718.75, 8765.625, 8812.5, 8859.375, 8906.25, 8953.125, 9000.0,
    9046.875, 9093.75, 9140.625, 9187.5, 9234.375, 9281.25, 9328.125, 9375.0, 9421.875, 9468.75,
    9515.625, 9562.5, 9609.375, 9656.25, 9703.125, 9750.0, 9796.875, 9843.75, 9890.625, 9937.5,
    9984.375, 10031.25, 10078.125, 10125.0, 10171.875, 10218.75, 10265.625, 10312.5, 10359.375,
    10406.25, 10453.125, 10500.0, 10546.875, 10593.75, 10640.625, 10687.5, 10734.375, 10781.25,
    10828.125, 10875.0, 10921.875, 10968.75, 11015.625, 11062.5, 11109.375, 11156.25, 11203.125,
    11250.0, 11296.875, 11343.75, 11390.625, 11437.5, 11484.375, 11531.25, 11578.125, 11625.0,
    11671.875, 11718.75, 11765.625, 11812.5, 11859.375, 11906.25, 11953.125, 12000.0, 12046.875,
    12093.75, 12140.625, 12187.5, 12234.375, 12281.25, 12328.125, 12375.0, 12421.875, 12468.75,
    12515.625, 12562.5, 12609.375, 12656.25, 12703.125, 12750.0, 12796.875, 12843.75, 12890.625,
    12937.5, 12984.375, 13031.25, 13078.125, 13125.0, 13171.875, 13218.75, 13265.625, 13312.5,
    13359.375, 13406.25, 13453.125, 13500.0, 13546.875, 13593.75, 13640.625, 13687.5, 13734.375,
    13781.25, 13828.125, 13875.0, 13921.875, 13968.75, 14015.625, 14062.5, 14109.375, 14156.25,
    14203.125, 14250.0, 14296.875, 14343.75, 14390.625, 14437.5, 14484.375, 14531.25, 14578.125,
    14625.0, 14671.875, 14718.75, 14765.625, 14812.5, 14859.375, 14906.25, 14953.125, 15000.0,
    15046.875, 15093.75, 15140.625, 15187.5, 15234.375, 15281.25, 15328.125, 15375.0, 15421.875,
    15468.75, 15515.625, 15562.5, 15609.375, 15656.25, 15703.125, 15750.0, 15796.875, 15843.75,
    15890.625, 15937.5, 15984.375, 16031.25, 16078.125, 16125.0, 16171.875, 16218.75, 16265.625,
    16312.5, 16359.375, 16406.25, 16453.125, 16500.0, 16546.875, 16593.75, 16640.625, 16687.5,
    16734.375, 16781.25, 16828.125, 16875.0, 16921.875, 16968.75, 17015.625, 17062.5, 17109.375,
    17156.25, 17203.125, 17250.0, 17296.875, 17343.75, 17390.625, 17437.5, 17484.375, 17531.25,
    17578.125, 17625.0, 17671.875, 17718.75, 17765.625, 17812.5, 17859.375, 17906.25, 17953.125,
    18000.0, 18046.875, 18093.75, 18140.625, 18187.5, 18234.375, 18281.25, 18328.125, 18375.0,
    18421.875, 18468.75, 18515.625, 18562.5, 18609.375, 18656.25, 18703.125, 18750.0, 18796.875,
    18843.75, 18890.625, 18937.5, 18984.375, 19031.25, 19078.125, 19125.0, 19171.875, 19218.75,
    19265.625, 19312.5, 19359.375, 19406.25, 19453.125, 19500.0, 19546.875, 19593.75, 19640.625,
    19687.5, 19734.375, 19781.25, 19828.125, 19875.0, 19921.875, 19968.75, 20015.625, 20062.5,
    20109.375, 20156.25, 20203.125, 20250.0, 20296.875, 20343.75, 20390.625, 20437.5, 20484.375,
    20531.25, 20578.125, 20625.0, 20671.875, 20718.75, 20765.625, 20812.5, 20859.375, 20906.25,
    20953.125, 21000.0, 21046.875, 21093.75, 21140.625, 21187.5, 21234.375, 21281.25, 21328.125,
    21375.0, 21421.875, 21468.75, 21515.625, 21562.5, 21609.375, 21656.25, 21703.125, 21750.0,
    21796.875, 21843.75, 21890.625, 21937.5, 21984.375, 22031.25, 22078.125, 22125.0, 22171.875,
    22218.75, 22265.625, 22312.5, 22359.375, 22406.25, 22453.125, 22500.0, 22546.875, 22593.75,
    22640.625, 22687.5, 22734.375, 22781.25, 22828.125, 22875.0, 22921.875, 22968.75, 23015.625,
    23062.5, 23109.375, 23156.25, 23203.125, 23250.0, 23296.875, 23343.75, 23390.625, 23437.5,
    23484.375, 23531.25, 23578.125, 23625.0, 23671.875, 23718.75, 23765.625, 23812.5, 23859.375,
    23906.25, 23953.125,
];
