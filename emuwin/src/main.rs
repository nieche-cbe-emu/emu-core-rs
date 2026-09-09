
#![windows_subsystem = "windows"]

#[cfg(not(windows))]
fn main() {
    eprintln!("emuwin 只在 Windows 上构建");
}

#[cfg(windows)]
mod app;

#[cfg(all(windows, feature = "core"))]
use emucore::session;
#[cfg(all(windows, not(feature = "core")))]
mod session_mock;
#[cfg(all(windows, not(feature = "core")))]
use session_mock as session;

#[cfg(windows)]
fn main() {
    app::run();
}
