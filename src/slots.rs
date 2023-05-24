//! Slots to save weights into.
//! In the GUI these slots are filled with Ctrl+0..9 and can be recalled by just pressing the number 0..9 again.
//!
//! Starting the GUI loads the config file from disk.
//! Exiting the GUI writes if back.

use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
};

use crate::Weights;

const SLOTS_NUM: usize = 10;
const DEFAULT_SLOTS_FILE: &str = "/tmp/slots";

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

    fn disk_path() -> String {
        match std::env::var("SLOTS_FILE") {
            Ok(path) => path,
            Err(_) => DEFAULT_SLOTS_FILE.to_owned(),
        }
    }

    pub fn write_to_disk(&self) {
        fn inner(slots: &Slots) -> Result<(), anyhow::Error> {
            let buf = serde_json::to_vec(&slots)?;
            let path = Slots::disk_path();
            let mut f = OpenOptions::new().write(true).create(true).open(&path)?;
            f.write_all(&buf)?;
            f.flush()?;
            Ok(())
        }

        inner(self).unwrap()
    }

    pub fn load_from_disk() -> Self {
        fn inner() -> Result<Slots, anyhow::Error> {
            let path = Slots::disk_path();
            let mut f = OpenOptions::new().read(true).open(&path)?;
            let mut buf = Vec::new();
            f.read_to_end(&mut buf)?;
            let slots = serde_json::from_slice(&buf)?;
            Ok(slots)
        }

        match inner() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{}", e);
                Slots::default()
            }
        }
    }
}
