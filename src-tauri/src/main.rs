//! Binary entry point — delegates to the library.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    captains_call_2027_lib::run();
}
