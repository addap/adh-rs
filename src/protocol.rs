use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::{fs, os::unix::net::UnixDatagram};

use crate::{Weights, SOCKET_PATH};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum GUICommand {
    SetWeights(Weights),
    Toggle,
    Quit,
}

const GUI_COMMAND_BUF_LEN: usize = 1024;

#[derive(Debug)]
pub struct Protocol {
    sock: UnixDatagram,
}

impl Protocol {
    pub fn new_raw(sock: UnixDatagram) -> Self {
        Self { sock }
    }

    pub fn new_recv() -> Result<Self, anyhow::Error> {
        let socket_path = SOCKET_PATH.as_path();
        if socket_path.exists() {
            fs::remove_file(socket_path).unwrap();
        }

        let sock = match UnixDatagram::bind(socket_path) {
            Ok(sock) => sock,
            Err(e) => {
                println!("Couldn't bind: {e:?}");
                return Err(e.into());
            }
        };

        Ok(Self { sock })
    }

    pub fn new_send() -> Result<Self, anyhow::Error> {
        let sock = UnixDatagram::unbound().unwrap();
        sock.connect(SOCKET_PATH.as_path()).unwrap();
        Ok(Protocol { sock })
    }

    pub fn send(&self, message: &GUICommand) -> Result<(), anyhow::Error> {
        let serialized_command = bincode::serde::encode_to_vec(message, bincode::config::standard())?;
        if serialized_command.len() > GUI_COMMAND_BUF_LEN {
            return Err(anyhow!("Gui Command too big to encode. Increase buffer size."));
        }

        let sent_bytes = self.sock.send(&serialized_command)?;
        if sent_bytes != serialized_command.len() {
            return Err(anyhow!("Socket send"));
        }
        Ok(())
    }

    pub fn recv(&self) -> Result<GUICommand, anyhow::Error> {
        let mut buf = vec![0; GUI_COMMAND_BUF_LEN];
        let read_bytes = self.sock.recv(&mut buf)?;

        let (command, _): (GUICommand, usize) =
            bincode::serde::decode_from_slice(&buf[..read_bytes], bincode::config::standard())?;
        Ok(command)
    }
}
