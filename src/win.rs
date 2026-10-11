//! Small Win32 helpers shared by the tray and the Configure window.

use std::path::PathBuf;

use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MB_SETFOREGROUND, MessageBoxW};
use windows::core::PCWSTR;

/// NUL-terminated UTF-16 for passing to Win32.
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Copy `s` into a fixed-size, NUL-terminated UTF-16 buffer, truncating
/// without splitting a surrogate pair.
pub fn copy_to_buffer(buffer: &mut [u16], s: &str) {
    let text = crate::status::truncate_utf16(s, buffer.len().saturating_sub(1));
    let units: Vec<u16> = text.encode_utf16().collect();
    buffer[..units.len()].copy_from_slice(&units);
    buffer[units.len()..].fill(0);
}

/// Show a modal error message box with no owner window.
pub fn error_box(title: &str, text: &str) {
    let title = wide(title);
    let text = wide(text);
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(text.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_SETFOREGROUND | MB_ICONERROR,
        );
    }
}

/// Directory containing the running executable.
pub fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe().ok()?.parent().map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_is_nul_terminated() {
        assert_eq!(wide("ab"), vec![b'a' as u16, b'b' as u16, 0]);
    }

    #[test]
    fn copy_to_buffer_truncates_and_terminates() {
        let mut buffer = [0xFFFFu16; 4];
        copy_to_buffer(&mut buffer, "abcdef");
        assert_eq!(buffer, [b'a' as u16, b'b' as u16, b'c' as u16, 0]);

        copy_to_buffer(&mut buffer, "x");
        assert_eq!(buffer, [b'x' as u16, 0, 0, 0]);
    }
}
