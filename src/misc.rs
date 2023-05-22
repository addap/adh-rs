// use fundsp::prelude::*;
// use rand::{self, distributions::Distribution, SeedableRng};
// use rustdct::DctPlanner;
// use std::f32::consts::PI;
// use std::sync::mpsc;

// use crate::generator::{gen_white_freqs, idct, CHUNK_SAMPLES};

// pub fn test_dct() {
//     let mut samples = Vec::new();
//     for i in 0..=512 {
//         let r = 220.0 * i as f32 / 512 as f32 * 2.0 * PI;
//         let s = sin(r);
//         samples.push(s);
//     }
//     println!("{:?}", samples);

//     dct(&mut samples);
//     for x in &mut samples {
//         if *x < 1.01 {
//             *x = 0.0;
//         }
//     }
//     println!("{:?}", samples)
// }

// pub fn test_dct2() {
//     fn help(i: usize) {
//         let mut samples = vec![0.0; 256];
//         samples[i] = 10.0;

//         idct(&mut samples);
//         // let filename = format!("testdct/{:0>3}.png", i);
//         // let root = BitMapBackend::new(&filename, (1280, 780)).into_drawing_area();
//         // root.fill(&WHITE).unwrap();
//         // let mut chart = ChartBuilder::on(&root)
//         //     .margin(5)
//         //     .x_label_area_size(30)
//         //     .y_label_area_size(30)
//         //     .build_cartesian_2d(0..256usize, -0.1f32..1f32)
//         //     .unwrap();

//         // chart.configure_mesh().draw().unwrap();

//         // chart
//         //     .draw_series(LineSeries::new(
//         //         samples.into_iter().enumerate().map(|(x, y)| (x, y)),
//         //         &RED,
//         //     ))
//         //     .unwrap();

//         // chart
//         //     .configure_series_labels()
//         //     .background_style(&WHITE.mix(0.8))
//         //     .border_style(&BLACK)
//         //     .draw()
//         //     .unwrap();

//         // root.present().unwrap();
//     }
//     for i in 0..256 {
//         help(i);
//     }
// }

// pub fn save_white_noise() {
//     let wave1 = Wave64::render(44100.0, 10.0, &mut (white()));
//     wave1.save_wav16("test.wav").expect("Could not save wave.");
// }

// pub fn gen_freqs_convert() {
//     // give every frequency the same weight
//     let mut samples = gen_white_freqs();

//     idct(&mut samples.as_mut_slice());
//     // println!("{:?}", samples);

//     // draw_samples(&samples, &samples);
//     let (tx, rx) = mpsc::channel();
//     play_samples(rx, samples);
// }

// pub fn gen_freqs_convert_low() {
//     // give every frequency the same weight
//     let mut samples = gen_white_freqs();

//     for i in CHUNK_SAMPLES / 2 + 1..CHUNK_SAMPLES {
//         samples[i] = 0.0;
//     }

//     idct(&mut samples.as_mut_slice());
//     // println!("{:?}", samples);

//     // draw_samples(&samples, &samples);
//     let (tx, rx) = mpsc::channel();
//     play_samples(rx, samples);
// }

// pub fn gen_freqs_convert_high() {
//     // give every frequency the same weight
//     let mut samples = gen_white_freqs();

//     for i in 0..CHUNK_SAMPLES / 2 {
//         samples[i] = 0.0;
//     }

//     idct(&mut samples.as_mut_slice());
//     // println!("{:?}", samples);

//     // draw_samples(&samples, &samples);
//     let (tx, rx) = mpsc::channel();
//     play_samples(rx, samples);
// }

// pub fn gen_white_noise_and_play() {
//     let mut samples = gen_white_fundsp();

//     let mut samples_copy = samples.clone();
//     dct(&mut samples_copy);

//     // println!("{:?}", samples_copy);
//     println!("DC Component {:}", samples_copy[0]);

//     idct(&mut samples_copy);
//     // normalize output

//     // draw_samples(&samples, &samples_copy);
//     let (tx, rx) = mpsc::channel();
//     play_samples(rx, samples_copy);
// }

// // fn draw_samples(s1: &[f32], s2: &[f32]) {
// //     let root = BitMapBackend::new("samples.png", (1280, 780)).into_drawing_area();
// //     root.fill(&WHITE).unwrap();
// //     let mut chart = ChartBuilder::on(&root)
// //         .margin(5)
// //         .x_label_area_size(30)
// //         .y_label_area_size(30)
// //         .build_cartesian_2d(0..SAMPLE_SIZE, -0.1f32..1f32)
// //         .unwrap();

// //     chart.configure_mesh().draw().unwrap();

// //     chart
// //         .draw_series(LineSeries::new(
// //             s1.iter().enumerate().map(|(x, y)| (x, *y)),
// //             &RED,
// //         ))
// //         .unwrap();
// //     chart
// //         .draw_series(LineSeries::new(
// //             s2.iter().enumerate().map(|(x, y)| (x, *y)),
// //             &BLUE,
// //         ))
// //         .unwrap();

// //     chart
// //         .configure_series_labels()
// //         .background_style(&WHITE.mix(0.8))
// //         .border_style(&BLACK)
// //         .draw()
// //         .unwrap();

// //     root.present().unwrap();
// // }

// pub fn gen_white_fundsp() -> Vec<f32> {
//     let mut c: An<Noise<f32>> = white();
//     c.reset(Some(44100.0));
//     c.allocate();

//     let mut samples = Vec::new();
//     for _ in 0..CHUNK_SAMPLES {
//         samples.push(c.get_mono());
//     }
//     samples
// }

// pub fn dct(fs: &mut [f32]) {
//     // let now = Instant::now();
//     let dct = DctPlanner::new().plan_dct2(fs.len());
//     dct.process_dct2(fs);

//     let scale: f32 = sqrt(2.0 / CHUNK_SAMPLES as f32);
//     for f in fs {
//         *f = *f * scale;
//     }

//     // println!("{:#?}", fs);
//     // println!("Took {}", now.elapsed().as_millis());
// }

// pub fn gen_white() -> Vec<f32> {
//     let r = rand::distributions::Uniform::new_inclusive(-1.0, 1.0);
//     let mut small_rng = rand::rngs::SmallRng::from_entropy();
//     let mut my_signal: Vec<f32> = r.sample_iter(small_rng).take(CHUNK_SAMPLES).collect();

//     my_signal
// }
