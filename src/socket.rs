use std::{fs, os::unix::net::UnixDatagram, path::Path};

use crate::SOCKET_PATH;

pub fn get_socket() -> Result<UnixDatagram, anyhow::Error> {
    let socket_path = Path::new(SOCKET_PATH);
    if socket_path.exists() {
        fs::remove_file(socket_path).unwrap();
    }

    match UnixDatagram::bind(socket_path) {
        Ok(sock) => Ok(sock),
        Err(e) => {
            println!("Couldn't bind: {e:?}");
            Err(e.into())
        }
    }
}

pub fn get_send() -> Result<UnixDatagram, anyhow::Error> {
    let sock = UnixDatagram::unbound().unwrap();
    sock.connect(SOCKET_PATH).unwrap();
    Ok(sock)
}
