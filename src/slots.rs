//! Slots to save weights into.
//! In the GUI these slots are filled with Ctrl+0..9 and can be recalled by just pressing the number 0..9 again.
//!
//! Starting the GUI loads the config file from disk.
//! Exiting the GUI writes if back.

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Write},
};
use xdg::BaseDirectories;

use crate::Weights;

const SLOTS_NUM: usize = 10;
const SLOTS_FILENAME: &str = "slots.txt";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Slots {
    slots: [Weights; SLOTS_NUM],
}

impl Slots {
    pub fn save_slot(&mut self, idx: usize, weights: Weights) {
        self.slots.get_mut(idx).map(|w| *w = weights);
    }

    pub fn recall_slot(&self, idx: usize) -> Weights {
        self.slots.get(idx).cloned().unwrap_or_default()
    }

    pub fn write_to_disk(&self, xdg_dirs: &BaseDirectories) {
        let inner = || -> Result<(), anyhow::Error> {
            let buf = serde_json::to_vec(&self)?;
            let path = xdg_dirs.place_config_file(SLOTS_FILENAME)?;
            let mut f = File::create(path)?;
            f.write_all(&buf)?;
            f.flush()?;
            Ok(())
        };

        inner().unwrap()
    }

    pub fn load_from_disk(xdg_dirs: &BaseDirectories) -> Self {
        let inner = || -> Result<Slots, anyhow::Error> {
            let path = xdg_dirs
                .find_config_file(SLOTS_FILENAME)
                .ok_or(anyhow!("Slots config file not found."))?;
            let mut f = File::open(path)?;
            let mut buf = Vec::new();
            f.read_to_end(&mut buf)?;
            let slots = serde_json::from_slice(&buf)?;
            Ok(slots)
        };

        match inner() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{}", e);
                Slots::default()
            }
        }
    }
}
