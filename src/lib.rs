pub mod generator;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Config {
    pub hz: f64,
    pub db: f64,
}
