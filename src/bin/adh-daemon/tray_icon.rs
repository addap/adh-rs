// use adh_rs::is_development;
// use gtk::prelude::*;
// use lazy_static::lazy_static;
// use libappindicator::{AppIndicator, AppIndicatorStatus};
// use std::env;
// use std::path::{Path, PathBuf};
// use std::sync::mpsc;

// use crate::DaemonCommand;

// lazy_static! {
//     /// Which icon to use for the sytem tray icon.
//     /// During development we use the the resource/ directory in the project's root.
//     /// Otherwise we use a resource/ directory installed on the system.
//     /// a.d. TODO how do I put the png into that directory during `cargo install`?
//     static ref ICON_PATH: PathBuf = {
//         if is_development() {
//             Path::new(env!("CARGO_MANIFEST_DIR"))
//                 .join("resources")
//                 .to_owned()
//         } else {
//             let home = std::env::var("HOME").expect("$HOME is unset");
//             Path::new(&home).join(".local/share/adh-rs/resources").to_owned()
//         }
//     };
// }

// /// Commands sent from the system tray icon.
// pub enum TrayCommand {
//     /// Run a new GUI instance.
//     RunGUI,
//     /// Toggle audio playback.
//     Toggle,
//     /// Quit the daemon.
//     Quit,
// }

// pub fn main(tx: mpsc::Sender<DaemonCommand>) {
//     // The tray icon uses GTK so we initialize it here.
//     // At the end of this function we give control over the thread to GTK.
//     gtk::init().unwrap();

//     // Configure the indicator widget and which icon it uses.
//     let mut indicator = AppIndicator::new("Adh-rs system tray icon", "");
//     indicator.set_status(AppIndicatorStatus::Active);
//     let icon_path = ICON_PATH.as_path();
//     indicator.set_icon_theme_path(icon_path.to_str().unwrap());
//     indicator.set_icon_full("tray-icon", "icon");

//     // Configure the menu entries when clicking on the icon.
//     let mut m = gtk::Menu::new();

//     let gui_entry = gtk::MenuItem::with_label("Run GUI");
//     gui_entry.connect_activate({
//         let tx = tx.clone();
//         move |_| {
//             tx.send(DaemonCommand::Tray(TrayCommand::RunGUI)).ok();
//         }
//     });
//     m.append(&gui_entry);

//     let toggle_entry = gtk::MenuItem::with_label("Toggle Playback");
//     toggle_entry.connect_activate({
//         let tx = tx.clone();
//         move |_| {
//             tx.send(DaemonCommand::Tray(TrayCommand::Toggle)).ok();
//         }
//     });
//     m.append(&toggle_entry);

//     let quit_entry = gtk::MenuItem::with_label("Quit");
//     quit_entry.connect_activate({
//         let tx = tx.clone();
//         move |_| {
//             gtk::main_quit();
//             tx.send(DaemonCommand::Tray(TrayCommand::Quit)).ok();
//         }
//     });
//     m.append(&quit_entry);

//     indicator.set_menu(&mut m);
//     m.show_all();

//     println!("Tray Icon: start");
//     // Now we give control to GTK.
//     gtk::main();
// }
