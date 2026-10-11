//! Device state, Configuration Manager problem codes, and the text shown for
//! them in the tray tooltip.

/// Name shown in the tray tooltip and window titles.
pub const APP_TITLE: &str = "Don't Touch Me Bro";

/// Configuration Manager problem codes (`CM_PROB_*`), the same values Device
/// Manager and `Win32_PnPEntity.ConfigManagerErrorCode` report.
/// <https://learn.microsoft.com/windows-hardware/drivers/install/device-manager-error-messages>
pub mod problem {
    pub const NONE: u32 = 0;
    pub const NOT_CONFIGURED: u32 = 1;
    pub const DRIVER_CORRUPTED: u32 = 3;
    pub const CANNOT_START: u32 = 10;
    pub const RESOURCE_CONFLICT: u32 = 12;
    pub const RESTART_REQUIRED: u32 = 14;
    pub const RESOURCES_UNIDENTIFIED: u32 = 16;
    pub const REINSTALL_DRIVERS: u32 = 18;
    pub const REGISTRY_ERROR: u32 = 19;
    pub const BEING_REMOVED: u32 = 21;
    pub const DISABLED: u32 = 22;
    pub const NOT_PRESENT: u32 = 24;
    pub const DRIVERS_NOT_INSTALLED: u32 = 28;
    pub const DISABLED_BY_FIRMWARE: u32 = 29;
    pub const DRIVER_LOAD_FAILED: u32 = 31;
    pub const STOPPED_REPORTED_PROBLEM: u32 = 43;
    /// `CM_PROB_PHANTOM`: the device is known but not currently connected.
    pub const NOT_CONNECTED: u32 = 45;
}

/// Short human-readable meaning of a problem code.
pub fn describe_problem(code: u32) -> String {
    let text = match code {
        problem::NONE => "Working properly",
        problem::NOT_CONFIGURED => "Not configured correctly",
        problem::DRIVER_CORRUPTED => "Driver may be corrupted",
        problem::CANNOT_START => "Device cannot start",
        problem::RESOURCE_CONFLICT => "Resource conflict",
        problem::RESTART_REQUIRED => "Restart required",
        problem::RESOURCES_UNIDENTIFIED => "Cannot identify all resources",
        problem::REINSTALL_DRIVERS => "Drivers need reinstalling",
        problem::REGISTRY_ERROR => "Registry configuration damaged",
        problem::BEING_REMOVED => "Being removed",
        problem::DISABLED => "Disabled",
        problem::NOT_PRESENT => "Not present",
        problem::DRIVERS_NOT_INSTALLED => "Drivers not installed",
        problem::DISABLED_BY_FIRMWARE => "Disabled by firmware",
        problem::DRIVER_LOAD_FAILED => "Driver failed to load",
        problem::STOPPED_REPORTED_PROBLEM => "Stopped after reporting a problem",
        problem::NOT_CONNECTED => "Not connected",
        other => return format!("Error code {other}"),
    };
    text.to_string()
}

/// One device as reported by the Configuration Manager.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeviceInfo {
    /// Device Instance Path, e.g. `HID\ELAN2D25&COL01\5&2B77D6B&0&0000`.
    pub instance_id: String,
    pub description: String,
    pub manufacturer: String,
    /// Setup class name, e.g. `HIDClass`.
    pub class: String,
    /// Problem code; [`problem::NONE`] means enabled and working.
    pub problem: u32,
}

impl DeviceInfo {
    /// Description, falling back to the instance ID when the device has none.
    pub fn display_name(&self) -> &str {
        if self.description.is_empty() {
            &self.instance_id
        } else {
            &self.description
        }
    }
}

/// What the tray knows about the configured device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeviceState {
    /// No device ID in the config file.
    NotConfigured,
    /// A device ID is configured but no such device exists.
    NotFound,
    Device(DeviceInfo),
}

/// Which of the three tray icons to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconKind {
    Enabled,
    Disabled,
    Error,
}

impl DeviceState {
    pub fn icon(&self) -> IconKind {
        match self {
            DeviceState::Device(info) if info.problem == problem::NONE => IconKind::Enabled,
            DeviceState::Device(info) if info.problem == problem::DISABLED => IconKind::Disabled,
            _ => IconKind::Error,
        }
    }

    /// Tooltip text, truncated to fit the tray's 127-character limit.
    pub fn tooltip(&self) -> String {
        let text = match self {
            DeviceState::NotConfigured => format!("{APP_TITLE} - No device configured"),
            DeviceState::NotFound => format!("{APP_TITLE} - Device not found"),
            DeviceState::Device(info) => match info.problem {
                problem::NONE => format!("{APP_TITLE} - {} Enabled", info.display_name()),
                problem::DISABLED => format!("{APP_TITLE} - {} Disabled", info.display_name()),
                code => format!(
                    "{APP_TITLE} - {}: {}",
                    info.display_name(),
                    describe_problem(code)
                ),
            },
        };
        truncate_utf16(&text, TOOLTIP_MAX_UTF16)
    }
}

/// `NOTIFYICONDATAW.szTip` holds 128 UTF-16 units including the terminator.
pub const TOOLTIP_MAX_UTF16: usize = 127;

/// Truncate `s` to at most `max_units` UTF-16 code units without splitting a
/// surrogate pair, for copying into fixed-size Win32 string buffers.
pub fn truncate_utf16(s: &str, max_units: usize) -> String {
    let mut units = 0;
    let mut end = 0;
    for (index, ch) in s.char_indices() {
        units += ch.len_utf16();
        if units > max_units {
            return s[..index].to_string();
        }
        end = index + ch.len_utf8();
    }
    s[..end].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(problem: u32) -> DeviceInfo {
        DeviceInfo {
            instance_id: r"HID\ELAN2D25&COL01\5&2B77D6B&0&0000".into(),
            description: "HID-compliant touch screen".into(),
            manufacturer: "Microsoft".into(),
            class: "HIDClass".into(),
            problem,
        }
    }

    #[test]
    fn icon_follows_problem_code() {
        assert_eq!(
            DeviceState::Device(device(problem::NONE)).icon(),
            IconKind::Enabled
        );
        assert_eq!(
            DeviceState::Device(device(problem::DISABLED)).icon(),
            IconKind::Disabled
        );
        assert_eq!(
            DeviceState::Device(device(problem::RESTART_REQUIRED)).icon(),
            IconKind::Error
        );
        assert_eq!(DeviceState::NotFound.icon(), IconKind::Error);
        assert_eq!(DeviceState::NotConfigured.icon(), IconKind::Error);
    }

    #[test]
    fn tooltip_text() {
        assert_eq!(
            DeviceState::Device(device(problem::NONE)).tooltip(),
            "Don't Touch Me Bro - HID-compliant touch screen Enabled"
        );
        assert_eq!(
            DeviceState::Device(device(problem::DISABLED)).tooltip(),
            "Don't Touch Me Bro - HID-compliant touch screen Disabled"
        );
        assert_eq!(
            DeviceState::Device(device(problem::NOT_CONNECTED)).tooltip(),
            "Don't Touch Me Bro - HID-compliant touch screen: Not connected"
        );
        assert_eq!(
            DeviceState::NotFound.tooltip(),
            "Don't Touch Me Bro - Device not found"
        );
        assert_eq!(
            DeviceState::NotConfigured.tooltip(),
            "Don't Touch Me Bro - No device configured"
        );
    }

    #[test]
    fn tooltip_falls_back_to_instance_id() {
        let mut info = device(problem::NONE);
        info.description.clear();
        assert!(
            DeviceState::Device(info)
                .tooltip()
                .contains(r"HID\ELAN2D25")
        );
    }

    #[test]
    fn tooltip_is_truncated() {
        let mut info = device(problem::NONE);
        info.description = "x".repeat(300);
        let tip = DeviceState::Device(info).tooltip();
        assert_eq!(tip.encode_utf16().count(), TOOLTIP_MAX_UTF16);
    }

    #[test]
    fn describe_known_and_unknown_codes() {
        assert_eq!(describe_problem(problem::DISABLED), "Disabled");
        assert_eq!(
            describe_problem(problem::RESOURCE_CONFLICT),
            "Resource conflict"
        );
        assert_eq!(
            describe_problem(problem::RESTART_REQUIRED),
            "Restart required"
        );
        assert_eq!(describe_problem(problem::NOT_CONNECTED), "Not connected");
        assert_eq!(describe_problem(99), "Error code 99");
    }

    #[test]
    fn truncate_never_splits_surrogate_pairs() {
        // Each emoji is two UTF-16 units.
        let s = "ab😀😀";
        assert_eq!(truncate_utf16(s, 10), s);
        assert_eq!(truncate_utf16(s, 4), "ab😀");
        assert_eq!(truncate_utf16(s, 3), "ab");
        assert_eq!(truncate_utf16(s, 0), "");
    }
}
