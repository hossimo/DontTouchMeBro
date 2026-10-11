//! Filtering and ordering the device list shown in the Configure window, and
//! where to put that window.

use crate::status::DeviceInfo;

/// Setup class shown by default; touch screens and pens live here.
pub const HID_CLASS: &str = "HIDClass";

/// Devices to show: HID devices only unless `show_all`, narrowed by a
/// case-insensitive `query` matched against every column. HID-only lists are
/// sorted by description; all-class lists by class, then description.
pub fn visible_devices<'a>(
    devices: &'a [DeviceInfo],
    show_all: bool,
    query: &str,
) -> Vec<&'a DeviceInfo> {
    let query = query.trim().to_lowercase();
    let mut visible: Vec<&DeviceInfo> = devices
        .iter()
        .filter(|d| show_all || d.class.eq_ignore_ascii_case(HID_CLASS))
        .filter(|d| query.is_empty() || matches(d, &query))
        .collect();

    visible.sort_by_cached_key(|d| {
        let class = if show_all {
            d.class.to_lowercase()
        } else {
            String::new()
        };
        (
            class,
            d.display_name().to_lowercase(),
            d.instance_id.to_lowercase(),
        )
    });
    visible
}

fn matches(device: &DeviceInfo, query: &str) -> bool {
    [
        &device.description,
        &device.manufacturer,
        &device.class,
        &device.instance_id,
    ]
    .iter()
    .any(|field| field.to_lowercase().contains(query))
}

/// Whether `a` and `b` name the same device (instance IDs are case-insensitive).
pub fn same_device(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

/// A rectangle in screen pixels (right/bottom exclusive), like Win32 `RECT`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// Top-left corner for a window of `size` opened from the tray: above and to
/// the left of the cursor, but kept inside `work_area` (the monitor minus the
/// taskbar) so it's never off-screen whichever edge the taskbar is on.
pub fn place_near(cursor: (i32, i32), size: (i32, i32), work_area: Rect) -> (i32, i32) {
    let (width, height) = size;
    // Keep the far edge on-screen first, then the near edge, so a window
    // bigger than the work area is pinned to its top-left.
    let x = (cursor.0 - width)
        .min(work_area.right - width)
        .max(work_area.left);
    let y = (cursor.1 - height)
        .min(work_area.bottom - height)
        .max(work_area.top);
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(class: &str, description: &str, id: &str) -> DeviceInfo {
        DeviceInfo {
            instance_id: id.into(),
            description: description.into(),
            manufacturer: "Contoso".into(),
            class: class.into(),
            problem: 0,
        }
    }

    fn sample() -> Vec<DeviceInfo> {
        vec![
            device("USB", "USB Root Hub", r"USB\ROOT_HUB30\1"),
            device(
                "HIDClass",
                "HID-compliant touch screen",
                r"HID\ELAN2D25&COL01\5&2B77",
            ),
            device(
                "HIDClass",
                "HID-compliant pen",
                r"HID\ELAN2D25&COL02\5&2B77",
            ),
            device("Display", "Intel Graphics", r"PCI\VEN_8086\3"),
            device("hidclass", "Another HID", r"HID\OTHER\1"),
        ]
    }

    fn names(list: &[&DeviceInfo]) -> Vec<String> {
        list.iter().map(|d| d.description.clone()).collect()
    }

    #[test]
    fn hid_only_by_default_sorted_by_description() {
        let devices = sample();
        assert_eq!(
            names(&visible_devices(&devices, false, "")),
            [
                "Another HID",
                "HID-compliant pen",
                "HID-compliant touch screen"
            ]
        );
    }

    #[test]
    fn show_all_sorts_by_class_then_description() {
        let devices = sample();
        assert_eq!(
            names(&visible_devices(&devices, true, "")),
            [
                "Intel Graphics",
                "Another HID",
                "HID-compliant pen",
                "HID-compliant touch screen",
                "USB Root Hub"
            ]
        );
    }

    #[test]
    fn query_matches_any_column_case_insensitively() {
        let devices = sample();
        assert_eq!(
            names(&visible_devices(&devices, false, "TOUCH")),
            ["HID-compliant touch screen"]
        );
        assert_eq!(
            names(&visible_devices(&devices, false, "col02")),
            ["HID-compliant pen"]
        );
        assert_eq!(
            names(&visible_devices(&devices, true, " usb ")),
            ["USB Root Hub"]
        );
        assert!(visible_devices(&devices, false, "usb").is_empty());
    }

    #[test]
    fn same_device_ignores_case_and_whitespace() {
        assert!(same_device(r"HID\ELAN\1", r" hid\elan\1 "));
        assert!(!same_device(r"HID\ELAN\1", r"HID\ELAN\2"));
    }

    const WORK: Rect = Rect {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1040,
    };

    #[test]
    fn opens_above_left_of_cursor_when_it_fits() {
        assert_eq!(place_near((1800, 1030), (700, 450), WORK), (1100, 580));
    }

    #[test]
    fn clamped_when_taskbar_is_top_left() {
        // Cursor near the top-left corner: the window would go off-screen.
        assert_eq!(place_near((50, 20), (700, 450), WORK), (0, 0));
    }

    #[test]
    fn clamped_to_second_monitor() {
        let second = Rect {
            left: 1920,
            top: 0,
            right: 3840,
            bottom: 1080,
        };
        assert_eq!(place_near((2000, 30), (700, 450), second), (1920, 0));
        assert_eq!(place_near((3839, 1079), (700, 450), second), (3139, 629));
    }

    #[test]
    fn window_larger_than_work_area_pins_to_top_left() {
        let small = Rect {
            left: 100,
            top: 100,
            right: 500,
            bottom: 400,
        };
        assert_eq!(place_near((450, 350), (700, 450), small), (100, 100));
    }
}
