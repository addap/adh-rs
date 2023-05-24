use serde::{Deserialize, Serialize};

pub mod audio_bridge;
pub mod chunk;
pub mod generator;
pub mod misc;
pub mod protocol;
pub mod slots;

pub const WEIGHTS_NUM: usize = 32;
pub const SEGMENTS_WEIGHT_MAX: f32 = 1.0;
pub static SOCKET_PATH: &'static str = "/tmp/adh-rs.socket";

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
