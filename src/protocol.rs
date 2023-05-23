use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, read},
    os::unix::net::UnixDatagram,
    path::Path,
};

use crate::{Weights, SOCKET_PATH};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Command {
    SetWeights(Weights),
    Toggle,
    Quit,
}

#[derive(Debug)]
pub struct Protocol {
    sock: UnixDatagram,
}

impl Protocol {
    pub fn new_recv() -> Result<Self, anyhow::Error> {
        let socket_path = Path::new(SOCKET_PATH);
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
        sock.connect(SOCKET_PATH).unwrap();
        Ok(Protocol { sock })
    }

    pub fn send(&self, message: &Command) -> Result<(), anyhow::Error> {
        let serialized_command = bincode::serialize(message)?;

        let sent_bytes = self.sock.send(&serialized_command)?;
        if sent_bytes != serialized_command.len() {
            return Err(anyhow!("Socket send"));
        }
        Ok(())
    }

    pub fn recv(&self) -> Result<Command, anyhow::Error> {
        let mut buf = vec![0; 1024];
        let read_bytes = self.sock.recv(&mut buf)?;

        let command: Command = bincode::deserialize(&buf[..read_bytes])?;
        Ok(command)
    }
}
