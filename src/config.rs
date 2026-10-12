//! The per-user config file holding the Device Instance Path to control.
//!
//! It lives at `%APPDATA%\DontTouchMeBro\device-id.txt`, so it survives
//! reinstalls and is found regardless of the working directory. The format is
//! one line of text; earlier versions wrote the same file, so it carries over.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const APP_DIR_NAME: &str = "DontTouchMeBro";
pub const CONFIG_FILE_NAME: &str = "device-id.txt";

/// `%APPDATA%\DontTouchMeBro`, or `None` if `APPDATA` isn't set.
pub fn default_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|appdata| PathBuf::from(appdata).join(APP_DIR_NAME))
}

/// The config file inside a given directory.
#[derive(Clone, Debug)]
pub struct Config {
    dir: PathBuf,
}

impl Config {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Config { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn path(&self) -> PathBuf {
        self.dir.join(CONFIG_FILE_NAME)
    }

    /// The configured device ID, or `None` if the file is missing or blank.
    /// Other I/O errors (permissions, locked file) are returned so the caller
    /// can report them instead of claiming nothing is configured.
    pub fn read(&self) -> io::Result<Option<String>> {
        match fs::read_to_string(self.path()) {
            Ok(text) => Ok(parse(&text)),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Save `device_id`, creating the directory if needed. Writes to a
    /// temporary file first so a failed write never leaves a truncated config.
    pub fn write(&self, device_id: &str) -> io::Result<()> {
        let device_id = device_id.trim();
        if device_id.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "the device ID is empty",
            ));
        }

        fs::create_dir_all(&self.dir)?;
        let temp = self.dir.join(format!("{CONFIG_FILE_NAME}.tmp"));
        fs::write(&temp, device_id)?;
        fs::rename(&temp, self.path()).inspect_err(|_| {
            let _ = fs::remove_file(&temp);
        })
    }

    /// One-time migration: very old versions kept `device-id.txt` next to the
    /// executable. If there's no config yet but `legacy_dir` has one, copy it.
    /// Returns whether a file was copied.
    pub fn migrate_legacy(&self, legacy_dir: &Path) -> io::Result<bool> {
        if self.path().exists() {
            return Ok(false);
        }
        let legacy = legacy_dir.join(CONFIG_FILE_NAME);
        if !legacy.is_file() {
            return Ok(false);
        }
        fs::create_dir_all(&self.dir)?;
        fs::copy(&legacy, self.path())?;
        Ok(true)
    }
}

/// Trim whitespace and a UTF-8 byte-order mark (Notepad may add one).
fn parse(text: &str) -> Option<String> {
    let id = text.trim_start_matches('\u{feff}').trim();
    (!id.is_empty()).then(|| id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = r"HID\ELAN2D25&COL01\5&2B77D6B&0&0000";

    fn temp_config() -> (tempfile::TempDir, Config) {
        let temp = tempfile::tempdir().unwrap();
        let config = Config::new(temp.path().join(APP_DIR_NAME));
        (temp, config)
    }

    #[test]
    fn missing_file_is_not_configured() {
        let (_temp, config) = temp_config();
        assert_eq!(config.read().unwrap(), None);
    }

    #[test]
    fn write_then_read_round_trips() {
        let (_temp, config) = temp_config();
        config.write(ID).unwrap();
        assert_eq!(config.read().unwrap().as_deref(), Some(ID));
        assert_eq!(fs::read_to_string(config.path()).unwrap(), ID);
        assert!(!config.dir().join("device-id.txt.tmp").exists());
    }

    #[test]
    fn write_replaces_existing_value() {
        let (_temp, config) = temp_config();
        config.write("first").unwrap();
        config.write(&format!("  {ID}\n")).unwrap();
        assert_eq!(config.read().unwrap().as_deref(), Some(ID));
    }

    #[test]
    fn blank_file_is_not_configured() {
        let (_temp, config) = temp_config();
        fs::create_dir_all(config.dir()).unwrap();
        fs::write(config.path(), " \r\n").unwrap();
        assert_eq!(config.read().unwrap(), None);
    }

    #[test]
    fn read_trims_whitespace_and_bom() {
        let (_temp, config) = temp_config();
        fs::create_dir_all(config.dir()).unwrap();
        fs::write(config.path(), format!("\u{feff}  {ID}\r\n")).unwrap();
        assert_eq!(config.read().unwrap().as_deref(), Some(ID));
    }

    #[test]
    fn empty_id_is_rejected() {
        let (_temp, config) = temp_config();
        let err = config.write("   ").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(!config.path().exists());
    }

    #[test]
    fn unreadable_config_is_an_error_not_unconfigured() {
        let (_temp, config) = temp_config();
        // A directory where the file should be can't be read as a file.
        fs::create_dir_all(config.path()).unwrap();
        assert!(config.read().is_err());
    }

    #[test]
    fn migrates_legacy_file_once() {
        let (temp, config) = temp_config();
        let legacy_dir = temp.path().join("exe");
        fs::create_dir_all(&legacy_dir).unwrap();
        fs::write(legacy_dir.join(CONFIG_FILE_NAME), ID).unwrap();

        assert!(config.migrate_legacy(&legacy_dir).unwrap());
        assert_eq!(config.read().unwrap().as_deref(), Some(ID));

        // An existing config is never overwritten.
        fs::write(legacy_dir.join(CONFIG_FILE_NAME), "other").unwrap();
        assert!(!config.migrate_legacy(&legacy_dir).unwrap());
        assert_eq!(config.read().unwrap().as_deref(), Some(ID));
    }

    #[test]
    fn migrate_without_legacy_file_does_nothing() {
        let (temp, config) = temp_config();
        assert!(!config.migrate_legacy(temp.path()).unwrap());
        assert!(!config.path().exists());
    }
}
