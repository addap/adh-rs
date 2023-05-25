use std::os::{fd::FromRawFd, unix::net::UnixDatagram};

use cpal::traits::StreamTrait;
use systemd::daemon;

use adh_rs::{
    audio_bridge::play,
    chunk::{BlendType, ChunkCollection},
    protocol::{self, Protocol},
};

fn get_protocol() -> Protocol {
    if let Some(arg) = std::env::args().nth(1) {
        if &arg == "--daemon" {
            let fd = daemon::listen_fds(false).unwrap().iter().next().unwrap();
            assert!(daemon::is_socket_unix(
                fd,
                Some(daemon::SocketType::Datagram),
                // None,
                daemon::Listening::NoListeningCheck,
                None::<String>
            )
            .unwrap());
            let sock = unsafe { UnixDatagram::from_raw_fd(fd) };
            Protocol::new_raw(sock)
        } else {
            Protocol::new_recv().unwrap()
        }
    } else {
        Protocol::new_recv().unwrap()
    }
}

fn main() -> Result<(), anyhow::Error> {
    // TODO if started with '--daemon', use systemd crate to get passed file descriptors.
    // use this to instantiate protocol.
    println!("acquiring socket");
    let protocol = get_protocol();
    println!("successfully got socket");
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
                    // TODO only set if successful
                    playing = !playing;
                }
            }
            protocol::Command::Quit => return Ok(()),
        }
    }
}
