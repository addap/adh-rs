use adh_rs::is_development;
use anyhow::anyhow;
use cpal::traits::StreamTrait;
use lazy_static::lazy_static;
use std::os::{fd::FromRawFd, unix::net::UnixDatagram};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use systemd::daemon;

mod tray_icon;

use adh_rs::{
    audio_bridge::play,
    protocol::{GUICommand, Protocol},
    samples::{BlendType, BlendingSamples},
};
// use tray_icon::TrayCommand;

lazy_static! {
    /// Which program to start when executing a new GUI process.
    /// During development we use the executable in cargo's target/ directory.
    /// Otherwise we use the executable on the PATH.
    static ref GUI_PROGRAM_NAME: PathBuf = {
        if is_development() {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("target/debug/adh-gui")
                .to_owned()
        } else {
            Path::new("adh-gui").to_owned()
        }
    };
}

/// Commands that can be sent to the daemon.
pub enum DaemonCommand {
    /// Commands from the system tray icon.
    // Tray(TrayCommand),
    /// Commands from the GUI.
    GUI(GUICommand),
}

/// Create a protocol instance for communicating with the GUI.
/// In development mode, the daemon creates the socket itself.
/// Otherwise, we get the socket file descriptor from systemd.
fn get_protocol() -> Protocol {
    if is_development() {
        Protocol::new_recv().unwrap()
    } else {
        fn get_systemd_socket() -> Option<Protocol> {
            let fd = daemon::listen_fds(false).ok()?.iter().next()?;
            assert!(daemon::is_socket_unix(
                fd,
                Some(daemon::SocketType::Datagram),
                // a.d. TODO the listening check failed, why?
                daemon::Listening::NoListeningCheck,
                None::<String>
            )
            .unwrap());
            let sock = unsafe { UnixDatagram::from_raw_fd(fd) };
            Some(Protocol::new_raw(sock))
        }

        // If getting the socket from systemd failed, fall back on a manually created one.
        match get_systemd_socket() {
            Some(protocol) => protocol,
            None => {
                eprintln!("Warning: not in development mode but systemd did not give us a socket. Falling back on manually created socket.");
                Protocol::new_recv().unwrap()
            }
        }
    }
}

/// Because the socket communication is blocking we spawn a thread to push commands received
/// over the socket also into the mpsc used for communicating with the system tray icon.
/// (there was an error when trying to use the nonblocking unix socket, maybe look into that again)
fn gui_relay(tx: mpsc::Sender<DaemonCommand>) -> Result<(), anyhow::Error> {
    let protocol = get_protocol();

    loop {
        let command = protocol.recv().unwrap();
        println!("Received Command.");
        tx.send(DaemonCommand::GUI(command))?;
    }
}

fn main() -> Result<(), anyhow::Error> {
    // Create the mpsc that receives both commands from the GUI and the system tray.
    let (tx, rx) = mpsc::channel();

    // Spawn a thread for the system tray icon (gtk somehow takes control of it so it needs to be its own thread).
    // Also span a thread to listen on the socket conntected to the GUI that will relay commands from the socket to the mpsc.
    // Both threads are immediately detached because we do not join them. When the damon quits, the threads will also be killed by the OS.
    // thread::spawn({
    //     let tx = tx.clone();
    //     move || tray_icon::main(tx)
    // });
    thread::spawn(move || gui_relay(tx));

    let mut audio_stream = None;
    let mut playing = false;

    loop {
        let command = rx.recv();
        match command {
            // We cannot run the GUI as a separate thread because iced wants to be tha main thread.
            // So we spawn a new process.
            // We pass along the --dev argument (it's to ugly but the best I came up with so far)
            // Ok(DaemonCommand::Tray(TrayCommand::RunGUI)) => {
            //     println!("exec gui process");
            //     let gui_path = GUI_PROGRAM_NAME.as_path();

            //     let mut gui_command = std::process::Command::new(gui_path);
            //     if is_development() {
            //         println!("Daemon is in dev mode so start the GUI also in dev mode.");
            //         gui_command.arg("--dev");
            //     }
            //     match gui_command.spawn() {
            //         Ok(_) => {}
            //         Err(e) => eprintln!("Spawning GUI failed: {e}"),
            //     }
            // }
            // Quit command can come from both the GUI and the system tray icon.
            // Returning from the main function here will the threads we spawned.
            Ok(/*DaemonCommand::Tray(TrayCommand::Quit) | */ DaemonCommand::GUI(GUICommand::Quit)) => {
                println!("Daemon quit");
                return Ok(());
            }
            // When receiving weights from the GUI we generate some noise chunks and create
            // a new audio stream that continuously plays the chunks blending between them.
            Ok(DaemonCommand::GUI(GUICommand::SetWeights(weights))) => {
                let samples1 = adh_rs::generator::gen_weighted_noise(&weights);
                let samples2 = adh_rs::generator::gen_weighted_noise(&weights);
                let chunks = BlendingSamples::new(vec![samples1, samples2])
                    .unwrap()
                    .with_blend(BlendType::Sigmoid);

                match play(chunks) {
                    Ok(new_audio_stream) => {
                        playing = true;
                        audio_stream = Some(new_audio_stream);
                    }
                    Err(e) => eprintln!("{}", e),
                }
            }
            // Some backends support pausing playback of the audio stream so we try it here.
            Ok(DaemonCommand::GUI(GUICommand::Toggle) /* | DaemonCommand::Tray(TrayCommand::Toggle)*/) => {
                if let Some(audio_stream) = &audio_stream {
                    let res = if playing {
                        audio_stream.stream.pause().map_err(|e| anyhow!(e))
                    } else {
                        audio_stream.stream.play().map_err(|e| anyhow!(e))
                    };

                    // Only update status if toggling was successful.
                    match res {
                        Ok(()) => playing = !playing,
                        Err(e) => eprintln!("{}", e),
                    }
                }
            }
            Err(e) => {
                eprintln!("{}", e);
            }
        }
    }
}
