pub mod audio_bridge;
pub mod chunk;
pub mod generator;
pub mod misc;

use serde::{Deserialize, Serialize};

pub const WEIGHTS_NUM: usize = 32;
pub const SEGMENTS_WEIGHT_MAX: f32 = 1.0;

#[derive(Debug, Clone, Copy)]
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
