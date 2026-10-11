//! Deciding what a tray click does, independent of the Windows device APIs so
//! it can be tested against a fake backend.

use std::fmt;

use crate::status::{DeviceInfo, DeviceState, describe_problem, problem};

/// Access to devices. Implemented for real by [`crate::device::SetupApi`].
pub trait DeviceBackend {
    /// Look up a device by instance ID (case-insensitive). `Ok(None)` if no
    /// such device exists.
    fn find(&self, instance_id: &str) -> Result<Option<DeviceInfo>, DeviceError>;

    /// Enable or disable a device.
    fn set_enabled(&self, instance_id: &str, enable: bool) -> Result<ChangeOutcome, DeviceError>;
}

/// Result of a successful enable/disable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChangeOutcome {
    /// Windows needs a restart before the change takes effect.
    pub restart_required: bool,
}

/// A failed device operation. `reason` is short enough to show the user;
/// the full message (for the log) adds what was being done.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceError {
    /// What was being attempted, e.g. `Disabling HID\...`; may be empty.
    pub context: String,
    /// Why it failed, e.g. `This device can't be disabled.`
    pub reason: String,
}

impl DeviceError {
    pub fn new(reason: impl Into<String>) -> Self {
        DeviceError {
            context: String::new(),
            reason: reason.into(),
        }
    }

    pub fn with_context(context: impl Into<String>, reason: impl Into<String>) -> Self {
        DeviceError {
            context: context.into(),
            reason: reason.into(),
        }
    }
}

impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.context.is_empty() {
            f.write_str(&self.reason)
        } else {
            write!(f, "{}: {}", self.context, self.reason)
        }
    }
}

impl std::error::Error for DeviceError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Enable,
    Disable,
}

impl Action {
    pub fn verb(self) -> &'static str {
        match self {
            Action::Enable => "enable",
            Action::Disable => "disable",
        }
    }
}

/// What clicking would do for a device in this state. Only a working or a
/// disabled device can be toggled; anything else (not connected, driver
/// problems, pending restart) is reported instead.
pub fn action_for(info: &DeviceInfo) -> Option<Action> {
    match info.problem {
        problem::NONE => Some(Action::Disable),
        problem::DISABLED => Some(Action::Enable),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeKind {
    Info,
    Warning,
    Error,
}

/// A tray balloon notification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    pub kind: NoticeKind,
    pub title: String,
    pub text: String,
}

impl Notice {
    fn new(kind: NoticeKind, title: impl Into<String>, text: impl Into<String>) -> Self {
        Notice {
            kind,
            title: title.into(),
            text: text.into(),
        }
    }
}

/// Everything the tray needs to do after a click.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ToggleReport {
    /// New state to display; `None` if it couldn't be determined.
    pub state: Option<DeviceState>,
    /// Balloon to show, if any.
    pub notice: Option<Notice>,
    /// Error to write to the log, if any.
    pub log: Option<String>,
}

/// Current state of the configured device.
pub fn query_state(
    backend: &dyn DeviceBackend,
    configured: Option<&str>,
) -> Result<DeviceState, DeviceError> {
    let Some(id) = configured.map(str::trim).filter(|id| !id.is_empty()) else {
        return Ok(DeviceState::NotConfigured);
    };
    Ok(match backend.find(id)? {
        Some(info) => DeviceState::Device(info),
        None => DeviceState::NotFound,
    })
}

/// Toggle the configured device based on its real current state (not the
/// icon, which may be stale) and describe the outcome.
pub fn toggle(backend: &dyn DeviceBackend, configured: Option<&str>) -> ToggleReport {
    let current = match query_state(backend, configured) {
        Ok(state) => state,
        Err(err) => {
            return ToggleReport {
                state: None,
                notice: Some(Notice::new(
                    NoticeKind::Error,
                    "Couldn't toggle device",
                    err.reason.clone(),
                )),
                log: Some(format!(
                    "Querying device {}: {err}",
                    configured.unwrap_or("")
                )),
            };
        }
    };

    let info = match &current {
        DeviceState::NotConfigured => {
            return ToggleReport {
                state: Some(current),
                notice: Some(Notice::new(
                    NoticeKind::Info,
                    "No device configured",
                    "Right-click the tray icon and choose Configure to pick a device.",
                )),
                log: None,
            };
        }
        DeviceState::NotFound => {
            let id = configured.unwrap_or_default().trim();
            return ToggleReport {
                state: Some(current.clone()),
                notice: Some(Notice::new(
                    NoticeKind::Warning,
                    "Can't toggle device",
                    format!("Device not found:\n{id}"),
                )),
                log: None,
            };
        }
        DeviceState::Device(info) => info.clone(),
    };

    let Some(action) = action_for(&info) else {
        return ToggleReport {
            notice: Some(Notice::new(
                NoticeKind::Warning,
                "Can't toggle device",
                format!(
                    "{}: {}",
                    info.display_name(),
                    describe_problem(info.problem)
                ),
            )),
            state: Some(current),
            log: None,
        };
    };

    let outcome = backend.set_enabled(&info.instance_id, action == Action::Enable);

    // Re-read the device either way so the icon shows what really happened.
    let state = query_state(backend, Some(&info.instance_id)).ok();

    match outcome {
        Err(err) => ToggleReport {
            notice: Some(Notice::new(
                NoticeKind::Error,
                format!("Failed to {} device", action.verb()),
                format!("{}: {}", info.display_name(), err.reason),
            )),
            log: Some(format!(
                "Failed to {} {}: {err}",
                action.verb(),
                info.instance_id
            )),
            state,
        },
        Ok(outcome) => {
            let pending_restart = outcome.restart_required
                || matches!(&state, Some(DeviceState::Device(d)) if d.problem == problem::RESTART_REQUIRED);
            ToggleReport {
                notice: pending_restart.then(|| {
                    Notice::new(
                        NoticeKind::Warning,
                        "Restart required",
                        format!(
                            "Windows needs to restart to {} {}.",
                            action.verb(),
                            info.display_name()
                        ),
                    )
                }),
                state,
                log: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    const ID: &str = r"HID\ELAN2D25&COL01\5&2B77D6B&0&0000";

    /// In-memory backend: one device whose problem code flips on set_enabled.
    #[derive(Default)]
    struct FakeBackend {
        device: RefCell<Option<DeviceInfo>>,
        find_error: Option<DeviceError>,
        set_error: Option<DeviceError>,
        restart_required: bool,
        calls: RefCell<Vec<(String, bool)>>,
    }

    impl FakeBackend {
        fn with_problem(problem: u32) -> Self {
            FakeBackend {
                device: RefCell::new(Some(DeviceInfo {
                    instance_id: ID.into(),
                    description: "Touch screen".into(),
                    manufacturer: "ELAN".into(),
                    class: "HIDClass".into(),
                    problem,
                    can_disable: true,
                })),
                ..Default::default()
            }
        }

        fn problem(&self) -> u32 {
            self.device.borrow().as_ref().unwrap().problem
        }
    }

    impl DeviceBackend for FakeBackend {
        fn find(&self, instance_id: &str) -> Result<Option<DeviceInfo>, DeviceError> {
            if let Some(err) = &self.find_error {
                return Err(err.clone());
            }
            Ok(self
                .device
                .borrow()
                .clone()
                .filter(|d| d.instance_id.eq_ignore_ascii_case(instance_id)))
        }

        fn set_enabled(
            &self,
            instance_id: &str,
            enable: bool,
        ) -> Result<ChangeOutcome, DeviceError> {
            self.calls
                .borrow_mut()
                .push((instance_id.to_string(), enable));
            if let Some(err) = &self.set_error {
                return Err(err.clone());
            }
            if let Some(device) = self.device.borrow_mut().as_mut() {
                device.problem = if enable {
                    problem::NONE
                } else {
                    problem::DISABLED
                };
            }
            Ok(ChangeOutcome {
                restart_required: self.restart_required,
            })
        }
    }

    #[test]
    fn action_depends_on_problem_code() {
        let mut info = DeviceInfo::default();
        assert_eq!(action_for(&info), Some(Action::Disable));
        info.problem = problem::DISABLED;
        assert_eq!(action_for(&info), Some(Action::Enable));
        info.problem = problem::NOT_CONNECTED;
        assert_eq!(action_for(&info), None);
        info.problem = problem::RESTART_REQUIRED;
        assert_eq!(action_for(&info), None);
    }

    #[test]
    fn enabled_device_gets_disabled() {
        let backend = FakeBackend::with_problem(problem::NONE);
        let report = toggle(&backend, Some(ID));
        assert_eq!(*backend.calls.borrow(), vec![(ID.to_string(), false)]);
        assert_eq!(backend.problem(), problem::DISABLED);
        assert!(
            matches!(report.state, Some(DeviceState::Device(ref d)) if d.problem == problem::DISABLED)
        );
        assert_eq!(report.notice, None);
        assert_eq!(report.log, None);
    }

    #[test]
    fn disabled_device_gets_enabled() {
        let backend = FakeBackend::with_problem(problem::DISABLED);
        let report = toggle(&backend, Some(ID));
        assert_eq!(*backend.calls.borrow(), vec![(ID.to_string(), true)]);
        assert!(
            matches!(report.state, Some(DeviceState::Device(ref d)) if d.problem == problem::NONE)
        );
    }

    #[test]
    fn lookup_is_case_insensitive_and_trimmed() {
        let backend = FakeBackend::with_problem(problem::NONE);
        let lower = format!("  {}  ", ID.to_lowercase());
        toggle(&backend, Some(&lower));
        assert_eq!(backend.calls.borrow().len(), 1);
    }

    #[test]
    fn not_configured_never_calls_the_backend() {
        let backend = FakeBackend::with_problem(problem::NONE);
        for configured in [None, Some(""), Some("   ")] {
            let report = toggle(&backend, configured);
            assert_eq!(report.state, Some(DeviceState::NotConfigured));
            assert_eq!(report.notice.unwrap().kind, NoticeKind::Info);
        }
        assert!(backend.calls.borrow().is_empty());
    }

    #[test]
    fn missing_device_is_reported_not_toggled() {
        let backend = FakeBackend::default();
        let report = toggle(&backend, Some(ID));
        assert_eq!(report.state, Some(DeviceState::NotFound));
        let notice = report.notice.unwrap();
        assert_eq!(notice.kind, NoticeKind::Warning);
        assert!(notice.text.contains(ID));
        assert!(backend.calls.borrow().is_empty());
    }

    #[test]
    fn device_in_error_state_is_reported_not_toggled() {
        let backend = FakeBackend::with_problem(problem::NOT_CONNECTED);
        let report = toggle(&backend, Some(ID));
        assert!(backend.calls.borrow().is_empty());
        assert_eq!(report.notice.unwrap().text, "Touch screen: Not connected");
    }

    #[test]
    fn failed_change_is_logged_and_shown() {
        let backend = FakeBackend {
            set_error: Some(DeviceError::new("Access is denied.")),
            ..FakeBackend::with_problem(problem::NONE)
        };
        let report = toggle(&backend, Some(ID));
        let notice = report.notice.unwrap();
        assert_eq!(notice.kind, NoticeKind::Error);
        assert_eq!(notice.title, "Failed to disable device");
        assert_eq!(notice.text, "Touch screen: Access is denied.");
        assert!(report.log.unwrap().contains("Access is denied."));
        // State is re-read, so the icon still shows the device as enabled.
        assert!(
            matches!(report.state, Some(DeviceState::Device(ref d)) if d.problem == problem::NONE)
        );
    }

    #[test]
    fn failed_lookup_is_logged_and_shown() {
        let backend = FakeBackend {
            find_error: Some(DeviceError::new("boom")),
            ..FakeBackend::with_problem(problem::NONE)
        };
        let report = toggle(&backend, Some(ID));
        assert_eq!(report.state, None);
        assert_eq!(report.notice.unwrap().kind, NoticeKind::Error);
        assert!(report.log.is_some());
        assert!(backend.calls.borrow().is_empty());
    }

    #[test]
    fn restart_required_is_reported() {
        let backend = FakeBackend {
            restart_required: true,
            ..FakeBackend::with_problem(problem::NONE)
        };
        let notice = toggle(&backend, Some(ID)).notice.unwrap();
        assert_eq!(notice.title, "Restart required");
        assert_eq!(
            notice.text,
            "Windows needs to restart to disable Touch screen."
        );
    }

    #[test]
    fn query_state_maps_lookup_results() {
        let backend = FakeBackend::with_problem(problem::DISABLED);
        assert_eq!(
            query_state(&backend, None).unwrap(),
            DeviceState::NotConfigured
        );
        assert!(matches!(
            query_state(&backend, Some(ID)).unwrap(),
            DeviceState::Device(_)
        ));
        assert_eq!(
            query_state(&backend, Some("nope")).unwrap(),
            DeviceState::NotFound
        );
    }
}
