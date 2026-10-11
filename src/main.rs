// No console window in release builds; debug builds keep one for output.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use donttouchmebro::{CONFIGURE_ARG, configure, logging, tray};

fn main() {
    logging::install_panic_hook();

    let configure_mode = std::env::args()
        .skip(1)
        .any(|arg| arg.eq_ignore_ascii_case(CONFIGURE_ARG));
    let result = if configure_mode {
        configure::run()
    } else {
        tray::run()
    };

    if let Err(message) = result {
        logging::error(&message);
        donttouchmebro::show_error("Don't Touch Me Bro", &message);
        std::process::exit(1);
    }
}
