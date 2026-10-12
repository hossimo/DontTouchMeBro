//! DontTouchMeBro: a Windows tray utility that enables and disables a single
//! device (originally a laptop touch screen) by its Device Instance Path.
//!
//! The tray process ([`tray`]) owns the notification icon and toggles the
//! device on click. The Configure window ([`configure`]) is an egui app that
//! runs as a separate `--configure` process and writes the chosen device to
//! the config file ([`config`]); the tray reloads it when that process exits.

#[cfg(not(windows))]
compile_error!("DontTouchMeBro only supports Windows.");

pub mod config;
pub mod configure;
pub mod device;
pub mod listing;
pub mod logging;
pub mod naming;
pub mod status;
pub mod toggle;
pub mod tray;
mod win;

/// Command-line switch that runs the Configure window instead of the tray.
pub const CONFIGURE_ARG: &str = "--configure";

/// Show an error message box (used by `main` for startup failures).
pub fn show_error(title: &str, message: &str) {
    win::error_box(title, message);
}
