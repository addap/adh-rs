use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{
    env,
    path::{Path, PathBuf},
};

pub mod audio_bridge;
pub mod generator;
pub mod protocol;
pub mod samples;
pub mod slots;

pub const WEIGHTS_NUM: usize = 32;
pub const SEGMENTS_WEIGHT_MAX: f32 = 1.0;

lazy_static! {
    /// For development, we use a socket in tmp/.
    /// Otherwise we use a socket located in $XDG_RUNTIME_DIR.
    static ref SOCKET_PATH: PathBuf = {
        if is_development() {
            Path::new("/tmp/adh-rs.sock").to_owned()
        } else {
            let xdg_runtime_dir = env::var("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR is unset");
            Path::new(&xdg_runtime_dir).join("adh-rs.sock").to_owned()
        }
    };
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Weights {
    pub v: [f32; WEIGHTS_NUM],
}

impl Default for Weights {
    fn default() -> Self {
        Self {
            v: [SEGMENTS_WEIGHT_MAX; WEIGHTS_NUM],
        }
    }
}

/// Function to check if the program is running in development mode.
/// Earlier we used cfg!(debug_assertions) but that's not great if we
/// want to locally test the release versions. So we use a separate command
/// line argument.
pub fn is_development() -> bool {
    // a.d. TODO better command line argument handling
    if let Some(arg) = std::env::args().nth(1) {
        if &arg == "--dev" {
            true
        } else {
            panic!("Unknown command line argument: {}", arg);
        }
    } else {
        false
    }
}
