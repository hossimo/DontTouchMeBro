//! Error log at `%APPDATA%\DontTouchMeBro\DontTouchMeBro.log`, plus a
//! `fatal_error.log` written if the app panics.
//!
//! Entries also go to the debugger (`OutputDebugString`, visible in DebugView).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use windows::Win32::System::Diagnostics::Debug::OutputDebugStringW;
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::core::PCWSTR;

use crate::config;
use crate::win::wide;

pub const LOG_FILE_NAME: &str = "DontTouchMeBro.log";
pub const FATAL_LOG_FILE_NAME: &str = "fatal_error.log";

/// The log is rolled over to `DontTouchMeBro.log.1` past this size.
const MAX_LOG_BYTES: u64 = 1024 * 1024;

/// Log an error. Never fails; logging problems are only sent to the debugger.
pub fn error(message: &str) {
    write_entry("ERROR", message);
}

/// Log something worth knowing that isn't an error.
pub fn info(message: &str) {
    write_entry("INFO", message);
}

/// Send a message to the debugger only (for frequent, low-value events).
pub fn debug(message: &str) {
    debug_output(&format!("{} DEBUG {message}", timestamp()));
}

fn write_entry(level: &str, message: &str) {
    let line = format!("{} {level} {message}", timestamp());
    debug_output(&line);
    // Unit tests exercise code that logs; keep them out of the real
    // %APPDATA% log.
    if cfg!(test) {
        return;
    }
    if let Some(dir) = config::default_dir()
        && let Err(err) = append(&dir, LOG_FILE_NAME, &line)
    {
        debug_output(&format!("Failed to write log: {err}"));
    }
}

/// Write `fatal_error.log` when a panic would otherwise vanish silently (the
/// release build has no console).
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        let line = format!("{} FATAL {panic}", timestamp());
        debug_output(&line);
        if let Some(dir) = config::default_dir() {
            let _ = fs::create_dir_all(&dir)
                .and_then(|()| fs::write(dir.join(FATAL_LOG_FILE_NAME), &line));
            let _ = append(&dir, LOG_FILE_NAME, &line);
        }
        default_hook(panic);
    }));
}

/// Append a line to `dir/name`, rolling the file over when it gets large.
pub fn append(dir: &Path, name: &str, line: &str) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    let path = dir.join(name);
    if fs::metadata(&path).is_ok_and(|meta| meta.len() > MAX_LOG_BYTES) {
        let rolled: PathBuf = dir.join(format!("{name}.1"));
        let _ = fs::remove_file(&rolled);
        fs::rename(&path, &rolled)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")
}

fn timestamp() -> String {
    let t = unsafe { GetLocalTime() };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
    )
}

fn debug_output(line: &str) {
    let text = wide(&format!("DontTouchMeBro: {line}\n"));
    unsafe { OutputDebugStringW(PCWSTR(text.as_ptr())) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_creates_and_appends() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("logs");
        append(&dir, "test.log", "one").unwrap();
        append(&dir, "test.log", "two").unwrap();
        let text = fs::read_to_string(dir.join("test.log")).unwrap();
        assert_eq!(text.lines().collect::<Vec<_>>(), ["one", "two"]);
    }

    #[test]
    fn append_rolls_over_large_logs() {
        let temp = tempfile::tempdir().unwrap();
        let big = "x".repeat(MAX_LOG_BYTES as usize + 1);
        fs::write(temp.path().join("test.log"), &big).unwrap();

        append(temp.path(), "test.log", "fresh").unwrap();
        assert_eq!(
            fs::read_to_string(temp.path().join("test.log")).unwrap(),
            "fresh\n"
        );
        assert_eq!(
            fs::read_to_string(temp.path().join("test.log.1")).unwrap(),
            big
        );
    }

    #[test]
    fn timestamp_has_expected_shape() {
        let ts = timestamp();
        assert_eq!(ts.len(), "2026-10-11 09:30:00.000".len());
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[10..11], " ");
    }
}
