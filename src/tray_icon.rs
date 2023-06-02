use lazy_static::lazy_static;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use gtk::prelude::*;
use libappindicator::{AppIndicator, AppIndicatorStatus};

pub enum TrayCommand {
    Quit,
    RunGUI,
}

lazy_static! {
    static ref ICON_PATH: PathBuf = {
        if cfg!(debug_assertions) {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("resources")
                .to_owned()
        } else {
            Path::new("/usr/share/adh-rs/").join("resources").to_owned()
        }
    };
}

pub fn main(tx: mpsc::Sender<TrayCommand>) {
    gtk::init().unwrap();

    let mut indicator = AppIndicator::new("Adh-rs system tray icon", "");
    indicator.set_status(AppIndicatorStatus::Active);
    let icon_path = ICON_PATH.as_path();
    indicator.set_icon_theme_path(icon_path.to_str().unwrap());
    indicator.set_icon_full("tray-icon", "icon");
    let mut m = gtk::Menu::new();

    let quit_entry = gtk::MenuItem::with_label("Quit");
    quit_entry.connect_activate({
        let tx = tx.clone();
        move |_| {
            gtk::main_quit();
            tx.send(TrayCommand::Quit).ok();
        }
    });
    m.append(&quit_entry);

    let gui_entry = gtk::MenuItem::with_label("Run GUI");
    gui_entry.connect_activate({
        let tx = tx.clone();
        move |_| {
            tx.send(TrayCommand::RunGUI).ok();
        }
    });
    m.append(&gui_entry);

    indicator.set_menu(&mut m);
    m.show_all();

    println!("Tray Icon: start");
    gtk::main();
}
