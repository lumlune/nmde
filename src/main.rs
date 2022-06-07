#![allow(mutable_borrow_reservation_conflict)]
#![allow(unused)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod io;
mod ui;
mod utils;

use ui::NmdApp;

fn main() {
    NmdApp::run();
}
