use serde_json::json;
use std::fs::OpenOptions;
use std::io::{self, Read, Write};

use adh_rs::Config;

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();

    let lines = input.split_terminator('\n');
    let mut values = Vec::new();

    for line in lines {
        let (hz, db) = line.split_once('\t').unwrap();
        let hz = hz.parse().unwrap();
        let db = db.parse().unwrap();
        values.push(Config { hz, db });
    }

    let mut f = OpenOptions::new()
        .write(true)
        .create(true)
        .open("./export.txt")
        .unwrap();
    f.write_all(serde_json::to_string(&values).unwrap().as_bytes());
    f.flush().unwrap();
}
