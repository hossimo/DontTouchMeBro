//! Turning Windows' device names into something a person recognises.
//!
//! Windows names each function of a device after its type ("HID-compliant
//! consumer control device", "USB Input Device"), so a keyboard or mouse
//! shows up as several identical-looking rows. The product name is usually
//! available elsewhere: as the name the hardware reports over USB/Bluetooth,
//! or on a parent device node. [`product_name`] picks it out of the device's
//! ancestry, which [`crate::device`] collects.

/// Names of one device node, as used for picking a product name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NodeNames {
    /// Friendly name, or the driver's description if there isn't one.
    pub name: String,
    /// What the hardware calls itself (`DEVPKEY_Device_BusReportedDeviceDesc`),
    /// e.g. a USB product string. Often empty.
    pub bus_name: String,
}

/// Whether `name` only describes a kind of device rather than a product.
pub fn is_generic_name(name: &str) -> bool {
    let name = name.trim().to_lowercase();
    const PREFIXES: [&str; 12] = [
        "hid-compliant",
        "hid ",
        "usb input device",
        "usb composite device",
        "usb root hub",
        "generic ",
        "bluetooth le service",
        "bluetooth le device",
        "bluetooth low energy gatt compliant",
        "bluetooth peripheral device",
        "bluetooth hid device",
        "i2c hid device",
    ];
    name.is_empty()
        || PREFIXES.iter().any(|prefix| name.starts_with(prefix))
        || name.ends_with(" enumerator")
        || name.contains("host controller")
        || name == "usb device"
}

/// The best product name in a device's ancestry, or `None` if every name is
/// generic.
///
/// `chain` starts with the device itself, followed by its parent,
/// grandparent and so on, already cut off where it leaves the physical
/// device (see [`crate::device`]). A specific name on the device itself
/// always wins. Otherwise, when the device belongs to a device container
/// (one physical product), the topmost specific name in it is used, since
/// that's the product as a whole; without a container, the nearest one.
pub fn product_name(chain: &[NodeNames], has_container: bool) -> Option<String> {
    let first = chain.first()?;
    if !is_generic_name(&first.name) {
        return Some(first.name.trim().to_string());
    }

    let mut best = None;
    for node in chain {
        let specific = [&node.name, &node.bus_name]
            .into_iter()
            .find(|name| !is_generic_name(name));
        if let Some(name) = specific {
            best = Some(name.trim().to_string());
            if !has_container {
                break;
            }
        }
    }
    best
}

/// Short label for the Type column from Windows' device description:
/// "HID-compliant consumer control device" -> "Consumer control". A
/// description that's really a product name (some drivers name the device
/// itself) isn't a type at all, so that gives "—".
pub fn function_label(description: &str) -> String {
    let description = description.trim();
    if !is_generic_name(description) {
        return "—".to_string();
    }
    // Common generic descriptions that read better shorter.
    const SHORTER: [(&str, &str); 5] = [
        ("usb input device", "USB input"),
        (
            "bluetooth low energy gatt compliant hid device",
            "Bluetooth LE input",
        ),
        ("bluetooth hid device", "Bluetooth input"),
        ("i2c hid device", "I2C input"),
        ("hid keyboard device", "Keyboard"),
    ];
    let lower = description.to_lowercase();
    if let Some((_, short)) = SHORTER.iter().find(|(long, _)| lower == *long) {
        return short.to_string();
    }
    const PREFIX: &str = "hid-compliant ";
    if !lower.starts_with(PREFIX) {
        return description.to_string();
    }
    let core = &description[PREFIX.len()..];
    let core = core
        .strip_suffix(" device")
        .or_else(|| core.strip_suffix(" Device"))
        .unwrap_or(core);
    if core.eq_ignore_ascii_case("device") || core.is_empty() {
        return "HID device".to_string();
    }
    let mut chars = core.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(name: &str, bus_name: &str) -> NodeNames {
        NodeNames {
            name: name.into(),
            bus_name: bus_name.into(),
        }
    }

    #[test]
    fn generic_names() {
        for name in [
            "",
            "HID-compliant device",
            "HID-compliant consumer control device",
            "HID Keyboard Device",
            "USB Input Device",
            "USB Composite Device",
            "USB Root Hub (USB 3.0)",
            "Bluetooth LE Service {00001812-0000-1000-8000-00805f9b34fb}",
            "Bluetooth LE Device c87926de2ed7",
            "Bluetooth Low Energy GATT compliant HID device",
            "Microsoft Bluetooth Enumerator",
            "Intel(R) USB 3.20 eXtensible Host Controller - 1.20 (Microsoft)",
            "I2C HID Device",
        ] {
            assert!(is_generic_name(name), "{name}");
        }
        for name in [
            "Razer DeathAdder V2",
            "Keychron Q6 HE",
            "SpaceMouse Wireless BT",
            "3Dconnexion KMJ Emulator",
            "MYSTIC LIGHT",
            "USB Audio",
        ] {
            assert!(!is_generic_name(name), "{name}");
        }
    }

    // The chains below are real ones from a test machine.

    #[test]
    fn a_specific_name_on_the_device_wins() {
        let chain = [
            node("3Dconnexion SpaceMouse Wireless Bluetooth Edition", ""),
            node("SpaceMouse Wireless BT", "Bluetooth LE Device c87926de2ed7"),
        ];
        assert_eq!(
            product_name(&chain, true).as_deref(),
            Some("3Dconnexion SpaceMouse Wireless Bluetooth Edition")
        );
    }

    #[test]
    fn usb_product_string_from_the_parent() {
        // Keychron consumer control -> USB Input Device -> USB Composite Device.
        let chain = [
            node("HID-compliant consumer control device", ""),
            node("USB Input Device", "Keychron Q6 HE"),
            node("USB Composite Device", "Keychron Q6 HE"),
        ];
        assert_eq!(
            product_name(&chain, true).as_deref(),
            Some("Keychron Q6 HE")
        );
    }

    #[test]
    fn with_a_container_the_topmost_name_wins() {
        // Razer's driver gives a child node a bus name of "RzVirt_01"; the
        // product is the USB device at the top of the container.
        let chain = [
            node("HID-compliant consumer control device", "RzVirt_01"),
            node("HID-compliant consumer control device", "RzVirt_01"),
            node("Razer DeathAdder V2", "Razer DeathAdder V2"),
            node("USB Composite Device", "Razer DeathAdder V2"),
        ];
        assert_eq!(
            product_name(&chain, true).as_deref(),
            Some("Razer DeathAdder V2")
        );
    }

    #[test]
    fn without_a_container_the_nearest_name_wins() {
        // MYSTIC LIGHT (an internal USB device) has no container.
        let chain = [
            node("HID-compliant device", ""),
            node("USB Input Device", "MYSTIC LIGHT"),
        ];
        assert_eq!(product_name(&chain, false).as_deref(), Some("MYSTIC LIGHT"));

        let chain = [
            node("HID-compliant game controller", ""),
            node("3Dconnexion KMJ Emulator", ""),
        ];
        assert_eq!(
            product_name(&chain, false).as_deref(),
            Some("3Dconnexion KMJ Emulator")
        );
    }

    #[test]
    fn bluetooth_le_device_name_from_the_parent() {
        let chain = [
            node(
                "Bluetooth Low Energy GATT compliant HID device",
                "Bluetooth LE Service {00001812-0000-1000-8000-00805f9b34fb}",
            ),
            node("SpaceMouse Wireless BT", "Bluetooth LE Device c87926de2ed7"),
        ];
        assert_eq!(
            product_name(&chain, true).as_deref(),
            Some("SpaceMouse Wireless BT")
        );
    }

    #[test]
    fn all_generic_gives_none() {
        let chain = [
            node("HID-compliant touch screen", ""),
            node("I2C HID Device", ""),
        ];
        assert_eq!(product_name(&chain, false), None);
        assert_eq!(product_name(&[], true), None);
    }

    #[test]
    fn function_labels() {
        assert_eq!(
            function_label("HID-compliant consumer control device"),
            "Consumer control"
        );
        assert_eq!(function_label("HID-compliant touch screen"), "Touch screen");
        assert_eq!(
            function_label("HID-compliant vendor-defined device"),
            "Vendor-defined"
        );
        assert_eq!(function_label("HID-compliant mouse"), "Mouse");
        assert_eq!(function_label("HID-compliant device"), "HID device");
        assert_eq!(function_label("USB Input Device"), "USB input");
        assert_eq!(
            function_label("Bluetooth Low Energy GATT compliant HID device"),
            "Bluetooth LE input"
        );
        assert_eq!(function_label("HID Keyboard Device"), "Keyboard");
        // Some drivers describe the device by product name; that's no type.
        assert_eq!(
            function_label("3Dconnexion SpaceMouse Wireless Bluetooth Edition"),
            "—"
        );
        assert_eq!(function_label("Razer DeathAdder V2"), "—");
    }
}
