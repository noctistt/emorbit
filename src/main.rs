#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod platform;

#[cfg(windows)]
fn main() {
    if let Err(error) = platform::run() {
        platform::report_error(&error.to_string());
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("EmOrbit runs on Windows. Its interaction logic can be tested on this platform with cargo test.");
}
