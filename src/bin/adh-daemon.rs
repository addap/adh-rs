use lazy_static::lazy_static;
use std::os::{fd::FromRawFd, unix::net::UnixDatagram};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

use adh_rs::tray_icon::TrayCommand;
use cpal::traits::StreamTrait;
use systemd::daemon;

use adh_rs::{
    audio_bridge::play,
    chunk::{BlendType, ChunkCollection},
    protocol::{self, Protocol},
};

lazy_static! {
    static ref GUI_PROGRAM_NAME: PathBuf = {
        if cfg!(debug_assertions) {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("target/debug/adh-gui")
                .to_owned()
        } else {
            Path::new("adh-gui").to_owned()
        }
    };
}

#[cfg(not(debug_assertions))]
const GUI_PROGRAM_NAME: &str = "adh-gui";

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

fn play_background() -> Result<(), anyhow::Error> {
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
            } // protocol::Command::Quit => return Ok(()),
        }
    }
}

fn main() -> Result<(), anyhow::Error> {
    // TODO if started with '--daemon', use systemd crate to get passed file descriptors.
    // use this to instantiate protocol.

    let (tx, rx) = mpsc::channel();
    thread::spawn(|| adh_rs::tray_icon::main(tx));
    thread::spawn(|| play_background());

    loop {
        let command = rx.recv().ok();
        match command {
            Some(TrayCommand::Quit) => {
                println!("Daemon quit");
                return Ok(());
            }
            Some(TrayCommand::RunGUI) => {
                println!("exec gui process");
                // could actually just run it as a thread I think. But I'm not sure that works for a binary crate.
                // thread::spawn(|| adh_rs::gui::main());
                std::process::Command::new(GUI_PROGRAM_NAME.as_path())
                    .spawn()
                    .expect("exec gui failed");
            }
            None => {}
        }
    }
}
