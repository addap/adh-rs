use adh_rs::{
    audio_bridge::play,
    chunk::{BlendType, ChunkCollection},
    socket, Weights,
};

fn main() -> Result<(), anyhow::Error> {
    let socket = socket::get_socket()?;
    let mut audio_stream;

    loop {
        let mut buf = vec![0; 1024];
        let read_bytes = socket.recv(&mut buf).unwrap();
        println!("Received Weights.");

        let weights: Weights = serde_json::from_slice(&buf[..read_bytes]).unwrap();

        let samples1 = adh_rs::generator::gen_weighted_noise(&weights);
        let samples2 = adh_rs::generator::gen_weighted_noise(&weights);
        // let chunks = SampleChunks::new(samples1).unwrap();
        let chunks = ChunkCollection::new(vec![samples1, samples2])
            .unwrap()
            .with_blend(BlendType::Sigmoid);

        audio_stream = play(chunks);
    }
}
