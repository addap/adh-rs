use cpal::traits::StreamTrait;

use adh_rs::{
    audio_bridge::play,
    chunk::{BlendType, ChunkCollection},
    protocol,
};

fn main() -> Result<(), anyhow::Error> {
    let protocol = protocol::Protocol::new_recv()?;
    let mut audio_stream = None;
    let mut playing = false;

    loop {
        let command = protocol.recv().unwrap();
        println!("Received Command.");

        match command {
            protocol::Command::SetWeights(weights) => {
                let samples1 = adh_rs::generator::gen_weighted_noise(&weights);
                let samples2 = adh_rs::generator::gen_weighted_noise(&weights);
                // let chunks = SampleChunks::new(samples1).unwrap();
                let chunks = ChunkCollection::new(vec![samples1, samples2])
                    .unwrap()
                    .with_blend(BlendType::Sigmoid);

                let new_audio_stream = play(chunks);
                playing = true;
                audio_stream = Some(new_audio_stream);
            }
            protocol::Command::Toggle => {
                if let Some(audio_stream) = &audio_stream {
                    if playing {
                        audio_stream.stream.pause().ok();
                    } else {
                        audio_stream.stream.play().ok();
                    }
                    // only set if successful
                    playing = !playing;
                }
            }
            protocol::Command::Quit => return Ok(()),
        }
    }
}
